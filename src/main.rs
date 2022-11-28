#![windows_subsystem = "windows"]

mod renderer;

use std::{error::Error, time::Instant};

use cgmath::{Deg, InnerSpace, Matrix4, Vector3, Vector4};
use raw_window_handle::{HasRawDisplayHandle, HasRawWindowHandle};
use winit::{
    dpi::LogicalSize,
    event::{DeviceEvent, ElementState, Event, KeyboardInput, VirtualKeyCode, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

use renderer::Renderer;

fn main() -> Result<(), Box<dyn Error>> {
    // Init Window
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title("vk-rs")
        .with_inner_size(LogicalSize::new(800, 600))
        .build(&event_loop)?;

    // Init App (including Vulkan)
    let mut renderer = Renderer::new(
        window.raw_display_handle(),
        window.raw_window_handle(),
        window.inner_size().width,
        window.inner_size().height,
    )?;
    let m0 = renderer.load_model("models/cube.obj", "textures/cube.png", false)?;
    let m1 = renderer.load_model("models/viking_room.obj", "textures/viking_room.png", false)?;
    renderer.model(m0).position.x = -5.0;
    renderer.model(m0).position.y = -0.25;
    renderer.model(m0).position.z = -6.0;
    renderer.model(m0).theta = 0.0;
    renderer.model(m1).position.x = 0.0;
    renderer.model(m1).position.y = -0.25;
    renderer.model(m1).position.z = -6.0;
    renderer.model(m1).theta = -90.0;
    let mut minimized = false;
    let mut tp1 = Instant::now();

    let mut yaw = 0.0;
    let mut pitch = 0.0;

    let mut keys = [false; 256];

    // Main Loop
    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Poll;

        match event {
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
                WindowEvent::KeyboardInput {
                    input:
                        KeyboardInput {
                            state,
                            virtual_keycode: Some(keycode),
                            ..
                        },
                    ..
                } => match (state, keycode) {
                    (ElementState::Pressed, VirtualKeyCode::Escape) => {
                        *control_flow = ControlFlow::Exit
                    }
                    (ElementState::Released, VirtualKeyCode::R) => {
                        renderer.camera.x = 0.0;
                        renderer.camera.y = 0.0;
                        renderer.camera.z = 0.0;
                        yaw = 0.0;
                        pitch = 0.0;
                    }
                    (ElementState::Pressed, _) => keys[keycode as usize] = true,
                    (ElementState::Released, _) => keys[keycode as usize] = false,
                },
                WindowEvent::Resized(size) => {
                    if size.width == 0 || size.height == 0 {
                        minimized = true;
                    } else {
                        minimized = false;
                        renderer.window_resized(size.width, size.height);
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

                    let forward = look_dir * 6.0 * time;
                    let right =
                        look_dir.cross(Vector3::new(0.0, 1.0, 0.0)).normalize() * 6.0 * time;

                    if keys[VirtualKeyCode::W as usize] {
                        // move forward
                        renderer.camera += forward;
                    }
                    if keys[VirtualKeyCode::S as usize] {
                        // move backwards
                        renderer.camera -= forward;
                    }
                    if keys[VirtualKeyCode::A as usize] {
                        // strafe left
                        renderer.camera -= right;
                    }
                    if keys[VirtualKeyCode::D as usize] {
                        // strafe right
                        renderer.camera += right;
                    }
                    if keys[VirtualKeyCode::Space as usize] {
                        // move up
                        renderer.camera.y += 6.0 * time;
                    }
                    if keys[VirtualKeyCode::C as usize] {
                        // move down
                        renderer.camera.y -= 6.0 * time;
                    }
                    if keys[VirtualKeyCode::Q as usize] {
                        // look left
                        yaw += 20.0 * time;
                    }
                    if keys[VirtualKeyCode::E as usize] {
                        // look right
                        yaw -= 20.0 * time;
                    }

                    renderer.target = renderer.camera - look_dir;

                    if keys[VirtualKeyCode::Up as usize] {
                        renderer.model(m0).position.z -= 8.0 * time;
                    }

                    if keys[VirtualKeyCode::Down as usize] {
                        renderer.model(m0).position.z += 8.0 * time;
                    }

                    if keys[VirtualKeyCode::Left as usize] {
                        renderer.model(m0).position.x -= 8.0 * time;
                    }

                    if keys[VirtualKeyCode::Right as usize] {
                        renderer.model(m0).position.x += 8.0 * time;
                    }

                    renderer.draw_frame();
                    window.set_title(
                        format!(
                            "vk-rs - XYZ: {:>11.5}, {:>11.5}, {:>11.5} - FPS: {:>5.0}",
                            renderer.camera.x,
                            renderer.camera.y,
                            renderer.camera.z,
                            1.0 / time,
                        )
                        .as_str(),
                    );
                }
            }
            Event::LoopDestroyed => {
                renderer.wait_idle();
            }
            _ => (),
        }
    });
}
