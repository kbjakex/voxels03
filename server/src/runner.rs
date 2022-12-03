use std::{sync::atomic::{AtomicBool, Ordering}, time::{Instant, Duration}};

use log::{debug, error};

use crate::server::Server;

pub const TICKS_PER_SECOND : u32 = 32;
pub const TICK_DURATION : Duration = Duration::from_nanos(1_000_000_000 / TICKS_PER_SECOND as u64);

pub fn run(server: &mut Server) {
    debug!("Server running @ {}Hz tick rate", TICKS_PER_SECOND);

    static SHOULD_STOP : AtomicBool = AtomicBool::new(false);
    ctrlc::set_handler(|| {
        println!();
        SHOULD_STOP.store(true, Ordering::Relaxed);
    }).unwrap();

    let mut last_sec = Instant::now();
    let mut current_tick = 0;
    let mut updates = 0;

    let server_start_time = Instant::now();
    while !SHOULD_STOP.load(Ordering::Relaxed) {
        if let Err(e) = Server::tick(server) {
            error!("Error while ticking server: {e}");
        }

        current_tick += 1;
        updates += 1;

        let time = Instant::now();
        if time - last_sec >= Duration::from_secs(10) {
            debug!("Updates per second {}", updates as f32 / 10.0);
            last_sec = time;
            updates = 0;
        }

        let target = server_start_time + current_tick * TICK_DURATION;
        if time < target {
            std::thread::sleep(target - time);
        }
    }
}
