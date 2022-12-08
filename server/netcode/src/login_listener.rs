
/*
 * This file contains all the code to listen to incoming connections,
 * respond to the login request and set up the connection.
 */

use flexstr::{ToSharedStr, SharedStr};
use glam::{Vec3, Vec2};
use log::{warn, debug, info};
use quinn::{Endpoint, Connection};
use shared::{net::{PROTOCOL_MAGIC, PROTOCOL_VERSION, NetworkId}, bits_and_bytes::ByteWriter};
use tokio::{task, sync::{oneshot, mpsc::{Sender, unbounded_channel, UnboundedSender}}};

use crate::{util::receive_bytes, net_thread::NetChannels, message::{ServerMsg, PlayerJoin}, channels};

pub async fn poll_new_connections(
    incoming: Endpoint,
    channels: NetChannels
) {
    info!("Now polling for connections!");
    while let Some(connecting) = incoming.accept().await {
        debug!("Received connection attempt, resolving...");
        let new_conn = match connecting.await {
            Ok(conn) => conn,
            Err(e) => {
                warn!("Connection failed: {}", e);
                continue;
            }
        };

        debug!("Connection established!");
        let channels = clone_per_client_channels(&channels);
        task::spawn(async move {
            if let Err(e) = login(new_conn, channels).await {
                warn!("Login attempt failed: {e}");
            }
        });
    }
}

fn clone_per_client_channels(all: &NetChannels) -> PerClientChannels {
    PerClientChannels {
        incoming: all.incoming.clone(),
        server_messages: all.server_messages.clone(),
    }
}

struct PerClientChannels {
    incoming: UnboundedSender<(NetworkId, Box<[u8]>)>,
    server_messages: Sender<ServerMsg>,
}

pub enum LoginResponse {
    Accepted {
        nid: NetworkId,
        position: Vec3,
        head_rotation: Vec2,
        world_seed: u64,
    },
    Denied {
        reason: Box<str>
    }
}

async fn login(connection: Connection, channels: PerClientChannels) -> anyhow::Result<()> {
    debug!("Trying to accept uni stream...");

    let (mut hello_send, mut hello_recv) = connection.accept_bi().await?;

    let mut buffer = vec![0; 32];
    let mut reader = receive_bytes(&mut hello_recv, &mut buffer).await?;
    debug!("Received login message! Length: {}", reader.bytes_remaining());
    
    if reader.bytes_remaining() < 6 // magic + protocol ver + username length + username >= 6
        || reader.read_u16() != PROTOCOL_MAGIC 
        || reader.read_u16() != PROTOCOL_VERSION 
    { 
        connection.close(quinn::VarInt::from_u32(1), b"Invalid login request");
        anyhow::bail!("Invalid login request");
    }
    
    let username = reader.read_str().to_shared_str();
    if username.len() < 3 {
        connection.close(quinn::VarInt::from_u32(2), b"Username too short");
        anyhow::bail!("Username too short");
    }

    debug!("Username: {username}. Generating network ID...");

    let (id_send, id_recv) = oneshot::channel();
    _ = channels.server_messages.send(ServerMsg::LoginRequest { username: username.clone(), id_channel: id_send }).await;
        
    let login_response = id_recv.await?;
    let nid = match login_response {
        LoginResponse::Accepted{ nid, position, head_rotation, world_seed } => {
            buffer.resize(32, 0);
            let mut writer = ByteWriter::new_for_message(&mut buffer);
            let payload = writer
                .write_u16(nid.raw())
                .write_f32(position.x)
                .write_f32(position.y)
                .write_f32(position.z)
                .write_f32(head_rotation.x)
                .write_f32(head_rotation.y)
                .write_u64(world_seed)
                .write_message_len()
                .bytes();

            hello_send.write_all(payload).await?;
            nid
        },
        LoginResponse::Denied{ reason } => {
            connection.close(quinn::VarInt::from_u32(2), reason.as_bytes());
            anyhow::bail!("Invalid login request");
        },
    };
    hello_send.finish().await?;

    if let Err(e) = client_connection(connection, username, nid, channels).await {
        warn!("Error in client connection: {e}");
    }

    Ok(())
}

async fn client_connection(
    connection: Connection,
    username: SharedStr,
    network_id: NetworkId,
    channels: PerClientChannels
) -> anyhow::Result<()> {
    /* let (chat_send_main, chat_recv_self) = unbounded_channel(); // c -> s
    let (entity_state_send, entity_state_recv) = unbounded_channel(); // s -> c

    let (chat_recv_driver, chat_send_driver) = {
        let (outgoing, mut incoming) = connection.accept_bi().await?;

        // Read the byte that was used to open the channel
        incoming.read_exact(&mut [0u8]).await?;

        let chat_recv_driver = task::spawn(channels::chat::recv_driver(
            incoming,
            username.clone(),
            network_id,
            channels.incoming,
        ));
        let chat_send_driver = task::spawn(channels::chat::send_driver(
            outgoing,
            chat_recv_self,
        ));

        (chat_recv_driver, chat_send_driver)
    }; */

    // Keep at the end so that Disconnect is definitely sent (no more early exits).
    // Disconnect must be sent to avoid leaking network ids
    _ = channels.server_messages
        .send(ServerMsg::PlayerJoined(PlayerJoin {
            username: username.clone(),
            nid: network_id,
        }))
        .await;

    /* tokio::select!(
        biased;
    ); */

    _ = channels.server_messages
        .send(ServerMsg::PlayerLeft(network_id))
        .await;

    debug!("Client with username \"{username}\" disconnected");
    Ok(())
}
