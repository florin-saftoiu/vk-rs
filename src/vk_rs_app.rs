#[cfg(debug_assertions)]
use std::ffi::c_void;
use std::ffi::{CStr, CString};
use std::{error::Error, fs::File, io::Read, path::Path};

#[cfg(debug_assertions)]
use ash::extensions::ext::DebugUtils;
use ash::{
    extensions::khr::{Surface, Swapchain},
    vk, Device, Entry, Instance,
};

#[cfg(debug_assertions)]
const VALIDATION_LAYERS: [&str; 1] = ["VK_LAYER_KHRONOS_validation"];

const DEVICE_EXTENSIONS: [&str; 1] = ["VK_KHR_swapchain"];

fn read_shader(path: &Path) -> Result<Vec<u8>, Box<dyn Error>> {
    let spv = File::open(path)?;
    Ok(spv.bytes().filter_map(|b| b.ok()).collect::<Vec<u8>>())
}

#[derive(Default)]
struct QueueFamilyIndices {
    graphics_family: Option<u32>,
    present_family: Option<u32>,
}

impl QueueFamilyIndices {
    pub fn is_complete(&self) -> bool {
        self.graphics_family.is_some() && self.present_family.is_some()
    }
}

struct SwapChainSupportDetails {
    capabilities: vk::SurfaceCapabilitiesKHR,
    formats: Vec<vk::SurfaceFormatKHR>,
    present_modes: Vec<vk::PresentModeKHR>,
}

pub struct VkRsApp {
    _entry: Entry,
    instance: Instance,
    #[cfg(debug_assertions)]
    debug_utils: Option<(DebugUtils, vk::DebugUtilsMessengerEXT)>,
    _physical_device: vk::PhysicalDevice,
    surface: (vk::SurfaceKHR, Surface),
    device: Device,
    _graphics_queue: vk::Queue,
    _present_queue: vk::Queue,
    swap_chain: (
        vk::SwapchainKHR,
        Swapchain,
        Vec<vk::Image>,
        vk::Format,
        vk::Extent2D,
    ),
    swap_chain_image_views: Vec<vk::ImageView>,
    pipeline_layout: vk::PipelineLayout,
}

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

impl VkRsApp {
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

    fn create_graphics_pipeline(
        device: &Device,
        swap_chain_extent: vk::Extent2D,
    ) -> Result<vk::PipelineLayout, Box<dyn Error>> {
        let vert_shader = read_shader(Path::new("shaders/vert.spv"))?;
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

        let frag_shader = read_shader(Path::new("shaders/frag.spv"))?;
        let frag_shader_module = Self::create_shader_module(device, &frag_shader)?;
        let frag_shader_entrypoint = CString::new("main").unwrap();
        let frag_shader_stage_info = vk::PipelineShaderStageCreateInfo {
            stage: vk::ShaderStageFlags::VERTEX,
            module: frag_shader_module,
            p_name: frag_shader_entrypoint.as_ptr(),
            ..Default::default()
        };
        #[cfg(debug_assertions)]
        println!("Fragment shader loaded.");

        let _shader_stages = [vert_shader_stage_info, frag_shader_stage_info];

        let _vertex_input_info = vk::PipelineVertexInputStateCreateInfo {
            ..Default::default()
        };

        let _input_assembly = vk::PipelineInputAssemblyStateCreateInfo {
            topology: vk::PrimitiveTopology::TRIANGLE_LIST,
            primitive_restart_enable: vk::FALSE,
            ..Default::default()
        };

        let viewport = vk::Viewport {
            width: swap_chain_extent.width as f32,
            height: swap_chain_extent.height as f32,
            max_depth: 1f32,
            ..Default::default()
        };

        let scissor = vk::Rect2D {
            extent: swap_chain_extent,
            ..Default::default()
        };

        let _viewport_state = vk::PipelineViewportStateCreateInfo {
            viewport_count: 1,
            p_viewports: &viewport,
            scissor_count: 1,
            p_scissors: &scissor,
            ..Default::default()
        };

        let _rasterizer = vk::PipelineRasterizationStateCreateInfo {
            line_width: 1f32,
            cull_mode: vk::CullModeFlags::BACK,
            front_face: vk::FrontFace::CLOCKWISE,
            ..Default::default()
        };

        let _multisampling = vk::PipelineMultisampleStateCreateInfo {
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

        let _color_blending = vk::PipelineColorBlendStateCreateInfo {
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
            ..Default::default()
        };

        let pipeline_layout =
            unsafe { device.create_pipeline_layout(&pipeline_layout_info, None) }?;
        #[cfg(debug_assertions)]
        println!("Pipeline layout created.");

        unsafe { device.destroy_shader_module(frag_shader_module, None) };
        #[cfg(debug_assertions)]
        println!("Fragment shader dropped.");

        unsafe { device.destroy_shader_module(vert_shader_module, None) };
        #[cfg(debug_assertions)]
        println!("Vertex shader dropped.");

        Ok(pipeline_layout)
    }

    fn query_swap_chain_support(
        physical_device: vk::PhysicalDevice,
        surface: vk::SurfaceKHR,
        surface_loader: &Surface,
    ) -> Result<SwapChainSupportDetails, Box<dyn Error>> {
        let capabilities = unsafe {
            surface_loader.get_physical_device_surface_capabilities(physical_device, surface)
        }
        .expect("Error querying swap chain capabilities !");

        let formats =
            unsafe { surface_loader.get_physical_device_surface_formats(physical_device, surface) }
                .expect("Error querying swap chain formats !");

        let present_modes = unsafe {
            surface_loader.get_physical_device_surface_present_modes(physical_device, surface)
        }
        .expect("Error querying swap chain present modes !");

        Ok(SwapChainSupportDetails {
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

    fn create_swap_chain(
        instance: &Instance,
        device: &Device,
        surface: &vk::SurfaceKHR,
        swap_chain_support_details: &SwapChainSupportDetails,
        device_queue_family_indices: &QueueFamilyIndices,
        width: u32,
        height: u32,
    ) -> Result<
        (
            vk::SwapchainKHR,
            Swapchain,
            Vec<vk::Image>,
            vk::Format,
            vk::Extent2D,
        ),
        Box<dyn Error>,
    > {
        let surface_format = Self::choose_swap_surface_format(&swap_chain_support_details.formats);
        let present_mode =
            Self::choose_swap_present_mode(&swap_chain_support_details.present_modes);
        let extent =
            Self::choose_swap_extent(&swap_chain_support_details.capabilities, width, height);
        // Require at least one more image than the minimum to avoid waiting for the driver to complete its job.
        let mut image_count = swap_chain_support_details.capabilities.min_image_count + 1;
        if swap_chain_support_details.capabilities.max_image_count > 0
            && image_count > swap_chain_support_details.capabilities.max_image_count
        {
            image_count = swap_chain_support_details.capabilities.max_image_count;
        }

        let swap_chain_create_info = vk::SwapchainCreateInfoKHR {
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
            pre_transform: swap_chain_support_details.capabilities.current_transform,
            composite_alpha: vk::CompositeAlphaFlagsKHR::OPAQUE,
            present_mode,
            clipped: vk::TRUE,
            old_swapchain: vk::SwapchainKHR::null(),
            ..Default::default()
        };

        let swap_chain_loader = Swapchain::new(instance, device);
        let swap_chain =
            unsafe { swap_chain_loader.create_swapchain(&swap_chain_create_info, None) }?;
        #[cfg(debug_assertions)]
        println!("Swap chain created.");

        let swap_chain_images = unsafe { swap_chain_loader.get_swapchain_images(swap_chain) }?;

        Ok((
            swap_chain,
            swap_chain_loader,
            swap_chain_images,
            surface_format.format,
            extent,
        ))
    }

    fn create_image_views(
        device: &Device,
        swap_chain_images: &[vk::Image],
        swap_chain_image_format: vk::Format,
    ) -> Result<Vec<vk::ImageView>, Box<dyn Error>> {
        let swap_chain_image_views = swap_chain_images
            .iter()
            .map(|image| {
                let image_view_create_info = vk::ImageViewCreateInfo {
                    image: *image,
                    view_type: vk::ImageViewType::TYPE_2D,
                    format: swap_chain_image_format,
                    components: vk::ComponentMapping {
                        r: vk::ComponentSwizzle::IDENTITY,
                        g: vk::ComponentSwizzle::IDENTITY,
                        b: vk::ComponentSwizzle::IDENTITY,
                        a: vk::ComponentSwizzle::IDENTITY,
                    },
                    subresource_range: vk::ImageSubresourceRange {
                        aspect_mask: vk::ImageAspectFlags::COLOR,
                        base_mip_level: 0,
                        level_count: 1,
                        base_array_layer: 0,
                        layer_count: 1,
                    },
                    ..Default::default()
                };
                let image_view = unsafe { device.create_image_view(&image_view_create_info, None) }
                    .expect("Error creating swap chain image view !");
                image_view
            })
            .collect();
        #[cfg(debug_assertions)]
        println!("Swap chain image views created.");

        Ok(swap_chain_image_views)
    }

    fn find_queue_families(
        instance: &Instance,
        physical_device: vk::PhysicalDevice,
        surface: vk::SurfaceKHR,
        surface_loader: &Surface,
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
                        index as u32,
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
        surface: vk::SurfaceKHR,
        surface_loader: &Surface,
    ) -> Result<
        (
            vk::PhysicalDevice,
            QueueFamilyIndices,
            SwapChainSupportDetails,
        ),
        Box<dyn Error>,
    > {
        let physical_devices = unsafe { instance.enumerate_physical_devices() }?;

        for &physical_device in physical_devices.iter() {
            let device_properties =
                unsafe { instance.get_physical_device_properties(physical_device) };
            let device_features = unsafe { instance.get_physical_device_features(physical_device) };
            let device_queue_family_indices =
                Self::find_queue_families(instance, physical_device, surface, surface_loader)?;

            if device_properties.device_type == vk::PhysicalDeviceType::DISCRETE_GPU
                && device_features.geometry_shader == vk::TRUE
                && device_queue_family_indices.is_complete()
                && Self::check_device_extensions_support(
                    instance,
                    physical_device,
                    &DEVICE_EXTENSIONS,
                )?
            {
                let swap_chain_support_details =
                    Self::query_swap_chain_support(physical_device, surface, surface_loader)?;

                if !swap_chain_support_details.formats.is_empty()
                    && !swap_chain_support_details.present_modes.is_empty()
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
                        swap_chain_support_details,
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
        }

        let device = unsafe { instance.create_device(physical_device, &device_create_info, None) }?;
        #[cfg(debug_assertions)]
        println!("Logical device created.");

        Ok(device)
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

    pub fn new(
        window_handle: &dyn raw_window_handle::HasRawWindowHandle,
        width: u32,
        height: u32,
    ) -> Result<Self, Box<dyn Error>> {
        // Init Vulkan
        // Ash loads Vulkan dynamically, ash::Entry is the library loader and the entrypoint into the Vulkan API.
        // In the future, Ash should also support loading Vulkan as a static library.
        let entry = unsafe { Entry::new() }?;

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

        let required_extensions = ash_window::enumerate_required_extensions(window_handle)?;

        #[cfg(debug_assertions)]
        {
            if enable_validation_layers {
                println!("Validation layers available.");

                let mut enabled_extension_names = required_extensions;
                enabled_extension_names.push(DebugUtils::name());
                let p_enabled_extension_names = enabled_extension_names
                    .iter()
                    .map(|e| e.as_ptr())
                    .collect::<Vec<*const i8>>();

                let enabled_layer_names = VALIDATION_LAYERS
                    .iter()
                    .map(|l| CString::new(*l).unwrap())
                    .collect::<Vec<CString>>();
                let p_enabled_layer_names = enabled_layer_names
                    .iter()
                    .map(|l| l.as_ptr())
                    .collect::<Vec<*const i8>>();

                let instance_debug_utils_messenger_create_info =
                    vk::DebugUtilsMessengerCreateInfoEXT {
                        message_severity: vk::DebugUtilsMessageSeverityFlagsEXT::all(),
                        message_type: vk::DebugUtilsMessageTypeFlagsEXT::all(),
                        pfn_user_callback: Some(vk_debug_utils_callback),
                        ..Default::default()
                    };

                let create_info = vk::InstanceCreateInfo {
                    p_next: &instance_debug_utils_messenger_create_info
                        as *const vk::DebugUtilsMessengerCreateInfoEXT
                        as *const c_void,
                    p_application_info: &app_info,
                    enabled_layer_count: p_enabled_layer_names.len() as u32,
                    pp_enabled_layer_names: p_enabled_layer_names.as_ptr(),
                    enabled_extension_count: enabled_extension_names.len() as u32,
                    pp_enabled_extension_names: p_enabled_extension_names.as_ptr(),
                    ..Default::default()
                };

                instance = unsafe { entry.create_instance(&create_info, None) }?;
                println!("Vulkan instance created.");

                let debug_utils_loader = DebugUtils::new(&entry, &instance);
                let messenger_create_info = vk::DebugUtilsMessengerCreateInfoEXT {
                    message_severity: vk::DebugUtilsMessageSeverityFlagsEXT::all(),
                    message_type: vk::DebugUtilsMessageTypeFlagsEXT::all(),
                    pfn_user_callback: Some(vk_debug_utils_callback),
                    ..Default::default()
                };
                let debug_utils_messenger = unsafe {
                    debug_utils_loader.create_debug_utils_messenger(&messenger_create_info, None)
                }?;
                println!("Debug messenger created.");

                debug_utils = Some((debug_utils_loader, debug_utils_messenger));
            } else {
                println!("Validation layers not available.");

                let p_enabled_extension_names = required_extensions
                    .iter()
                    .map(|e| e.as_ptr())
                    .collect::<Vec<*const i8>>();

                let create_info = vk::InstanceCreateInfo {
                    p_application_info: &app_info,
                    enabled_extension_count: required_extensions.len() as u32,
                    pp_enabled_extension_names: p_enabled_extension_names.as_ptr(),
                    ..Default::default()
                };
                instance = unsafe { entry.create_instance(&create_info, None) }?;
                println!("Vulkan instance created.");

                debug_utils = None;
            }
        }

        #[cfg(not(debug_assertions))]
        {
            let p_enabled_extension_names = required_extensions
                .iter()
                .map(|e| e.as_ptr())
                .collect::<Vec<*const i8>>();
            let create_info = vk::InstanceCreateInfo {
                p_application_info: &app_info,
                enabled_extension_count: required_extensions.len() as u32,
                pp_enabled_extension_names: p_enabled_extension_names.as_ptr(),
                ..Default::default()
            };
            instance = unsafe { entry.create_instance(&create_info, None) }?;
        }

        let surface =
            unsafe { ash_window::create_surface(&entry, &instance, window_handle, None) }?;
        let surface_loader = Surface::new(&entry, &instance);
        #[cfg(debug_assertions)]
        println!("Window surface created.");

        let (physical_device, queue_family_indices, swap_chain_support_details) =
            Self::pick_physical_device(&instance, surface, &surface_loader)?;
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

        let (
            swap_chain,
            swap_chain_loader,
            swap_chain_images,
            swap_chain_image_format,
            swap_chain_extent,
        ) = Self::create_swap_chain(
            &instance,
            &device,
            &surface,
            &swap_chain_support_details,
            &queue_family_indices,
            width,
            height,
        )?;

        let swap_chain_image_views =
            Self::create_image_views(&device, &swap_chain_images, swap_chain_image_format)?;

        let pipeline_layout = Self::create_graphics_pipeline(&device, swap_chain_extent)?;

        Ok(Self {
            // The entry has to live as long as the app, otherwise you get an access violation when destroying instance.
            _entry: entry,
            instance,
            #[cfg(debug_assertions)]
            debug_utils,
            _physical_device: physical_device,
            surface: (surface, surface_loader),
            device,
            _graphics_queue: graphics_queue,
            _present_queue: present_queue,
            swap_chain: (
                swap_chain,
                swap_chain_loader,
                swap_chain_images,
                swap_chain_image_format,
                swap_chain_extent,
            ),
            swap_chain_image_views,
            pipeline_layout,
        })
    }

    pub fn draw_frame(&mut self) {}
}

impl Drop for VkRsApp {
    fn drop(&mut self) {
        unsafe {
            self.device
                .destroy_pipeline_layout(self.pipeline_layout, None)
        };
        #[cfg(debug_assertions)]
        println!("Pipeline layout dropped.");

        for image_view in self.swap_chain_image_views.iter() {
            unsafe { self.device.destroy_image_view(*image_view, None) }
        }
        #[cfg(debug_assertions)]
        println!("Swap chain image views dropped.");

        let (swap_chain, swap_chain_loader, _, _, _) = &self.swap_chain;
        unsafe { swap_chain_loader.destroy_swapchain(*swap_chain, None) };
        #[cfg(debug_assertions)]
        println!("Swap chain dropped.");

        unsafe { self.device.destroy_device(None) };
        #[cfg(debug_assertions)]
        println!("Logical device dropped.");

        let (surface, surface_loader) = &self.surface;
        unsafe { surface_loader.destroy_surface(*surface, None) };
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
