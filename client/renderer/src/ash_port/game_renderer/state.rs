use std::ffi::CStr;

use ash::vk;

use anyhow::Result;
use common::assets;
use log::debug;
use crate::ash_port::vulkan::{Vk, util::{make_shader_module, make_shader_stage_create_info}};

pub struct Pipeline {
    pub handle: vk::Pipeline,
    pub layout: vk::PipelineLayout
}

pub struct State {
    // Render passes
    pub main_render_pass: vk::RenderPass,
    pub main_pass_framebuffers: Vec<vk::Framebuffer>,

    // Pipelines
    pub full_block_pipeline: Pipeline,

    // Descriptor sets
    pub descriptor_pool: vk::DescriptorPool,
    pub full_block_dset_layout: vk::DescriptorSetLayout,
    pub full_block_dset: vk::DescriptorSet,
}

impl State {
    pub fn handle_window_resize(&mut self, vk: &Vk) {
        // TODO
    }
}

pub fn init(vk: &Vk) -> anyhow::Result<State> {
    let wnd_extent = vk.swapchain.surface.extent;

    debug!("{wnd_extent:?}; {}", vk.swapchain.image_views.len());

    let (main_render_pass, main_pass_framebuffers) = unsafe {
        let pass = vk.device.create_render_pass(&vk::RenderPassCreateInfo::builder()
            .attachments(&[
                // Attachment 0:
                vk::AttachmentDescription::builder()
                .initial_layout(vk::ImageLayout::UNDEFINED)
                .final_layout(vk::ImageLayout::PRESENT_SRC_KHR)
                .format(vk.swapchain.surface.format.format)
                .load_op(vk::AttachmentLoadOp::DONT_CARE)
                .store_op(vk::AttachmentStoreOp::STORE)
                .samples(vk::SampleCountFlags::TYPE_1)
                .stencil_load_op(vk::AttachmentLoadOp::DONT_CARE)
                .stencil_store_op(vk::AttachmentStoreOp::DONT_CARE)
                .flags(vk::AttachmentDescriptionFlags::empty())
                .build()
            ])
            .subpasses(&[vk::SubpassDescription::builder()
                .pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS)
                .color_attachments(&[vk::AttachmentReference::builder()
                    .attachment(0) // index to the array passed with attachments() above
                    .layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL) // layout to use during rendering
                    .build()
                ])
                //.depth_stencil_attachment(depth_stencil_attachment)
                .input_attachments(&[])
                .preserve_attachments(&[])
                .resolve_attachments(&[])
                .flags(vk::SubpassDescriptionFlags::empty())
                .build()
            ])
            .dependencies(&[
                // Synchronization for writing to the color attachment
                vk::SubpassDependency::builder()
                .src_subpass(vk::SUBPASS_EXTERNAL)
                .dst_subpass(0)
                .src_access_mask(vk::AccessFlags::empty())
                .dst_access_mask(vk::AccessFlags::COLOR_ATTACHMENT_WRITE)
                .src_stage_mask(vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)
                .dst_stage_mask(vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)
                .dependency_flags(vk::DependencyFlags::BY_REGION)
                .build()
            ])
            .flags(vk::RenderPassCreateFlags::empty())
        , None)?;

        let framebuffers = vk.swapchain.image_views.iter().map(|view| {
            vk.device.create_framebuffer(&vk::FramebufferCreateInfo::builder()
                .render_pass(pass)
                .attachments(&[*view])
                .width(vk.swapchain.surface.extent.width)
                .height(vk.swapchain.surface.extent.height)
                .layers(1)
            , None).unwrap()
        }).collect();
        (pass, framebuffers)
    };

    let descriptor_pool = unsafe {vk.device.create_descriptor_pool(
        &vk::DescriptorPoolCreateInfo::builder()
            .pool_sizes(&[
                vk::DescriptorPoolSize::builder().descriptor_count(1).ty(vk::DescriptorType::STORAGE_BUFFER).build()
            ])
            .max_sets(2)
        , None)?
    };

    let full_block_dset_layout = unsafe { vk.device.create_descriptor_set_layout(&vk::DescriptorSetLayoutCreateInfo::builder()
            .bindings(&[
                vk::DescriptorSetLayoutBinding::builder()
                    .binding(0)
                    .stage_flags(vk::ShaderStageFlags::VERTEX)
                    .descriptor_count(1)
                    .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                    .build()  
            ])
        , None)? 
    };

    let full_block_dset = unsafe { vk.device.allocate_descriptor_sets(
        &vk::DescriptorSetAllocateInfo::builder()
            .descriptor_pool(descriptor_pool)
            .set_layouts(&[full_block_dset_layout])
        )?[0]
    };

    let full_block_pipeline = unsafe {
        let vert_shader = make_shader_module(assets::shaders::TEXTURED_FULL_CUBE_VERT, vk)?;
        let frag_shader = make_shader_module(assets::shaders::TEXTURED_LIT_FRAG, vk)?;

        let layout = vk.device.create_pipeline_layout(&vk::PipelineLayoutCreateInfo::builder()
            .push_constant_ranges(&[])
            .set_layouts(&[full_block_dset_layout])
            .flags(vk::PipelineLayoutCreateFlags::empty())
        , None)?;

        let handle = vk.device.create_graphics_pipelines(vk::PipelineCache::null(), &[
            vk::GraphicsPipelineCreateInfo::builder()
            .render_pass(main_render_pass)
            .layout(layout)
            .stages(&[
                make_shader_stage_create_info(vert_shader, vk::ShaderStageFlags::VERTEX),
                make_shader_stage_create_info(frag_shader, vk::ShaderStageFlags::FRAGMENT)
            ])
            .depth_stencil_state(&vk::PipelineDepthStencilStateCreateInfo::builder()
                .depth_test_enable(false)
                .stencil_test_enable(false)
            )
            .input_assembly_state(&vk::PipelineInputAssemblyStateCreateInfo::builder()
                .primitive_restart_enable(false)
                .topology(vk::PrimitiveTopology::TRIANGLE_LIST)
            )
            .dynamic_state(&vk::PipelineDynamicStateCreateInfo::builder()
                .dynamic_states(&[])
            )
            .rasterization_state(&vk::PipelineRasterizationStateCreateInfo::builder()
                .cull_mode(vk::CullModeFlags::BACK)
                .depth_bias_enable(false)
                .depth_clamp_enable(false)
                .front_face(vk::FrontFace::COUNTER_CLOCKWISE)
                .rasterizer_discard_enable(false)
                .polygon_mode(vk::PolygonMode::FILL)
                .line_width(1.0)
            )
            .viewport_state(&vk::PipelineViewportStateCreateInfo::builder()
                .scissors(&[vk::Rect2D::builder()
                    .offset(vk::Offset2D::builder().build())
                    .extent(wnd_extent)
                    .build()
                ])
                .viewport_count(1)
                .viewports(&[vk::Viewport::builder()
                    .x(0.0)
                    .y(wnd_extent.height as f32)
                    .width(wnd_extent.width as f32)
                    .height(-(wnd_extent.height as f32))
                    .min_depth(0.0)
                    .max_depth(1.0)
                    .build(),
                ])
            )
            .vertex_input_state(&vk::PipelineVertexInputStateCreateInfo::builder()
                .vertex_attribute_descriptions(&[])
                .vertex_binding_descriptions(&[])
                .flags(vk::PipelineVertexInputStateCreateFlags::empty())
                .build()
            )
            .color_blend_state(&vk::PipelineColorBlendStateCreateInfo::builder()
                .attachments(&[])
                .logic_op_enable(false)
            )
            .multisample_state(&vk::PipelineMultisampleStateCreateInfo::builder()
                .sample_shading_enable(false)
                .rasterization_samples(vk::SampleCountFlags::TYPE_1)
            )
            .base_pipeline_handle(vk::Pipeline::null())
            .subpass(0)
            .flags(vk::PipelineCreateFlags::empty())
            .build()
        ], None).unwrap()[0];

        vk.device.destroy_shader_module(vert_shader, None);
        vk.device.destroy_shader_module(frag_shader, None);

        Pipeline {
            handle,
            layout,
        }
    };

    Ok(State {
        main_render_pass,
        main_pass_framebuffers,
        
        full_block_pipeline,
        
        descriptor_pool,
        full_block_dset_layout,
        full_block_dset,
    })
}
