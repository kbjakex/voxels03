#[test]
pub fn test_roundtrip() {
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
