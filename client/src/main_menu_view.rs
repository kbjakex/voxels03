use log::{warn, info, debug};
use netcode::Connecting;
use winit::event::Event;

use crate::{views::{StateChange, switch_to, View}, resources::Resources};

fn start_connecting() -> Connecting {
    netcode::try_connect("127.0.0.1:29477".parse().unwrap(), "Player1".into())
}

pub struct MainMenuView {
    connecting: Connecting
}

impl MainMenuView {
    pub fn new() -> Self {
        Self {
            // Todo obviously only start connecting once username and address have been entered
            connecting: start_connecting(),
        }
    }
}

impl MainMenuView {
    pub fn on_enter_view(&mut self, _res: &mut Resources) -> anyhow::Result<()> {
        debug!("Entering main menu view");
        Ok(())
    }

    pub fn on_exit_view(&mut self, _res: &mut Resources) -> anyhow::Result<()> {
        debug!("Leaving main menu view");
        Ok(())
    }

    pub fn on_update(&mut self, _res: &mut Resources) -> Option<Box<StateChange>> {
        match self.connecting.tick() {
            Ok(None) => {},
            Ok(Some((response, _connection))) => {
                info!("Connected! {response:?}");
                return switch_to(View::game());
            }
            Err(e) => {
                warn!("Error: {e}, retrying...");
                self.connecting = start_connecting();
            }
        }
        None
    }

    pub fn on_event(&mut self, _event: Event<()>, _res: &mut Resources) -> Option<Box<StateChange>> {
        None
    }
}
