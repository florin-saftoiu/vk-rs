use std::{error::Error, ffi::CStr};
#[cfg(debug_assertions)]
use std::ffi::CString;

use ash::{vk, Entry, Instance};
#[cfg(debug_assertions)]
use ash::extensions::ext::DebugUtils;

pub struct VkRsApp {
    _entry: Entry,
    instance: Instance
}

impl VkRsApp {
    #[cfg(debug_assertions)]
    fn check_validation_layers_support(entry: &ash::Entry, layer_names: &[&str]) -> Result<bool, Box<dyn Error>> {
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
        } else {
            println!("Validation layers not available.");
        }

        if enable_validation_layers {
            required_extensions.push(DebugUtils::name());
        }

        let enabled_layer_names = validation_layers.iter().map(|l| CString::new(*l).unwrap()).collect::<Vec<CString>>();
        let p_enabled_layer_names = enabled_layer_names.iter().map(|l| l.as_ptr()).collect::<Vec<*const i8>>();
        let p_enabled_extension_names = required_extensions.iter().map(|e| e.as_ptr()).collect::<Vec<*const i8>>();
        let create_info = if enable_validation_layers {
            vk::InstanceCreateInfo {
                p_application_info: &app_info,
                enabled_layer_count: validation_layers.len() as u32,
                pp_enabled_layer_names: p_enabled_layer_names.as_ptr(),
                enabled_extension_count: required_extensions.len() as u32,
                pp_enabled_extension_names: p_enabled_extension_names.as_ptr(),
                ..Default::default()
            }
        } else {
            vk::InstanceCreateInfo {
                p_application_info: &app_info,
                enabled_extension_count: required_extensions.len() as u32,
                pp_enabled_extension_names: p_enabled_extension_names.as_ptr(),
                ..Default::default()
            }
        };
        let instance = unsafe { entry.create_instance(&create_info, None) }?;

        println!("Vulkan instance created.");

        Ok(VkRsApp {
            // The entry has to live as long as the app, otherwise you get an access violation when destroying instance.
            _entry: entry,
            instance
        })
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
        // The ash::Entry used to create the instance has to be alive when calling ash::Instance::destroy_instance.
        unsafe { self.instance.destroy_instance(None) };
        #[cfg(debug_assertions)]
        println!("Vulkan instance dropped.");
    }
}
