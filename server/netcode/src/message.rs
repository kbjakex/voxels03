use flexstr::SharedStr;
use shared::{
    serialization::{ByteReader, ByteWriter},
    net::NetworkId,
};
use tokio::sync::oneshot;

use crate::login_listener::LoginResponse;

pub enum InMsg<'a> {
    Chat(&'a str),
}

impl InMsg<'_> {
    const CHAT: u8 = 1;
}

impl<'a> InMsg<'a> {
    pub fn decode(stream: &'a [u8]) -> Self {
        let mut reader = ByteReader::new(stream);
        match stream[0] {
            Self::CHAT => Self::Chat(reader.read_str()),
            _ => unreachable!(),
        }
    }

    pub fn encode(&self, dst: &mut ByteWriter) {
        match self {
            InMsg::Chat(msg) => dst.write_u8(Self::CHAT).write_str(msg),
        };
    }
}

pub struct PlayerJoin {
    pub nid: NetworkId,
    pub username: SharedStr,
}

pub enum ServerMsg {
    LoginRequest {
        username: SharedStr,
        id_channel: oneshot::Sender<LoginResponse>,
    },
    PlayerJoined(PlayerJoin),
    PlayerLeft(NetworkId),
}
