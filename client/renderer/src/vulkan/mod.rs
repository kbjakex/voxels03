pub mod uploader;
pub mod util;
mod debug_callback;

use std::{ffi::CStr, ops::Deref};

use anyhow::{anyhow, Result};
use ash::{vk, Entry, Instance};
use gpu_allocator::{vulkan::{AllocatorCreateDesc}, AllocatorDebugSettings};
use log::debug;
use raw_window_handle::{HasRawDisplayHandle, HasRawWindowHandle};
use winit::window::Window;

use self::{phys_device_selection::GraphicsDeviceDetails, uploader::Uploader, debug_callback::DebugMessageHandler};

pub const PRESENT_MODE: vk::PresentModeKHR = vk::PresentModeKHR::FIFO;

pub type SurfaceLoader = ash::extensions::khr::Surface;
pub type SwapchainLoader = ash::extensions::khr::Swapchain;
pub type GpuAllocator = gpu_allocator::vulkan::Allocator;

#[derive(Clone)]
pub struct Surface {
    pub loader: SurfaceLoader,
    pub handle: vk::SurfaceKHR,
    pub format: vk::SurfaceFormatKHR,
    pub extent: vk::Extent2D,
}

pub struct Swapchain {
    pub handle: vk::SwapchainKHR,
    pub loader: SwapchainLoader,
    pub images: Vec<vk::Image>,
    pub image_views: Vec<vk::ImageView>,

    pub present_mode: vk::PresentModeKHR,

    // Not part of the swapchain, but convenient to have here
    pub surface: Surface,
}

impl Swapchain {
    pub fn recreate(&self, new_extent: vk::Extent2D, vk: &Vk) -> Result<Self> {
        let surface = self.surface.clone();
        debug!("SWAPCHAIN RECREATED");
        unsafe { create_swapchain(&vk.instance, &vk.device, surface, new_extent, Some(self.handle)) }
    }
}

pub struct Device {
    pub handle: ash::Device,
    pub physical: vk::PhysicalDevice,
    pub mem_properties: vk::PhysicalDeviceMemoryProperties,
    pub limits: vk::PhysicalDeviceLimits,
    pub kind: vk::PhysicalDeviceType,

    pub queue_family_idx: u32,
    pub queue: vk::Queue, // for all operations: compute, graphics, present, transfer
}

// `vk.device.handle.foo()` is extremely, extremely common.
// This enables doing `vk.device.foo()`.
impl Deref for Device {
    type Target = ash::Device;

    fn deref(&self) -> &Self::Target {
        &self.handle
    }
}

pub struct Vk {
    pub entry: Entry,
    pub instance: Instance,
    pub device: Device,
    pub swapchain: Swapchain,
    pub command_pool: vk::CommandPool,

    pub allocator: GpuAllocator,
    pub uploader: Uploader,

    debug_msg_handler: Option<DebugMessageHandler>,
}

impl Vk {
    pub fn init(window: &Window) -> Result<Box<Self>> {
        let monitor = window.current_monitor().unwrap();
        let wnd_size = monitor.size().to_logical(monitor.scale_factor());
        let wnd_extent = vk::Extent2D {
            width: wnd_size.width,
            height: wnd_size.height,
        };

        unsafe {
            let entry = Entry::load()?;
            let instance = create_instance(&entry, window)?;
            let debug_msg_handler = Some(DebugMessageHandler::new(&entry, &instance));
            let surface = create_surface_partial(&entry, &instance, window)?;
            let device = create_device(&instance, &surface)?;
            let swapchain = create_swapchain(&instance, &device, surface, wnd_extent, None)?;

            let command_pool = create_command_pool(&device);

            let mut allocator = GpuAllocator::new(&AllocatorCreateDesc {
                instance: instance.clone(), // 200-byte copy...
                device: device.handle.clone(), // over 1400-byte copy...
                physical_device: device.physical,
                debug_settings: AllocatorDebugSettings::default(),
                buffer_device_address: false,
            })?;

            let uploader = Uploader::new(&device, &mut allocator)?;

            Ok(Box::new(Self {
                entry,
                instance,
                device,
                swapchain,
                command_pool,
                debug_msg_handler,
                allocator,
                uploader,
            }))
        }
    }

    pub fn destroy_self(&mut self) {
        // Destroying happens in the opposite order of creation.
        unsafe {
            self.device.handle.destroy_command_pool(self.command_pool, None);

            for (&image, &view) in self.swapchain.images.iter().zip(self.swapchain.image_views.iter()) {
                self.device.handle.destroy_image_view(view, None);
                self.device.handle.destroy_image(image, None);
            }
            self.swapchain.loader.destroy_swapchain(self.swapchain.handle, None);
            
            let surface = &self.swapchain.surface;
            surface.loader.destroy_surface(surface.handle, None);

            self.device.handle.destroy_device(None);

            if let Some(messenger) = self.debug_msg_handler.take() {
                messenger.debug_utils.destroy_debug_utils_messenger(messenger.debug_callback, None);
            }

            self.instance.destroy_instance(None);
        }
    }
}

fn get_device_features() -> vk::PhysicalDeviceFeatures {
    vk::PhysicalDeviceFeatures {
        ..Default::default()
    }
}

// Below is purely Vulkan initialization code. Probably not very interesting.

unsafe fn create_command_pool(device: &Device) -> vk::CommandPool {
    let pool_create_info = vk::CommandPoolCreateInfo::builder()
        .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER)
        .queue_family_index(device.queue_family_idx);

    device.handle.create_command_pool(&pool_create_info, None).unwrap()
}

unsafe fn create_swapchain(
    instance: &Instance,
    device: &Device,
    surface: Surface,
    window_extent: vk::Extent2D,
    old_handle: Option<vk::SwapchainKHR>,
) -> Result<Swapchain> {
    let surface_format = swapchain_init::select_surface_format(device, &surface)?;
    let present_mode = swapchain_init::select_present_mode(device, &surface, PRESENT_MODE)?;

    let surface_capabilities = unsafe {
        surface
            .loader
            .get_physical_device_surface_capabilities(device.physical, surface.handle)
    }?;

    let mut image_count = surface_capabilities.min_image_count + 1;
    if surface_capabilities.max_image_count > 0
        && image_count > surface_capabilities.max_image_count
    {
        image_count = surface_capabilities.max_image_count;
    }

    let swapchain_info = vk::SwapchainCreateInfoKHR::builder()
        .surface(surface.handle)
        .min_image_count(image_count)
        .image_format(surface_format.format)
        .image_color_space(surface_format.color_space)
        .image_extent(surface_capabilities.current_extent)
        .image_array_layers(1)
        .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT)
        .image_sharing_mode(vk::SharingMode::EXCLUSIVE)
        .pre_transform(surface_capabilities.current_transform)
        .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
        .present_mode(present_mode)
        .clipped(true)
        .old_swapchain(old_handle.unwrap_or(vk::SwapchainKHR::null()));

    let loader = SwapchainLoader::new(instance, &device.handle);
    let handle = unsafe { loader.create_swapchain(&swapchain_info, None) }?;

    let images = unsafe { loader.get_swapchain_images(handle) }?;
    let image_views = images
        .iter()
        .map(|img| swapchain_init::image_view_for_image(*img, device, surface_format.format).unwrap())
        .collect();

    let surface_extent = match surface_capabilities.current_extent.width {
        u32::MAX => window_extent,
        _ => surface_capabilities.current_extent,
    };

    Ok(Swapchain {
        handle,
        loader,
        images,
        image_views,
        present_mode,
        surface: Surface {
            format: surface_format,
            extent: surface_extent,
            ..surface
        },
    })
}

unsafe fn create_device(instance: &Instance, surface: &Surface) -> Result<Device> {
    let GraphicsDeviceDetails {
        queue_idx,
        physical_device,
        properties,
        extensions,
    } = phys_device_selection::choose_physical_device(surface, instance)?;

    let priorities = [1.0];

    let queue_info = vk::DeviceQueueCreateInfo::builder()
        .queue_family_index(queue_idx)
        .queue_priorities(&priorities);

    let enabled_features = get_device_features();
    let device_create_info = vk::DeviceCreateInfo::builder()
        .queue_create_infos(std::slice::from_ref(&queue_info))
        .enabled_extension_names(&extensions)
        .enabled_features(&enabled_features);

    let handle = instance.create_device(physical_device, &device_create_info, None)?;

    let queue = handle.get_device_queue(queue_idx as u32, 0);

    let mem_properties = instance.get_physical_device_memory_properties(physical_device);

    Ok(Device {
        handle,
        physical: physical_device,
        mem_properties,
        limits: properties.limits,
        kind: properties.device_type,
        queue_family_idx: queue_idx,
        queue,
    })
}

unsafe fn create_surface_partial(entry: &Entry, instance: &Instance, window: &Window) -> Result<Surface> {
    let handle = ash_window::create_surface(
        &entry,
        &instance,
        window.raw_display_handle(),
        window.raw_window_handle(),
        None,
    )
    .map_err(|e| anyhow!("Surface creation failed: {e}"))?;

    let loader = ash::extensions::khr::Surface::new(entry, instance);

    // Format and size are deduced in create_device(), because
    // they require device-specifc information, yet creating the device
    // requires information from the SurfaceKHR...
    Ok(Surface {
        loader,
        handle,
        format: vk::SurfaceFormatKHR::default(),
        extent: vk::Extent2D::default(),
    })
}

unsafe fn create_instance(entry: &Entry, window: &Window) -> Result<Instance> {
    let app_name = CStr::from_bytes_with_nul_unchecked(b"voxels03\0");

    let appinfo = vk::ApplicationInfo::builder()
        .application_name(app_name)
        .application_version(0)
        .engine_name(app_name)
        .engine_version(0)
        .api_version(vk::make_api_version(0, 1, 2, 0));

    let layer_name_ptrs =
        [CStr::from_bytes_with_nul_unchecked(b"VK_LAYER_KHRONOS_validation\0").as_ptr()];

    let mut extension_name_ptrs =
            ash_window::enumerate_required_extensions(window.raw_display_handle())
                .unwrap()
                .to_vec();
    extension_name_ptrs.push(ash::extensions::ext::DebugUtils::name().as_ptr());

    let create_info = vk::InstanceCreateInfo::builder()
        .application_info(&appinfo)
        .enabled_layer_names(&layer_name_ptrs)
        .enabled_extension_names(&extension_name_ptrs)
        .flags(vk::InstanceCreateFlags::default());

    let instance = entry.create_instance(&create_info, None)?;

    Ok(instance)
}

mod swapchain_init {
    use anyhow::bail;
    use log::debug;

    use super::*;

    pub fn image_view_for_image(
        image: vk::Image,
        gpu: &Device,
        format: vk::Format,
    ) -> Result<vk::ImageView> {
        
        let image_view_info = vk::ImageViewCreateInfo::builder()
            .image(image)
            .view_type(vk::ImageViewType::TYPE_2D)
            .format(format)
            .subresource_range(
                vk::ImageSubresourceRange::builder()
                    .aspect_mask(vk::ImageAspectFlags::COLOR)
                    .base_mip_level(0)
                    .level_count(1)
                    .base_array_layer(0)
                    .layer_count(1)
                    .build(),
            );
        unsafe { gpu.handle.create_image_view(&image_view_info, None) }
            .map_err(|e| anyhow!("Image view creation failed: {e}"))
    }

    pub fn select_surface_format(
        device: &Device,
        surface: &Surface,
    ) -> Result<vk::SurfaceFormatKHR> {
        let formats = unsafe {
            surface
                .loader
                .get_physical_device_surface_formats(device.physical, surface.handle)
        }?;

        let res = formats.iter().find(|surface_format| {
            debug!("Found surface format: {surface_format:?}");
            surface_format.format == vk::Format::B8G8R8A8_SRGB
                && surface_format.color_space == vk::ColorSpaceKHR::SRGB_NONLINEAR
        });
        //.or_else(|| formats.get(0));

        match res {
            Some(format) => {
                debug!("Selected surface format: {format:?}");
                Ok(*format)
            }
            None => bail!("select_surface_format: No surface formats found!"),
        }
    }

    pub fn select_present_mode(
        device: &Device,
        surface: &Surface,
        desired: vk::PresentModeKHR,
    ) -> Result<vk::PresentModeKHR> {
        let present_modes = unsafe {
            surface
                .loader
                .get_physical_device_surface_present_modes(device.physical, surface.handle)
        }?;

        Ok(*present_modes
            .iter()
            .find(|&present_mode| *present_mode == desired)
            .unwrap_or(&vk::PresentModeKHR::FIFO))
    }
}

mod phys_device_selection {
    use super::*;

    pub struct GraphicsDeviceDetails {
        pub queue_idx: u32,
        pub physical_device: vk::PhysicalDevice,
        pub properties: vk::PhysicalDeviceProperties,
        // These are desired but also present
        pub extensions: Vec<*const i8>,
    }

    pub unsafe fn choose_physical_device(
        surface: &Surface,
        instance: &Instance,
    ) -> Result<GraphicsDeviceDetails> {
        let phys_device_details = instance
            .enumerate_physical_devices()
            .expect("Physical device error")
            .into_iter()
            .filter_map(|phys_device| get_gpu_details_if_suitable(phys_device, instance, surface))
            .min_by_key(|details| match details.properties.device_type {
                vk::PhysicalDeviceType::DISCRETE_GPU => 0,
                vk::PhysicalDeviceType::INTEGRATED_GPU => 1,
                vk::PhysicalDeviceType::VIRTUAL_GPU => 2,
                vk::PhysicalDeviceType::CPU => 3,
                vk::PhysicalDeviceType::OTHER => 4,
                _ => 5,
            });

        phys_device_details.ok_or_else(|| anyhow!("Failed to find a GPU!"))
    }

    fn get_gpu_details_if_suitable(
        phys_device: vk::PhysicalDevice,
        instance: &Instance,
        surface: &Surface,
    ) -> Option<GraphicsDeviceDetails> {
        // 1. It has to support a) graphics and presentation and b) transfer. Might be in the same queue.
        // Noteworthy: graphics and compute imply transfer even if transfer bit is not set.
        let queue_idx = match pick_queue_family(instance, phys_device, surface) {
            Some(idx) => idx,
            None => return None,
        };

        let properties = unsafe { instance.get_physical_device_properties(phys_device) };

        // 2. It has to support the desired extensions (only swapchain support right now)
        let desired_device_extensions: Vec<_> = [SwapchainLoader::name().as_ptr()].into();

        let supported_device_extensions =
            unsafe { instance.enumerate_device_extension_properties(phys_device) }.ok()?;

        let device_extensions_supported =
            desired_device_extensions.iter().all(|device_extension| {
                let device_extension = unsafe { CStr::from_ptr(*device_extension) };

                supported_device_extensions.iter().any(|properties| unsafe {
                    CStr::from_ptr(properties.extension_name.as_ptr()) == device_extension
                })
            });

        if !device_extensions_supported {
            return None;
        }

        Some(GraphicsDeviceDetails {
            queue_idx,
            physical_device: phys_device,
            properties,
            extensions: desired_device_extensions,
        })
    }

    fn pick_queue_family(
        instance: &Instance,
        phys_device: vk::PhysicalDevice,
        surface: &Surface,
    ) -> Option<u32> {
        let queue_family_props =
            unsafe { instance.get_physical_device_queue_family_properties(phys_device) };

        for (i, props) in queue_family_props.iter().enumerate() {
            use vk::QueueFlags as qf;
            // Note that | is union, not 'or'...
            if !props
                .queue_flags
                .contains(qf::GRAPHICS | qf::COMPUTE | qf::TRANSFER)
            {
                continue;
            }

            // Require presentation capabilities
            let present_support = unsafe {
                surface.loader.get_physical_device_surface_support(
                    phys_device,
                    i as u32,
                    surface.handle,
                )
            }
            .is_ok();

            if !present_support {
                continue;
            }

            return Some(i as _);
        }
        None
    }
}
