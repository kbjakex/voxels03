use std::net::SocketAddr;

use flexstr::SharedStr;
use glam::{Vec3, Vec2};
use log::info;
use quinn::{Endpoint, Connection};
use shared::{net::NetworkId, bits_and_bytes::ByteWriter};

use crate::util::receive_bytes;

/*
 * This file contains all the code for connecting to the server and receiving
 * the login response. Nothing less and nothing more. Data beyond the login
 * response will be received through the QUIC channels as usual, outside of here.
*/

#[derive(Debug)]
pub struct LoginResponse {
    pub nid: NetworkId,
    pub position: Vec3,
    pub head_rotation: Vec2,
    pub world_seed: u64
}

pub async fn try_connect(
    server_address: SocketAddr,
    username: &SharedStr,
) -> anyhow::Result<(Endpoint, Connection, LoginResponse)> {
    let endpoint = setup::make_client_endpoint().unwrap();

    info!("Connecting to {}...", server_address);
    let conn = endpoint.connect(server_address, "localhost")?.await?;

    let mut buf = [0u8; 256];
    let mut writer = ByteWriter::new_for_message(&mut buf);
    writer.write_u16(shared::net::PROTOCOL_MAGIC);
    writer.write_u16(shared::net::PROTOCOL_VERSION);
    writer.write_str(username.as_str());
    writer.write_message_len();

    let (mut hello_send, mut hello_recv) = conn.open_bi().await?;
    hello_send.write_all(writer.bytes()).await?;

    let mut recv_buf = Vec::new();
    let mut reader = receive_bytes(&mut hello_recv, &mut recv_buf).await?;
    if reader.bytes_remaining() < 30 {
        anyhow::bail!("Invalid login response from server, got only {} bytes", reader.bytes_remaining());
    }

    let response = LoginResponse {
        nid: NetworkId::from_raw(reader.read_u16()),
        position: Vec3 {
            x: reader.read_f32(),
            y: reader.read_f32(),
            z: reader.read_f32(),
        },
        head_rotation: Vec2 {
            x: reader.read_f32(), // Yaw
            y: reader.read_f32(), // Pitch
        },
        world_seed: reader.read_u64(),
    };

    Ok((endpoint, conn, response))
}

mod setup {
    use std::{error::Error, sync::Arc};

    use quinn::{ClientConfig, Endpoint};

    pub(super) fn make_client_endpoint() -> Result<Endpoint, Box<dyn Error>> {
        let mut endpoint = Endpoint::client("0.0.0.0:0".parse()?)?;
        let crypto = rustls::ClientConfig::builder()
            .with_safe_defaults()
            .with_custom_certificate_verifier(Arc::new(SkipServerVerification))
            .with_no_client_auth();
        endpoint.set_default_client_config(ClientConfig::new(std::sync::Arc::new(crypto)));
        Ok(endpoint)
    }

    struct SkipServerVerification;

    impl rustls::client::ServerCertVerifier for SkipServerVerification {
        fn verify_server_cert(
            &self,
            _end_entity: &rustls::Certificate,
            _intermediates: &[rustls::Certificate],
            _server_name: &rustls::ServerName,
            _scts: &mut dyn Iterator<Item = &[u8]>,
            _ocsp_response: &[u8],
            _now: std::time::SystemTime,
        ) -> Result<rustls::client::ServerCertVerified, rustls::Error> {
            Ok(rustls::client::ServerCertVerified::assertion())
        }
    }

}