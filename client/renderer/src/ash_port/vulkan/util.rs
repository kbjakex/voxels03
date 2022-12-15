use std::ffi::CStr;

use anyhow::Result;
use ash::vk::{BufferUsageFlags, self, BufferCreateInfo, BufferCreateFlags, SharingMode};
use gpu_allocator::{vulkan::{Allocation, AllocationCreateDesc}, MemoryLocation};

use super::Vk;


pub struct GpuBuffer {
    pub allocation: Allocation,
    pub buffer: vk::Buffer,
    pub size: u32
}

pub fn allocate_buffer(
    allocation_name: &'static str,
    vk: &Vk,
    size: u32,
    usage: BufferUsageFlags,
    location: MemoryLocation,
) -> anyhow::Result<GpuBuffer> {
    // Note: this doesn't allocate anything yet!
    let buffer = unsafe {
        vk.device.handle.create_buffer(&BufferCreateInfo::builder()
            .flags(BufferCreateFlags::empty())
            .size(size as u64)
            .usage(usage)
            .sharing_mode(SharingMode::EXCLUSIVE)
            .queue_family_indices(&[vk.device.queue_family_idx])
        , None)?
    };

    let buffer_mem_reqs = unsafe {
        vk.device.handle.get_buffer_memory_requirements(buffer)
    };

    let allocation = vk.allocator.borrow_mut().allocate(&AllocationCreateDesc {
        name: allocation_name,
        requirements: buffer_mem_reqs,
        location: location,
        linear: true,
    })?;

    Ok(GpuBuffer { allocation, buffer, size })
}

pub fn make_shader_module(code: &[u8], vk: &Vk) -> Result<vk::ShaderModule> {
    let spir_v = ash::util::read_spv(&mut std::io::Cursor::new(code))?;

    unsafe {
        let module = vk.device.create_shader_module(&vk::ShaderModuleCreateInfo::builder()
            .code(&spir_v)
            .flags(vk::ShaderModuleCreateFlags::empty())
        , None)?;

        Ok(module)
    }
}

pub fn make_shader_stage_create_info(module: vk::ShaderModule, stage: vk::ShaderStageFlags) -> vk::PipelineShaderStageCreateInfo {
    vk::PipelineShaderStageCreateInfo::builder()
        .stage(stage)
        .module(module)
        .name(unsafe { CStr::from_bytes_with_nul_unchecked(b"main\0") })
        .flags(vk::PipelineShaderStageCreateFlags::empty())
        .build()
}