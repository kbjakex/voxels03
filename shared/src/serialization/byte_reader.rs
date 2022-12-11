pub struct ByteReader<'a> {
    src: &'a [u8],
    pos: usize,
}

impl<'a> ByteReader<'a> {
    pub fn new(src: &'a [u8]) -> Self {
        Self { src, pos: 0 }
    }

    pub fn bytes(&self) -> &[u8] {
        &self.src[self.pos..]
    }

    pub fn mark_start(&mut self) {
        self.src = &self.src[self.pos..];
        self.reset();
    }

    pub fn reset(&mut self) {
        self.pos = 0;
    }

    pub fn total_src_size(&self) -> usize {
        self.src.len()
    }

    pub fn bytes_remaining(&self) -> usize {
        self.src.len() - self.pos
    }

    pub fn bytes_read(&self) -> usize {
        self.pos
    }

    pub fn has_n_more(&self, n: usize) -> bool {
        self.bytes_remaining() >= n
    }

    pub fn skip(&mut self, n: usize) {
        self.pos += n;
    }

    pub fn back(&mut self, n: usize) {
        self.pos -= n;
    }

    pub fn read(&mut self, dst: &mut [u8]) {
        dst.copy_from_slice(&self.src[self.pos..self.pos + dst.len()]);
        self.pos += dst.len();
    }

    pub fn read_u8(&mut self) -> u8 {
        let p = self.pos;
        self.pos += 1;

        // Is this assert needed?
        assert!(
            self.pos < self.src.len(),
            "ByteReader::read_u8: not enough bytes"
        );
        self.src[p]
    }

    pub fn read_u16(&mut self) -> u16 {
        let p = self.pos;
        self.pos += 2;

        assert!(
            self.pos < self.src.len(),
            "ByteReader::read_u16: not enough bytes"
        );
        u16::from_le_bytes(self.src[p..].try_into().unwrap()) // i hate this
    }

    pub fn read_u32(&mut self) -> u32 {
        let p = self.pos;
        self.pos += 4;

        assert!(
            self.pos < self.src.len(),
            "ByteReader::read_u32: not enough bytes"
        );
        u32::from_le_bytes(self.src[p..].try_into().unwrap())
    }

    pub fn read_u64(&mut self) -> u64 {
        let p = self.pos;
        self.pos += 8;

        assert!(
            self.pos < self.src.len(),
            "ByteReader::read_u64: not enough bytes"
        );
        u64::from_le_bytes(self.src[p..].try_into().unwrap())
    }

    pub fn read_i8(&mut self) -> i8 {
        self.read_u8() as i8
    }

    pub fn read_i16(&mut self) -> i16 {
        self.read_u16() as i16
    }

    pub fn read_i32(&mut self) -> i32 {
        self.read_u32() as i32
    }

    pub fn read_i64(&mut self) -> i64 {
        self.read_u64() as i64
    }

    pub fn read_f32(&mut self) -> f32 {
        f32::from_bits(self.read_u32())
    }

    pub fn read_f64(&mut self) -> f64 {
        f64::from_bits(self.read_u64())
    }

    pub fn read_str(&mut self) -> &'a str {
        let len = self.read_u16() as usize;

        let pos = self.pos;
        self.pos += len;

        std::str::from_utf8(&self.src[pos..self.pos]).unwrap()
    }

    pub fn read_bool(&mut self) -> bool {
        self.read_u8() != 0
    }
}
