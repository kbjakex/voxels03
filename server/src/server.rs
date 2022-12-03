use std::time::SystemTime;

use log::{error, info};
use crate::{runner::TICK_DURATION};

pub struct State {
    pub current_tick: u32,
}

pub struct Server {
    pub state: State,
}

impl Server {

}

// Tick logic
impl Server {
    pub fn tick(self: &mut Server) -> anyhow::Result<()> {
        self.state.current_tick += 1;
        Ok(())
    }
}

impl Server {
    pub fn start() -> anyhow::Result<Self> {
        let state = State {
            current_tick: 0,
        };

        let server = Server {
            state,
        };

        Ok(server)
    }

    pub fn shutdown(self: Server) -> anyhow::Result<()> {
        Ok(())
    }
}