use quinn::RecvStream;
use shared::serialization::ByteReader;

// This exact same code exists separately in client and server now, but adding it to `shared` would
// require adding at least Quinn as a dependency, and it is huge... hmm.

pub async fn receive_bytes<'a>(stream: &mut RecvStream, buf: &'a mut Vec<u8>) -> anyhow::Result<ByteReader<'a>> {
    let mut header = [0u8; 2];
    stream.read_exact(&mut header).await?;

    let length = ByteReader::new(&header).read_u16();

    buf.resize(length as usize, 0);
    stream.read_exact(buf).await?;

    Ok(ByteReader::new(buf))
}
/*  let mut header = [0u8; 2];
    stream.read_exact(&mut header[0..2]).await?;

    let mut length = header[0] as usize;    
    if length > 127 {
        length = length - 128 + ((header[1] as usize) << 7);
    }
    
    buf.resize(length, 0);
    let slice = if length > 127 {
        &mut buf[..length]
    } else {
        buf[0] = header[1];
        &mut buf[1..length]
    };

    stream.read_exact(slice).await?;
    Ok(ByteReader::new(&mut buf[..]))
} */