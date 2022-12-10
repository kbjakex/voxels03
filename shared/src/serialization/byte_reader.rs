pub struct ByteReader<'a> {
    src: &'a [u8],
    pos: usize,
}

#[allow(unused)]
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
        #[cfg(not(debug_assertions))]
        dst.copy_from_slice(unsafe { self.src.get_unchecked(self.pos..self.pos + dst.len()) });
        #[cfg(debug_assertions)]
        dst.copy_from_slice(&self.src[self.pos..self.pos + dst.len()]);
        self.pos += dst.len();
    }

    pub fn read_varint15(&mut self) -> u16 {
        let b1 = self.read_u8();
        if (b1 & 128) != 0 {
            (b1 as u16 & 127) | ((self.read_u8() as u16) << 7)
        } else {
            b1 as u16
        }
    }

    pub fn read_u8(&mut self) -> u8 {
        let p = self.pos;
        self.pos += 1;
        debug_assert!(p < self.src.len());
        u8::from_le_bytes(unsafe { [*self.src.get_unchecked(p)] })
    }

    pub fn read_u16(&mut self) -> u16 {
        let p = self.pos;
        self.pos += 2;
        debug_assert!(p + 1 < self.src.len());
        u16::from_le_bytes(unsafe { [*self.src.get_unchecked(p), *self.src.get_unchecked(p + 1)] })
    }

    pub fn read_u32(&mut self) -> u32 {
        let p = self.pos;
        self.pos += 4;
        debug_assert!(p + 3 < self.src.len());
        u32::from_le_bytes(unsafe {
            [
                *self.src.get_unchecked(p),
                *self.src.get_unchecked(p + 1),
                *self.src.get_unchecked(p + 2),
                *self.src.get_unchecked(p + 3),
            ]
        })
    }

    pub fn read_u64(&mut self) -> u64 {
        let p = self.pos;
        self.pos += 8;
        debug_assert!(p + 7 < self.src.len());
        u64::from_le_bytes(unsafe {
            [
                *self.src.get_unchecked(p),
                *self.src.get_unchecked(p + 1),
                *self.src.get_unchecked(p + 2),
                *self.src.get_unchecked(p + 3),
                *self.src.get_unchecked(p + 4),
                *self.src.get_unchecked(p + 5),
                *self.src.get_unchecked(p + 6),
                *self.src.get_unchecked(p + 7),
            ]
        })
    }

    pub fn read_i8(&mut self) -> i8 {
        let p = self.pos;
        self.pos += 1;
        debug_assert!(p < self.src.len());
        i8::from_le_bytes(unsafe { [*self.src.get_unchecked(p)] })
    }

    pub fn read_i16(&mut self) -> i16 {
        let p = self.pos;
        self.pos += 2;
        debug_assert!(p + 1 < self.src.len());
        i16::from_le_bytes(unsafe { [*self.src.get_unchecked(p), *self.src.get_unchecked(p + 1)] })
    }

    pub fn read_i32(&mut self) -> i32 {
        let p = self.pos;
        self.pos += 4;
        debug_assert!(p + 3 < self.src.len());
        i32::from_le_bytes(unsafe {
            [
                *self.src.get_unchecked(p),
                *self.src.get_unchecked(p + 1),
                *self.src.get_unchecked(p + 2),
                *self.src.get_unchecked(p + 3),
            ]
        })
    }

    pub fn read_i64(&mut self) -> i64 {
        let p = self.pos;
        self.pos += 8;
        debug_assert!(p + 7 < self.src.len());
        i64::from_le_bytes(unsafe {
            [
                *self.src.get_unchecked(p),
                *self.src.get_unchecked(p + 1),
                *self.src.get_unchecked(p + 2),
                *self.src.get_unchecked(p + 3),
                *self.src.get_unchecked(p + 4),
                *self.src.get_unchecked(p + 5),
                *self.src.get_unchecked(p + 6),
                *self.src.get_unchecked(p + 7),
            ]
        })
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
        #[cfg(not(debug_assertions))]
        unsafe {
            std::str::from_utf8_unchecked(&self.src[pos..pos + len])
        }
        #[cfg(debug_assertions)]
        std::str::from_utf8(&self.src[pos..pos + len]).unwrap()
    }

    pub fn read_bool(&mut self) -> bool {
        self.read_u8() != 0
    }
}
