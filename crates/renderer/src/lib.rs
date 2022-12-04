use std::sync::Arc;

use vulkano::VulkanLibrary;

pub struct Renderer {
    _vulkan: Arc<VulkanLibrary>,
}

impl Renderer {
    pub fn new() -> Self {
        // This can absolutely error, but I don't think there is *any* value
        // in trying to handle errors properly at this stage. If this fails,
        // the application will just fail to launch, and for the user it makes
        // little to no difference whether it's a crash or a catch-and-print.

        let vulkan = VulkanLibrary::new().unwrap();

        Self { 
            _vulkan: vulkan
        }
    }
}

impl Renderer {
    #[cold]
    pub fn handle_window_resize(&mut self, (_new_width, _new_height): (u32, u32)) {
        // TODO
        // Remember to check if old swapchain size == new size (which happens *often*) to early exit
    }
}

