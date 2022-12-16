use std::ffi::CStr;

use anyhow::Result;
use ash::vk::{self, BufferCreateFlags, BufferCreateInfo, BufferUsageFlags, SharingMode};
use gpu_allocator::{
    vulkan::{Allocation, AllocationCreateDesc},
    MemoryLocation,
};

use super::{Device, GpuAllocator, Vk};

pub struct GpuBuffer {
    pub allocation: Allocation,
    pub handle: vk::Buffer,
    pub size: u32,
}

pub fn allocate_buffer_and_bind(
    allocation_name: &'static str,
    device: &Device,
    allocator: &mut GpuAllocator,
    size: u32,
    usage: BufferUsageFlags,
    location: MemoryLocation,
) -> anyhow::Result<GpuBuffer> {
    let buf =
        allocate_buffer_without_binding(allocation_name, device, allocator, size, usage, location)?;

    if let Err(e) = unsafe {
        device.bind_buffer_memory(buf.handle, buf.allocation.memory(), buf.allocation.offset())
    } {
        allocator.free(buf.allocation)?;
        return Err(e.into());
    }

    Ok(buf)
}

pub fn allocate_buffer_without_binding(
    allocation_name: &'static str,
    device: &Device,
    allocator: &mut GpuAllocator,
    size: u32,
    usage: BufferUsageFlags,
    location: MemoryLocation,
) -> anyhow::Result<GpuBuffer> {
    // Note: this doesn't allocate anything yet!
    let buffer = unsafe {
        device.create_buffer(
            &BufferCreateInfo::builder()
                .flags(BufferCreateFlags::empty())
                .size(size as u64)
                .usage(usage)
                .sharing_mode(SharingMode::EXCLUSIVE)
                .queue_family_indices(&[device.queue_family_idx]),
            None,
        )?
    };

    let buffer_mem_reqs = unsafe { device.get_buffer_memory_requirements(buffer) };

    let allocation = allocator.allocate(&AllocationCreateDesc {
        name: allocation_name,
        requirements: buffer_mem_reqs,
        location: location,
        linear: true,
    })?;

    Ok(GpuBuffer {
        allocation,
        handle: buffer,
        size,
    })
}

pub fn make_shader_module(code: &[u8], vk: &Vk) -> Result<vk::ShaderModule> {
    let spir_v = ash::util::read_spv(&mut std::io::Cursor::new(code))?;

    unsafe {
        let module = vk.device.create_shader_module(
            &vk::ShaderModuleCreateInfo::builder()
                .code(&spir_v)
                .flags(vk::ShaderModuleCreateFlags::empty()),
            None,
        )?;

        Ok(module)
    }
}

pub fn make_shader_stage_create_info(
    module: vk::ShaderModule,
    stage: vk::ShaderStageFlags,
) -> vk::PipelineShaderStageCreateInfo {
    vk::PipelineShaderStageCreateInfo::builder()
        .stage(stage)
        .module(module)
        .name(unsafe { CStr::from_bytes_with_nul_unchecked(b"main\0") })
        .flags(vk::PipelineShaderStageCreateFlags::empty())
        .build()
}

// One monstrosity of a macro to simplify renderpass creation
// No more of forgetting to update attachment index or worrying about that at all
// No more messing with subpasses when you're never going to have more than one

/// Order of attachments determines the `attachment` field you'd have in
/// `vk::AttachmentRef`s.
macro_rules! render_pass {
    (
        device: $device:expr,
        bind_point: $bind_point:expr,
        attachments: [
            // Tricks to allow "either" color or depth attachments at any position
            $(
                depth {
                    // In many cases, the defaults for a depth attachment are enough
                    // so no point requiring any fields; allowing to specify any fields
                    // in any order is much nicer instead
                    $(
                        $depthfieldname1:ident: $depthfieldvalue1:expr
                    ),* $(,)?
                }$(,)?
            )? // Technically should be * to be truly "able to add depth attachment at any position",
            // but you should only have at most one depth attachment anyway
            $(
                color {
                    // This could be replaced similarly to depth attachment fields,
                    // but I think it's nice to 1) enforce this structure, 2)
                    // make sure you never forget any of these, and 3) have the
                    // render_layout here, which is a field of a different struct.
                    format: $format:expr,
                    initial_layout: $initial_layout:expr,
                    render_layout: $layout:expr,
                    final_layout: $final_layout:expr,
                    load_op: $load_op:expr,
                }
                $(
                    , depth {
                        $(
                            $depthfieldname2:ident: $depthfieldvalue2:expr
                        ),* $(,)?
                    }
                )? $(,)?
            )*
        ]$(,)?
        dependencies: [
            $($dependency:expr)*
        ]$(,)?
    ) => {
        {
            let bind_point = $bind_point;
            let mut depth_attachment_ref = ash::vk::AttachmentReference::builder();
            depth_attachment_ref.layout = ash::vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL;
            depth_attachment_ref.attachment = !0;

            let mut attachment_idx = 0;
            $({
                // Match on one of the fields to generate this whenever the depth entry is present
                $(
                    let $depthfieldname1 = 0;
                    _ = $depthfieldname1;
                )*
                attachment_idx += 1;
            };)?
            let color_attachment_refs = [
                $({
                    let attachment_ref = ash::vk::AttachmentReference::builder()
                        .layout($layout)
                        .attachment({
                            let idx = attachment_idx;
                            attachment_idx += 1;
                            idx
                        })
                        .build();

                    // Increment for depth entries
                    $({
                        $(
                            let $depthfieldname2 = 0; // to match on the depth entry
                            _ = $depthfieldname2;
                        )*
                        depth_attachment_ref.attachment = attachment_idx;
                        attachment_idx += 1;
                    };)?
                    attachment_ref
                }),*
            ];
            _ = attachment_idx;

            let attachments = [
                $(
                    {
                        let mut desc = ash::vk::AttachmentDescription::builder()
                            .initial_layout(ash::vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL)
                            .final_layout(ash::vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL)
                            .samples(ash::vk::SampleCountFlags::TYPE_1)
                            .store_op(ash::vk::AttachmentStoreOp::STORE)
                            .build();
                        desc.format = ash::vk::Format::D32_SFLOAT;
                        $(
                            desc.$depthfieldname1 = $depthfieldvalue1;
                        )*
                        desc
                    },
                )?
                $(
                    ash::vk::AttachmentDescription::builder()
                        .format($format)
                        .initial_layout($initial_layout)
                        .final_layout($final_layout)
                        .samples(ash::vk::SampleCountFlags::TYPE_1)
                        .load_op($load_op)
                        .store_op(ash::vk::AttachmentStoreOp::STORE)
                        .build()
                    $(
                        , {
                            let mut desc = ash::vk::AttachmentDescription::builder()
                                .initial_layout(ash::vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL)
                                .final_layout(ash::vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL)
                                .store_op(ash::vk::AttachmentStoreOp::STORE)
                                .build();
                            desc.format = ash::vk::Format::D32_SFLOAT;
                            $(
                                desc.$depthfieldname2 = $depthfieldvalue2;
                            )*
                            desc
                        },
                    )?
                )*
            ];

            let dependencies = [
                $(
                    $dependency.build()
                ),*
            ];

            let mut subpass_desc = ash::vk::SubpassDescription::builder()
                .pipeline_bind_point(bind_point)
                .color_attachments(&color_attachment_refs);

            if depth_attachment_ref.attachment != !0 {
                subpass_desc = subpass_desc
                    .depth_stencil_attachment(&depth_attachment_ref);
            }

            let pass_create_info = ash::vk::RenderPassCreateInfo::builder()
                .attachments(&attachments)
                .subpasses(std::slice::from_ref(&subpass_desc))
                .dependencies(&dependencies)
                .flags(ash::vk::RenderPassCreateFlags::empty());

            #[allow(unused_unsafe)]
            unsafe {
                $device.create_render_pass(&pass_create_info, None)
            }
        }
    }
}

pub(crate) use render_pass;
