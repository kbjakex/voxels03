pub struct BitReader<'a> {
    current: u64,
    bits_left: u32,
    buf_pos: usize,
    buf: &'a [u8],
}

impl<'a> BitReader<'a> {
    #[inline]
    pub fn new(buf: &'a [u8]) -> Self {
        let mut ret = Self {
            buf,
            bits_left: 64,
            buf_pos: 0,
            current: 0,
        };

        ret.current = (ret.read() as u64) | ((ret.read() as u64) << 32);
        ret
    }

    // Reads the next chunk of 32 bits from the buffer, or zero bits
    // if past the end
    #[inline]
    fn read(&mut self) -> u32 {
        let left = 4.min(self.buf.len() - self.buf_pos);
        let mut bytes = [0u8; 4];
        bytes[..left].copy_from_slice(&self.buf[self.buf_pos..self.buf_pos + left]);
        self.buf_pos += left;
        return u32::from_le_bytes(bytes);
    }

    #[inline]
    pub fn uint(&mut self, num_bits: u32) -> u32 {
        debug_assert!(num_bits <= 32);

        let result = self.current & !(!0 << num_bits);

        self.bits_left -= num_bits;
        self.current >>= num_bits;

        if self.bits_left < 32 {
            self.current |= (self.read() as u64) << self.bits_left;
            self.bits_left += 32;
        }

        result as u32
    }

    #[inline]
    pub fn int(&mut self, num_bits: u32) -> i32 {
        let u = self.uint(num_bits);
        (u as i64 - (1 << (num_bits - 1))) as i32
    }

    #[inline]
    pub fn bool(&mut self) -> bool {
        self.uint(1) != 0
    }
}
