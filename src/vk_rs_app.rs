use std::{error::Error, ffi::{CStr, CString}};

use ash::{vk, Entry, Instance};

pub struct VkRsApp {
    _entry: Entry,
    instance: Instance
}

impl VkRsApp {
    fn check_validation_layers_support(entry: &ash::Entry, layer_names: &[&str]) -> Result<bool, Box<dyn Error>> {
        let available_layers_properties = entry.enumerate_instance_layer_properties()?;

        #[cfg(debug_assertions)]
        {
            println!("Available Vulkan layers :");
            for layer in available_layers_properties.iter() {
                let layer_name = unsafe { CStr::from_ptr(layer.layer_name.as_ptr()).to_str().to_owned() }?;
                println!("{}", layer_name);
            }
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

    pub fn new() -> Result<Self, Box<dyn Error>> {
        // Init Vulkan
        // Ash loads Vulkan dynamically, ash::Entry is the library loader and the entrypoint into the Vulkan API.
        // In the future, Ash should also support loading Vulkan as a static library.
        let entry = unsafe { Entry::new() }?;

        let validation_layers = ["VK_LAYER_KHRONOS_validation"];
        let enable_validation_layers = Self::check_validation_layers_support(&entry, &validation_layers)?;
        
        #[cfg(debug_assertions)]
        if enable_validation_layers {
            println!("Validation layers available.");
        } else {
            println!("Validation layers not avaialable.");
        }

        let app_info = vk::ApplicationInfo {
            api_version: vk::make_api_version(0, 1, 0, 0),
            ..Default::default()
        };
        let enabled_layer_names = validation_layers.iter().map(|l| CString::new(*l).unwrap()).collect::<Vec<CString>>();
        let p_enabled_layer_names = enabled_layer_names.iter().map(|l| l.as_ptr()).collect::<Vec<*const i8>>();
        let create_info = if enable_validation_layers {
                vk::InstanceCreateInfo {
                    p_application_info: &app_info,
                    enabled_layer_count: validation_layers.len() as u32,
                    pp_enabled_layer_names: p_enabled_layer_names.as_ptr(),
                    ..Default::default()
                }
            } else {
                vk::InstanceCreateInfo {
                    p_application_info: &app_info,
                    ..Default::default()
                }
            };
        let instance = unsafe { entry.create_instance(&create_info, None) }?;

        #[cfg(debug_assertions)]
        println!("Vulkan instance created.");

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
