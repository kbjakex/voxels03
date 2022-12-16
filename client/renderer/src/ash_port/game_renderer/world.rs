use std::num::NonZeroU32;

use ash::vk::{self, BufferUsageFlags, MemoryHeapFlags};
use bytemuck::{Pod, Zeroable};
use glam::{uvec3, IVec3, UVec3};
use gpu_allocator::MemoryLocation;
use log::debug;
use xalloc::SysTlsf;

use crate::ash_port::{
    vulkan::{self, util::GpuBuffer, Vk},
    RendererBase,
};

use super::state::State;

pub fn build_test_chunk() -> Vec<FaceData> {
    let mut res = Vec::new();

    let faces = [
        Facing::Nx,
        Facing::Ny,
        Facing::Nz,
        Facing::Px,
        Facing::Py,
        Facing::Pz,
    ];

    for x in (0..32).step_by(2) {
        for y in (0..32).step_by(2) {
            for z in (0..32).step_by(2) {
                for face in faces {
                    res.push(FaceData::new(uvec3(x, y, z), face, 0));
                }
            }
        }
    }

    res
}

const XY: u32 = 0b11 << 14;
const XZ: u32 = 0b10 << 14;
const YZ: u32 = 0b01 << 14;
const FLIP: u32 = 0b100 << 14;

#[repr(u32)]
#[derive(Clone, Copy)]
pub enum Facing {
    // Encodes the F and NN in FaceData
    Nx = YZ,
    Ny = XZ,
    Nz = XY,
    Px = YZ | FLIP,
    Py = XZ | FLIP,
    Pz = XY | FLIP,
}

/// Represents a block face in a form ready to be processed
/// by the GPU. The format is:
// [XXXX XYYY][YYZZ ZZZF][NN?? ??II][IIII IIII]
// where
//   X/Y/Z: position, duh
//   F: "flip" (true/false), i.e, whether to push the face vertices along the negative normal by one unit
//   NN: plane: 11 <=> 110 <=> XY, 10 <=> 101 <=> XZ; 01 <=> 011 <=> YZ
//   I: texture id <=> block id
//   ?: unused for now
#[repr(transparent)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct FaceData(u32);

impl FaceData {
    /// xyz: each component should be in range 0..31
    /// block_id: 10 bits max
    #[inline]
    pub const fn new(xyz: UVec3, facing: Facing, block_id: u16) -> Self {
        let mut res = 0;
        res |= block_id as u32;
        res |= facing as u32;
        res |= xyz.z << 17;
        res |= xyz.y << 22;
        res |= xyz.x << 27;
        Self(res)
    }
}

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

    test_chunk_faces: u32,

    gpu_buffer: GpuBuffer,
    chunk_mesh_allocator: SysTlsf<u32>,

    index_buffer: GpuBuffer,
}

// Each face needs 6 indices, and there are 32³/2*6 = 98304 faces, so
// worst case is 32³/2*6*6 = 589824 indices or 2359296 bytes, or 2.25 MiB, for a checkerboard.
// Those shall be placed at the end of the buffer...
const INDEX_BUFFER_SIZE: u32 = 589824;

impl RenderWorld {
    /// pub(crate) because this should definitely be ran only after all other resources
    /// (framebuffers, textures and such) have been allocated, because this allocates
    /// memory very greedily. A detail worth keeping hidden within the crate.
    pub(crate) fn new(
        player_chunk_pos: IVec3,
        renderer: &mut RendererBase,
        state: &State,
    ) -> anyhow::Result<Self> {
        let buffer = allocate_mesh_buffer(&mut renderer.vk);

        let vk = &mut renderer.vk;

        let test_chunk = build_test_chunk();

        let index_buffer = vulkan::util::allocate_buffer_and_bind(
            "Index Buffer",
            &vk.device,
            &mut vk.allocator,
            INDEX_BUFFER_SIZE * std::mem::size_of::<u32>() as u32,
            vk::BufferUsageFlags::INDEX_BUFFER | vk::BufferUsageFlags::TRANSFER_DST,
            MemoryLocation::GpuOnly,
        )?;

        let indices = generate_indices();
        vk.uploader
            .upload_to_buffer(&indices, index_buffer.handle, 0)?;
        vk.uploader
            .upload_to_buffer(&test_chunk, buffer.handle, 0)?;

        let suballocator = SysTlsf::new(buffer.size);

        unsafe {
            vk.device.update_descriptor_sets(
                &[vk::WriteDescriptorSet::builder()
                    .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                    .dst_array_element(0)
                    .dst_binding(0)
                    .dst_set(state.descriptors.full_block.handle)
                    .buffer_info(&[vk::DescriptorBufferInfo::builder()
                        .buffer(buffer.handle)
                        .offset(0)
                        .range(buffer.size as u64)
                        .build()])
                    .build()],
                &[],
            );
        }

        Ok(Self {
            chunks: vec![].into_boxed_slice(),
            offset: player_chunk_pos,

            test_chunk_faces: test_chunk.len() as u32,

            gpu_buffer: buffer,
            chunk_mesh_allocator: suballocator,
            index_buffer,
        })
    }

    pub fn render(&mut self, cmd: vk::CommandBuffer, vk: &Vk, state: &State) -> anyhow::Result<()> {
        unsafe {
            vk.device.cmd_bind_descriptor_sets(
                cmd,
                vk::PipelineBindPoint::GRAPHICS,
                state.full_block_pipeline.layout,
                0,
                &[state.descriptors.full_block.handle],
                &[],
            );
            vk.device.cmd_bind_pipeline(
                cmd,
                vk::PipelineBindPoint::GRAPHICS,
                state.full_block_pipeline.handle,
            );
            vk.device.cmd_bind_index_buffer(
                cmd,
                self.index_buffer.handle,
                0,
                vk::IndexType::UINT32,
            );
            vk.device
                .cmd_draw_indexed(cmd, self.test_chunk_faces * 6, 1, 0, 0, 0);
        }

        Ok(())
    }

    pub fn update_chunk_mesh(&mut self, chunk_pos: IVec3, mesh: ChunkMeshView) {}
}

fn allocate_mesh_buffer(vk: &mut Vk) -> GpuBuffer {
    debug!("Allocating mesh buffer");
    let mem_properties = &vk.device.mem_properties;

    let mut total_memory = get_device_local_memory_heap_size(mem_properties);
    if vk.device.kind != vk::PhysicalDeviceType::DISCRETE_GPU {
        // iGPUs share RAM with the CPU, so the reported amount available is massive.
        // Arbitrarily cap to 2GB for these devices. 70% of that is still a fair amount
        total_memory = total_memory.min(1 << 31);
    }

    debug!("Total device-local memory: {total_memory}");
    // Greedily try to allocate until one works
    for percentage in [70, 55, 45, 30, 20, 15] {
        let mut size = total_memory * percentage / 100;
        size = size.min(vk.device.limits.max_storage_buffer_range as usize);

        if let Ok(buffer) = vulkan::util::allocate_buffer_and_bind(
            "Mesh buffer",
            &vk.device,
            &mut vk.allocator,
            size as u32,
            BufferUsageFlags::STORAGE_BUFFER | BufferUsageFlags::TRANSFER_DST,
            MemoryLocation::GpuOnly,
        ) {
            debug!(
                "Allocation success with p={percentage}%; allocated {} bytes",
                buffer.size
            );
            return buffer;
        }
    }
    // Not much you can do to recover here
    panic!("Couldn't allocate GPU memory for chunk meshes!")
}

fn get_device_local_memory_heap_size(properties: &vk::PhysicalDeviceMemoryProperties) -> usize {
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

fn generate_indices() -> Vec<u32> {
    (0..INDEX_BUFFER_SIZE)
        .map(|i| [0, 1, 2, 2, 1, 3][i as usize % 6] + i / 6 * 4)
        .collect()
}
