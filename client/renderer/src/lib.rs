mod vulkan;
pub mod camera;
pub mod game_renderer;

use std::sync::Arc;

use log::warn;
use vulkan::{CmdBufBuilder, VkState};
use vulkano::{
    command_buffer::{
        AutoCommandBufferBuilder, CommandBufferUsage,
    },
    swapchain::{
        acquire_next_image, SwapchainCreateInfo, SwapchainCreationError, SwapchainPresentInfo,
    },
    sync::{self, FlushError, GpuFuture},
};
use winit::window::Window;

// The renderer crate.
// Ideally the implementation details are kept blackboxed from the client, so
// primarily, anything Vulkan related should stay contained here.


/// A generic base for rendering. This is mostly here to contain the
/// context-independent state and do the dirtywork (sync, swapchain management,
/// etc) that isn't interesting to look at.
pub struct Renderer {
    pub(crate) vk: VkState,

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

        let previous_frame_end = Some(sync::now(vk.device.clone()).boxed());

        Self {
            vk,

            recreate_swapchain: false,
            previous_frame_end,
        }
    }
}

impl Renderer {
    pub(crate) fn render<F>(&mut self, mut callback: F) -> anyhow::Result<()> 
        where F: FnMut(&Renderer, &mut CmdBufBuilder, /* image index */ usize) -> anyhow::Result<()>
    {
        // Do the dirty & uninteresting & generic work to keep actual render function clean
        let vk = &self.vk;
        let (image_index, _suboptimal, acquire_future) =
            acquire_next_image(vk.swapchain.clone(), None)?;

        let mut builder = AutoCommandBufferBuilder::primary(
            &self.vk.command_buffer_allocator,
            self.vk.queue.queue_family_index(),
            CommandBufferUsage::OneTimeSubmit,
        )
        .unwrap();

        callback(self, &mut builder, image_index as usize)?;

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
