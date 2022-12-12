use quinn::RecvStream;
use shared::serialization::ByteReader;

pub async fn receive_bytes<'a>(stream: &mut RecvStream, buf: &'a mut Vec<u8>) -> anyhow::Result<ByteReader<'a>> {
    let mut header = [0u8; 2];
    stream.read_exact(&mut header).await?;

    let length = ByteReader::new(&header).read_u16();

    buf.resize(length as usize, 0);
    stream.read_exact(buf).await?;

    Ok(ByteReader::new(buf))
}