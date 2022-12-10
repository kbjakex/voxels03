use flexstr::SharedStr;
use quinn::{RecvStream, SendStream};
use shared::{serialization::ByteWriter, net::NetworkId};
use tokio::sync::mpsc::{UnboundedSender, UnboundedReceiver};

use crate::util::receive_bytes;

pub(super) mod chat {
    use super::*;

    pub async fn recv_driver(
        mut incoming: RecvStream,
        username: SharedStr,
        id: NetworkId,
        to_server: UnboundedSender<(NetworkId, Box<[u8]>)>,
    ) -> anyhow::Result<()> {
        let mut buf = Vec::new();
        loop {
            let mut stream = receive_bytes(&mut incoming, &mut buf).await?;
            
            let message = format!("{username}: {}", stream.read_str());
            _ = to_server.send((id, message.into_bytes().into_boxed_slice()));
        }
    }

    pub async fn send_driver(
        mut outgoing: SendStream,
        mut messages: UnboundedReceiver<Box<[u8]>>,
    ) -> anyhow::Result<()> {
        let mut buf = [0u8; 512];
        while let Some(message) = messages.recv().await {
            debug_assert!(message.len() < buf.len(), "Chat message too long! ({}/{} bytes)", message.len(), buf.len());

            let mut writer = ByteWriter::new_for_message(&mut buf);
            writer.write(&message)
                .write_message_len();

            outgoing.write_all(&writer.bytes()).await?;
        }
        Ok(())
    }
}