pub struct BitWriter<'a> {
    current: u64,
    bit_pos: u32,
    buf: &'a mut [u8],
    bits_written: usize,
}

// Writing
impl<'a> BitWriter<'a> {
    #[inline]
    pub fn new(buf: &'a mut [u8]) -> Self {
        debug_assert!(
            buf.len() % 4 == 0,
            "Buffer length must be a multiple of 4 to avoid surprises"
        );
        Self {
            buf,
            bit_pos: 0,
            current: 0,
            bits_written: 0,
        }
    }

    #[inline]
    fn write(&mut self, val: u32) {
        if self.buf.len() >= 4 {
            let (dst, rest) = std::mem::take(&mut self.buf).split_at_mut(4);
            dst.copy_from_slice(&val.to_le_bytes());
            self.buf = rest;
            self.bits_written += 32;
        } else {
            debug_assert!(false, "BitWriter: write() out of bounds");
        }
    }

    #[inline]
    pub fn uint(&mut self, value: u32, num_bits: u32) -> u32 {
        debug_assert!(num_bits <= 32);
        //debug_assert_eq!(0, value as u64 >> num_bits);

        self.current |= (value as u64) << self.bit_pos;
        self.bit_pos += num_bits;

        if self.bit_pos >= 32 {
            self.write(self.current as u32);
            self.current >>= 32;
            self.bit_pos -= 32;
        }

        value
    }

    #[inline]
    pub fn int(&mut self, value: i32, num_bits: u32) -> i32 {
        self.uint(
            ((value as u32).wrapping_add(1 << (num_bits - 1))) & !(!0 << num_bits),
            num_bits,
        );
        value
    }

    #[inline]
    pub fn bool(&mut self, b: bool) -> bool {
        self.uint(b as u32, 1);
        b
    }

    #[inline]
    pub fn flush_partials(&mut self) {
        if self.bit_pos == 0 {
            return;
        }
        self.write(self.current as u32 & !(0xFFFF_FFFF << self.bit_pos));
        self.bits_written -= 32; // write() assumes all 32 bits are used
        self.bits_written += self.bit_pos as usize;
    }

    #[inline]
    pub fn bits_written(&self) -> usize {
        self.bits_written
    }

    #[inline]
    pub fn compute_bytes_written(&self) -> usize {
        (self.bits_written + 7) / 8
    }
}
