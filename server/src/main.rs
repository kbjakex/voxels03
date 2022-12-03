use std::panic::AssertUnwindSafe;

use log::{error, info};
use runner::run;
use server::Server;

pub mod runner;
pub mod server;

fn main() {
    init_logger();

    let mut server = match Server::start() {
        Ok(server) => server,
        Err(e) => {
            error!("Startup failed: {e}");
            return;
        }
    };

    let result = std::panic::catch_unwind(AssertUnwindSafe(|| {
        run(&mut server);
    }));
    if result.is_err() {
        error!("FATAL: Server crashed while ticking")
    }

    info!("Stopping server...");
    if let Err(e) = Server::shutdown(server) {
        error!("Error during shutdown: {e}");
    }
    info!("Stopped.");
}

fn init_logger() {
    fern::Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                "{} [{} : {}] {}",
                chrono::Local::now().format("%H:%M:%S"),
                &record.target()[record.target().find(':').map_or(0, |i| i + 2)..],
                record.level(),
                message
            ))
        })
        .level(log::LevelFilter::Debug)
        .chain(std::io::stdout())
        .chain(fern::log_file("output.log").unwrap())
        .apply()
        .unwrap();

    info!("Logger initialized");
}

