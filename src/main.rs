#![windows_subsystem = "windows"]

mod vk_rs_app;

use std::{error::Error, time::Instant};

use cgmath::{Deg, Matrix4, Vector3, Vector4};
use raw_window_handle::{HasRawDisplayHandle, HasRawWindowHandle};
use winit::{
    dpi::LogicalSize,
    event::{DeviceEvent, ElementState, Event, KeyboardInput, VirtualKeyCode, WindowEvent},
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
    let mut a_pressed = false;
    let mut d_pressed = false;
    let mut space_pressed = false;
    let mut c_pressed = false;
    let mut q_pressed = false;
    let mut e_pressed = false;

    let mut yaw = 0.0;
    let mut pitch = 0.0;

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
                        (ElementState::Pressed, Some(VirtualKeyCode::A)) => {
                            a_pressed = true;
                        }
                        (ElementState::Released, Some(VirtualKeyCode::A)) => {
                            a_pressed = false;
                        }
                        (ElementState::Pressed, Some(VirtualKeyCode::D)) => {
                            d_pressed = true;
                        }
                        (ElementState::Released, Some(VirtualKeyCode::D)) => {
                            d_pressed = false;
                        }
                        (ElementState::Pressed, Some(VirtualKeyCode::Space)) => {
                            space_pressed = true;
                        }
                        (ElementState::Released, Some(VirtualKeyCode::Space)) => {
                            space_pressed = false;
                        }
                        (ElementState::Pressed, Some(VirtualKeyCode::C)) => {
                            c_pressed = true;
                        }
                        (ElementState::Released, Some(VirtualKeyCode::C)) => {
                            c_pressed = false;
                        }
                        (ElementState::Pressed, Some(VirtualKeyCode::Q)) => {
                            q_pressed = true;
                        }
                        (ElementState::Released, Some(VirtualKeyCode::Q)) => {
                            q_pressed = false;
                        }
                        (ElementState::Pressed, Some(VirtualKeyCode::E)) => {
                            e_pressed = true;
                        }
                        (ElementState::Released, Some(VirtualKeyCode::E)) => {
                            e_pressed = false;
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
            Event::DeviceEvent {
                device_id: _,
                event,
            } => match event {
                DeviceEvent::MouseMotion { delta: (dx, dy) } => {
                    yaw -= dx as f32 * 0.1;
                    pitch -= dy as f32 * 0.1;
                }
                _ => (),
            },
            Event::MainEventsCleared => {
                if !minimized {
                    let tp2 = Instant::now();
                    let time = tp2.duration_since(tp1).as_secs_f32();
                    tp1 = tp2;

                    let target = Vector4::new(0.0, 0.0, -1.0, 1.0);
                    let camera_rotation =
                        Matrix4::from_angle_y(Deg(yaw)) * Matrix4::from_angle_x(Deg(pitch));
                    let look_dir = (camera_rotation * target).truncate();
                    vk_rs_app.target = vk_rs_app.camera - look_dir;

                    let forward = look_dir * 6.0 * time;
                    let left = look_dir.cross(Vector3::new(0.0, 1.0, 0.0)) * 6.0 * time;

                    if w_pressed {
                        // move forward
                        vk_rs_app.camera += forward;
                    }
                    if s_pressed {
                        // move backwards
                        vk_rs_app.camera -= forward;
                    }
                    if a_pressed {
                        // strafe left
                        vk_rs_app.camera -= left;
                    }
                    if d_pressed {
                        // strafe right
                        vk_rs_app.camera += left;
                    }
                    if space_pressed {
                        // move up
                        vk_rs_app.camera.y += 6.0 * time;
                    }
                    if c_pressed {
                        // move down
                        vk_rs_app.camera.y -= 6.0 * time;
                    }
                    if q_pressed {
                        // look left
                        yaw += 20.0 * time;
                    }
                    if e_pressed {
                        // look right
                        yaw -= 20.0 * time;
                    }

                    vk_rs_app.draw_frame();
                }
            }
            Event::LoopDestroyed => {
                vk_rs_app.loop_destroyed();
            }
            _ => (),
        }
    });
}
