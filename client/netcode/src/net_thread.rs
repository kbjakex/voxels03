use std::net::SocketAddr;

use flexstr::SharedStr;
use log::{error, debug};
use tokio::sync::{oneshot, mpsc::{Sender, Receiver}};

use crate::login::{LoginResponse, self};

// Other end to lib::Channels
pub struct NetChannels {
    // Net -> Main
    pub incoming: Sender<Box<[u8]>>,

    // Main -> Net
    pub chat: Receiver<Box<[u8]>>,
    pub stop: oneshot::Receiver<()> // command to terminate network thread
}

#[tokio::main(flavor = "current_thread")]
async fn net_main(
    server_address: SocketAddr,
    username: SharedStr,
    channels: NetChannels,
    on_connect: oneshot::Sender<Result<LoginResponse, Box<str>>>,
) -> anyhow::Result<()> {
    let (endpoint, mut _connection, response) = match login::try_connect(server_address, &username).await {
        Ok(tuple) => tuple,
        Err(e) => {
            let _ = on_connect.send(Err(format!("Connection failed: {e}").into_boxed_str()));
            return Ok(());
        }
    };

    if on_connect.send(Ok(response)).is_err() {
        debug!("Main thread dropped on_connect channel");
        return Ok(());
    }
    
    let disconnect = channels.stop;
    tokio::select!(
        _ = disconnect => {}
    );

    debug!("Stopping network thread");
    endpoint.close(quinn::VarInt::from_u32(1), &[]); // Notify server
    endpoint.wait_idle().await; // Wait for clean shutdown
    debug!("Network thread stopped");
    Ok(())
}

pub fn start(
    server_address: SocketAddr,
    username: SharedStr,
    channels: NetChannels,
    on_connect: oneshot::Sender<Result<LoginResponse, Box<str>>>,
) {
    if let Err(e) = net_main(server_address, username, channels, on_connect) {
        error!("Error in network thread: {}", e);
    }
}
