use std::error::Error;

use ash::{vk, Entry, Instance};

pub struct VkRsApp {
    _entry: Entry,
    instance: Instance
}

impl VkRsApp {
    pub fn new() -> Result<Self, Box<dyn Error>> {
        // Init Vulkan
        // Ash loads Vulkan dynamically, ash::Entry is the library loader and the entrypoint into the Vulkan API.
        // In the future, Ash should also support loading Vulkan as a static library.
        let _entry = unsafe { Entry::new() }?;
        let app_info = vk::ApplicationInfo {
            api_version: vk::make_api_version(0, 1, 0, 0),
            ..Default::default()
        };
        let create_info = vk::InstanceCreateInfo {
            p_application_info: &app_info,
            ..Default::default()
        };
        let instance = unsafe { _entry.create_instance(&create_info, None) }?;
        #[cfg(debug_assertions)]
        println!("Vulkan instance created.");
        
        Ok(VkRsApp {
            // The entry has to live as long as the app, otherwise you get an access violation when destroying instance.
            _entry,
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
