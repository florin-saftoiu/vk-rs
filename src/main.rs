#![windows_subsystem = "windows"]

mod vk_rs_app;

use std::{error::Error, time::Instant};

use winit::{
    dpi::LogicalSize,
    event::{ElementState, Event, KeyboardInput, VirtualKeyCode, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

use vk_rs_app::VkRsApp;

fn main() -> Result<(), Box<dyn Error>> {
    // Init Window
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title("vk-rs")
        .with_inner_size(LogicalSize::new(800, 600))
        .build(&event_loop)?;

    // Init App (including Vulkan)
    let mut vk_rs_app = VkRsApp::new(
        &window,
        window.inner_size().width,
        window.inner_size().height,
    )?;
    let mut minimized = false;
    let tp1 = Instant::now();

    // Main Loop
    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Poll;

        match event {
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
                WindowEvent::KeyboardInput { input, .. } => match input {
                    KeyboardInput {
                        state,
                        virtual_keycode,
                        ..
                    } => match (state, virtual_keycode) {
                        (ElementState::Pressed, Some(VirtualKeyCode::Escape)) => {
                            *control_flow = ControlFlow::Exit
                        }
                        _ => (),
                    },
                },
                WindowEvent::Resized(size) => {
                    if size.width == 0 || size.height == 0 {
                        minimized = true;
                    } else {
                        minimized = false;
                        vk_rs_app.window_resized(size.width, size.height);
                    }
                }
                _ => (),
            },
            Event::MainEventsCleared => {
                if !minimized {
                    let tp2 = Instant::now();
                    let time = tp2.duration_since(tp1).as_secs_f32();
                    vk_rs_app.draw_frame(time);
                }
            }
            Event::LoopDestroyed => {
                vk_rs_app.loop_destroyed();
            }
            _ => (),
        }
    });
}
