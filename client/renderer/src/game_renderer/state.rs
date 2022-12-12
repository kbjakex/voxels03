use std::sync::Arc;

use vulkano::{
    image::{view::ImageView, ImageAccess, SwapchainImage},
    pipeline::graphics::viewport::Viewport,
    render_pass::{Framebuffer, FramebufferCreateInfo, RenderPass},
};

use crate::vulkan::VkState;

pub struct State {
    pub main_pass: Arc<RenderPass>,

    pub viewport: Viewport,
    pub framebuffers: Vec<Arc<Framebuffer>>,
}

impl State {
    pub fn handle_window_resize(&mut self, vk: &VkState) {
        self.framebuffers = create_framebuffers(
            &vk.swapchain_images,
            self.main_pass.clone(),
            &mut self.viewport,
        );
    }
}

pub fn init(vk: &VkState) -> anyhow::Result<State> {
    let main_pass = vulkano::single_pass_renderpass!(
        vk.device.clone(),
        attachments: {
            color: {
                load: Clear,
                store: Store,
                format: vk.swapchain.image_format(),
                samples: 1,
            }
        },
        pass: {
            color: [color],
            depth_stencil: {}
        }
    )?;

    let mut viewport = Viewport {
        origin: [0.0, 0.0],
        dimensions: [0.0, 0.0],
        depth_range: 0.0..1.0,
    };

    let framebuffers = create_framebuffers(&vk.swapchain_images, main_pass.clone(), &mut viewport);

    Ok(State {
        main_pass,
        framebuffers,
        viewport,
    })
}

fn create_framebuffers(
    images: &[Arc<SwapchainImage>],
    render_pass: Arc<RenderPass>,
    viewport: &mut Viewport,
) -> Vec<Arc<Framebuffer>> {
    let dimensions = images[0].dimensions().width_height();
    viewport.dimensions = [dimensions[0] as f32, dimensions[1] as f32];

    images
        .iter()
        .map(|image| {
            let view = ImageView::new_default(image.clone()).unwrap();
            Framebuffer::new(
                render_pass.clone(),
                FramebufferCreateInfo {
                    attachments: vec![view],
                    ..Default::default()
                },
            )
            .unwrap()
        })
        .collect::<Vec<_>>()
}
