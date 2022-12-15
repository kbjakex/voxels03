use ash::vk;
use glam::IVec3;

use self::{world::RenderWorld, state::State};

use super::RendererBase;

pub mod world;
mod state;

pub struct GameRenderer {
    pub world: RenderWorld,
    state: State
}

impl GameRenderer {
    pub fn new(player_chunk_pos: IVec3, renderer: &RendererBase) -> anyhow::Result<Self> {
        let world = RenderWorld::new(player_chunk_pos, renderer)?;
        let state = state::init(&renderer.vk)?;

        Ok(Self {
            world,
            state
        })
    }
}

impl GameRenderer {
    fn render_inner(&mut self, renderer: &RendererBase, cmd: vk::CommandBuffer, image_index: usize) -> anyhow::Result<()> {
        let vk = &renderer.vk;
        let state = &self.state;

        unsafe {
            vk.device.cmd_begin_render_pass(cmd, &vk::RenderPassBeginInfo::builder()
                .clear_values(&[vk::ClearValue{
                    color: vk::ClearColorValue {
                        float32: [0.5, 0.0, 0.0, 1.0]
                    }
                }])
                .framebuffer(state.main_pass_framebuffers[image_index])
                .render_pass(state.main_render_pass)
                .render_area(vk::Rect2D {
                    offset: vk::Offset2D::builder().build(),
                    extent: vk.swapchain.surface.extent,
                })
            , vk::SubpassContents::INLINE);

            vk.device.cmd_end_render_pass(cmd);
        }

        Ok(())
    }

    pub fn render(&mut self, renderer: &mut RendererBase) -> anyhow::Result<()> {
        renderer.render(|renderer, commands, image_index| {
            self.render_inner(renderer, commands, image_index)
        })
    }
}

impl GameRenderer {
    #[cold]
    pub fn handle_window_resize(&mut self, renderer: &RendererBase) {
        self.state.handle_window_resize(&renderer.vk);
    }
}
