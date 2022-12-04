pub mod state;

use winit::event::Event;

use crate::views::StateChange;

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
    pub fn on_enter_view(&mut self) -> anyhow::Result<()> {

        Ok(())
    }

    pub fn on_exit_view(&mut self) -> anyhow::Result<()> {

        Ok(())
    }

    pub fn on_event(&mut self, _event: Event<()>) -> Option<Box<StateChange>> {

        None
    }
}
