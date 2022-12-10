// These files should either be re-checked for safety, or be done away with.
// A good contender would be <https://crates.io/crates/bit_serializer>,
// but it is lacking in ergonomics: everything requires unwraps, and there
// obviously isn't a built-in way to easily write packet length.

pub mod bit_reader;
pub mod bit_writer;
pub mod byte_reader;
pub mod byte_writer;

pub use bit_reader::*;
pub use bit_writer::*;
pub use byte_reader::*;
pub use byte_writer::*;

mod tests;

#[inline]
pub fn f32_to_fixed(f: f32, fractional_bits: u32) -> u32 {
    (f * (1 << fractional_bits) as f32).round() as i32 as u32
}

#[inline]
pub fn fixed_to_f32(fp: u32, fractional_bits: u32) -> f32 {
    (fp as i32) as f32 / (1 << fractional_bits) as f32
}

#[inline]
pub fn round_to_frac_bits(f: f32, fractional_bits: u32) -> f32 {
    fixed_to_f32(f32_to_fixed(f, fractional_bits), fractional_bits)
}
