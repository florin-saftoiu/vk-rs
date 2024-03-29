mod model;
mod tools;
mod types;

#[cfg(debug_assertions)]
use std::ffi::c_void;
use std::ffi::{CStr, CString};
use std::{error::Error, path::Path};

#[cfg(debug_assertions)]
use ash::extensions::ext::DebugUtils;
use ash::{
    extensions::khr::{Surface, Swapchain},
    vk, Device, Entry, Instance,
};
use cgmath::{Deg, Matrix4, Point3, SquareMatrix, Vector3};

use model::{Model, Texture};
use types::{Align16, QueueFamilyIndices, SwapchainSupportDetails, UniformBufferObject, Vertex};

#[cfg(debug_assertions)]
const VALIDATION_LAYERS: [&str; 1] = ["VK_LAYER_KHRONOS_validation"];

const DEVICE_EXTENSIONS: [&str; 1] = ["VK_KHR_swapchain"];
const MAX_FRAMES_IN_FLIGHT: usize = 2;
const MAX_MODELS: usize = 2;

#[cfg(debug_assertions)]
unsafe extern "system" fn vk_debug_utils_callback(
    message_severity: vk::DebugUtilsMessageSeverityFlagsEXT,
    message_type: vk::DebugUtilsMessageTypeFlagsEXT,
    p_callback_data: *const vk::DebugUtilsMessengerCallbackDataEXT,
    _p_user_data: *mut c_void,
) -> vk::Bool32 {
    let msg_severity = match message_severity {
        vk::DebugUtilsMessageSeverityFlagsEXT::VERBOSE => "[VERBOSE]",
        vk::DebugUtilsMessageSeverityFlagsEXT::INFO => "[INFO]",
        vk::DebugUtilsMessageSeverityFlagsEXT::WARNING => "[WARNING]",
        vk::DebugUtilsMessageSeverityFlagsEXT::ERROR => "[ERROR]",
        _ => "[UNKNOWN_SEVERITY]",
    };

    let msg_type = match message_type {
        vk::DebugUtilsMessageTypeFlagsEXT::GENERAL => "[GENERAL]",
        vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE => "[PERFORMANCE]",
        vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION => "[VALIDATION]",
        _ => "[UNKNOWN_TYPE]",
    };

    let msg = CStr::from_ptr((*p_callback_data).p_message);

    println!("{} {} {:?}", msg_severity, msg_type, msg);

    vk::FALSE
}

pub struct Renderer {
    _entry: Entry,
    instance: Instance,
    #[cfg(debug_assertions)]
    debug_utils: Option<(DebugUtils, vk::DebugUtilsMessengerEXT)>,
    physical_device: vk::PhysicalDevice,
    surface_loader: Surface,
    surface: vk::SurfaceKHR,
    device: Device,
    graphics_queue: vk::Queue,
    present_queue: vk::Queue,
    swapchain_loader: Swapchain,
    swapchain: vk::SwapchainKHR,
    swapchain_images: Vec<vk::Image>,
    swapchain_image_format: vk::Format,
    swapchain_extent: vk::Extent2D,
    swapchain_image_views: Vec<vk::ImageView>,
    render_pass: vk::RenderPass,
    global_descriptor_set_layout: vk::DescriptorSetLayout,
    model_descriptor_set_layout: vk::DescriptorSetLayout,
    pipeline_layout: vk::PipelineLayout,
    graphics_pipeline: vk::Pipeline,
    swapchain_framebuffers: Vec<vk::Framebuffer>,
    command_pool: vk::CommandPool,
    command_buffers: Vec<vk::CommandBuffer>,
    image_available_semaphores: Vec<vk::Semaphore>,
    render_finished_semaphores: Vec<vk::Semaphore>,
    in_flight_fences: Vec<vk::Fence>,
    current_frame: usize,
    width: u32,
    height: u32,
    framebuffer_resized: bool,
    models: Vec<Model>,
    global_uniform_buffers: Vec<vk::Buffer>,
    global_uniform_buffers_memory: Vec<vk::DeviceMemory>,
    descriptor_pool: vk::DescriptorPool,
    global_descriptor_sets: Vec<vk::DescriptorSet>,
    texture_sampler: vk::Sampler,
    depth_image: vk::Image,
    depth_image_memory: vk::DeviceMemory,
    depth_image_view: vk::ImageView,
    pub theta: f32,
    pub camera: Point3<f32>,
    pub target: Point3<f32>,
}

impl Renderer {
    pub fn model(&mut self, i: usize) -> &mut Model {
        &mut self.models[i]
    }

    fn cleanup_swapchain(&mut self) {
        unsafe { self.device.destroy_image_view(self.depth_image_view, None) };
        #[cfg(debug_assertions)]
        println!("Depth image view dropped.");

        unsafe { self.device.destroy_image(self.depth_image, None) };
        #[cfg(debug_assertions)]
        println!("Depth image dropped.");

        unsafe { self.device.free_memory(self.depth_image_memory, None) };
        #[cfg(debug_assertions)]
        println!("Depth image memory freed.");

        for framebuffer in self.swapchain_framebuffers.iter() {
            unsafe { self.device.destroy_framebuffer(*framebuffer, None) }
        }
        #[cfg(debug_assertions)]
        println!("Framebuffers dropped.");

        unsafe { self.device.destroy_pipeline(self.graphics_pipeline, None) };
        #[cfg(debug_assertions)]
        println!("Graphics pipeline dropped.");
        unsafe {
            self.device
                .destroy_pipeline_layout(self.pipeline_layout, None)
        };
        #[cfg(debug_assertions)]
        println!("Pipeline layout dropped.");

        unsafe { self.device.destroy_render_pass(self.render_pass, None) };
        #[cfg(debug_assertions)]
        println!("Render pass dropped.");

        for image_view in self.swapchain_image_views.iter() {
            unsafe { self.device.destroy_image_view(*image_view, None) }
        }
        #[cfg(debug_assertions)]
        println!("Swapchain image views dropped.");

        unsafe {
            self.swapchain_loader
                .destroy_swapchain(self.swapchain, None)
        };
        #[cfg(debug_assertions)]
        println!("Swapchain dropped.");
    }

    fn recreate_swapchain(&mut self) -> Result<(), Box<dyn Error>> {
        unsafe { self.device.device_wait_idle() }?;

        self.cleanup_swapchain();

        let swapchain_support_details = Self::query_swapchain_support(
            self.physical_device,
            &self.surface_loader,
            self.surface,
        )?;
        let device_queue_family_indices = Self::find_queue_families(
            &self.instance,
            self.physical_device,
            &self.surface_loader,
            self.surface,
        )?;

        let (swapchain, swapchain_images, swapchain_image_format, swapchain_extent) =
            Self::create_swapchain(
                &self.swapchain_loader,
                &self.surface,
                &swapchain_support_details,
                &device_queue_family_indices,
                self.width,
                self.height,
            )?;

        let swapchain_image_views =
            Self::create_image_views(&self.device, &swapchain_images, swapchain_image_format)?;

        let render_pass = Self::create_render_pass(
            &self.device,
            &self.instance,
            self.physical_device,
            swapchain_image_format,
        )?;

        let (pipeline_layout, graphics_pipeline) = Self::create_graphics_pipeline(
            &self.device,
            swapchain_extent,
            render_pass,
            &[
                self.global_descriptor_set_layout,
                self.model_descriptor_set_layout,
            ],
        )?;

        let (depth_image, depth_image_memory, depth_image_view) = Self::create_depth_resources(
            &self.instance,
            self.physical_device,
            &self.device,
            swapchain_extent,
            self.graphics_queue,
            self.command_pool,
        )?;

        let swapchain_framebuffers = Self::create_framebuffers(
            &self.device,
            &swapchain_image_views,
            swapchain_extent,
            render_pass,
            &depth_image_view,
        )?;

        self.swapchain = swapchain;
        self.swapchain_images = swapchain_images;
        self.swapchain_image_format = swapchain_image_format;
        self.swapchain_extent = swapchain_extent;
        self.swapchain_image_views = swapchain_image_views;
        self.render_pass = render_pass;
        self.pipeline_layout = pipeline_layout;
        self.graphics_pipeline = graphics_pipeline;
        self.depth_image = depth_image;
        self.depth_image_memory = depth_image_memory;
        self.depth_image_view = depth_image_view;
        self.swapchain_framebuffers = swapchain_framebuffers;

        Ok(())
    }

    fn cleanup_model(&self, model: &Model) {
        for i in 0..MAX_FRAMES_IN_FLIGHT {
            unsafe { self.device.destroy_buffer(model.uniform_buffers()[i], None) };
            unsafe {
                self.device
                    .free_memory(model.uniform_buffers_memory()[i], None)
            };
        }
        #[cfg(debug_assertions)]
        println!("Uniform buffers dropped and uniform buffers memory freed.");

        unsafe {
            self.device
                .destroy_image_view(model.texture_image_view(), None)
        };
        #[cfg(debug_assertions)]
        println!("Texture image view dropped.");

        unsafe { self.device.destroy_image(model.texture_image(), None) };
        #[cfg(debug_assertions)]
        println!("Texture image dropped.");

        unsafe { self.device.free_memory(model.texture_image_memory(), None) };
        #[cfg(debug_assertions)]
        println!("Texture image memory freed.");

        unsafe { self.device.destroy_buffer(model.index_buffer(), None) };
        #[cfg(debug_assertions)]
        println!("Index buffer dropped.");

        unsafe { self.device.free_memory(model.index_buffer_memory(), None) };
        #[cfg(debug_assertions)]
        println!("Index buffer memory freed.");

        unsafe { self.device.destroy_buffer(model.vertex_buffer(), None) };
        #[cfg(debug_assertions)]
        println!("Vertex buffer dropped.");

        unsafe { self.device.free_memory(model.vertex_buffer_memory(), None) };
        #[cfg(debug_assertions)]
        println!("Vertex buffer memory freed.");
    }

    fn create_sync_objects(
        device: &Device,
    ) -> Result<(Vec<vk::Semaphore>, Vec<vk::Semaphore>, Vec<vk::Fence>), Box<dyn Error>> {
        let semaphore_info = vk::SemaphoreCreateInfo::default();
        let fence_info = vk::FenceCreateInfo {
            flags: vk::FenceCreateFlags::SIGNALED,
            ..Default::default()
        };

        let mut image_available_semaphores = vec![];
        let mut render_finished_semaphores = vec![];
        let mut in_flight_fences = vec![];

        for _ in 0..MAX_FRAMES_IN_FLIGHT {
            image_available_semaphores
                .push(unsafe { device.create_semaphore(&semaphore_info, None) }?);
            render_finished_semaphores
                .push(unsafe { device.create_semaphore(&semaphore_info, None) }?);
            in_flight_fences.push(unsafe { device.create_fence(&fence_info, None) }?);
        }

        #[cfg(debug_assertions)]
        println!("Sync objects created.");

        Ok((
            image_available_semaphores,
            render_finished_semaphores,
            in_flight_fences,
        ))
    }

    fn record_command_buffer(
        &self,
        command_buffer: vk::CommandBuffer,
        image_index: u32,
    ) -> Result<(), Box<dyn Error>> {
        let begin_info = vk::CommandBufferBeginInfo {
            ..Default::default()
        };
        unsafe {
            self.device
                .begin_command_buffer(command_buffer, &begin_info)
        }?;
        #[cfg(debug_assertions)]
        println!("Begin command buffer.");

        let clear_values = [
            vk::ClearValue {
                color: vk::ClearColorValue {
                    float32: [0f32, 0f32, 0f32, 1f32],
                },
            },
            vk::ClearValue {
                depth_stencil: vk::ClearDepthStencilValue {
                    depth: 1f32,
                    stencil: 0,
                },
            },
        ];
        let render_pass_info = vk::RenderPassBeginInfo {
            render_pass: self.render_pass,
            framebuffer: self.swapchain_framebuffers[image_index as usize],
            render_area: vk::Rect2D {
                extent: self.swapchain_extent,
                ..Default::default()
            },
            clear_value_count: clear_values.len() as u32,
            p_clear_values: clear_values.as_ptr(),
            ..Default::default()
        };
        unsafe {
            self.device.cmd_begin_render_pass(
                command_buffer,
                &render_pass_info,
                vk::SubpassContents::INLINE,
            )
        };
        #[cfg(debug_assertions)]
        println!("Begin render pass command added.");

        unsafe {
            self.device.cmd_bind_pipeline(
                command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                self.graphics_pipeline,
            )
        };
        #[cfg(debug_assertions)]
        println!("Bind graphics pipeline command added.");

        unsafe {
            self.device.cmd_bind_descriptor_sets(
                command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                self.pipeline_layout,
                0,
                &[self.global_descriptor_sets[self.current_frame]],
                &[],
            )
        };
        #[cfg(debug_assertions)]
        println!("Bind global descriptor sets command added.");

        for model in self.models.iter() {
            unsafe {
                self.device.cmd_bind_vertex_buffers(
                    command_buffer,
                    0,
                    &[model.vertex_buffer()],
                    &[0],
                )
            }
            #[cfg(debug_assertions)]
            println!("Bind vertex buffers command added.");

            unsafe {
                self.device.cmd_bind_index_buffer(
                    command_buffer,
                    model.index_buffer(),
                    0,
                    vk::IndexType::UINT32,
                )
            }
            #[cfg(debug_assertions)]
            println!("Bind index buffer command added.");

            unsafe {
                self.device.cmd_bind_descriptor_sets(
                    command_buffer,
                    vk::PipelineBindPoint::GRAPHICS,
                    self.pipeline_layout,
                    1,
                    &[model.descriptor_sets()[self.current_frame]],
                    &[],
                )
            };
            #[cfg(debug_assertions)]
            println!("Bind model descriptor sets command added.");

            unsafe {
                self.device.cmd_draw_indexed(
                    command_buffer,
                    model.indices().len() as u32,
                    1,
                    0,
                    0,
                    0,
                )
            };
            #[cfg(debug_assertions)]
            println!("Draw indexed command added.");
        }

        unsafe { self.device.cmd_end_render_pass(command_buffer) };
        #[cfg(debug_assertions)]
        println!("End render pass command added.");

        unsafe { self.device.end_command_buffer(command_buffer) }?;
        #[cfg(debug_assertions)]
        println!("End command buffer.");

        Ok(())
    }

    fn create_command_buffers(
        device: &Device,
        command_pool: vk::CommandPool,
    ) -> Result<Vec<vk::CommandBuffer>, Box<dyn Error>> {
        let alloc_info = vk::CommandBufferAllocateInfo {
            command_pool,
            command_buffer_count: MAX_FRAMES_IN_FLIGHT as u32,
            ..Default::default()
        };
        let command_buffers = unsafe { device.allocate_command_buffers(&alloc_info) }?;
        #[cfg(debug_assertions)]
        println!("Command buffers allocated.");

        Ok(command_buffers)
    }

    fn find_memory_type(
        instance: &Instance,
        physical_device: &vk::PhysicalDevice,
        type_filter: u32,
        properties: vk::MemoryPropertyFlags,
    ) -> Result<u32, Box<dyn Error>> {
        let mem_properties =
            unsafe { instance.get_physical_device_memory_properties(*physical_device) };

        for i in 0..mem_properties.memory_type_count {
            if type_filter & (1 << i) > 0
                && mem_properties.memory_types[i as usize].property_flags & properties == properties
            {
                return Ok(i);
            }
        }

        Err("No suitable memory type found !")?
    }

    fn create_buffer(
        instance: &Instance,
        physical_device: &vk::PhysicalDevice,
        device: &Device,
        size: vk::DeviceSize,
        usage: vk::BufferUsageFlags,
        properties: vk::MemoryPropertyFlags,
    ) -> Result<(vk::Buffer, vk::DeviceMemory), Box<dyn Error>> {
        let buffer_info = vk::BufferCreateInfo {
            size,
            usage,
            sharing_mode: vk::SharingMode::EXCLUSIVE,
            ..Default::default()
        };
        let buffer = unsafe { device.create_buffer(&buffer_info, None) }?;
        #[cfg(debug_assertions)]
        println!("Buffer created.");

        let mem_requirements = unsafe { device.get_buffer_memory_requirements(buffer) };
        let alloc_info = vk::MemoryAllocateInfo {
            allocation_size: mem_requirements.size,
            memory_type_index: Self::find_memory_type(
                instance,
                physical_device,
                mem_requirements.memory_type_bits,
                properties,
            )?,
            ..Default::default()
        };
        let buffer_memory = unsafe { device.allocate_memory(&alloc_info, None) }?;
        #[cfg(debug_assertions)]
        println!("Buffer memory allocated.");

        unsafe { device.bind_buffer_memory(buffer, buffer_memory, 0) }?;

        Ok((buffer, buffer_memory))
    }

    fn copy_buffer(
        &self,
        src_buffer: vk::Buffer,
        dst_buffer: vk::Buffer,
        size: vk::DeviceSize,
    ) -> Result<(), Box<dyn Error>> {
        let command_buffer = Self::begin_single_time_commands(&self.device, self.command_pool)?;

        let copy_region = vk::BufferCopy {
            src_offset: 0,
            dst_offset: 0,
            size,
            ..Default::default()
        };
        unsafe {
            self.device
                .cmd_copy_buffer(command_buffer, src_buffer, dst_buffer, &[copy_region])
        };
        #[cfg(debug_assertions)]
        println!("Copy command added.");

        Self::end_single_time_commands(
            &self.device,
            command_buffer,
            self.command_pool,
            self.graphics_queue,
        )?;

        Ok(())
    }

    fn create_vertex_buffer(
        &self,
        vertices: &[Vertex],
    ) -> Result<(vk::Buffer, vk::DeviceMemory), Box<dyn Error>> {
        let buffer_size = (std::mem::size_of::<Vertex>() * vertices.len()) as u64;
        let (staging_buffer, staging_buffer_memory) = Self::create_buffer(
            &self.instance,
            &self.physical_device,
            &self.device,
            buffer_size,
            vk::BufferUsageFlags::TRANSFER_SRC,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
        )?;
        #[cfg(debug_assertions)]
        println!("Vertex staging buffer created.");

        let data = unsafe {
            self.device.map_memory(
                staging_buffer_memory,
                0,
                buffer_size,
                vk::MemoryMapFlags::default(),
            )
        }? as *mut Vertex;
        unsafe { data.copy_from_nonoverlapping(vertices.as_ptr(), vertices.len()) };
        unsafe { self.device.unmap_memory(staging_buffer_memory) };
        #[cfg(debug_assertions)]
        println!("Vertex staging buffer memory copied.");

        let (vertex_buffer, vertex_buffer_memory) = Self::create_buffer(
            &self.instance,
            &self.physical_device,
            &self.device,
            buffer_size,
            vk::BufferUsageFlags::TRANSFER_DST | vk::BufferUsageFlags::VERTEX_BUFFER,
            vk::MemoryPropertyFlags::DEVICE_LOCAL,
        )?;
        #[cfg(debug_assertions)]
        println!("Vertex buffer created.");

        self.copy_buffer(staging_buffer, vertex_buffer, buffer_size)?;
        #[cfg(debug_assertions)]
        println!("Vertex staging buffer copied to vertex buffer.");

        unsafe { self.device.destroy_buffer(staging_buffer, None) };
        #[cfg(debug_assertions)]
        println!("Vertex staging buffer dropped.");
        unsafe { self.device.free_memory(staging_buffer_memory, None) };
        #[cfg(debug_assertions)]
        println!("Vertex staging buffer memory freed.");

        Ok((vertex_buffer, vertex_buffer_memory))
    }

    fn create_index_buffer(
        &self,
        indices: &[u32],
    ) -> Result<(vk::Buffer, vk::DeviceMemory), Box<dyn Error>> {
        let buffer_size = (std::mem::size_of::<u32>() * indices.len()) as u64;
        let (staging_buffer, staging_buffer_memory) = Self::create_buffer(
            &self.instance,
            &self.physical_device,
            &self.device,
            buffer_size,
            vk::BufferUsageFlags::TRANSFER_SRC,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
        )?;
        #[cfg(debug_assertions)]
        println!("Index staging buffer created.");

        let data = unsafe {
            self.device.map_memory(
                staging_buffer_memory,
                0,
                buffer_size,
                vk::MemoryMapFlags::default(),
            )
        }? as *mut u32;
        unsafe { data.copy_from_nonoverlapping(indices.as_ptr(), indices.len()) };
        unsafe { self.device.unmap_memory(staging_buffer_memory) };
        #[cfg(debug_assertions)]
        println!("Index staging buffer memory copied.");

        let (index_buffer, index_buffer_memory) = Self::create_buffer(
            &self.instance,
            &self.physical_device,
            &self.device,
            buffer_size,
            vk::BufferUsageFlags::TRANSFER_DST | vk::BufferUsageFlags::INDEX_BUFFER,
            vk::MemoryPropertyFlags::DEVICE_LOCAL,
        )?;
        #[cfg(debug_assertions)]
        println!("Index buffer created.");

        self.copy_buffer(staging_buffer, index_buffer, buffer_size)?;
        #[cfg(debug_assertions)]
        println!("Index staging buffer copied to index buffer.");

        unsafe { self.device.destroy_buffer(staging_buffer, None) };
        #[cfg(debug_assertions)]
        println!("Index staging buffer dropped.");
        unsafe { self.device.free_memory(staging_buffer_memory, None) };
        #[cfg(debug_assertions)]
        println!("Index staging buffer memory freed.");

        Ok((index_buffer, index_buffer_memory))
    }

    fn create_global_uniform_buffers(
        instance: &Instance,
        physical_device: &vk::PhysicalDevice,
        device: &Device,
    ) -> Result<(Vec<vk::Buffer>, Vec<vk::DeviceMemory>), Box<dyn Error>> {
        let buffer_size = std::mem::size_of::<UniformBufferObject>() as u64;

        let mut uniform_buffers = vec![];
        let mut uniform_buffers_memory = vec![];
        for _ in 0..MAX_FRAMES_IN_FLIGHT {
            let (uniform_buffer, uniform_buffer_memory) = Self::create_buffer(
                instance,
                physical_device,
                device,
                buffer_size,
                vk::BufferUsageFlags::UNIFORM_BUFFER,
                vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
            )?;
            uniform_buffers.push(uniform_buffer);
            uniform_buffers_memory.push(uniform_buffer_memory);
        }

        #[cfg(debug_assertions)]
        println!("Uniform buffers and uniform buffers memory created.");

        Ok((uniform_buffers, uniform_buffers_memory))
    }

    fn create_model_uniform_buffers(
        &self,
    ) -> Result<(Vec<vk::Buffer>, Vec<vk::DeviceMemory>), Box<dyn Error>> {
        let buffer_size = std::mem::size_of::<UniformBufferObject>() as u64;

        let mut uniform_buffers = vec![];
        let mut uniform_buffers_memory = vec![];
        for _ in 0..MAX_FRAMES_IN_FLIGHT {
            let (uniform_buffer, uniform_buffer_memory) = Self::create_buffer(
                &self.instance,
                &self.physical_device,
                &self.device,
                buffer_size,
                vk::BufferUsageFlags::UNIFORM_BUFFER,
                vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
            )?;
            uniform_buffers.push(uniform_buffer);
            uniform_buffers_memory.push(uniform_buffer_memory);
        }

        #[cfg(debug_assertions)]
        println!("Uniform buffers and uniform buffers memory created.");

        Ok((uniform_buffers, uniform_buffers_memory))
    }

    fn create_descriptor_pool(device: &Device) -> Result<vk::DescriptorPool, Box<dyn Error>> {
        let pool_sizes = [
            vk::DescriptorPoolSize {
                ty: vk::DescriptorType::UNIFORM_BUFFER,
                descriptor_count: ((1 + MAX_MODELS) * MAX_FRAMES_IN_FLIGHT) as u32,
                ..Default::default()
            },
            vk::DescriptorPoolSize {
                ty: vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
                descriptor_count: (MAX_MODELS * MAX_FRAMES_IN_FLIGHT) as u32,
                ..Default::default()
            },
        ];
        let pool_info = vk::DescriptorPoolCreateInfo {
            pool_size_count: pool_sizes.len() as u32,
            p_pool_sizes: pool_sizes.as_ptr(),
            max_sets: ((1 + MAX_MODELS) * MAX_FRAMES_IN_FLIGHT) as u32,
            ..Default::default()
        };
        let descriptor_pool = unsafe { device.create_descriptor_pool(&pool_info, None) }?;
        #[cfg(debug_assertions)]
        println!("Descriptor pool created.");

        Ok(descriptor_pool)
    }

    fn create_global_descriptor_sets(
        device: &Device,
        descriptor_pool: vk::DescriptorPool,
        descriptor_set_layout: vk::DescriptorSetLayout,
        uniform_buffers: &[vk::Buffer],
    ) -> Result<Vec<vk::DescriptorSet>, Box<dyn Error>> {
        let layouts = vec![descriptor_set_layout; MAX_FRAMES_IN_FLIGHT];
        let alloc_info = vk::DescriptorSetAllocateInfo {
            descriptor_pool,
            descriptor_set_count: MAX_FRAMES_IN_FLIGHT as u32,
            p_set_layouts: layouts.as_ptr(),
            ..Default::default()
        };
        let descriptor_sets = unsafe { device.allocate_descriptor_sets(&alloc_info) }?;
        #[cfg(debug_assertions)]
        println!("Global descriptor sets created.");

        for i in 0..MAX_FRAMES_IN_FLIGHT {
            let buffer_info = vk::DescriptorBufferInfo {
                buffer: uniform_buffers[i],
                offset: 0,
                range: std::mem::size_of::<UniformBufferObject>() as u64,
            };
            let descriptor_writes = [vk::WriteDescriptorSet {
                dst_set: descriptor_sets[i],
                dst_binding: 0,
                dst_array_element: 0,
                descriptor_type: vk::DescriptorType::UNIFORM_BUFFER,
                descriptor_count: 1,
                p_buffer_info: &buffer_info,
                ..Default::default()
            }];
            unsafe { device.update_descriptor_sets(&descriptor_writes, &[]) };
        }

        Ok(descriptor_sets)
    }

    fn create_model_descriptor_sets(
        &self,
        uniform_buffers: &[vk::Buffer],
        texture_image_view: vk::ImageView,
    ) -> Result<Vec<vk::DescriptorSet>, Box<dyn Error>> {
        let layouts = vec![self.model_descriptor_set_layout; MAX_FRAMES_IN_FLIGHT];
        let alloc_info = vk::DescriptorSetAllocateInfo {
            descriptor_pool: self.descriptor_pool,
            descriptor_set_count: MAX_FRAMES_IN_FLIGHT as u32,
            p_set_layouts: layouts.as_ptr(),
            ..Default::default()
        };
        let descriptor_sets = unsafe { self.device.allocate_descriptor_sets(&alloc_info) }?;
        #[cfg(debug_assertions)]
        println!("Model descriptor sets created.");

        for i in 0..MAX_FRAMES_IN_FLIGHT {
            let buffer_info = vk::DescriptorBufferInfo {
                buffer: uniform_buffers[i],
                offset: 0,
                range: std::mem::size_of::<UniformBufferObject>() as u64,
            };
            let image_info = vk::DescriptorImageInfo {
                image_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
                image_view: texture_image_view,
                sampler: self.texture_sampler,
            };
            let descriptor_writes = [
                vk::WriteDescriptorSet {
                    dst_set: descriptor_sets[i],
                    dst_binding: 0,
                    dst_array_element: 0,
                    descriptor_type: vk::DescriptorType::UNIFORM_BUFFER,
                    descriptor_count: 1,
                    p_buffer_info: &buffer_info,
                    ..Default::default()
                },
                vk::WriteDescriptorSet {
                    dst_set: descriptor_sets[i],
                    dst_binding: 1,
                    dst_array_element: 0,
                    descriptor_type: vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
                    descriptor_count: 1,
                    p_image_info: &image_info,
                    ..Default::default()
                },
            ];
            unsafe { self.device.update_descriptor_sets(&descriptor_writes, &[]) };
        }

        Ok(descriptor_sets)
    }

    fn create_command_pool(
        device: &Device,
        device_queue_family_indices: &QueueFamilyIndices,
    ) -> Result<vk::CommandPool, Box<dyn Error>> {
        let pool_info = vk::CommandPoolCreateInfo {
            flags: vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER,
            queue_family_index: device_queue_family_indices
                .graphics_family
                .expect("Missing graphics queue family index !"),
            ..Default::default()
        };
        let command_pool = unsafe { device.create_command_pool(&pool_info, None) }?;
        #[cfg(debug_assertions)]
        println!("Command pool created.");

        Ok(command_pool)
    }

    fn begin_single_time_commands(
        device: &Device,
        command_pool: vk::CommandPool,
    ) -> Result<vk::CommandBuffer, Box<dyn Error>> {
        let alloc_info = vk::CommandBufferAllocateInfo {
            level: vk::CommandBufferLevel::PRIMARY,
            command_pool,
            command_buffer_count: 1,
            ..Default::default()
        };
        let command_buffer = unsafe { device.allocate_command_buffers(&alloc_info) }?[0];
        #[cfg(debug_assertions)]
        println!("Single time command buffer allocated.");

        let begin_info = vk::CommandBufferBeginInfo {
            flags: vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT,
            ..Default::default()
        };
        unsafe { device.begin_command_buffer(command_buffer, &begin_info) }?;
        #[cfg(debug_assertions)]
        println!("Begin single time command buffer.");

        Ok(command_buffer)
    }

    fn end_single_time_commands(
        device: &Device,
        command_buffer: vk::CommandBuffer,
        command_pool: vk::CommandPool,
        graphics_queue: vk::Queue,
    ) -> Result<(), Box<dyn Error>> {
        unsafe { device.end_command_buffer(command_buffer) }?;
        #[cfg(debug_assertions)]
        println!("End single time command buffer.");

        let submit_info = vk::SubmitInfo {
            command_buffer_count: 1,
            p_command_buffers: &command_buffer,
            ..Default::default()
        };
        unsafe { device.queue_submit(graphics_queue, &[submit_info], vk::Fence::null()) }?;
        #[cfg(debug_assertions)]
        println!("Single time command buffer submitted.");
        unsafe { device.queue_wait_idle(graphics_queue) }?;
        #[cfg(debug_assertions)]
        println!("Graphics queue idle.");

        unsafe { device.free_command_buffers(command_pool, &[command_buffer]) };
        #[cfg(debug_assertions)]
        println!("Single time command buffer freed.");

        Ok(())
    }

    fn create_image(
        instance: &Instance,
        physical_device: &vk::PhysicalDevice,
        device: &Device,
        width: u32,
        height: u32,
        format: vk::Format,
        tiling: vk::ImageTiling,
        usage: vk::ImageUsageFlags,
        properties: vk::MemoryPropertyFlags,
    ) -> Result<(vk::Image, vk::DeviceMemory), Box<dyn Error>> {
        let image_info = vk::ImageCreateInfo {
            image_type: vk::ImageType::TYPE_2D,
            extent: vk::Extent3D {
                width,
                height,
                depth: 1,
            },
            mip_levels: 1,
            array_layers: 1,
            format,
            tiling,
            initial_layout: vk::ImageLayout::UNDEFINED,
            usage,
            sharing_mode: vk::SharingMode::EXCLUSIVE,
            samples: vk::SampleCountFlags::TYPE_1,
            ..Default::default()
        };
        let image = unsafe { device.create_image(&image_info, None) }?;
        #[cfg(debug_assertions)]
        println!("Image created.");

        let mem_requirements = unsafe { device.get_image_memory_requirements(image) };
        let alloc_info = vk::MemoryAllocateInfo {
            allocation_size: mem_requirements.size,
            memory_type_index: Self::find_memory_type(
                instance,
                physical_device,
                mem_requirements.memory_type_bits,
                properties,
            )?,
            ..Default::default()
        };
        let image_memory = unsafe { device.allocate_memory(&alloc_info, None) }?;
        #[cfg(debug_assertions)]
        println!("Image memory allocated.");

        unsafe { device.bind_image_memory(image, image_memory, 0) }?;

        Ok((image, image_memory))
    }

    fn copy_buffer_to_image(
        &self,
        buffer: vk::Buffer,
        image: vk::Image,
        width: u32,
        height: u32,
    ) -> Result<(), Box<dyn Error>> {
        let command_buffer = Self::begin_single_time_commands(&self.device, self.command_pool)?;

        let region = vk::BufferImageCopy {
            buffer_offset: 0,
            buffer_row_length: 0,
            buffer_image_height: 0,
            image_subresource: vk::ImageSubresourceLayers {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                mip_level: 0,
                base_array_layer: 0,
                layer_count: 1,
                ..Default::default()
            },
            image_offset: vk::Offset3D { x: 0, y: 0, z: 0 },
            image_extent: vk::Extent3D {
                width,
                height,
                depth: 1,
            },
            ..Default::default()
        };

        unsafe {
            self.device.cmd_copy_buffer_to_image(
                command_buffer,
                buffer,
                image,
                vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                &[region],
            )
        };

        Self::end_single_time_commands(
            &self.device,
            command_buffer,
            self.command_pool,
            self.graphics_queue,
        )?;

        Ok(())
    }

    fn transition_image_layout(
        device: &Device,
        graphics_queue: vk::Queue,
        command_pool: vk::CommandPool,
        image: vk::Image,
        format: vk::Format,
        old_layout: vk::ImageLayout,
        new_layout: vk::ImageLayout,
    ) -> Result<(), Box<dyn Error>> {
        let command_buffer = Self::begin_single_time_commands(device, command_pool)?;

        let src_access_mask;
        let dst_access_mask;
        let source_stage;
        let destination_stage;
        if old_layout == vk::ImageLayout::UNDEFINED
            && new_layout == vk::ImageLayout::TRANSFER_DST_OPTIMAL
        {
            src_access_mask = vk::AccessFlags::empty();
            dst_access_mask = vk::AccessFlags::TRANSFER_WRITE;
            source_stage = vk::PipelineStageFlags::TOP_OF_PIPE;
            destination_stage = vk::PipelineStageFlags::TRANSFER;
        } else if old_layout == vk::ImageLayout::TRANSFER_DST_OPTIMAL
            && new_layout == vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL
        {
            src_access_mask = vk::AccessFlags::TRANSFER_WRITE;
            dst_access_mask = vk::AccessFlags::SHADER_READ;
            source_stage = vk::PipelineStageFlags::TRANSFER;
            destination_stage = vk::PipelineStageFlags::FRAGMENT_SHADER;
        } else if old_layout == vk::ImageLayout::UNDEFINED
            && new_layout == vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL
        {
            src_access_mask = vk::AccessFlags::empty();
            dst_access_mask = vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_READ
                | vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_WRITE;
            source_stage = vk::PipelineStageFlags::TOP_OF_PIPE;
            destination_stage = vk::PipelineStageFlags::EARLY_FRAGMENT_TESTS;
        } else {
            return Err("Unsupported layout transition !")?;
        }

        let barrier = vk::ImageMemoryBarrier {
            old_layout,
            new_layout,
            src_queue_family_index: vk::QUEUE_FAMILY_IGNORED,
            dst_queue_family_index: vk::QUEUE_FAMILY_IGNORED,
            image,
            subresource_range: vk::ImageSubresourceRange {
                aspect_mask: match new_layout {
                    vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL => {
                        let mut aspect_mask = vk::ImageAspectFlags::DEPTH;
                        if Self::has_stencil_component(format) {
                            aspect_mask |= vk::ImageAspectFlags::STENCIL;
                        }
                        aspect_mask
                    }
                    _ => vk::ImageAspectFlags::COLOR,
                },
                base_mip_level: 0,
                level_count: 1,
                base_array_layer: 0,
                layer_count: 1,
                ..Default::default()
            },
            src_access_mask,
            dst_access_mask,
            ..Default::default()
        };

        unsafe {
            device.cmd_pipeline_barrier(
                command_buffer,
                source_stage,
                destination_stage,
                vk::DependencyFlags::empty(),
                &[],
                &[],
                &[barrier],
            )
        };

        Self::end_single_time_commands(device, command_buffer, command_pool, graphics_queue)?;

        Ok(())
    }

    fn create_texture_image(
        &self,
        texture: &Texture,
    ) -> Result<(vk::Image, vk::DeviceMemory), Box<dyn Error>> {
        let image_size = (texture.width() * texture.height() * 4) as vk::DeviceSize;

        let (staging_buffer, staging_buffer_memory) = Self::create_buffer(
            &self.instance,
            &self.physical_device,
            &self.device,
            image_size,
            vk::BufferUsageFlags::TRANSFER_SRC,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
        )?;
        #[cfg(debug_assertions)]
        println!("Texture staging buffer created.");

        let data = unsafe {
            self.device.map_memory(
                staging_buffer_memory,
                0,
                image_size,
                vk::MemoryMapFlags::default(),
            )
        }? as *mut u8;
        unsafe { data.copy_from_nonoverlapping(texture.pixels().as_ptr(), texture.pixels().len()) };
        unsafe { self.device.unmap_memory(staging_buffer_memory) };
        #[cfg(debug_assertions)]
        println!("Texture staging buffer memory copied.");

        let (texture_image, texture_image_memory) = Self::create_image(
            &self.instance,
            &self.physical_device,
            &self.device,
            texture.width(),
            texture.height(),
            vk::Format::R8G8B8A8_SRGB,
            vk::ImageTiling::OPTIMAL,
            vk::ImageUsageFlags::TRANSFER_DST | vk::ImageUsageFlags::SAMPLED,
            vk::MemoryPropertyFlags::DEVICE_LOCAL,
        )?;
        #[cfg(debug_assertions)]
        println!("Texture image created.");

        Self::transition_image_layout(
            &self.device,
            self.graphics_queue,
            self.command_pool,
            texture_image,
            vk::Format::R8G8B8A8_SRGB,
            vk::ImageLayout::UNDEFINED,
            vk::ImageLayout::TRANSFER_DST_OPTIMAL,
        )?;
        self.copy_buffer_to_image(
            staging_buffer,
            texture_image,
            texture.width(),
            texture.height(),
        )?;
        Self::transition_image_layout(
            &self.device,
            self.graphics_queue,
            self.command_pool,
            texture_image,
            vk::Format::R8G8B8A8_SRGB,
            vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
        )?;

        unsafe { self.device.destroy_buffer(staging_buffer, None) };
        #[cfg(debug_assertions)]
        println!("Texture staging buffer dropped.");
        unsafe { self.device.free_memory(staging_buffer_memory, None) };
        #[cfg(debug_assertions)]
        println!("Texture staging buffer memory freed.");

        Ok((texture_image, texture_image_memory))
    }

    fn create_image_view(
        device: &Device,
        image: vk::Image,
        format: vk::Format,
        aspect_flags: vk::ImageAspectFlags,
    ) -> Result<vk::ImageView, Box<dyn Error>> {
        let view_info = vk::ImageViewCreateInfo {
            image,
            view_type: vk::ImageViewType::TYPE_2D,
            format,
            subresource_range: vk::ImageSubresourceRange {
                aspect_mask: aspect_flags,
                base_mip_level: 0,
                level_count: 1,
                base_array_layer: 0,
                layer_count: 1,
                ..Default::default()
            },
            ..Default::default()
        };

        let image_view = unsafe { device.create_image_view(&view_info, None) }?;

        Ok(image_view)
    }

    fn create_texture_image_view(
        &self,
        texture_image: vk::Image,
    ) -> Result<vk::ImageView, Box<dyn Error>> {
        let image_view = Self::create_image_view(
            &self.device,
            texture_image,
            vk::Format::R8G8B8A8_SRGB,
            vk::ImageAspectFlags::COLOR,
        )?;
        #[cfg(debug_assertions)]
        println!("Texture image view created.");

        Ok(image_view)
    }

    fn create_texture_sampler(
        instance: &Instance,
        physical_device: vk::PhysicalDevice,
        device: &Device,
    ) -> Result<vk::Sampler, Box<dyn Error>> {
        let properties = unsafe { instance.get_physical_device_properties(physical_device) };
        let sampler_info = vk::SamplerCreateInfo {
            mag_filter: vk::Filter::LINEAR,
            min_filter: vk::Filter::LINEAR,
            address_mode_u: vk::SamplerAddressMode::REPEAT,
            address_mode_v: vk::SamplerAddressMode::REPEAT,
            address_mode_w: vk::SamplerAddressMode::REPEAT,
            anisotropy_enable: vk::TRUE,
            max_anisotropy: properties.limits.max_sampler_anisotropy,
            border_color: vk::BorderColor::INT_OPAQUE_BLACK,
            unnormalized_coordinates: vk::FALSE,
            compare_enable: vk::FALSE,
            compare_op: vk::CompareOp::ALWAYS,
            mipmap_mode: vk::SamplerMipmapMode::LINEAR,
            mip_lod_bias: 0.0,
            min_lod: 0.0,
            max_lod: 0.0,
            ..Default::default()
        };

        let texture_sampler = unsafe { device.create_sampler(&sampler_info, None) }?;
        #[cfg(debug_assertions)]
        println!("Texture sampler created.");

        Ok(texture_sampler)
    }

    fn create_framebuffers(
        device: &Device,
        swapchain_image_views: &[vk::ImageView],
        swapchain_extent: vk::Extent2D,
        render_pass: vk::RenderPass,
        depth_image_view: &vk::ImageView,
    ) -> Result<Vec<vk::Framebuffer>, Box<dyn Error>> {
        let swapchain_framebuffers = swapchain_image_views
            .iter()
            .map(|swapchain_image_view| {
                let attachments = [*swapchain_image_view, *depth_image_view];
                let framebuffer_info = vk::FramebufferCreateInfo {
                    render_pass,
                    attachment_count: attachments.len() as u32,
                    p_attachments: attachments.as_ptr(),
                    width: swapchain_extent.width,
                    height: swapchain_extent.height,
                    layers: 1,
                    ..Default::default()
                };
                let framebuffer = unsafe { device.create_framebuffer(&framebuffer_info, None) }
                    .expect("Error creating framebuffer !");
                framebuffer
            })
            .collect();
        #[cfg(debug_assertions)]
        println!("Framebuffers created.");

        Ok(swapchain_framebuffers)
    }

    fn create_render_pass(
        device: &Device,
        instance: &Instance,
        physical_device: vk::PhysicalDevice,
        swapchain_image_format: vk::Format,
    ) -> Result<vk::RenderPass, Box<dyn Error>> {
        let color_attachment = vk::AttachmentDescription {
            format: swapchain_image_format,
            samples: vk::SampleCountFlags::TYPE_1,
            load_op: vk::AttachmentLoadOp::CLEAR,
            stencil_load_op: vk::AttachmentLoadOp::DONT_CARE,
            stencil_store_op: vk::AttachmentStoreOp::DONT_CARE,
            initial_layout: vk::ImageLayout::UNDEFINED,
            final_layout: vk::ImageLayout::PRESENT_SRC_KHR,
            ..Default::default()
        };

        let color_attachment_ref = vk::AttachmentReference {
            layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
            ..Default::default()
        };

        let depth_format = Self::find_depth_format(instance, physical_device)?;
        let depth_attachment = vk::AttachmentDescription {
            format: depth_format,
            samples: vk::SampleCountFlags::TYPE_1,
            load_op: vk::AttachmentLoadOp::CLEAR,
            store_op: vk::AttachmentStoreOp::DONT_CARE,
            stencil_load_op: vk::AttachmentLoadOp::DONT_CARE,
            stencil_store_op: vk::AttachmentStoreOp::DONT_CARE,
            initial_layout: vk::ImageLayout::UNDEFINED,
            final_layout: vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL,
            ..Default::default()
        };

        let depth_attachment_ref = vk::AttachmentReference {
            attachment: 1,
            layout: vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL,
            ..Default::default()
        };

        let subpass = vk::SubpassDescription {
            pipeline_bind_point: vk::PipelineBindPoint::GRAPHICS,
            color_attachment_count: 1,
            p_color_attachments: &color_attachment_ref,
            p_depth_stencil_attachment: &depth_attachment_ref,
            ..Default::default()
        };

        let dependency = vk::SubpassDependency {
            src_subpass: vk::SUBPASS_EXTERNAL,
            src_stage_mask: vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT
                | vk::PipelineStageFlags::EARLY_FRAGMENT_TESTS,
            dst_stage_mask: vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT
                | vk::PipelineStageFlags::EARLY_FRAGMENT_TESTS,
            dst_access_mask: vk::AccessFlags::COLOR_ATTACHMENT_WRITE
                | vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_WRITE,
            ..Default::default()
        };

        let attachments = [color_attachment, depth_attachment];
        let render_pass_info = vk::RenderPassCreateInfo {
            attachment_count: attachments.len() as u32,
            p_attachments: attachments.as_ptr(),
            subpass_count: 1,
            p_subpasses: &subpass,
            dependency_count: 1,
            p_dependencies: &dependency,
            ..Default::default()
        };

        let render_pass = unsafe { device.create_render_pass(&render_pass_info, None) }?;
        #[cfg(debug_assertions)]
        println!("Render pass created.");

        Ok(render_pass)
    }

    fn create_shader_module(
        device: &Device,
        shader: &[u8],
    ) -> Result<vk::ShaderModule, Box<dyn Error>> {
        let shader_module_create_info = vk::ShaderModuleCreateInfo {
            code_size: shader.len(),
            p_code: shader.as_ptr() as *const u32,
            ..Default::default()
        };

        let shader_module =
            unsafe { device.create_shader_module(&shader_module_create_info, None) }?;

        Ok(shader_module)
    }

    fn create_global_descriptor_set_layout(
        device: &Device,
    ) -> Result<vk::DescriptorSetLayout, Box<dyn Error>> {
        let bindings = [vk::DescriptorSetLayoutBinding {
            binding: 0,
            descriptor_type: vk::DescriptorType::UNIFORM_BUFFER,
            descriptor_count: 1,
            stage_flags: vk::ShaderStageFlags::VERTEX,
            ..Default::default()
        }];
        let layout_info = vk::DescriptorSetLayoutCreateInfo {
            binding_count: bindings.len() as u32,
            p_bindings: bindings.as_ptr(),
            ..Default::default()
        };
        let descriptor_set_layout =
            unsafe { device.create_descriptor_set_layout(&layout_info, None) }?;

        Ok(descriptor_set_layout)
    }

    fn create_model_descriptor_set_layout(
        device: &Device,
    ) -> Result<vk::DescriptorSetLayout, Box<dyn Error>> {
        let bindings = [
            vk::DescriptorSetLayoutBinding {
                binding: 0,
                descriptor_type: vk::DescriptorType::UNIFORM_BUFFER,
                descriptor_count: 1,
                stage_flags: vk::ShaderStageFlags::VERTEX,
                ..Default::default()
            },
            vk::DescriptorSetLayoutBinding {
                binding: 1,
                descriptor_type: vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
                descriptor_count: 1,
                stage_flags: vk::ShaderStageFlags::FRAGMENT,
                ..Default::default()
            },
        ];
        let layout_info = vk::DescriptorSetLayoutCreateInfo {
            binding_count: bindings.len() as u32,
            p_bindings: bindings.as_ptr(),
            ..Default::default()
        };
        let descriptor_set_layout =
            unsafe { device.create_descriptor_set_layout(&layout_info, None) }?;

        Ok(descriptor_set_layout)
    }

    fn create_graphics_pipeline(
        device: &Device,
        swapchain_extent: vk::Extent2D,
        render_pass: vk::RenderPass,
        descriptor_set_layouts: &[vk::DescriptorSetLayout],
    ) -> Result<(vk::PipelineLayout, vk::Pipeline), Box<dyn Error>> {
        let vert_shader = tools::read_shader(Path::new("shaders/vert.spv"))?;
        let vert_shader_module = Self::create_shader_module(device, &vert_shader)?;
        let vert_shader_entrypoint = CString::new("main").unwrap();
        let vert_shader_stage_info = vk::PipelineShaderStageCreateInfo {
            stage: vk::ShaderStageFlags::VERTEX,
            module: vert_shader_module,
            p_name: vert_shader_entrypoint.as_ptr(),
            ..Default::default()
        };
        #[cfg(debug_assertions)]
        println!("Vertex shader loaded.");

        let frag_shader = tools::read_shader(Path::new("shaders/frag.spv"))?;
        let frag_shader_module = Self::create_shader_module(device, &frag_shader)?;
        let frag_shader_entrypoint = CString::new("main").unwrap();
        let frag_shader_stage_info = vk::PipelineShaderStageCreateInfo {
            stage: vk::ShaderStageFlags::FRAGMENT,
            module: frag_shader_module,
            p_name: frag_shader_entrypoint.as_ptr(),
            ..Default::default()
        };
        #[cfg(debug_assertions)]
        println!("Fragment shader loaded.");

        let shader_stages = [vert_shader_stage_info, frag_shader_stage_info];

        let binding_description = Vertex::get_binding_description();
        let attribute_descriptions = Vertex::get_attribute_descriptions();
        let vertex_input_info = vk::PipelineVertexInputStateCreateInfo {
            vertex_binding_description_count: 1,
            vertex_attribute_description_count: attribute_descriptions.len() as u32,
            p_vertex_binding_descriptions: &binding_description,
            p_vertex_attribute_descriptions: attribute_descriptions.as_ptr(),
            ..Default::default()
        };

        let input_assembly = vk::PipelineInputAssemblyStateCreateInfo {
            topology: vk::PrimitiveTopology::TRIANGLE_LIST,
            primitive_restart_enable: vk::FALSE,
            ..Default::default()
        };

        let viewport = vk::Viewport {
            width: swapchain_extent.width as f32,
            height: swapchain_extent.height as f32,
            max_depth: 1f32,
            ..Default::default()
        };

        let scissor = vk::Rect2D {
            extent: swapchain_extent,
            ..Default::default()
        };

        let viewport_state = vk::PipelineViewportStateCreateInfo {
            viewport_count: 1,
            p_viewports: &viewport,
            scissor_count: 1,
            p_scissors: &scissor,
            ..Default::default()
        };

        let rasterizer = vk::PipelineRasterizationStateCreateInfo {
            line_width: 1f32,
            cull_mode: vk::CullModeFlags::BACK,
            front_face: vk::FrontFace::COUNTER_CLOCKWISE,
            ..Default::default()
        };

        let multisampling = vk::PipelineMultisampleStateCreateInfo {
            rasterization_samples: vk::SampleCountFlags::TYPE_1,
            min_sample_shading: 1f32,
            ..Default::default()
        };

        let color_blend_attachment = vk::PipelineColorBlendAttachmentState {
            color_write_mask: vk::ColorComponentFlags::R
                | vk::ColorComponentFlags::G
                | vk::ColorComponentFlags::B
                | vk::ColorComponentFlags::A,
            src_color_blend_factor: vk::BlendFactor::ONE,
            src_alpha_blend_factor: vk::BlendFactor::ONE,
            ..Default::default()
        };

        let color_blending = vk::PipelineColorBlendStateCreateInfo {
            logic_op: vk::LogicOp::COPY,
            attachment_count: 1,
            p_attachments: &color_blend_attachment,
            ..Default::default()
        };

        let dynamic_states = [vk::DynamicState::VIEWPORT, vk::DynamicState::LINE_WIDTH];

        let _dynamic_state = vk::PipelineDynamicStateCreateInfo {
            dynamic_state_count: 2,
            p_dynamic_states: dynamic_states.as_ptr(),
            ..Default::default()
        };

        let pipeline_layout_info = vk::PipelineLayoutCreateInfo {
            set_layout_count: descriptor_set_layouts.len() as u32,
            p_set_layouts: descriptor_set_layouts.as_ptr(),
            ..Default::default()
        };

        let pipeline_layout =
            unsafe { device.create_pipeline_layout(&pipeline_layout_info, None) }?;
        #[cfg(debug_assertions)]
        println!("Pipeline layout created.");

        let depth_stencil = vk::PipelineDepthStencilStateCreateInfo {
            depth_test_enable: vk::TRUE,
            depth_write_enable: vk::TRUE,
            depth_compare_op: vk::CompareOp::LESS,
            depth_bounds_test_enable: vk::FALSE,
            min_depth_bounds: 0.0,
            max_depth_bounds: 0.0,
            stencil_test_enable: vk::FALSE,
            front: vk::StencilOpState::default(),
            back: vk::StencilOpState::default(),
            ..Default::default()
        };

        let pipeline_infos = [vk::GraphicsPipelineCreateInfo {
            stage_count: 2,
            p_stages: shader_stages.as_ptr(),
            p_vertex_input_state: &vertex_input_info,
            p_input_assembly_state: &input_assembly,
            p_viewport_state: &viewport_state,
            p_rasterization_state: &rasterizer,
            p_multisample_state: &multisampling,
            p_color_blend_state: &color_blending,
            layout: pipeline_layout,
            render_pass,
            base_pipeline_handle: vk::Pipeline::null(),
            base_pipeline_index: -1,
            p_depth_stencil_state: &depth_stencil,
            ..Default::default()
        }];

        let graphics_pipelines = unsafe {
            device.create_graphics_pipelines(vk::PipelineCache::null(), &pipeline_infos, None)
        }
        .expect("Error creating graphics pipeline !");
        #[cfg(debug_assertions)]
        println!("Graphics pipeline created.");

        unsafe { device.destroy_shader_module(frag_shader_module, None) };
        #[cfg(debug_assertions)]
        println!("Fragment shader dropped.");

        unsafe { device.destroy_shader_module(vert_shader_module, None) };
        #[cfg(debug_assertions)]
        println!("Vertex shader dropped.");

        Ok((pipeline_layout, graphics_pipelines[0]))
    }

    fn query_swapchain_support(
        physical_device: vk::PhysicalDevice,
        surface_loader: &Surface,
        surface: vk::SurfaceKHR,
    ) -> Result<SwapchainSupportDetails, Box<dyn Error>> {
        let capabilities = unsafe {
            surface_loader.get_physical_device_surface_capabilities(physical_device, surface)
        }
        .expect("Error querying swapchain capabilities !");

        let formats =
            unsafe { surface_loader.get_physical_device_surface_formats(physical_device, surface) }
                .expect("Error querying swapchain formats !");

        let present_modes = unsafe {
            surface_loader.get_physical_device_surface_present_modes(physical_device, surface)
        }
        .expect("Error querying swapchain present modes !");

        Ok(SwapchainSupportDetails {
            capabilities,
            formats,
            present_modes,
        })
    }

    fn choose_swap_surface_format(
        available_formats: &[vk::SurfaceFormatKHR],
    ) -> vk::SurfaceFormatKHR {
        for available_format in available_formats.iter() {
            if available_format.format == vk::Format::B8G8R8A8_SRGB
                && available_format.color_space == vk::ColorSpaceKHR::SRGB_NONLINEAR
            {
                return *available_format;
            }
        }
        return available_formats[0];
    }

    fn choose_swap_present_mode(
        available_present_modes: &[vk::PresentModeKHR],
    ) -> vk::PresentModeKHR {
        for available_present_mode in available_present_modes.iter() {
            if *available_present_mode == vk::PresentModeKHR::MAILBOX {
                return *available_present_mode;
            }
        }
        return vk::PresentModeKHR::FIFO;
    }

    fn choose_swap_extent(
        capabilities: &vk::SurfaceCapabilitiesKHR,
        width: u32,
        height: u32,
    ) -> vk::Extent2D {
        if capabilities.current_extent.width != u32::MAX {
            return capabilities.current_extent;
        } else {
            vk::Extent2D {
                width: num::clamp(
                    width,
                    capabilities.min_image_extent.width,
                    capabilities.max_image_extent.width,
                ),
                height: num::clamp(
                    height,
                    capabilities.min_image_extent.height,
                    capabilities.max_image_extent.height,
                ),
            }
        }
    }

    fn create_swapchain(
        swapchain_loader: &Swapchain,
        surface: &vk::SurfaceKHR,
        swapchain_support_details: &SwapchainSupportDetails,
        device_queue_family_indices: &QueueFamilyIndices,
        width: u32,
        height: u32,
    ) -> Result<(vk::SwapchainKHR, Vec<vk::Image>, vk::Format, vk::Extent2D), Box<dyn Error>> {
        let surface_format = Self::choose_swap_surface_format(&swapchain_support_details.formats);
        let present_mode = Self::choose_swap_present_mode(&swapchain_support_details.present_modes);
        let extent =
            Self::choose_swap_extent(&swapchain_support_details.capabilities, width, height);
        // Require at least one more image than the minimum to avoid waiting for the driver to complete its job.
        let mut image_count = swapchain_support_details.capabilities.min_image_count + 1;
        if swapchain_support_details.capabilities.max_image_count > 0
            && image_count > swapchain_support_details.capabilities.max_image_count
        {
            image_count = swapchain_support_details.capabilities.max_image_count;
        }

        let swapchain_create_info = vk::SwapchainCreateInfoKHR {
            surface: *surface,
            min_image_count: image_count,
            image_format: surface_format.format,
            image_color_space: surface_format.color_space,
            image_extent: extent,
            image_array_layers: 1,
            image_usage: vk::ImageUsageFlags::COLOR_ATTACHMENT,
            image_sharing_mode: if device_queue_family_indices.graphics_family
                != device_queue_family_indices.present_family
            {
                vk::SharingMode::CONCURRENT
            } else {
                vk::SharingMode::EXCLUSIVE
            },
            queue_family_index_count: if device_queue_family_indices.graphics_family
                != device_queue_family_indices.present_family
            {
                2
            } else {
                0
            },
            p_queue_family_indices: if device_queue_family_indices.graphics_family
                != device_queue_family_indices.present_family
            {
                vec![
                    device_queue_family_indices
                        .graphics_family
                        .expect("Missing graphics queue family index !"),
                    device_queue_family_indices
                        .present_family
                        .expect("Missing present queue family index !"),
                ]
                .as_ptr()
            } else {
                vec![].as_ptr()
            },
            pre_transform: swapchain_support_details.capabilities.current_transform,
            composite_alpha: vk::CompositeAlphaFlagsKHR::OPAQUE,
            present_mode,
            clipped: vk::TRUE,
            old_swapchain: vk::SwapchainKHR::null(),
            ..Default::default()
        };

        let swapchain = unsafe { swapchain_loader.create_swapchain(&swapchain_create_info, None) }?;
        #[cfg(debug_assertions)]
        println!("Swapchain created.");

        let swapchain_images = unsafe { swapchain_loader.get_swapchain_images(swapchain) }?;

        Ok((swapchain, swapchain_images, surface_format.format, extent))
    }

    fn create_image_views(
        device: &Device,
        swapchain_images: &[vk::Image],
        swapchain_image_format: vk::Format,
    ) -> Result<Vec<vk::ImageView>, Box<dyn Error>> {
        let swapchain_image_views = swapchain_images
            .iter()
            .map(|image| {
                let image_view = Self::create_image_view(
                    device,
                    *image,
                    swapchain_image_format,
                    vk::ImageAspectFlags::COLOR,
                )?;
                Ok(image_view)
            })
            .collect();
        #[cfg(debug_assertions)]
        println!("Swapchain image views created.");

        swapchain_image_views
    }

    fn find_supported_format(
        instance: &Instance,
        physical_device: vk::PhysicalDevice,
        candidates: &[vk::Format],
        tiling: vk::ImageTiling,
        features: vk::FormatFeatureFlags,
    ) -> Result<vk::Format, Box<dyn Error>> {
        for &format in candidates.iter() {
            let props =
                unsafe { instance.get_physical_device_format_properties(physical_device, format) };
            if tiling == vk::ImageTiling::LINEAR && props.linear_tiling_features.contains(features)
            {
                return Ok(format);
            } else if tiling == vk::ImageTiling::OPTIMAL
                && props.optimal_tiling_features.contains(features)
            {
                return Ok(format);
            }
        }
        Err("No suitable format found !")?
    }

    fn find_depth_format(
        instance: &Instance,
        physical_device: vk::PhysicalDevice,
    ) -> Result<vk::Format, Box<dyn Error>> {
        Self::find_supported_format(
            instance,
            physical_device,
            &[
                vk::Format::D32_SFLOAT,
                vk::Format::D32_SFLOAT_S8_UINT,
                vk::Format::D24_UNORM_S8_UINT,
            ],
            vk::ImageTiling::OPTIMAL,
            vk::FormatFeatureFlags::DEPTH_STENCIL_ATTACHMENT,
        )
    }

    fn has_stencil_component(format: vk::Format) -> bool {
        format == vk::Format::D32_SFLOAT_S8_UINT || format == vk::Format::D24_UNORM_S8_UINT
    }

    fn create_depth_resources(
        instance: &Instance,
        physical_device: vk::PhysicalDevice,
        device: &Device,
        swapchain_extent: vk::Extent2D,
        graphics_queue: vk::Queue,
        command_pool: vk::CommandPool,
    ) -> Result<(vk::Image, vk::DeviceMemory, vk::ImageView), Box<dyn Error>> {
        let depth_format = Self::find_depth_format(instance, physical_device)?;
        let (depth_image, depth_image_memory) = Self::create_image(
            instance,
            &physical_device,
            device,
            swapchain_extent.width,
            swapchain_extent.height,
            depth_format,
            vk::ImageTiling::OPTIMAL,
            vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT,
            vk::MemoryPropertyFlags::DEVICE_LOCAL,
        )?;
        let depth_image_view = Self::create_image_view(
            device,
            depth_image,
            depth_format,
            vk::ImageAspectFlags::DEPTH,
        )?;

        Self::transition_image_layout(
            device,
            graphics_queue,
            command_pool,
            depth_image,
            depth_format,
            vk::ImageLayout::UNDEFINED,
            vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL,
        )?;

        Ok((depth_image, depth_image_memory, depth_image_view))
    }

    fn find_queue_families(
        instance: &Instance,
        physical_device: vk::PhysicalDevice,
        surface_loader: &Surface,
        surface: vk::SurfaceKHR,
    ) -> Result<QueueFamilyIndices, Box<dyn Error>> {
        // Vulkan commands are submitted in queues. There are multiple families of queues and each family allows certain commands.
        // We need to find the indices of the queue families that allow the commands we need.
        let device_queue_families_properties =
            unsafe { instance.get_physical_device_queue_family_properties(physical_device) };
        let mut device_queue_family_indices = QueueFamilyIndices::default();
        let mut index = 0;
        for device_queue_family_property in device_queue_families_properties.iter() {
            if device_queue_family_property.queue_count > 0
                && device_queue_family_property
                    .queue_flags
                    .contains(vk::QueueFlags::GRAPHICS)
            {
                device_queue_family_indices.graphics_family = Some(index);
            }

            if device_queue_family_property.queue_count > 0
                && unsafe {
                    surface_loader.get_physical_device_surface_support(
                        physical_device,
                        index,
                        surface,
                    )
                }?
            {
                device_queue_family_indices.present_family = Some(index);
            }

            if device_queue_family_indices.is_complete() {
                break;
            }

            index += 1;
        }
        Ok(device_queue_family_indices)
    }

    fn pick_physical_device(
        instance: &Instance,
        surface_loader: &Surface,
        surface: vk::SurfaceKHR,
    ) -> Result<
        (
            vk::PhysicalDevice,
            QueueFamilyIndices,
            SwapchainSupportDetails,
        ),
        Box<dyn Error>,
    > {
        let physical_devices = unsafe { instance.enumerate_physical_devices() }?;

        for &physical_device in physical_devices.iter() {
            let device_properties =
                unsafe { instance.get_physical_device_properties(physical_device) };
            let device_features = unsafe { instance.get_physical_device_features(physical_device) };
            let device_queue_family_indices =
                Self::find_queue_families(instance, physical_device, surface_loader, surface)?;

            if device_properties.device_type == vk::PhysicalDeviceType::DISCRETE_GPU
                && device_features.geometry_shader == vk::TRUE
                && device_queue_family_indices.is_complete()
                && Self::check_device_extensions_support(
                    instance,
                    physical_device,
                    &DEVICE_EXTENSIONS,
                )?
                && device_features.sampler_anisotropy == vk::TRUE
            {
                let swapchain_support_details =
                    Self::query_swapchain_support(physical_device, surface_loader, surface)?;

                if !swapchain_support_details.formats.is_empty()
                    && !swapchain_support_details.present_modes.is_empty()
                {
                    #[cfg(debug_assertions)]
                    {
                        let device_name = unsafe {
                            CStr::from_ptr(device_properties.device_name.as_ptr())
                                .to_str()
                                .to_owned()
                        }?;

                        println!("Found suitable device : {} !", device_name);
                    }

                    return Ok((
                        physical_device,
                        device_queue_family_indices,
                        swapchain_support_details,
                    ));
                }
            }
        }

        Err("No suitable device found !")?
    }

    fn create_logical_device(
        #[cfg(debug_assertions)] enable_validation_layers: bool,
        instance: &Instance,
        physical_device: vk::PhysicalDevice,
        device_queue_family_indices: &QueueFamilyIndices,
    ) -> Result<Device, Box<dyn Error>> {
        let queue_priority = 1f32;
        let device_queue_create_info = vk::DeviceQueueCreateInfo {
            queue_family_index: device_queue_family_indices
                .graphics_family
                .expect("Missing graphics queue family index !"),
            queue_count: 1,
            p_queue_priorities: &queue_priority,
            ..Default::default()
        };
        let device_features = vk::PhysicalDeviceFeatures {
            sampler_anisotropy: vk::TRUE,
            ..Default::default()
        };

        let device_create_info;
        let enabled_extension_names = DEVICE_EXTENSIONS
            .iter()
            .map(|e| CString::new(*e).unwrap())
            .collect::<Vec<CString>>();
        let p_enabled_extension_names = enabled_extension_names
            .iter()
            .map(|e| e.as_ptr())
            .collect::<Vec<*const i8>>();

        #[cfg(debug_assertions)]
        {
            if enable_validation_layers {
                let enabled_layer_names = VALIDATION_LAYERS
                    .iter()
                    .map(|l| CString::new(*l).unwrap())
                    .collect::<Vec<CString>>();
                let p_enabled_layer_names = enabled_layer_names
                    .iter()
                    .map(|l| l.as_ptr())
                    .collect::<Vec<*const i8>>();

                device_create_info = vk::DeviceCreateInfo {
                    p_queue_create_infos: &device_queue_create_info,
                    queue_create_info_count: 1,
                    p_enabled_features: &device_features,
                    enabled_extension_count: p_enabled_extension_names.len() as u32,
                    pp_enabled_extension_names: p_enabled_extension_names.as_ptr(),
                    enabled_layer_count: p_enabled_layer_names.len() as u32,
                    pp_enabled_layer_names: p_enabled_layer_names.as_ptr(),
                    ..Default::default()
                };

                let device = unsafe { instance.create_device(physical_device, &device_create_info, None) }?;
                #[cfg(debug_assertions)]
                println!("Logical device created.");

                Ok(device)
            } else {
                device_create_info = vk::DeviceCreateInfo {
                    p_queue_create_infos: &device_queue_create_info,
                    queue_create_info_count: 1,
                    p_enabled_features: &device_features,
                    enabled_extension_count: p_enabled_extension_names.len() as u32,
                    pp_enabled_extension_names: p_enabled_extension_names.as_ptr(),
                    enabled_layer_count: 0,
                    ..Default::default()
                };

                let device = unsafe { instance.create_device(physical_device, &device_create_info, None) }?;
                #[cfg(debug_assertions)]
                println!("Logical device created.");

                Ok(device)
            }
        }

        #[cfg(not(debug_assertions))]
        {
            device_create_info = vk::DeviceCreateInfo {
                p_queue_create_infos: &device_queue_create_info,
                queue_create_info_count: 1,
                p_enabled_features: &device_features,
                enabled_extension_count: p_enabled_extension_names.len() as u32,
                pp_enabled_extension_names: p_enabled_extension_names.as_ptr(),
                enabled_layer_count: 0,
                ..Default::default()
            };


            let device = unsafe { instance.create_device(physical_device, &device_create_info, None) }?;

            Ok(device)
        }
    }

    #[cfg(debug_assertions)]
    fn check_validation_layers_support(
        entry: &Entry,
        layer_names: &[&str],
    ) -> Result<bool, Box<dyn Error>> {
        let available_layers_properties = entry.enumerate_instance_layer_properties()?;

        println!("Available Vulkan layers :");
        for layer in available_layers_properties.iter() {
            let layer_name = unsafe {
                CStr::from_ptr(layer.layer_name.as_ptr())
                    .to_str()
                    .to_owned()
            }?;
            println!("{}", layer_name);
        }

        for layer_name in layer_names.iter() {
            let mut layer_is_available = false;
            for available_layer in available_layers_properties.iter() {
                let available_layer_name = unsafe {
                    CStr::from_ptr(available_layer.layer_name.as_ptr())
                        .to_str()
                        .to_owned()
                }?;
                if *layer_name == available_layer_name {
                    layer_is_available = true;
                    break;
                }
            }
            if !layer_is_available {
                return Ok(false);
            }
        }

        Ok(true)
    }

    fn check_device_extensions_support(
        instance: &Instance,
        physical_device: vk::PhysicalDevice,
        extension_names: &[&str],
    ) -> Result<bool, Box<dyn Error>> {
        let available_extensions_properties =
            unsafe { instance.enumerate_device_extension_properties(physical_device) }?;

        println!("Available Vulkan extensions :");
        for extension in available_extensions_properties.iter() {
            let extension_name = unsafe {
                CStr::from_ptr(extension.extension_name.as_ptr())
                    .to_str()
                    .to_owned()
            }?;
            println!("{}", extension_name);
        }

        for extension_name in extension_names.iter() {
            let mut extension_is_available = false;
            for available_extension in available_extensions_properties.iter() {
                let available_extension_name = unsafe {
                    CStr::from_ptr(available_extension.extension_name.as_ptr())
                        .to_str()
                        .to_owned()
                }?;
                if *extension_name == available_extension_name {
                    extension_is_available = true;
                    break;
                }
            }
            if !extension_is_available {
                return Ok(false);
            }
        }

        Ok(true)
    }

    #[cfg(debug_assertions)]
    fn new_debug_utils_messenger_create_info() -> vk::DebugUtilsMessengerCreateInfoEXT {
        vk::DebugUtilsMessengerCreateInfoEXT {
            message_severity: vk::DebugUtilsMessageSeverityFlagsEXT::VERBOSE
                | vk::DebugUtilsMessageSeverityFlagsEXT::WARNING
                | vk::DebugUtilsMessageSeverityFlagsEXT::ERROR,
            message_type: vk::DebugUtilsMessageTypeFlagsEXT::GENERAL
                | vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION
                | vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE,
            pfn_user_callback: Some(vk_debug_utils_callback),
            ..Default::default()
        }
    }

    pub fn new(
        display_handle: raw_window_handle::RawDisplayHandle,
        window_handle: raw_window_handle::RawWindowHandle,
        width: u32,
        height: u32,
    ) -> Result<Self, Box<dyn Error>> {
        // Init Vulkan
        // Ash loads Vulkan dynamically, ash::Entry is the library loader and the entrypoint into the Vulkan API.
        let entry = Entry::linked();

        let app_info = vk::ApplicationInfo {
            api_version: vk::make_api_version(0, 1, 0, 0),
            ..Default::default()
        };

        let instance;
        #[cfg(debug_assertions)]
        let debug_utils;
        #[cfg(debug_assertions)]
        let enable_validation_layers =
            Self::check_validation_layers_support(&entry, &VALIDATION_LAYERS)?;

        let required_extensions = ash_window::enumerate_required_extensions(display_handle)?;

        #[cfg(debug_assertions)]
        {
            if enable_validation_layers {
                println!("Validation layers available.");

                let mut enabled_extension_names = required_extensions.to_vec();
                enabled_extension_names.push(DebugUtils::name().as_ptr());

                let enabled_layer_names = VALIDATION_LAYERS
                    .iter()
                    .map(|l| CString::new(*l).unwrap())
                    .collect::<Vec<CString>>();
                let p_enabled_layer_names = enabled_layer_names
                    .iter()
                    .map(|l| l.as_ptr())
                    .collect::<Vec<*const i8>>();

                let instance_debug_utils_messenger_create_info =
                    Self::new_debug_utils_messenger_create_info();

                let create_info = vk::InstanceCreateInfo {
                    p_next: &instance_debug_utils_messenger_create_info
                        as *const vk::DebugUtilsMessengerCreateInfoEXT
                        as *const c_void,
                    p_application_info: &app_info,
                    enabled_layer_count: p_enabled_layer_names.len() as u32,
                    pp_enabled_layer_names: p_enabled_layer_names.as_ptr(),
                    enabled_extension_count: enabled_extension_names.len() as u32,
                    pp_enabled_extension_names: enabled_extension_names.as_ptr(),
                    ..Default::default()
                };

                instance = unsafe { entry.create_instance(&create_info, None) }?;
                println!("Vulkan instance created.");

                let debug_utils_loader = DebugUtils::new(&entry, &instance);
                let messenger_create_info = Self::new_debug_utils_messenger_create_info();
                let debug_utils_messenger = unsafe {
                    debug_utils_loader.create_debug_utils_messenger(&messenger_create_info, None)
                }?;
                println!("Debug messenger created.");

                debug_utils = Some((debug_utils_loader, debug_utils_messenger));
            } else {
                println!("Validation layers not available.");

                let create_info = vk::InstanceCreateInfo {
                    p_application_info: &app_info,
                    enabled_extension_count: required_extensions.len() as u32,
                    pp_enabled_extension_names: required_extensions.as_ptr(),
                    ..Default::default()
                };
                instance = unsafe { entry.create_instance(&create_info, None) }?;
                println!("Vulkan instance created.");

                debug_utils = None;
            }
        }

        #[cfg(not(debug_assertions))]
        {
            let create_info = vk::InstanceCreateInfo {
                p_application_info: &app_info,
                enabled_extension_count: required_extensions.len() as u32,
                pp_enabled_extension_names: required_extensions.as_ptr(),
                ..Default::default()
            };
            instance = unsafe { entry.create_instance(&create_info, None) }?;
        }

        let surface_loader = Surface::new(&entry, &instance);
        let surface = unsafe {
            ash_window::create_surface(&entry, &instance, display_handle, window_handle, None)
        }?;
        #[cfg(debug_assertions)]
        println!("Window surface created.");

        let (physical_device, queue_family_indices, swapchain_support_details) =
            Self::pick_physical_device(&instance, &surface_loader, surface)?;
        let device = Self::create_logical_device(
            #[cfg(debug_assertions)]
            enable_validation_layers,
            &instance,
            physical_device,
            &queue_family_indices,
        )?;

        let graphics_queue = unsafe {
            device.get_device_queue(
                queue_family_indices
                    .graphics_family
                    .expect("Missing graphics queue family index !"),
                0,
            )
        };
        #[cfg(debug_assertions)]
        println!("Graphics queue handle retrieved.");

        let present_queue = unsafe {
            device.get_device_queue(
                queue_family_indices
                    .present_family
                    .expect("Missing present queue family index !"),
                0,
            )
        };
        #[cfg(debug_assertions)]
        println!("Present queue handle retrieved.");

        let swapchain_loader = Swapchain::new(&instance, &device);
        let (swapchain, swapchain_images, swapchain_image_format, swapchain_extent) =
            Self::create_swapchain(
                &swapchain_loader,
                &surface,
                &swapchain_support_details,
                &queue_family_indices,
                width,
                height,
            )?;

        let swapchain_image_views =
            Self::create_image_views(&device, &swapchain_images, swapchain_image_format)?;

        let render_pass =
            Self::create_render_pass(&device, &instance, physical_device, swapchain_image_format)?;

        let global_descriptor_set_layout = Self::create_global_descriptor_set_layout(&device)?;
        let model_descriptor_set_layout = Self::create_model_descriptor_set_layout(&device)?;

        let (pipeline_layout, graphics_pipeline) = Self::create_graphics_pipeline(
            &device,
            swapchain_extent,
            render_pass,
            &[global_descriptor_set_layout, model_descriptor_set_layout],
        )?;

        let command_pool = Self::create_command_pool(&device, &queue_family_indices)?;

        let (depth_image, depth_image_memory, depth_image_view) = Self::create_depth_resources(
            &instance,
            physical_device,
            &device,
            swapchain_extent,
            graphics_queue,
            command_pool,
        )?;

        let swapchain_framebuffers = Self::create_framebuffers(
            &device,
            &swapchain_image_views,
            swapchain_extent,
            render_pass,
            &depth_image_view,
        )?;

        let texture_sampler = Self::create_texture_sampler(&instance, physical_device, &device)?;

        let (global_uniform_buffers, global_uniform_buffers_memory) =
            Self::create_global_uniform_buffers(&instance, &physical_device, &device)?;

        let descriptor_pool = Self::create_descriptor_pool(&device)?;

        let global_descriptor_sets = Self::create_global_descriptor_sets(
            &device,
            descriptor_pool,
            global_descriptor_set_layout,
            &global_uniform_buffers,
        )?;

        let command_buffers = Self::create_command_buffers(&device, command_pool)?;

        let (image_available_semaphores, render_finished_semaphores, in_flight_fences) =
            Self::create_sync_objects(&device)?;

        Ok(Self {
            // The entry has to live as long as the app, otherwise you get an access violation when destroying instance.
            _entry: entry,
            instance,
            #[cfg(debug_assertions)]
            debug_utils,
            physical_device,
            surface_loader,
            surface,
            device,
            graphics_queue,
            present_queue,
            swapchain_loader,
            swapchain,
            swapchain_images,
            swapchain_image_format,
            swapchain_extent,
            swapchain_image_views,
            render_pass,
            global_descriptor_set_layout,
            model_descriptor_set_layout,
            pipeline_layout,
            graphics_pipeline,
            swapchain_framebuffers,
            command_pool,
            command_buffers,
            image_available_semaphores,
            render_finished_semaphores,
            in_flight_fences,
            current_frame: 0,
            width,
            height,
            framebuffer_resized: false,
            models: vec![],
            global_uniform_buffers,
            global_uniform_buffers_memory,
            descriptor_pool,
            global_descriptor_sets,
            texture_sampler,
            depth_image,
            depth_image_memory,
            depth_image_view,
            theta: 0.0,
            camera: Point3 {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            target: Point3 {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
        })
    }

    pub fn load_model(
        &mut self,
        obj: &str,
        texture: &str,
        triangulate: bool,
    ) -> Result<usize, Box<dyn Error>> {
        self.models
            .push(Model::new(&self, obj, texture, triangulate)?);

        Ok(self.models.len() - 1)
    }

    fn update_global_uniform_buffer(&self, current_image: usize) {
        let mut ubo = UniformBufferObject {
            model: Align16(Matrix4::identity()),
            view: Align16(Matrix4::look_at_lh(
                self.camera,
                self.target,
                Vector3::new(0.0, 1.0, 0.0),
            )),
            proj: Align16(cgmath::perspective(
                Deg(90.0),
                self.swapchain_extent.width as f32 / self.swapchain_extent.height as f32,
                0.1,
                1000.0,
            )),
        };
        ubo.proj[1][1] *= -1.0;

        let data = unsafe {
            self.device.map_memory(
                self.global_uniform_buffers_memory[current_image],
                0,
                std::mem::size_of::<UniformBufferObject>() as u64,
                vk::MemoryMapFlags::empty(),
            )
        }
        .expect("Error mapping memory to uniform buffer !")
            as *mut UniformBufferObject;
        unsafe { data.copy_from_nonoverlapping(&ubo as *const UniformBufferObject, 1) };
        unsafe {
            self.device
                .unmap_memory(self.global_uniform_buffers_memory[current_image])
        };
        #[cfg(debug_assertions)]
        println!("Uniform buffer memory copied.");
    }

    fn update_model_uniform_buffer(&self, current_image: usize, model: &Model) {
        let ubo = UniformBufferObject {
            model: Align16(
                Matrix4::from_translation(Vector3::new(
                    model.position.x,
                    model.position.y,
                    model.position.z,
                )) * Matrix4::from_angle_y(Deg(model.theta))
                    * Matrix4::from_angle_x(Deg(model.theta)),
            ),
            view: Align16(Matrix4::identity()),
            proj: Align16(Matrix4::identity()),
        };

        let data = unsafe {
            self.device.map_memory(
                model.uniform_buffers_memory()[current_image],
                0,
                std::mem::size_of::<UniformBufferObject>() as u64,
                vk::MemoryMapFlags::empty(),
            )
        }
        .expect("Error mapping memory to uniform buffer !")
            as *mut UniformBufferObject;
        unsafe { data.copy_from_nonoverlapping(&ubo as *const UniformBufferObject, 1) };
        unsafe {
            self.device
                .unmap_memory(model.uniform_buffers_memory()[current_image])
        };
        #[cfg(debug_assertions)]
        println!("Uniform buffer memory copied.");
    }

    pub fn draw_frame(&mut self) {
        unsafe {
            self.device.wait_for_fences(
                &[self.in_flight_fences[self.current_frame]],
                true,
                u64::MAX,
            )
        }
        .expect("Error waiting for fence !");

        let (image_index, _) = match unsafe {
            self.swapchain_loader.acquire_next_image(
                self.swapchain,
                u64::MAX,
                self.image_available_semaphores[self.current_frame],
                vk::Fence::null(),
            )
        } {
            Ok(result) => result,
            Err(err) => match err {
                vk::Result::ERROR_OUT_OF_DATE_KHR => {
                    self.recreate_swapchain()
                        .expect("Error recreating swapchain !");
                    return;
                }
                _ => panic!("Error acquiring next image !"),
            },
        };

        self.update_global_uniform_buffer(self.current_frame);
        for model in self.models.iter() {
            self.update_model_uniform_buffer(self.current_frame, model);
        }

        unsafe {
            self.device
                .reset_fences(&[self.in_flight_fences[self.current_frame]])
        }
        .expect("Error resetting fence !");

        unsafe {
            self.device.reset_command_buffer(
                self.command_buffers[self.current_frame],
                vk::CommandBufferResetFlags::default(),
            )
        }
        .expect("Error resetting command buffer !");
        self.record_command_buffer(self.command_buffers[self.current_frame], image_index)
            .expect("Error recording command buffer !");

        let wait_semaphores = [self.image_available_semaphores[self.current_frame]];
        let wait_stages = [vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT];
        let signal_semaphores = [self.render_finished_semaphores[self.current_frame]];
        let submit_infos = [vk::SubmitInfo {
            wait_semaphore_count: 1,
            p_wait_semaphores: wait_semaphores.as_ptr(),
            p_wait_dst_stage_mask: wait_stages.as_ptr(),
            command_buffer_count: 1,
            p_command_buffers: &self.command_buffers[self.current_frame],
            signal_semaphore_count: 1,
            p_signal_semaphores: signal_semaphores.as_ptr(),
            ..Default::default()
        }];
        unsafe {
            self.device.queue_submit(
                self.graphics_queue,
                &submit_infos,
                self.in_flight_fences[self.current_frame],
            )
        }
        .expect("Error submitting command buffer !");

        let swapchains = [self.swapchain];
        let present_info = vk::PresentInfoKHR {
            wait_semaphore_count: 1,
            p_wait_semaphores: signal_semaphores.as_ptr(),
            swapchain_count: 1,
            p_swapchains: swapchains.as_ptr(),
            p_image_indices: &image_index,
            ..Default::default()
        };
        let result = unsafe {
            self.swapchain_loader
                .queue_present(self.present_queue, &present_info)
        };
        let framebuffer_resized = match result {
            Ok(_) => self.framebuffer_resized,
            Err(err) => match err {
                vk::Result::ERROR_OUT_OF_DATE_KHR => true,
                _ => panic!("Error presenting to swapchain !"),
            },
        };
        if framebuffer_resized {
            self.framebuffer_resized = false;
            self.recreate_swapchain()
                .expect("Error recreating swapchain !");
        }

        self.current_frame = (self.current_frame + 1) % MAX_FRAMES_IN_FLIGHT;
    }

    pub fn window_resized(&mut self, width: u32, height: u32) {
        self.width = width;
        self.height = height;
        self.framebuffer_resized = true;
    }

    pub fn wait_idle(&self) {
        unsafe {
            self.device
                .device_wait_idle()
                .expect("Error waiting for operations to finish !")
        };
    }
}

impl Drop for Renderer {
    fn drop(&mut self) {
        for model in self.models.iter() {
            self.cleanup_model(model);
        }

        self.cleanup_swapchain();

        unsafe { self.device.destroy_sampler(self.texture_sampler, None) };
        #[cfg(debug_assertions)]
        println!("Texture sampler dropped.");

        for i in 0..MAX_FRAMES_IN_FLIGHT {
            unsafe {
                self.device
                    .destroy_buffer(self.global_uniform_buffers[i], None)
            };
            unsafe {
                self.device
                    .free_memory(self.global_uniform_buffers_memory[i], None)
            };
        }
        #[cfg(debug_assertions)]
        println!("Uniform buffers dropped and uniform buffers memory freed.");

        unsafe {
            self.device
                .destroy_descriptor_pool(self.descriptor_pool, None)
        };
        #[cfg(debug_assertions)]
        println!("Descriptor pool dropped.");

        unsafe {
            self.device
                .destroy_descriptor_set_layout(self.model_descriptor_set_layout, None)
        };
        #[cfg(debug_assertions)]
        println!("Model descriptor set layout dropped.");

        unsafe {
            self.device
                .destroy_descriptor_set_layout(self.global_descriptor_set_layout, None)
        };
        #[cfg(debug_assertions)]
        println!("Global descriptor set layout dropped.");

        for i in 0..MAX_FRAMES_IN_FLIGHT {
            unsafe {
                self.device
                    .destroy_semaphore(self.render_finished_semaphores[i], None)
            };
            unsafe {
                self.device
                    .destroy_semaphore(self.image_available_semaphores[i], None)
            };
            unsafe { self.device.destroy_fence(self.in_flight_fences[i], None) }
        }
        #[cfg(debug_assertions)]
        println!("Sync objects dropped.");

        unsafe { self.device.destroy_command_pool(self.command_pool, None) };
        #[cfg(debug_assertions)]
        println!("Command pool dropped.");

        unsafe { self.device.destroy_device(None) };
        #[cfg(debug_assertions)]
        println!("Logical device dropped.");

        unsafe { self.surface_loader.destroy_surface(self.surface, None) };
        #[cfg(debug_assertions)]
        println!("Window surface dropped.");

        #[cfg(debug_assertions)]
        if let Some((debug_utils_loader, debug_utils_messenger)) = &self.debug_utils {
            unsafe {
                debug_utils_loader.destroy_debug_utils_messenger(*debug_utils_messenger, None)
            };
            println!("Debug messenger dropped.");
        }

        // The ash::Entry used to create the instance has to be alive when calling ash::Instance::destroy_instance.
        unsafe { self.instance.destroy_instance(None) };
        #[cfg(debug_assertions)]
        println!("Vulkan instance dropped.");
    }
}
