pub mod state;

use log::debug;
use netcode::login::LoginResponse;
use renderer::game_renderer::GameRenderer;
use winit::event::{Event, WindowEvent};

use crate::{views::StateChange, resources::Resources, world::chunk::WorldBlockPosExt};

use self::state::GameState;

pub struct GameView {
    _state: GameState,
    renderer: GameRenderer,
}

impl GameView {
    pub fn new(login_response: LoginResponse, res: &mut Resources) -> Self {
        Self {
            _state: GameState {

            },
            renderer: GameRenderer::new(login_response.position.as_ivec3().to_chunk_pos(), &res.renderer)
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
        self.renderer.render(&mut res.renderer).unwrap();
        None
    }

    pub fn on_event(&mut self, event: Event<()>, res: &mut Resources) -> Option<Box<StateChange>> {
        if let Event::WindowEvent { event: WindowEvent::Resized(..), ..} = event {
            self.renderer.handle_window_resize(&res.renderer);
        }
        None
    }
}
