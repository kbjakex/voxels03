use ash::vk;
use glam::IVec3;

use crate::camera::Camera;

use self::{state::State, world::RenderWorld};

use super::RendererBase;

pub mod world;
mod state;

pub struct GameRenderer {
    pub world: RenderWorld,
    state: State
}

impl GameRenderer {
    pub fn new(player_chunk_pos: IVec3, renderer: &mut RendererBase) -> anyhow::Result<Self> {
        let state = state::init(&mut renderer.vk)?;
        let world = RenderWorld::new(player_chunk_pos, renderer, &state)?;

        Ok(Self {
            world,
            state
        })
    }
}

impl GameRenderer {
    fn render_inner(&mut self, camera: &Camera, renderer: &RendererBase, cmd: vk::CommandBuffer, image_index: usize) -> anyhow::Result<()> {
        let vk = &renderer.vk;
        let state = &self.state;

        unsafe {
            vk.device.cmd_begin_render_pass(cmd, &vk::RenderPassBeginInfo::builder()
                .clear_values(&[vk::ClearValue{
                    color: vk::ClearColorValue {
                        float32: [0.0, 0.0, 0.0, 1.0]
                    }
                }])
                .framebuffer(state.main_pass_framebuffers[image_index])
                .render_pass(state.main_render_pass)
                .render_area(vk::Rect2D {
                    offset: vk::Offset2D::builder().build(),
                    extent: vk.swapchain.surface.extent,
                })
            , vk::SubpassContents::INLINE);

            let mvp = camera.proj_view_matrix().to_cols_array();
            let mvp_bytes = bytemuck::cast_slice(&mvp);
            vk.device.cmd_push_constants(cmd, state.full_block_pipeline.layout, vk::ShaderStageFlags::VERTEX, 0, mvp_bytes);

            self.world.render(cmd, vk, state)?;

            vk.device.cmd_end_render_pass(cmd);
        }

        Ok(())
    }

    pub fn render(&mut self, camera: &Camera, renderer: &mut RendererBase) -> anyhow::Result<()> {
        renderer.render(|renderer, commands, image_index| {
            self.render_inner(camera, renderer, commands, image_index)
        })
    }
}

impl GameRenderer {
    #[cold]
    pub fn handle_window_resize(&mut self, renderer: &RendererBase) {
        self.state.handle_window_resize(&renderer.vk);
    }
}
