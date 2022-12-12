use glam::IVec3;
use vulkano::command_buffer::{RenderPassBeginInfo, SubpassContents};

use crate::{Renderer, vulkan::CmdBufBuilder};

use self::{world::RenderWorld, state::State};




pub mod world;
mod state;

pub struct GameRenderer {
    pub world: RenderWorld,
    state: State
}

impl GameRenderer {
    pub fn new(player_chunk_pos: IVec3, renderer: &Renderer) -> Self {
        Self {
            world: RenderWorld::new(player_chunk_pos, renderer),
            state: state::init(&renderer.vk).unwrap()
        }
    }
}

impl GameRenderer {
    fn render_inner(&mut self, renderer: &Renderer, commands: &mut CmdBufBuilder, image_index: usize) -> anyhow::Result<()> {
        let _vk = &renderer.vk;
        let state = &self.state;

        commands
            .begin_render_pass(
                RenderPassBeginInfo {
                    clear_values: vec![Some([0.0, 0.0, 0.0, 1.0].into())],
                    ..RenderPassBeginInfo::framebuffer(state.framebuffers[image_index].clone())
                },
                SubpassContents::Inline,
            )?
            .set_viewport(0, [state.viewport.clone()])
            .end_render_pass()?;

        Ok(())
    }

    pub fn render(&mut self, renderer: &mut Renderer) -> anyhow::Result<()> {
        renderer.render(|renderer, commands, image_index| {
            self.render_inner(renderer, commands, image_index)
        })
    }
}

impl GameRenderer {
    #[cold]
    pub fn handle_window_resize(&mut self, renderer: &Renderer) {
        self.state.handle_window_resize(&renderer.vk);
    }
}
