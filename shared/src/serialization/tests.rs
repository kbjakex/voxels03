#[test]
pub fn test_bit_rw_roundtrip() {
    let mut buf = [0u8; 28];
    let mut writer = super::BitWriter::new(&mut buf);

    writer.uint(0x123, 12);
    writer.bool(true);
    writer.uint(0x4D, 7);
    writer.uint(0xFFFF_FFFF, 32);
    writer.uint(0xAAAA, 16);
    writer.int(-12345678, 28);
    writer.int(134217727, 28);
    writer.int(0, 28);
    writer.int(-134217728, 28);

    println!("{}", writer.compute_bytes_written());

    writer.uint(0xAB, 8);
    writer.uint(0xCD, 8);

    writer.flush_partials();

    println!("{}", writer.compute_bytes_written());

    assert_eq!(writer.bits_written(), 196);
    assert_eq!(writer.compute_bytes_written(), 25);

    let buf = &buf[..25];

    let mut reader = super::BitReader::new(buf);
    assert_eq!(reader.uint(12), 0x123);
    assert_eq!(reader.bool(), true);
    assert_eq!(reader.uint(7), 0x4D);
    assert_eq!(reader.uint(32), 0xFFFF_FFFF);
    assert_eq!(reader.uint(16), 0xAAAA);
    assert_eq!(reader.int(28), -12345678);
    assert_eq!(reader.int(28), 134217727);
    assert_eq!(reader.int(28), 0);
    assert_eq!(reader.int(28), -134217728);

    // Little-endian, so 0xAB, 0xCD => 0xCD_AB
    assert_eq!(reader.uint(16), 0xCDAB);

    // Any reads past the end are zeros
    assert_eq!(reader.uint(32), 0);
    assert_eq!(reader.uint(32), 0);
    assert_eq!(reader.uint(32), 0);
    assert_eq!(reader.uint(32), 0);
    assert_eq!(reader.uint(32), 0);
}

#[test]
fn test_byte_rw_roundtrip() {
    let mut buf = [0u8; 32];
    let mut writer = super::ByteWriter::new(&mut buf);

    writer.write_bool(true);
    writer.write_bool(false);
    writer.write_u8(0x13);
    writer.write_i8(-0x13);
    writer.write_u16(0xAAAA);
    writer.write_i16(0x7FFF);
    writer.write_u32(0xFFFF_FFFF);
    writer.write_i32(-1);
    writer.write_u64(0x1234_5678_9876_5432);
    writer.write_i64(-0x123456789);

    assert_eq!(writer.bytes_written(), 32);

    let mut reader = super::ByteReader::new(&buf);
    assert_eq!(reader.read_bool(), true);
    assert_eq!(reader.read_bool(), false);
    assert_eq!(reader.read_u8(), 0x13);
    assert_eq!(reader.read_i8(), -0x13);
    assert_eq!(reader.read_u16(), 0xAAAA);
    assert_eq!(reader.read_i16(), 0x7FFF);
    assert_eq!(reader.read_u32(), 0xFFFF_FFFF);
    assert_eq!(reader.read_i32(), -1);
    assert_eq!(reader.read_u64(), 0x1234_5678_9876_5432);
    assert_eq!(reader.read_i64(), -0x123456789);
}