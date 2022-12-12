use std::num::NonZeroU32;

use glam::IVec3;
use log::debug;
use vulkano::{
    buffer::{
        sys::{BufferCreateInfo, RawBuffer, Buffer},
        BufferCreateFlags, BufferUsage,
    },
    memory::{
        allocator::MemoryAlloc, DeviceMemory, ExternalMemoryHandleTypes, MemoryAllocateInfo,
        MemoryHeapFlags, MemoryProperties, MemoryPropertyFlags, DedicatedAllocation, MemoryAllocateFlags,
    },
    sync::Sharing, device::physical::PhysicalDeviceType,
};
use xalloc::SysTlsf;

use crate::{vulkan::VkState, Renderer};

/// Represents a block face in a form ready to be processed
/// by the GPU.
pub struct FaceData(pub u64);

/// A view to the mesh of a 16³ region.
pub struct ChunkMeshView<'a> {
    /// The faces to render, grouped and sorted by their normal in this order:
    /// +X, -X, +Y, -Y, +Z, -Z
    pub faces: &'a [FaceData],
    /// Indices to the start of each group of faces in `faces`.
    /// First group (+X) starts at 0, so not included.
    pub axis_offsets: [u32; 5],
}

pub struct RenderChunk {
    /// Number of faces in the chunk.
    num_faces: NonZeroU32,
    /// Offset to the buffer, in faces.
    offset: u32,
}

pub struct RenderWorld {
    chunks: Box<[Option<RenderChunk>]>,
    offset: IVec3,

    gpu_buffer: Buffer,
    chunk_mesh_allocator: SysTlsf<u32>,
}

impl RenderWorld {
    /// pub(crate) because this should definitely be ran only after all other resources
    /// (framebuffers, textures and such) have been allocated, because this allocates
    /// memory very greedily. A detail worth keeping hidden within the crate.
    pub(crate) fn new(player_chunk_pos: IVec3, renderer: &Renderer) -> Self {
        let buffer = allocate_mesh_buffer(&renderer.vk);
        let suballocator = SysTlsf::new(buffer.size() as _);

        Self {
            chunks: vec![].into_boxed_slice(),
            offset: player_chunk_pos,
            gpu_buffer: buffer,
            chunk_mesh_allocator: suballocator,
        }
    }

    pub fn update_chunk_mesh(&mut self, chunk_pos: IVec3, mesh: ChunkMeshView) {}
}

fn allocate_mesh_buffer(vk: &VkState) -> Buffer {
    debug!("Allocating mesh buffer");
    let mem_properties = vk.device.physical_device().memory_properties();

    let mut total_memory = get_device_local_memory_heap_size(mem_properties);
    if vk.device.physical_device().properties().device_type == PhysicalDeviceType::IntegratedGpu {
        // iGPUs share RAM with the CPU, so the reported amount available is massive.
        // Arbitrarily cap to 2GB for these devices. 70% of that is still a fair amount
        total_memory = total_memory.min(1 << 31);
    }

    debug!("Total device-local memory: {total_memory}");
    // Greedily try to allocate until one works
    for percentage in [70, 55, 45, 30, 20, 15] {
        let size = total_memory * percentage / 100;

        if let Ok(buffer) = try_allocate_buffer(vk, size, mem_properties) {
            debug!("Allocation success with p={percentage}%; allocated {} bytes", buffer.size());
            return buffer;
        }
    }
    // Not much you can do to recover here
    panic!("Couldn't allocate GPU memory for chunk meshes!")
}

fn try_allocate_buffer(
    vk: &VkState,
    buffer_size_bytes: usize,
    mem_properties: &MemoryProperties,
) -> anyhow::Result<Buffer> {
    // Note: this doesn't allocate anything yet!
    let buffer = RawBuffer::new(
        vk.device.clone(),
        BufferCreateInfo {
            flags: BufferCreateFlags::default(),
            sharing: Sharing::Exclusive,
            size: buffer_size_bytes as u64,
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
    let memory_type_index = mem_properties
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
        vk.device.clone(),
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

fn get_device_local_memory_heap_size(properties: &MemoryProperties) -> usize {
    // Tricky to implement properly: there is no one API call to get you the
    // total size of device-local memory because that doesn't make sense: the memory
    // can be split over multiple heaps. So yes, multiple heaps may have he device local
    // bit, and not all of them are equally good or even possible candidates...
    // 
    // There is also a bug right now on devices with integrated GPUs, because those show
    // most of the RAM as the 'size'. There is an extension to query a more realistic budget, 
    // which should probably be used:
    // https://registry.khronos.org/vulkan/specs/1.3-extensions/man/html/VkPhysicalDeviceMemoryBudgetPropertiesEXT.html 
    for heap in &properties.memory_heaps {
        if heap.flags.contains(MemoryHeapFlags::DEVICE_LOCAL) {
            return heap.size as usize;
        }
    }
    // There is always at least one device local heap: see description at
    // https://registry.khronos.org/vulkan/specs/1.3-extensions/man/html/VkPhysicalDeviceMemoryProperties.html
    unreachable!()
}
