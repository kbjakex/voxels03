use log::debug;

pub struct ByteWriter<'a> {
    dst: &'a mut [u8],
    pos: usize,
}

impl<'a> ByteWriter<'a> {
    pub fn new(dst: &'a mut [u8]) -> Self {
        Self { dst, pos: 0 }
    }

    pub fn new_for_message(dst: &'a mut [u8]) -> Self {
        Self { dst, pos: 2 }
    }

    pub fn bytes_written(&self) -> usize {
        self.pos as _
    }

    pub fn space_remaining(&self) -> usize {
        self.dst.len() - self.pos
    }

    pub fn skip(&mut self, count: usize) -> &mut Self {
        self.pos += count;
        
        self
    }

    pub fn write(&mut self, src: &[u8]) -> &mut Self {
        self.dst[self.pos..self.pos + src.len()].copy_from_slice(src);
        self.pos += src.len();
        
        self
    }

    pub fn write_message_len(&mut self) -> &mut Self {
        // Probably could be replaced with a scoped-macro guard thing
        let len = (self.bytes_written() as u16).saturating_sub(2);
        self.dst[..2].copy_from_slice(&u16::to_le_bytes(len));
        
        self
    }

    pub fn write_u8(&mut self, x: u8) -> &mut Self {
        self.dst[self.pos] = x;
        self.pos += 1;

        self
    }

    pub fn write_u16(&mut self, x: u16) -> &mut Self {
        let bytes = u16::to_le_bytes(x);
        self.dst[self.pos..self.pos + 2].copy_from_slice(&bytes);
        self.pos += bytes.len();

        self
    }

    pub fn write_u32(&mut self, x: u32) -> &mut Self {
        let bytes = u32::to_le_bytes(x);
        self.dst[self.pos..self.pos + 4].copy_from_slice(&bytes);
        self.pos += bytes.len();

        self
    }

    pub fn write_u64(&mut self, x: u64) -> &mut Self {
        let bytes = u64::to_le_bytes(x);
        self.dst[self.pos..self.pos + 8].copy_from_slice(&bytes);
        self.pos += bytes.len();

        self
    }

    pub fn write_i8(&mut self, x: i8) -> &mut Self {
        self.write_u8(x as _)
    }

    pub fn write_i16(&mut self, x: i16) -> &mut Self {
        self.write_u16(x as _)
    }

    pub fn write_i32(&mut self, x: i32) -> &mut Self {
        self.write_u32(x as _)
    }

    pub fn write_i64(&mut self, x: i64) -> &mut Self {
        self.write_u64(x as _)
    }

    pub fn write_f32(&mut self, x: f32) -> &mut Self {
        self.write_u32(f32::to_bits(x))
    }

    pub fn write_f64(&mut self, x: f64) -> &mut Self {
        self.write_u64(f64::to_bits(x))
    }

    pub fn write_str(&mut self, x: &str) -> &mut Self {
        assert!(
            x.len() <= u16::MAX as usize,
            "ByteWriter::write_str: string too long"
        );
        self.write_u16(x.len() as u16);
        self.write(x.as_bytes())
    }

    pub fn write_bool(&mut self, x: bool) -> &mut Self {
        self.write_u8(x as u8)
    }

    pub fn bytes(&self) -> &[u8] {
        &self.dst[..self.pos]
    }

    pub fn into_bytes(self) -> &'a [u8] {
        &self.dst[..self.pos]
    }
}
