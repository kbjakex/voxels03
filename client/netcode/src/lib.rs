pub mod login;
pub mod message;
pub mod net_thread;
mod util;

use std::{net::SocketAddr, thread::JoinHandle};

use flexstr::SharedStr;
use login::LoginResponse;
use net_thread::NetChannels;
use tokio::sync::{
    mpsc::{channel, Receiver, Sender},
    oneshot,
};

// Other end to net::NetChannels
pub struct Channels {
    // Net -> Main
    incoming: Receiver<Box<[u8]>>,

    // Main -> Net
    pub chat: Sender<Box<[u8]>>,
    stop: Option<oneshot::Sender<()>>,
}

/// An established connection
pub struct ServerConnection {
    handle: JoinHandle<()>,
    channels: Channels,
}

impl ServerConnection {
    #[inline(always)]
    pub fn open(&self) -> bool {
        !self.handle.is_finished()
    }

    pub fn poll(&mut self) -> Option<Box<[u8]>> {
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

/// Represents an ongoing connection attempt.
pub struct Connecting {
    handle: Option<(JoinHandle<()>, Channels)>,
    on_connect: oneshot::Receiver<Result<LoginResponse, Box<str>>>,
}

impl Connecting {
    /// Polls the connection state.
    /// Returns:
    ///  Ok(None) if nothing has changed (connection still pending)
    ///  Ok(Some(..)) if the connection has been successfully established
    ///  Err(string) upon failure, with the string describing the failure.
    pub fn tick(&mut self) -> anyhow::Result<Option<(LoginResponse, ServerConnection)>, Box<str>> {
        match self.on_connect.try_recv() {
            Ok(Ok(response)) => {
                // unwrap(): safe. on_connect is oneshot, this can never be reached twice.
                let (handle, channels) = self.handle.take().unwrap();
                Ok(Some((
                    response,
                    ServerConnection {
                        handle,
                        channels,
                    },
                )))
            }
            Ok(Err(msg)) => Err(msg),
            Err(oneshot::error::TryRecvError::Empty) => Ok(None),
            Err(e) => Err(format!("Connection failed: {e}").into_boxed_str()),
        }
    }
}

pub fn try_connect(address: SocketAddr, username: SharedStr) -> Connecting {
    let (incoming_send, incoming_recv) = channel(128);
    let (chat_send, chat_recv) = channel(128);
    let (stop_send, stop_recv) = oneshot::channel();

    let channels = Channels {
        incoming: incoming_recv,

        chat: chat_send,
        stop: Some(stop_send),
    };

    let net_channels = NetChannels {
        incoming: incoming_send,
        
        chat: chat_recv,
        stop: stop_recv,
    };

    let (on_connect_send, on_connect_recv) = oneshot::channel();

    let handle = std::thread::Builder::new()
        .name("Network Thread".to_owned())
        .spawn(move || net_thread::start(address, username, net_channels, on_connect_send))
        .unwrap();

    Connecting {
        handle: Some((handle, channels)),
        on_connect: on_connect_recv,
    }
}
