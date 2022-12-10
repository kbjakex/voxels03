pub struct ByteWriter<'a> {
    dst: &'a mut [u8],
    pos: u32,
}

#[allow(unused)]
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
        self.dst.len() - self.pos as usize
    }

    pub fn skip(&mut self, count: usize) -> &mut Self {
        self.pos += count as u32;
        self
    }

    pub fn write(&mut self, src: &[u8]) -> &mut Self {
        debug_assert!(src.len() <= self.dst.len() - self.pos as usize);

        unsafe {
            self.dst
                .get_unchecked_mut(self.pos as usize..self.pos as usize + src.len())
        }
        .copy_from_slice(src);
        self.pos += src.len() as u32;
        self
    }

    pub fn write_message_len(&mut self) -> &mut Self {
        let len = (self.bytes_written() as u16).saturating_sub(2);
        let off = ByteWriter::new(self.dst).write_varint15_r(len);
        let (_, new) = std::mem::take(&mut self.dst).split_at_mut(off);
        self.dst = new;
        self.pos -= off as u32;
        self
    }

    // write right-aligned 15-bit varint
    pub fn write_varint15_r(&mut self, mut x: u16) -> usize {
        debug_assert!(
            x < 32768,
            "value {x} too large, varint15 needs a control bit"
        );

        if x > 127 {
            x = (x & 127) | ((x & !127) << 1) | 128;
            self.write_u16(x);
            0
        } else {
            self.write_u8(7); // skip one
            self.write_u8(x as u8);
            1
        }
    }

    pub fn write_varint15(&mut self, mut x: u16) -> &mut Self {
        debug_assert!(
            x < 32768,
            "value {x} too large, varint15 needs a control bit"
        );

        if x < 128 {
            self.write_u8(x as u8 & 127);
        } else {
            self.write_u16((x & 127) | ((x & !127) << 1) | 128);
        }
        self
    }

    pub fn write_u8(&mut self, x: u8) -> &mut Self {
        debug_assert!(self.dst.len() - self.pos as usize >= 1);

        let bytes = u8::to_le_bytes(x);
        let p = self.pos as usize;
        self.pos += 1;
        unsafe {
            *self.dst.get_unchecked_mut(p) = *bytes.get_unchecked(0);
        }
        self
    }

    pub fn write_u16(&mut self, x: u16) -> &mut Self {
        debug_assert!(self.dst.len() - self.pos as usize >= 2);

        let bytes = u16::to_le_bytes(x);
        let p = self.pos as usize;
        self.pos += 2;
        unsafe {
            *self.dst.get_unchecked_mut(p) = *bytes.get_unchecked(0);
            *self.dst.get_unchecked_mut(p + 1) = *bytes.get_unchecked(1);
        }
        self
    }

    pub fn write_u32(&mut self, x: u32) -> &mut Self {
        debug_assert!(self.dst.len() - self.pos as usize >= 4);

        let bytes = u32::to_le_bytes(x);
        let p = self.pos as usize;
        self.pos += 4;
        unsafe {
            *self.dst.get_unchecked_mut(p) = *bytes.get_unchecked(0);
            *self.dst.get_unchecked_mut(p + 1) = *bytes.get_unchecked(1);
            *self.dst.get_unchecked_mut(p + 2) = *bytes.get_unchecked(2);
            *self.dst.get_unchecked_mut(p + 3) = *bytes.get_unchecked(3);
        }
        self
    }

    pub fn write_u64(&mut self, x: u64) -> &mut Self {
        debug_assert!(self.dst.len() - self.pos as usize >= 8);

        let bytes = u64::to_le_bytes(x);
        let p = self.pos as usize;
        self.pos += 8;
        unsafe {
            *self.dst.get_unchecked_mut(p) = *bytes.get_unchecked(0);
            *self.dst.get_unchecked_mut(p + 1) = *bytes.get_unchecked(1);
            *self.dst.get_unchecked_mut(p + 2) = *bytes.get_unchecked(2);
            *self.dst.get_unchecked_mut(p + 3) = *bytes.get_unchecked(3);
            *self.dst.get_unchecked_mut(p + 4) = *bytes.get_unchecked(4);
            *self.dst.get_unchecked_mut(p + 5) = *bytes.get_unchecked(5);
            *self.dst.get_unchecked_mut(p + 6) = *bytes.get_unchecked(6);
            *self.dst.get_unchecked_mut(p + 7) = *bytes.get_unchecked(7);
        }
        self
    }

    pub fn write_i8(&mut self, x: i8) -> &mut Self {
        debug_assert!(self.dst.len() - self.pos as usize >= 1);

        let bytes = i8::to_le_bytes(x);
        let p = self.pos as usize;
        self.pos += 1;
        unsafe {
            *self.dst.get_unchecked_mut(p) = *bytes.get_unchecked(0);
        }
        self
    }

    pub fn write_i16(&mut self, x: i16) -> &mut Self {
        debug_assert!(self.dst.len() - self.pos as usize >= 2);

        let bytes = i16::to_le_bytes(x);
        let p = self.pos as usize;
        self.pos += 2;
        unsafe {
            *self.dst.get_unchecked_mut(p) = *bytes.get_unchecked(0);
            *self.dst.get_unchecked_mut(p + 1) = *bytes.get_unchecked(1);
        }
        self
    }

    pub fn write_i32(&mut self, x: i32) -> &mut Self {
        debug_assert!(self.dst.len() - self.pos as usize >= 4);

        let bytes = i32::to_le_bytes(x);
        let p = self.pos as usize;
        self.pos += 4;
        unsafe {
            *self.dst.get_unchecked_mut(p) = *bytes.get_unchecked(0);
            *self.dst.get_unchecked_mut(p + 1) = *bytes.get_unchecked(1);
            *self.dst.get_unchecked_mut(p + 2) = *bytes.get_unchecked(2);
            *self.dst.get_unchecked_mut(p + 3) = *bytes.get_unchecked(3);
        }
        self
    }

    pub fn write_i64(&mut self, x: i64) -> &mut Self {
        debug_assert!(self.dst.len() - self.pos as usize >= 8);

        let bytes = i64::to_le_bytes(x);
        let p = self.pos as usize;
        self.pos += 8;
        unsafe {
            *self.dst.get_unchecked_mut(p) = *bytes.get_unchecked(0);
            *self.dst.get_unchecked_mut(p + 1) = *bytes.get_unchecked(1);
            *self.dst.get_unchecked_mut(p + 2) = *bytes.get_unchecked(2);
            *self.dst.get_unchecked_mut(p + 3) = *bytes.get_unchecked(3);
            *self.dst.get_unchecked_mut(p + 4) = *bytes.get_unchecked(4);
            *self.dst.get_unchecked_mut(p + 5) = *bytes.get_unchecked(5);
            *self.dst.get_unchecked_mut(p + 6) = *bytes.get_unchecked(6);
            *self.dst.get_unchecked_mut(p + 7) = *bytes.get_unchecked(7);
        }
        self
    }

    pub fn write_f32(&mut self, x: f32) -> &mut Self {
        self.write_u32(x.to_bits());
        self
    }

    pub fn write_f64(&mut self, x: f64) -> &mut Self {
        self.write_u64(x.to_bits());
        self
    }

    pub fn write_str(&mut self, x: &str) -> &mut Self {
        self.write_u16(x.len() as u16);
        self.write(x.as_bytes());
        self
    }

    pub fn write_bool(&mut self, x: bool) -> &mut Self {
        self.write_u8(x as u8);
        self
    }

    pub fn bytes(&self) -> &[u8] {
        &self.dst[..self.pos as usize]
    }

    pub fn into_bytes(self) -> &'a [u8] {
        &self.dst[..self.pos as usize]
    }
}
