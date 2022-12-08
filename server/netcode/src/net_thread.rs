use std::net::SocketAddr;

use shared::net::NetworkId;
use tokio::{sync::{oneshot, mpsc::{UnboundedSender, Sender}}};
use log::{error, debug};

use crate::{login_listener::poll_new_connections, message::ServerMsg};

// Other end to lib::Channels
pub struct NetChannels {
    // Net -> Main
    pub incoming: UnboundedSender<(NetworkId, Box<[u8]>)>,
    pub server_messages: Sender<ServerMsg>,

    // Main -> Net
    pub stop: oneshot::Receiver<()>,
}

#[tokio::main(flavor = "multi_thread", worker_threads = 2)]
async fn net_main(
    address: SocketAddr,
    channels: NetChannels,
    on_ready: oneshot::Sender<Result<(), Box<str>>>,
) {
    let incoming = match setup::make_server_endpoint(address) {
        Ok(incoming) => incoming,
        Err(e) => {
            error!("Failed to create server endpoint! Error: {e}");
            on_ready.send(Err(format!("Failed to create endpoint: {e}").into_boxed_str())).unwrap();
            return;
        }
    };

    on_ready.send(Ok(())).unwrap(); // unwrap(): crashing is probably not a terrible solution on failure

    poll_new_connections(incoming, channels).await;
    debug!("Network thread terminating...");
}

pub fn start(
    address: SocketAddr,
    channels: NetChannels,
    on_ready: oneshot::Sender<Result<(), Box<str>>>
) {
    net_main(address, channels, on_ready);
}

mod setup {
    use std::sync::Arc;

    use log::info;
    use quinn::{Endpoint, ServerConfig};

    use super::*;

    pub fn make_server_endpoint(bind_addr: SocketAddr) -> anyhow::Result<Endpoint> {
        let (server_config, _) = configure_server()?;
        let endpoint = Endpoint::server(server_config, bind_addr)?;

        info!(
            "Network thread listening for connections on {}",
            endpoint.local_addr()?
        );
        Ok(endpoint)
    }

    /// Returns default server configuration along with its certificate.
    #[allow(clippy::field_reassign_with_default)] // https://github.com/rust-lang/rust-clippy/issues/6527
    fn configure_server() -> anyhow::Result<(ServerConfig, Vec<u8>)> {
        let cert = rcgen::generate_simple_self_signed(vec!["localhost".into()]).unwrap();
        let cert_der = cert.serialize_der().unwrap();
        let priv_key = cert.serialize_private_key_der();
        let priv_key = rustls::PrivateKey(priv_key);
        let cert_chain = vec![rustls::Certificate(cert_der.clone())];

        let mut server_config = ServerConfig::with_single_cert(cert_chain, priv_key)?;
        Arc::get_mut(&mut server_config.transport)
            .unwrap()
            .keep_alive_interval(Some(std::time::Duration::from_millis(6000)));

        Ok((server_config, cert_der))
    }
}
