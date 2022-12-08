use log::{warn, info};
use netcode::Connecting;
use winit::event::Event;

use crate::{views::{StateChange, exit, switch_to, View}};


pub struct MainMenuView {
    connecting: Connecting
}

impl MainMenuView {
    pub fn new() -> Self {
        Self {
            // Todo obviously only start connecting once username and address have been entered
            connecting: netcode::try_connect("127.0.0.1:29477".parse().unwrap(), "jetp250".into())
        }
    }
}

impl MainMenuView {
    pub fn on_enter_view(&mut self) -> anyhow::Result<()> {

        Ok(())
    }

    pub fn on_exit_view(&mut self) -> anyhow::Result<()> {

        Ok(())
    }

    pub fn on_event(&mut self, event: Event<()>) -> Option<Box<StateChange>> {
        if let Event::MainEventsCleared = event {
            match self.connecting.tick() {
                Ok(None) => {},
                Ok(Some((response, _connection))) => {
                    info!("Connected! {response:?}");
                    return switch_to(View::game());
                }
                Err(e) => {
                    warn!("Err: {e}");
                    return exit();
                }
            }
        }
        None
    }
}
