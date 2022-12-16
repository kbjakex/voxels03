pub mod game_renderer;
pub mod camera;
mod vulkan;

use anyhow::Result;
use ash::vk;
use log::debug;
use winit::window::Window;

use self::vulkan::Vk;

// Sometimes called 'frames in flight'
const FRAME_OVERLAP: usize = 2;

// The renderer crate.
// Ideally the implementation details are kept blackboxed from the client, so
// primarily, anything Vulkan related should stay contained here.

#[derive(Default, Clone, Copy)]
struct PerFrameObjects {
    present_semaphore: vk::Semaphore,
    render_semaphore: vk::Semaphore,
    render_fence: vk::Fence,

    main_command_buffer: vk::CommandBuffer,
}

/// A generic base for rendering. This is mostly here to contain the
/// context-independent state and do the dirtywork (sync, swapchain management,
/// etc) that isn't interesting to look at.
pub struct RendererBase {
    pub(crate) vk: Box<Vk>,

    per_frame_objects: [PerFrameObjects; FRAME_OVERLAP],
    frame_count: usize,
}

impl RendererBase {
    pub fn new(window: &Window) -> Self {
        // This can absolutely error, but I don't think there is *any* value
        // in trying to handle errors properly at this stage. If this fails,
        // the application will just fail to launch, and for the user it makes
        // little to no difference whether it's a crash or a catch-and-print.
        // Until a fancier launching system is implemented anyhow.
        let vk = Vk::init(window).unwrap();

        let per_frame_objects = create_per_frame_objects(&vk).unwrap();

        Self {
            vk,
            per_frame_objects,
            frame_count: 0,
        }
    }
}

impl RendererBase {
    pub(crate) fn render<F>(&mut self, mut callback: F) -> anyhow::Result<()>
    where
        F: FnMut(
            &RendererBase,
            vk::CommandBuffer,
            /* image index */ usize,
        ) -> anyhow::Result<()>,
    {
        // Do the dirty & uninteresting & generic work to keep actual render function clean
        self.vk.uploader.flush_staged(&self.vk.device)?;
        self.vk.uploader.wait_fence_if_unfinished(&self.vk.device)?;

        let vk = &self.vk;
        let frame = &self.per_frame_objects[self.frame_count % FRAME_OVERLAP];
        let cmd = frame.main_command_buffer;

        let image_index = unsafe {
            // Make sure the GPU has finished rendering the last frame that used the same per-frame
            // objects.
            vk.device
                .wait_for_fences(&[frame.render_fence], true, u64::MAX)?;
            vk.device.reset_fences(&[frame.render_fence])?;

            vk.device
                .reset_command_buffer(cmd, vk::CommandBufferResetFlags::empty())?;

            let (image_index, _is_suboptimal) = vk.swapchain.loader.acquire_next_image(
                self.vk.swapchain.handle,
                1_000_000_000,
                frame.present_semaphore,
                vk::Fence::null(),
            )?;

            vk.device.begin_command_buffer(
                cmd,
                &vk::CommandBufferBeginInfo::builder()
                    .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT),
            )?;
            image_index
        };

        callback(self, cmd, image_index as usize)?;

        unsafe {
            vk.device.end_command_buffer(cmd)?;
            
            // Submit the work to the GPU, and sync-wise,
            // 1. Wait until the presentation of the previous frame using the same per-frame objects is finished
            //    (so if FRAME_OVERLAP was 1, this would always wait for the previous frame to have finished presenting)
            // 2. Once everything is done and frame is ready, signal the render semaphore so that the image can be presented
            vk.device.queue_submit(vk.device.queue, &[vk::SubmitInfo::builder()
                .wait_dst_stage_mask(&[vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT])
                .wait_semaphores(&[frame.present_semaphore])
                .signal_semaphores(&[frame.render_semaphore])
                .command_buffers(&[cmd])
                .build()
            ], frame.render_fence)?;

            // Waits until the work submitted above has finished (by waiting on the render semaphore) and then
            // presents the frame on screen.
            vk.swapchain.loader.queue_present(vk.device.queue, &vk::PresentInfoKHR::builder()
                .swapchains(&[vk.swapchain.handle])
                .wait_semaphores(&[frame.render_semaphore])
                .image_indices(&[image_index])
            )?;
        }

        self.frame_count += 1;

        Ok(())
    }
}

impl RendererBase {
    #[cold]
    pub fn handle_window_resize(&mut self, (new_width, new_height): (u32, u32)) {
        let new_extent = vk::Extent2D {
            width: new_width,
            height: new_height,
        };
        if new_extent == self.vk.swapchain.surface.extent {
            return;
        }

        let vk = &mut self.vk;

        let new_swapchain = match vk.swapchain.recreate(new_extent, vk) {
            Ok(r) => r,
            Err(e) => panic!("Failed to recreate swapchain: {:?}", e),
        };
        debug!("Swapchain recreated!");

        vk.swapchain = new_swapchain;
    }
}

fn create_per_frame_objects(vk: &Vk) -> Result<[PerFrameObjects; FRAME_OVERLAP]> {
    let mut objects = [PerFrameObjects::default(); FRAME_OVERLAP];

    for object in &mut objects {
        object.present_semaphore = unsafe {
            vk.device
                .create_semaphore(&vk::SemaphoreCreateInfo::builder(), None)?
        };
        object.render_semaphore = unsafe {
            vk.device
                .create_semaphore(&vk::SemaphoreCreateInfo::builder(), None)?
        };
        object.render_fence = unsafe {
            vk.device.create_fence(
                &vk::FenceCreateInfo::builder().flags(vk::FenceCreateFlags::SIGNALED),
                None,
            )?
        };
    }

    let command_buffers = unsafe {
        vk.device.allocate_command_buffers(
            &vk::CommandBufferAllocateInfo::builder()
                .command_pool(vk.command_pool)
                .level(vk::CommandBufferLevel::PRIMARY)
                .command_buffer_count(FRAME_OVERLAP as _)
                .build(),
        )?
    };
    for i in 0..FRAME_OVERLAP {
        objects[i].main_command_buffer = command_buffers[i];
    }

    Ok(objects)
}
