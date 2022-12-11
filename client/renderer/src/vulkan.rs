use std::sync::Arc;

use vulkano::{
    device::{
        physical::{PhysicalDevice, PhysicalDeviceType},
        Device, DeviceCreateInfo, DeviceExtensions, QueueCreateInfo, QueueFlags, Queue,
    },
    image::{ImageUsage, SwapchainImage},
    instance::{Instance, InstanceCreateInfo},
    swapchain::{Surface, Swapchain, SwapchainCreateInfo, SwapchainCreationError},
    VulkanLibrary, memory::allocator::{StandardMemoryAllocator}, command_buffer::{allocator::{StandardCommandBufferAllocator, CommandBufferAllocator}, AutoCommandBufferBuilder, PrimaryAutoCommandBuffer},
};
use vulkano_win::create_surface_from_winit;
use winit::window::Window;

pub type CmdBufBuilder = AutoCommandBufferBuilder<
    PrimaryAutoCommandBuffer<<StandardCommandBufferAllocator as CommandBufferAllocator>::Alloc>,
    StandardCommandBufferAllocator,
>;

pub struct VkState {
    pub lib: Arc<VulkanLibrary>,
    pub instance: Arc<Instance>,
    pub physical_device: Arc<PhysicalDevice>,
    pub device: Arc<Device>,
    pub queue: Arc<Queue>,
    pub swapchain: Arc<Swapchain>,
    pub swapchain_images: Vec<Arc<SwapchainImage>>,
    pub allocator: Arc<StandardMemoryAllocator>,
    pub command_buffer_allocator: StandardCommandBufferAllocator,
}

fn get_device_extensions() -> DeviceExtensions {
    DeviceExtensions {
        khr_swapchain: true,
        khr_storage_buffer_storage_class: true,
        ..DeviceExtensions::empty()
    }
}

pub fn init_vulkan(window: Arc<Window>) -> anyhow::Result<VkState> {
    let lib = VulkanLibrary::new()?;

    let required_instance_extensions = vulkano_win::required_extensions(&lib);
    let instance = Instance::new(
        lib.clone(),
        InstanceCreateInfo {
            enabled_extensions: required_instance_extensions,
            // Enable enumerating devices that use non-conformant vulkan implementations. (ex. MoltenVK)
            enumerate_portability: true,
            ..Default::default()
        },
    )?;

    let surface = create_surface_from_winit(window, instance.clone())?;

    let device_extensions = get_device_extensions();

    let (physical_device, queue_family_index) =
        choose_physical_device(surface.clone(), instance.clone(), device_extensions);

    let (device, mut queues) = Device::new(
        physical_device.clone(),
        DeviceCreateInfo {
            enabled_extensions: device_extensions,
            queue_create_infos: vec![QueueCreateInfo {
                queue_family_index,
                ..Default::default()
            }],
            ..Default::default()
        },
    )?;

    let queue = queues.next().unwrap(); // All devices have at least one queue

    let (swapchain, swapchain_images) = create_swapchain(surface.clone(), device.clone())?;

    let allocator = Arc::new(StandardMemoryAllocator::new_default(device.clone()));

    let command_buffer_allocator =
        StandardCommandBufferAllocator::new(device.clone(), Default::default());

    Ok(VkState {
        lib,
        instance,
        physical_device,
        device,
        queue,
        swapchain,
        swapchain_images,
        allocator,
        command_buffer_allocator,
    })
}

fn choose_physical_device(
    surface: Arc<Surface>,
    instance: Arc<Instance>,
    extensions: DeviceExtensions,
) -> (Arc<PhysicalDevice>, u32) {
    instance
        .enumerate_physical_devices()
        .unwrap()
        .filter(|p| p.supported_extensions().contains(&extensions))
        .filter_map(|p| {
            p.queue_family_properties()
                .iter()
                .enumerate()
                .position(|(i, q)| {
                    q.queue_flags.intersects(QueueFlags::GRAPHICS)
                        && p.surface_support(i as u32, &surface).unwrap_or(false)
                })
                .map(|i| (p, i as u32))
        })
        .min_by_key(|(p, _)| match p.properties().device_type {
            PhysicalDeviceType::DiscreteGpu => 0,
            PhysicalDeviceType::IntegratedGpu => 1,
            PhysicalDeviceType::VirtualGpu => 2,
            PhysicalDeviceType::Cpu => 3,
            PhysicalDeviceType::Other => 4,
            _ => 5,
        })
        .unwrap() // all users necessarily have at least one gpu...
}

fn create_swapchain(
    surface: Arc<Surface>,
    device: Arc<Device>,
) -> Result<(Arc<Swapchain>, Vec<Arc<SwapchainImage>>), SwapchainCreationError> {
    let surface_capabilities = device
        .physical_device()
        .surface_capabilities(&surface, Default::default())
        .unwrap();
    let image_format = Some(
        device
            .physical_device()
            .surface_formats(&surface, Default::default())
            .unwrap()[0]
            .0,
    );
    let window = surface.object().unwrap().downcast_ref::<Window>().unwrap();

    Swapchain::new(
        device.clone(),
        surface.clone(),
        SwapchainCreateInfo {
            min_image_count: surface_capabilities.min_image_count,
            image_format,
            image_extent: window.inner_size().into(),
            image_usage: ImageUsage::COLOR_ATTACHMENT,
            composite_alpha: surface_capabilities
                .supported_composite_alpha
                .into_iter()
                .next()
                .unwrap(),
            ..Default::default()
        },
    )
}
