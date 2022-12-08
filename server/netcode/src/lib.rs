pub mod channels;
pub mod login_listener;
pub mod message;
pub mod net_thread;
pub mod util;

use std::{net::SocketAddr, thread::JoinHandle};

use anyhow::anyhow;
use message::ServerMsg;
use net_thread::NetChannels;
use shared::net::NetworkId;
use tokio::sync::{
    mpsc::{channel, unbounded_channel, Receiver, UnboundedReceiver},
    oneshot,
};

// Other end to net::NetChannels
pub struct Channels {
    // Net -> Main
    pub incoming: UnboundedReceiver<(NetworkId, Box<[u8]>)>,
    pub server_messages: Receiver<ServerMsg>,

    // Main -> Net
    pub stop: Option<oneshot::Sender<()>>,
}

pub struct NetServer {
    handle: JoinHandle<()>,
    channels: Channels,
}

impl NetServer {
    #[inline(always)]
    pub fn open(&self) -> bool {
        !self.handle.is_finished()
    }

    pub fn poll(&mut self) -> Option<(NetworkId, Box<[u8]>)> {
        self.channels.incoming.try_recv().ok()
    }

    pub fn stop(&mut self) {
        if let Some(channel) = self.channels.stop.take() {
            _ = channel.send(());
        }
    }

    pub fn channels(&mut self) -> Option<&mut Channels> {
        if self.open() {
            Some(&mut self.channels)
        } else {
            None
        }
    }
}

impl NetServer {
    /// Sets up the server. Blocks until it is up and running, ready
    /// to receive connections.
    pub fn start(bind_address: SocketAddr) -> anyhow::Result<Self> {
        let (incoming_send, incoming_recv) = unbounded_channel();
        let (server_msg_send, server_msg_recv) = channel(32);
        let (stop_send, stop_recv) = oneshot::channel();

        let channels = Channels {
            incoming: incoming_recv,
            server_messages: server_msg_recv,
            stop: Some(stop_send),
        };

        let net_channels = NetChannels {
            incoming: incoming_send,
            server_messages: server_msg_send,
            stop: stop_recv,
        };

        let (on_ready_send, on_ready_recv) = oneshot::channel();

        let handle = std::thread::Builder::new()
            .name("Network Thread".to_owned())
            .spawn(move || net_thread::start(bind_address, net_channels, on_ready_send))
            .unwrap();

        on_ready_recv.blocking_recv()?.map_err(|e| anyhow!(e))?;

        Ok(NetServer { handle, channels })
    }
}
