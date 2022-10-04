#![windows_subsystem = "windows"]

mod vk_rs_app;

use std::{error::Error, time::Instant};

use raw_window_handle::{HasRawDisplayHandle, HasRawWindowHandle};
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
        window.raw_display_handle(),
        window.raw_window_handle(),
        window.inner_size().width,
        window.inner_size().height,
    )?;
    let mut minimized = false;
    let mut tp1 = Instant::now();

    let mut w_pressed = false;
    let mut s_pressed = false;

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
                        (ElementState::Pressed, Some(VirtualKeyCode::W)) => {
                            w_pressed = true;
                        }
                        (ElementState::Released, Some(VirtualKeyCode::W)) => {
                            w_pressed = false;
                        }
                        (ElementState::Pressed, Some(VirtualKeyCode::S)) => {
                            s_pressed = true;
                        }
                        (ElementState::Released, Some(VirtualKeyCode::S)) => {
                            s_pressed = false;
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
                    tp1 = tp2;
                    vk_rs_app.draw_frame(time, w_pressed, s_pressed);
                }
            }
            Event::LoopDestroyed => {
                vk_rs_app.loop_destroyed();
            }
            _ => (),
        }
    });
}
