#![windows_subsystem = "windows"]

use winit::{
    dpi::LogicalSize,
    event::{ElementState, Event, KeyboardInput, VirtualKeyCode, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};
fn main() {
    let event_loop = EventLoop::new();
    let _window = WindowBuilder::new()
        .with_title("vk-rs")
        .with_inner_size(LogicalSize::new(800, 600))
        .build(&event_loop)
        .unwrap();

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;

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
                            dbg!();
                            *control_flow = ControlFlow::Exit
                        }
                        _ => (),
                    },
                },
                _ => (),
            },
            _ => (),
        }
    });
}
