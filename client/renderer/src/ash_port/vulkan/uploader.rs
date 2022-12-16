use ash::vk::{self, BufferUsageFlags};

use anyhow::{bail, Result};
use bytemuck::Pod;
use gpu_allocator::{
    MemoryLocation,
};
use log::debug;

use crate::ash_port::vulkan;

use super::{Device, GpuAllocator, util::GpuBuffer};

const STAGING_BUFFER_SIZE: u64 = 1 << 24; // 16 MiB (same as Sodium)

#[derive(Clone, Copy)]
enum MemCopyOp {
    Buf2Buffer {
        dst: vk::Buffer,
        src_offset: u32,
        dst_offset: u32,
        size: u32,
    },
    Buf2Image {
        dst: vk::Image,
        extent: vk::Extent2D,
        range: vk::ImageSubresourceRange,
        shader_stages: vk::PipelineStageFlags,
        src_offset: u32,
    },
}

struct MipGenData {
    image: vk::Image,
    size: vk::Extent2D,
    range: vk::ImageSubresourceRange,
}

pub struct Uploader {
    pool: vk::CommandPool,
    commands: vk::CommandBuffer,

    upload_fence: vk::Fence,

    buffer: GpuBuffer,
    staging_buffer_head: u32,
    pending_copy_ops: Vec<MemCopyOp>,
    pending_mip_gens: Vec<MipGenData>,

    wait_needed: bool,
}

impl Uploader {
    pub fn new(device: &Device, allocator: &mut GpuAllocator) -> Result<Self> {
        let fence_info = vk::FenceCreateInfo::builder();
        let fence = unsafe { device.create_fence(&fence_info, None) }?;

        let cmd_pool_info =
            vk::CommandPoolCreateInfo::builder().queue_family_index(device.queue_family_idx);

        let cmd_pool = unsafe { device.create_command_pool(&cmd_pool_info, None) }?;
        let cmd_buf_allocate_info = vk::CommandBufferAllocateInfo::builder()
            .command_pool(cmd_pool)
            .level(vk::CommandBufferLevel::PRIMARY)
            .command_buffer_count(1);
        let cmds = unsafe {
            device
                .handle
                .allocate_command_buffers(&cmd_buf_allocate_info)
        }?;

        let buffer = vulkan::util::allocate_buffer_and_bind(
            "Staging Buffer",
            device,
            allocator,
            STAGING_BUFFER_SIZE as _,
            BufferUsageFlags::TRANSFER_SRC,
            MemoryLocation::CpuToGpu,
        )?;

        Ok(Uploader {
            pool: cmd_pool,
            commands: cmds[0],
            upload_fence: fence,
            buffer,
            staging_buffer_head: 0,
            pending_copy_ops: Vec::new(),
            pending_mip_gens: Vec::new(),
            wait_needed: false,
        })
    }

    pub fn destroy_self(&mut self, device: &Device, allocator: &mut GpuAllocator) -> Result<()> {
        unsafe {
            device.handle.destroy_fence(self.upload_fence, None);
            device.handle.destroy_command_pool(self.pool, None);
            device.handle.destroy_buffer(self.buffer.handle, None);
        }
        allocator.free(std::mem::take(&mut self.buffer.allocation))?;
        Ok(())
    }

    pub fn upload_to_buffer<T: Pod>(
        // Pod  => Copy => Clone => Sized
        &mut self,
        data: &[T],
        dst_buf: vk::Buffer,
        dst_buf_offset: u32, // in bytes
    ) -> Result<()> {
        let bytes = bytemuck::cast_slice(data);

        self.upload_bytes_to_buffer(bytes, dst_buf, dst_buf_offset)
    }

    pub fn upload_bytes_to_buffer(
        &mut self,
        data: &[u8],
        dst: vk::Buffer,
        offset: u32, // offset in bytes to the dst buffer
    ) -> Result<()> {
        if data.is_empty() {
            return Ok(());
        }

        if self.staging_buffer_head + data.len() as u32 >= self.buffer.allocation.size() as u32 {
            // TODO: What should actually be done is allocating another buffer,
            // but so far this has never happened, and that adds a non-trivial
            // amount of complexity
            bail!(
                "Staging buffer ran out of space! Uploaded {} bytes, head was at {}/{}",
                data.len(),
                self.staging_buffer_head,
                self.buffer.allocation.size()
            );
        }

        unsafe {
            // unwrap(): Some is always returned when memory is host-visible, which is the whole point here
            let mapped_ptr = self.buffer.allocation.mapped_ptr().unwrap().as_ptr().offset(self.staging_buffer_head as isize);
            std::ptr::copy_nonoverlapping(data.as_ptr(), mapped_ptr.cast(), data.len());
        }

        debug!(
            "Queued buffer copy of {} bytes with dst offset {offset}",
            data.len()
        );
        self.pending_copy_ops.push(MemCopyOp::Buf2Buffer {
            dst,
            src_offset: self.staging_buffer_head,
            dst_offset: offset,
            size: data.len() as _,
        });
        self.staging_buffer_head += data.len() as u32;

        Ok(())
    }

    pub fn flush_staged(&mut self, device: &Device) -> Result<()> {
        self.wait_fence_if_unfinished(device)?;
        unsafe {
            device
                .handle
                .reset_command_pool(self.pool, vk::CommandPoolResetFlags::empty())
        }?;

        unsafe {
            device.handle.begin_command_buffer(
                self.commands,
                &vk::CommandBufferBeginInfo::builder()
                    .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT),
            )
        }?;

        let cmd = self.commands;
        let src = self.buffer.handle;
        for &task in &self.pending_copy_ops {
            match task {
                MemCopyOp::Buf2Buffer {
                    dst,
                    src_offset,
                    dst_offset,
                    size,
                } => unsafe {
                    debug!("Buffer copy of {size} bytes with src offset {src_offset}, dst_offset {dst_offset}");
                    device.handle.cmd_copy_buffer(
                        cmd,
                        src,
                        dst,
                        &[vk::BufferCopy::builder()
                            .dst_offset(dst_offset as _)
                            .src_offset(src_offset as _)
                            .size(size as _)
                            .build()],
                    );
                },
                MemCopyOp::Buf2Image {
                    dst,
                    extent,
                    range,
                    shader_stages,
                    src_offset,
                } => unsafe {
                    device.handle.cmd_pipeline_barrier(
                        cmd,
                        vk::PipelineStageFlags::TOP_OF_PIPE,
                        vk::PipelineStageFlags::TRANSFER,
                        vk::DependencyFlags::empty(),
                        &[],
                        &[],
                        &[vk::ImageMemoryBarrier::builder()
                            .image(dst)
                            .old_layout(vk::ImageLayout::UNDEFINED)
                            .new_layout(vk::ImageLayout::TRANSFER_DST_OPTIMAL)
                            .src_access_mask(vk::AccessFlags::empty())
                            .dst_access_mask(vk::AccessFlags::TRANSFER_WRITE)
                            .subresource_range(range)
                            .build()],
                    );
                    device.handle.cmd_copy_buffer_to_image(
                        cmd,
                        src,
                        dst,
                        vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                        &[vk::BufferImageCopy::builder()
                            .buffer_offset(src_offset as _)
                            .buffer_row_length(0)
                            .buffer_image_height(0)
                            .image_extent(vk::Extent3D {
                                width: extent.width,
                                height: extent.height,
                                depth: 1,
                            })
                            .image_subresource(vk::ImageSubresourceLayers {
                                aspect_mask: vk::ImageAspectFlags::COLOR,
                                mip_level: range.base_mip_level,
                                base_array_layer: range.base_array_layer,
                                layer_count: range.layer_count,
                            })
                            .build()],
                    );
                    device.handle.cmd_pipeline_barrier(
                        cmd,
                        vk::PipelineStageFlags::TRANSFER,
                        shader_stages,
                        vk::DependencyFlags::empty(),
                        &[],
                        &[],
                        &[vk::ImageMemoryBarrier::builder()
                            .image(dst)
                            .old_layout(vk::ImageLayout::TRANSFER_DST_OPTIMAL)
                            .new_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
                            .src_access_mask(vk::AccessFlags::TRANSFER_WRITE)
                            .dst_access_mask(vk::AccessFlags::SHADER_READ)
                            .subresource_range(range)
                            .build()],
                    );
                },
            }
        }

        unsafe { device.handle.end_command_buffer(self.commands) }?;

        unsafe {
            let cmd_buffers = [self.commands];
            device.handle.queue_submit(
                device.queue,
                &[vk::SubmitInfo::builder()
                    .command_buffers(&cmd_buffers)
                    .build()],
                self.upload_fence,
            )
        }?;
        self.wait_needed = true;
        self.pending_copy_ops.clear();
        self.staging_buffer_head = 0;

        if self.pending_mip_gens.is_empty() {
            return Ok(());
        }
        // wait immediately
        self.wait_fence_if_unfinished(device)?;

        unsafe {
            device
                .handle
                .reset_command_pool(self.pool, vk::CommandPoolResetFlags::empty())
        }?;

        unsafe {
            device.handle.begin_command_buffer(
                self.commands,
                &vk::CommandBufferBeginInfo::builder()
                    .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT),
            )
        }?;

        for mip_gen_ops in &self.pending_mip_gens {
            unsafe {
                device.handle.cmd_pipeline_barrier(
                    self.commands,
                    vk::PipelineStageFlags::TOP_OF_PIPE,
                    vk::PipelineStageFlags::TRANSFER,
                    vk::DependencyFlags::empty(),
                    &[],
                    &[],
                    &[vk::ImageMemoryBarrier::builder()
                        .old_layout(vk::ImageLayout::UNDEFINED)
                        .new_layout(vk::ImageLayout::TRANSFER_DST_OPTIMAL)
                        .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                        .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                        .image(mip_gen_ops.image)
                        .subresource_range(mip_gen_ops.range)
                        .build()],
                );
            }

            let mut barrier = vk::ImageMemoryBarrier::builder()
                .image(mip_gen_ops.image)
                .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                .subresource_range(
                    *vk::ImageSubresourceRange::builder()
                        .aspect_mask(mip_gen_ops.range.aspect_mask)
                        .base_array_layer(0)
                        .layer_count(1)
                        .level_count(1),
                );

            for layer in 0..mip_gen_ops.range.layer_count {
                barrier.subresource_range.base_array_layer = layer;
                let mut mip_width = mip_gen_ops.size.width;
                let mut mip_height = mip_gen_ops.size.height;
                for level in 1..mip_gen_ops.range.level_count {
                    barrier.subresource_range.base_mip_level = level - 1;
                    barrier.old_layout = vk::ImageLayout::TRANSFER_DST_OPTIMAL;
                    barrier.new_layout = vk::ImageLayout::TRANSFER_SRC_OPTIMAL;
                    barrier.src_access_mask = vk::AccessFlags::TRANSFER_WRITE;
                    barrier.dst_access_mask = vk::AccessFlags::TRANSFER_READ;

                    unsafe {
                        device.handle.cmd_pipeline_barrier(
                            self.commands,
                            vk::PipelineStageFlags::TRANSFER,
                            vk::PipelineStageFlags::TRANSFER,
                            vk::DependencyFlags::empty(),
                            &[],
                            &[],
                            &[*barrier],
                        );
                    }

                    let sub_width = (mip_width / 2).max(1);
                    let sub_height = (mip_height / 2).max(1);

                    let blit = vk::ImageBlit::builder()
                        .src_offsets([
                            *vk::Offset3D::builder().x(0).y(0).z(0),
                            *vk::Offset3D::builder()
                                .x(mip_width as _)
                                .y(mip_height as _)
                                .z(1),
                        ])
                        .src_subresource(
                            *vk::ImageSubresourceLayers::builder()
                                .aspect_mask(mip_gen_ops.range.aspect_mask)
                                .mip_level(level - 1)
                                .base_array_layer(layer)
                                .layer_count(1),
                        )
                        .dst_offsets([
                            *vk::Offset3D::builder().x(0).y(0).z(0),
                            *vk::Offset3D::builder()
                                .x(sub_width as _)
                                .y(sub_height as _)
                                .z(1),
                        ])
                        .dst_subresource(
                            *vk::ImageSubresourceLayers::builder()
                                .aspect_mask(mip_gen_ops.range.aspect_mask)
                                .mip_level(level as _)
                                .base_array_layer(layer)
                                .layer_count(1),
                        );

                    unsafe {
                        device.handle.cmd_blit_image(
                            self.commands,
                            mip_gen_ops.image,
                            vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
                            mip_gen_ops.image,
                            vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                            &[*blit],
                            vk::Filter::LINEAR,
                        );
                    }

                    barrier.old_layout = vk::ImageLayout::TRANSFER_SRC_OPTIMAL;
                    barrier.new_layout = vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL;
                    barrier.src_access_mask = vk::AccessFlags::TRANSFER_READ;
                    barrier.dst_access_mask = vk::AccessFlags::SHADER_READ;

                    unsafe {
                        device.handle.cmd_pipeline_barrier(
                            self.commands,
                            vk::PipelineStageFlags::TRANSFER,
                            vk::PipelineStageFlags::FRAGMENT_SHADER,
                            vk::DependencyFlags::empty(),
                            &[],
                            &[],
                            &[*barrier],
                        );
                    }

                    if mip_width > 1 {
                        mip_width /= 2;
                    }
                    if mip_height > 1 {
                        mip_height /= 2;
                    }
                }
                barrier.subresource_range.base_mip_level = mip_gen_ops.range.level_count - 1;
                barrier.old_layout = vk::ImageLayout::TRANSFER_DST_OPTIMAL;
                barrier.new_layout = vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL;
                barrier.src_access_mask = vk::AccessFlags::TRANSFER_WRITE;
                barrier.dst_access_mask = vk::AccessFlags::SHADER_READ;

                unsafe {
                    device.handle.cmd_pipeline_barrier(
                        self.commands,
                        vk::PipelineStageFlags::TRANSFER,
                        vk::PipelineStageFlags::FRAGMENT_SHADER,
                        vk::DependencyFlags::empty(),
                        &[],
                        &[],
                        &[*barrier],
                    );
                }
            }
        }

        unsafe { device.handle.end_command_buffer(self.commands) }?;

        unsafe {
            let cmd_buffers = [self.commands];
            device.handle.queue_submit(
                device.queue,
                &[vk::SubmitInfo::builder()
                    .command_buffers(&cmd_buffers)
                    .build()],
                self.upload_fence,
            )
        }?;
        self.wait_needed = true;
        self.pending_mip_gens.clear();

        Ok(())
    }

    pub fn wait_fence_if_unfinished(&mut self, device: &Device) -> Result<()> {
        if self.wait_needed {
            unsafe {
                device
                    .handle
                    .wait_for_fences(&[self.upload_fence], true, u64::MAX)
            }?;
            unsafe { device.handle.reset_fences(&[self.upload_fence]) }?;
            self.wait_needed = false;
        }
        Ok(())
    }
}
