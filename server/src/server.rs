use glam::{Vec3, Vec2};
use log::{error, info};
use netcode::{message::{InMsg, ServerMsg}, NetServer, login_listener::LoginResponse};
use shared::net::NetworkId;

pub struct State {
    pub current_tick: u32,
    pub net_server: NetServer,
}

pub struct Server {
    pub state: State,
}

impl Server {}

// Tick logic
impl Server {
    pub fn tick(self: &mut Server) -> anyhow::Result<()> {
        if let Err(e) = self.process_net_messages() {
            error!("Error while processing incoming network data: {e}");
        }

        self.state.current_tick += 1;
        Ok(())
    }

    fn process_net_messages(&mut self) -> anyhow::Result<()> {
        let Some(channels) = self.state.net_server.channels() else {
            return Ok(());
        };

        while let Ok(msg) = channels.server_messages.try_recv() {
            match msg {
                ServerMsg::LoginRequest { username: _, id_channel } => {
                    _ = id_channel.send(LoginResponse::Accepted {
                        nid: NetworkId::from_raw(0),
                        position: Vec3::ZERO,
                        head_rotation: Vec2::ZERO,
                        world_seed: 0,
                    });
                },
                ServerMsg::PlayerJoined(info) => {
                    info!("Player {} joined! ({})", info.username, info.nid);
                },
                ServerMsg::PlayerLeft(nid) => {
                    info!("Player {} left", nid);
                },
            }
        }

        while let Some((nid, bytes)) = self.state.net_server.poll() {
            match InMsg::decode(&bytes) {
                InMsg::Chat(msg) => info!("Received chat message '{msg}' from {nid}"),
            }
        }
        Ok(())
    }
}

impl Server {
    pub fn start() -> anyhow::Result<Self> {
        let state = State {
            current_tick: 0,
            net_server: NetServer::start("0.0.0.0:29477".parse().unwrap())?,
        };

        let server = Server { state };

        Ok(server)
    }

    pub fn shutdown(self: Server) -> anyhow::Result<()> {
        Ok(())
    }
}
