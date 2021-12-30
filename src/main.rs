#![windows_subsystem = "windows"]

use std::error::Error;

use ash::{vk, Entry};
use winit::{
    dpi::LogicalSize,
    event::{ElementState, Event, KeyboardInput, VirtualKeyCode, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

fn main() -> Result<(), Box<dyn Error>> {
    let event_loop = EventLoop::new();
    let _window = WindowBuilder::new()
        .with_title("vk-rs")
        .with_inner_size(LogicalSize::new(800, 600))
        .build(&event_loop)?;

    // Ash loads Vulkan dynamically, ash::Entry is the library loader and the entrypoint into the Vulkan API.
    // In the future, Ash should also support loading Vulkan as a static library.
    let entry = unsafe { Entry::new() }?;
    let app_info = vk::ApplicationInfo {
        api_version: vk::make_api_version(0, 1, 0, 0),
        ..Default::default()
    };
    let create_info = vk::InstanceCreateInfo {
        p_application_info: &app_info,
        ..Default::default()
    };
    let instance = unsafe { entry.create_instance(&create_info, None) }?;

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Poll;

        match event {
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::CloseRequested => {
                    unsafe { instance.destroy_instance(None) };
                    *control_flow = ControlFlow::Exit
                }
                WindowEvent::KeyboardInput { input, .. } => match input {
                    KeyboardInput {
                        state,
                        virtual_keycode,
                        ..
                    } => match (state, virtual_keycode) {
                        (ElementState::Pressed, Some(VirtualKeyCode::Escape)) => {
                            unsafe { instance.destroy_instance(None) };
                            *control_flow = ControlFlow::Exit
                        }
                        _ => (),
                    },
                },
                _ => (),
            },
            Event::MainEventsCleared => {}
            _ => (),
        }
    });
}
