use std::{error::Error, ffi::CStr};
#[cfg(debug_assertions)]
use std::ffi::{CString, c_void};

use ash::{vk, Entry, Instance};
#[cfg(debug_assertions)]
use ash::{extensions::ext::DebugUtils, vk::DebugUtilsMessengerEXT};

pub struct VkRsApp {
    _entry: Entry,
    instance: Instance,
    #[cfg(debug_assertions)]
    debug_utils: Option<(DebugUtils, DebugUtilsMessengerEXT)>,
}

#[cfg(debug_assertions)]
unsafe extern "system" fn vk_debug_utils_callback (
    message_severity: vk::DebugUtilsMessageSeverityFlagsEXT,
    message_type: vk::DebugUtilsMessageTypeFlagsEXT,
    p_callback_data: *const vk::DebugUtilsMessengerCallbackDataEXT,
    _p_user_data: *mut c_void
) -> vk::Bool32 {
    let msg_severity = match message_severity {
        vk::DebugUtilsMessageSeverityFlagsEXT::VERBOSE => "[VERBOSE]",
        vk::DebugUtilsMessageSeverityFlagsEXT::INFO => "[INFO]",
        vk::DebugUtilsMessageSeverityFlagsEXT::WARNING => "[WARNING]",
        vk::DebugUtilsMessageSeverityFlagsEXT::ERROR => "[ERROR]",
        _ => "[UNKNOWN_SEVERITY]"
    };

    let msg_type = match message_type {
        vk::DebugUtilsMessageTypeFlagsEXT::GENERAL => "[GENERAL]",
        vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE => "[PERFORMANCE]",
        vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION => "[VALIDATION]",
        _ => "[UNKNOWN_TYPE]"
    };

    let msg = CStr::from_ptr((*p_callback_data).p_message);
    
    println!("{} {} {:?}", msg_severity, msg_type, msg);

    vk::FALSE
}

impl VkRsApp {
    fn pick_physical_device(instance: &Instance) -> Result<vk::PhysicalDevice, Box<dyn Error>> {
        let physical_devices = unsafe { instance.enumerate_physical_devices() }?;

        for physical_device in physical_devices.iter() {
            let device_properties = unsafe { instance.get_physical_device_properties(*physical_device) };
            let device_features = unsafe { instance.get_physical_device_features(*physical_device) };

            if device_properties.device_type == vk::PhysicalDeviceType::DISCRETE_GPU &&
                    device_features.geometry_shader == vk::TRUE {
                let device_name = unsafe { CStr::from_ptr(device_properties.device_name.as_ptr()).to_str().to_owned() }?;
                println!("Found suitable device : {} !", device_name);
            
                return Ok(*physical_device)
            }
        }

        Err("No suitable device found !")?
    }

    #[cfg(debug_assertions)]
    fn check_validation_layers_support(entry: &Entry, layer_names: &[&str]) -> Result<bool, Box<dyn Error>> {
        let available_layers_properties = entry.enumerate_instance_layer_properties()?;

        println!("Available Vulkan layers :");
        for layer in available_layers_properties.iter() {
            let layer_name = unsafe { CStr::from_ptr(layer.layer_name.as_ptr()).to_str().to_owned() }?;
            println!("{}", layer_name);
        }

        for layer_name in layer_names.iter() {
            let mut layer_is_available = false;
            for available_layer in available_layers_properties.iter() {
                let available_layer_name = unsafe { CStr::from_ptr(available_layer.layer_name.as_ptr()).to_str().to_owned() }?;
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

    #[cfg(debug_assertions)]
    pub fn new(mut required_extensions: Vec<&CStr>) -> Result<Self, Box<dyn Error>> {
        // Init Vulkan
        // Ash loads Vulkan dynamically, ash::Entry is the library loader and the entrypoint into the Vulkan API.
        // In the future, Ash should also support loading Vulkan as a static library.
        let entry = unsafe { Entry::new() }?;

        let app_info = vk::ApplicationInfo {
            api_version: vk::make_api_version(0, 1, 0, 0),
            ..Default::default()
        };

        let validation_layers = ["VK_LAYER_KHRONOS_validation"];
        let enable_validation_layers = Self::check_validation_layers_support(&entry, &validation_layers)?;
        
        if enable_validation_layers {
            println!("Validation layers available.");

            required_extensions.push(DebugUtils::name());
            let p_enabled_extension_names = required_extensions.iter().map(|e| e.as_ptr()).collect::<Vec<*const i8>>();

            let enabled_layer_names = validation_layers.iter().map(|l| CString::new(*l).unwrap()).collect::<Vec<CString>>();
            let p_enabled_layer_names = enabled_layer_names.iter().map(|l| l.as_ptr()).collect::<Vec<*const i8>>();

            let instance_debug_utils_messenger_create_info = vk::DebugUtilsMessengerCreateInfoEXT {
                message_severity: vk::DebugUtilsMessageSeverityFlagsEXT::all(),
                message_type: vk::DebugUtilsMessageTypeFlagsEXT::all(),
                pfn_user_callback: Some(vk_debug_utils_callback),
                ..Default::default()
            };

            let create_info = vk::InstanceCreateInfo {
                p_next: &instance_debug_utils_messenger_create_info as *const vk::DebugUtilsMessengerCreateInfoEXT as *const c_void,
                p_application_info: &app_info,
                enabled_layer_count: validation_layers.len() as u32,
                pp_enabled_layer_names: p_enabled_layer_names.as_ptr(),
                enabled_extension_count: required_extensions.len() as u32,
                pp_enabled_extension_names: p_enabled_extension_names.as_ptr(),
                ..Default::default()
            };

            let instance = unsafe { entry.create_instance(&create_info, None) }?;
        
            println!("Vulkan instance created.");

            let debug_utils_loader = DebugUtils::new(&entry, &instance);
            let messenger_create_info = vk::DebugUtilsMessengerCreateInfoEXT {
                message_severity: vk::DebugUtilsMessageSeverityFlagsEXT::all(),
                message_type: vk::DebugUtilsMessageTypeFlagsEXT::all(),
                pfn_user_callback: Some(vk_debug_utils_callback),
                ..Default::default()
            };
            let debug_utils_messenger = unsafe { debug_utils_loader.create_debug_utils_messenger(&messenger_create_info, None) }?;

            let _physical_device = Self::pick_physical_device(&instance)?;

            return Ok(VkRsApp {
                // The entry has to live as long as the app, otherwise you get an access violation when destroying instance.
                _entry: entry,
                instance,
                debug_utils: Some((debug_utils_loader, debug_utils_messenger))
            })
        } else {
            println!("Validation layers not available.");

            let p_enabled_extension_names = required_extensions.iter().map(|e| e.as_ptr()).collect::<Vec<*const i8>>();

            let create_info = vk::InstanceCreateInfo {
                p_application_info: &app_info,
                enabled_extension_count: required_extensions.len() as u32,
                pp_enabled_extension_names: p_enabled_extension_names.as_ptr(),
                ..Default::default()
            };
            let instance = unsafe { entry.create_instance(&create_info, None) }?;
    
            println!("Vulkan instance created.");
    
            return Ok(VkRsApp {
                // The entry has to live as long as the app, otherwise you get an access violation when destroying instance.
                _entry: entry,
                instance,
                debug_utils: None
            })
        }
    }

    #[cfg(not(debug_assertions))]
    pub fn new(required_extensions: Vec<&CStr>) -> Result<Self, Box<dyn Error>> {
        // Init Vulkan
        // Ash loads Vulkan dynamically, ash::Entry is the library loader and the entrypoint into the Vulkan API.
        // In the future, Ash should also support loading Vulkan as a static library.
        let entry = unsafe { Entry::new() }?;

        let app_info = vk::ApplicationInfo {
            api_version: vk::make_api_version(0, 1, 0, 0),
            ..Default::default()
        };

        let p_enabled_extension_names = required_extensions.iter().map(|e| e.as_ptr()).collect::<Vec<*const i8>>();
        let create_info = vk::InstanceCreateInfo {
            p_application_info: &app_info,
            enabled_extension_count: required_extensions.len() as u32,
            pp_enabled_extension_names: p_enabled_extension_names.as_ptr(),
            ..Default::default()
        };
        let instance = unsafe { entry.create_instance(&create_info, None) }?;

        Ok(VkRsApp {
            // The entry has to live as long as the app, otherwise you get an access violation when destroying instance.
            _entry: entry,
            instance
        })
    }

    pub fn draw_frame(&mut self) {

    }
}

impl Drop for VkRsApp {
    fn drop(&mut self) {
        #[cfg(debug_assertions)]
        if let Some((debug_utils_loader, debug_utils_messenger)) = &self.debug_utils {
            unsafe { debug_utils_loader.destroy_debug_utils_messenger(*debug_utils_messenger, None) };
        }

        // The ash::Entry used to create the instance has to be alive when calling ash::Instance::destroy_instance.
        unsafe { self.instance.destroy_instance(None) };
        #[cfg(debug_assertions)]
        println!("Vulkan instance dropped.");
    }
}
