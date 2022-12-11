pub mod state;

use log::debug;
use winit::event::Event;

use crate::{views::StateChange, resources::Resources};

use self::state::GameState;

pub struct GameView {
    _state: GameState
}

impl GameView {
    pub fn new() -> Self {
        Self {
            _state: GameState {

            }
        }
    }
}

impl GameView {
    pub fn on_enter_view(&mut self, _res: &mut Resources) -> anyhow::Result<()> {
        debug!("Entering game view");
        Ok(())
    }

    pub fn on_exit_view(&mut self, _res: &mut Resources) -> anyhow::Result<()> {
        debug!("Leaving game view");
        Ok(())
    }

    pub fn on_update(&mut self, res: &mut Resources) -> Option<Box<StateChange>> {
        res.renderer.render().unwrap();
        None
    }

    pub fn on_event(&mut self, _event: Event<()>, _res: &mut Resources) -> Option<Box<StateChange>> {

        None
    }
}
