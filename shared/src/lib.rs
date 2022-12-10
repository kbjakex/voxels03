pub mod anti_jitter;
pub mod net;
pub mod serialization;

use std::time::Duration;


pub const TICKS_PER_SECOND : u32 = 32;
pub const TICK_DURATION : Duration = Duration::from_nanos(1_000_000_000 / TICKS_PER_SECOND as u64);
