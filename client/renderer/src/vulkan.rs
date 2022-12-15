use std::sync::Arc;

use anyhow::anyhow;

use vulkano::{
    buffer::{
        sys::{Buffer, BufferCreateInfo, RawBuffer},
        BufferCreateFlags, BufferUsage, CpuAccessibleBuffer,
    },
    command_buffer::{
        allocator::{CommandBufferAllocator, StandardCommandBufferAllocator},
        AutoCommandBufferBuilder, PrimaryAutoCommandBuffer, CommandBufferUsage,
    },
    device::{
        physical::{PhysicalDevice, PhysicalDeviceType},
        Device, DeviceCreateInfo, DeviceExtensions, Queue, QueueCreateInfo, QueueFlags,
    },
    image::{ImageUsage, SwapchainImage},
    instance::{Instance, InstanceCreateInfo},
    memory::{
        allocator::{MemoryAlloc, StandardMemoryAllocator},
        DedicatedAllocation, DeviceMemory, ExternalMemoryHandleTypes, MemoryAllocateFlags,
        MemoryAllocateInfo, MemoryProperties, MemoryPropertyFlags,
    },
    swapchain::{PresentMode, Surface, Swapchain, SwapchainCreateInfo, SwapchainCreationError},
    sync::Sharing,
    VulkanLibrary,
};
use vulkano_win::create_surface_from_winit;
use winit::window::Window;

pub const PRESENT_MODE: PresentMode = PresentMode::Fifo;

// As per the Vulkano docs: this should ideally be big enough
// that all allocations for a frame fit in one arena. Chunks are
// kind of large, and there can be many of them, so it should
// be decently large.
// The Sodium Minecraft mod uses a staging buffer of 16 MiB from
// what I know, though I'm not sure what exactly it does with it,
// since OpenGL handles uploads for you
pub const STAGING_ARENA_SIZE: u64 = 8192 * 1024; // 8 MiB

pub type CmdBufBuilder = AutoCommandBufferBuilder<
    PrimaryAutoCommandBuffer<<StandardCommandBufferAllocator as CommandBufferAllocator>::Alloc>,
    StandardCommandBufferAllocator,
>;

pub struct VkState {
    pub lib: Arc<VulkanLibrary>,
    pub instance: Arc<Instance>,
    pub device: Arc<Device>,
    pub queue: Arc<Queue>,
    pub swapchain: Arc<Swapchain>,
    pub swapchain_images: Vec<Arc<SwapchainImage>>,
    pub allocator: Arc<StandardMemoryAllocator>,
    pub command_buffer_allocator: StandardCommandBufferAllocator,

    // Pool of host-visible GPU memory for uploads (CPU->GPU transfers)
    pub staging: Arc<CpuAccessibleBuffer<[u8]>>,
}

impl VkState {
    pub fn new_command_buf(&self) -> anyhow::Result<CmdBufBuilder> {
        AutoCommandBufferBuilder::primary(
            &self.command_buffer_allocator,
            self.queue.queue_family_index(),
            CommandBufferUsage::OneTimeSubmit,
        ).map_err(|e| anyhow!(e))
    }
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

    let staging = unsafe {
        // Not sure why this is unsafe? Docs don't explain what's unsafe about this, and this is what I need
        CpuAccessibleBuffer::uninitialized_array(&allocator, STAGING_ARENA_SIZE, BufferUsage::TRANSFER_SRC, false)
    }?;

    Ok(VkState {
        lib,
        instance,
        device,
        queue,
        swapchain,
        swapchain_images,
        allocator,
        command_buffer_allocator,

        staging
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
            present_mode: PRESENT_MODE,
            ..Default::default()
        },
    )
}

pub fn allocate_buffer(
    device: Arc<Device>,
    size_bytes: u64,
    memory_properties: Option<&MemoryProperties>,
    usage: BufferUsage,
) -> anyhow::Result<Buffer> {
    // Note: this doesn't allocate anything yet!
    let buffer = RawBuffer::new(
        device.clone(),
        BufferCreateInfo {
            flags: BufferCreateFlags::default(),
            sharing: Sharing::Exclusive,
            size: size_bytes as u64,
            // TRANSFER_DST is needed to be able to copy from staging buffer into this buffer
            usage: BufferUsage::STORAGE_BUFFER | BufferUsage::TRANSFER_DST,
            external_memory_handle_types: ExternalMemoryHandleTypes::empty(),
            ..Default::default()
        },
    )?;

    let buffer_mem_reqs = buffer.memory_requirements();

    // Find a suitable memory type. These are generally ordered approximately
    // best first, worst last, so pick the first one that works.
    // This is also what the official documentation recommends at
    // https://registry.khronos.org/vulkan/specs/1.3-extensions/man/html/VkPhysicalDeviceMemoryProperties.html
    let memory_type_index = memory_properties
        .unwrap_or_else(|| device.physical_device().memory_properties())
        .memory_types
        .iter()
        .enumerate()
        .find_map(|(i, mem_type)| {
            (((1 << i as u32) & buffer_mem_reqs.memory_type_bits) != 0
                && mem_type
                    .property_flags
                    .contains(MemoryPropertyFlags::DEVICE_LOCAL))
            .then_some(i as u32)
        })
        .unwrap();

    let allocation = MemoryAlloc::new(DeviceMemory::allocate(
        device,
        MemoryAllocateInfo {
            allocation_size: buffer_mem_reqs.size,
            memory_type_index,
            dedicated_allocation: Some(DedicatedAllocation::Buffer(&buffer)),
            export_handle_types: ExternalMemoryHandleTypes::empty(),
            flags: MemoryAllocateFlags::empty(),
            ..Default::default()
        },
    )?)?;

    let buffer = buffer.bind_memory(allocation).map_err(|(err, ..)| err)?;

    Ok(buffer)
}
