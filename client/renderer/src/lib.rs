mod state;
mod vulkan;

use std::sync::Arc;

use log::warn;
use state::State;
use vulkan::{VkState, CmdBufBuilder};
use vulkano::{
    command_buffer::{
        AutoCommandBufferBuilder, CommandBufferUsage,
    },
    swapchain::{
        acquire_next_image, SwapchainCreateInfo, SwapchainCreationError,
        SwapchainPresentInfo,
    },
    sync::{self, FlushError, GpuFuture},
};
use winit::window::Window;

pub struct Renderer {
    vk: VkState,
    state: State,

    recreate_swapchain: bool,
    previous_frame_end: Option<Box<dyn GpuFuture>>,
}

impl Renderer {
    pub fn new(window: Arc<Window>) -> Self {
        // These can absolutely error, but I don't think there is *any* value
        // in trying to handle errors properly at this stage. If this fails,
        // the application will just fail to launch, and for the user it makes
        // little to no difference whether it's a crash or a catch-and-print.
        // Until a fancier launching system is implemented anyhow.
        let vk = vulkan::init_vulkan(window).unwrap();
        let state = state::init(&vk).unwrap();

        let previous_frame_end = Some(sync::now(vk.device.clone()).boxed());

        Self {
            vk,
            state,

            recreate_swapchain: false,
            previous_frame_end,
        }
    }
}

impl Renderer {
    fn render_inner(&mut self, commands: &mut CmdBufBuilder) -> anyhow::Result<()> {
        
        Ok(())
    }

    pub fn render(&mut self) -> anyhow::Result<()> {
        // Do the dirty uninteresting generic work to keep actual render function clean
        let vk = &self.vk;
        let (image_index, _suboptimal, acquire_future) =
            acquire_next_image(vk.swapchain.clone(), None)?;

        let mut builder = AutoCommandBufferBuilder::primary(
            &self.vk.command_buffer_allocator,
            self.vk.queue.queue_family_index(),
            CommandBufferUsage::OneTimeSubmit,
        )
        .unwrap();

        self.render_inner(&mut builder)?;

        let vk = &self.vk;
        let command_buffer = builder.build()?;

        let future = self
            .previous_frame_end
            .take()
            .unwrap()
            .join(acquire_future)
            .then_execute(vk.queue.clone(), command_buffer)
            .unwrap()
            .then_swapchain_present(
                vk.queue.clone(),
                SwapchainPresentInfo::swapchain_image_index(vk.swapchain.clone(), image_index),
            )
            .then_signal_fence_and_flush();

        match future {
            Ok(future) => {
                self.previous_frame_end = Some(future.boxed());
            }
            Err(FlushError::OutOfDate) => {
                self.recreate_swapchain = true;
                self.previous_frame_end = Some(sync::now(vk.device.clone()).boxed());
            }
            Err(e) => {
                warn!("Failed to flush future: {:?}", e);
                self.previous_frame_end = Some(sync::now(vk.device.clone()).boxed());
            }
        }

        Ok(())
    }
}

impl Renderer {
    #[cold]
    pub fn handle_window_resize(&mut self, (new_width, new_height): (u32, u32)) {
        // TODO
        // Remember to check if old swapchain size == new size (which happens *often*) to early exit

        let vk = &mut self.vk;

        let (new_swapchain, _new_images) = match vk.swapchain.recreate(SwapchainCreateInfo {
            image_extent: [new_width, new_height],
            ..vk.swapchain.create_info()
        }) {
            Ok(r) => r,
            Err(SwapchainCreationError::ImageExtentNotSupported { .. }) => return,
            Err(e) => panic!("Failed to recreate swapchain: {:?}", e),
        };

        vk.swapchain = new_swapchain;
        self.recreate_swapchain = false;
    }
}
