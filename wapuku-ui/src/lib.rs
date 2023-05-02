mod state;
mod resources;
mod mesh_model;
mod texture;
mod camera;
mod light;


use std::sync::Arc;
use log::{debug, trace};
use winit::{
    event::*,
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};
use winit::dpi::PhysicalSize;
use winit::event_loop::EventLoopBuilder;
use wasm_bindgen::prelude::*;
use winit::platform::web::WindowExtWebSys;
use winit::window::Fullscreen;
use crate::state::State;
use wapuku_model::polars_df::parquet_scan;

pub use wasm_bindgen_rayon::init_thread_pool;


#[wasm_bindgen(start)]
pub async fn run() {//async should be ok https://github.com/rustwasm/wasm-bindgen/issues/1904 

    

    // let runtime = RuntimeEnv::default();
    // let config  = SessionConfig::default();
    
    std::panic::set_hook(Box::new(console_error_panic_hook::hook));
    console_log::init_with_level(log::Level::Debug).expect("Couldn't initialize logger");

    debug!("run");

    parquet_scan();

    let event_loop = EventLoopBuilder::<()>::with_user_event().build();
    let window = WindowBuilder::new().with_resizable(true).build(&event_loop).unwrap();

    // window.set_inner_size(PhysicalSize::new(450, 400));

    web_sys::window()
        .and_then(|win| win.document())
        .and_then(|doc| {
            let dst = doc.get_element_by_id("wapuku_canvas")?;
            let canvas = web_sys::Element::from(window.canvas());
            dst.append_child(&canvas).ok()?;
            Some(())
        })
        .expect("Couldn't append canvas to document body.");

    debug!("running");
    
    let mut state = State::new(window).await;

    event_loop.run(move |event, _, control_flow| {
        debug!("event_loop.run={:?}", event);
        
        match event {
            Event::RedrawRequested(window_id) if window_id == state.window().id() => {
                state.update();
                match state.render() {
                    Ok(_) => {}
                    // Reconfigure the surface if lost
                    Err(wgpu::SurfaceError::Lost) => state.resize(state.size),
                    // The system is out of memory, we should probably quit
                    Err(wgpu::SurfaceError::OutOfMemory) => *control_flow = ControlFlow::Exit,
                    // All other errors (Outdated, Timeout) should be resolved by the next frame
                    Err(e) => eprintln!("{:?}", e),
                }
            }
            Event::MainEventsCleared => {
                // RedrawRequested will only trigger once, unless we manually
                // request it.
                state.window().request_redraw();
            }
            Event::WindowEvent {
                event: ref window_event,
                window_id,
            } if window_id == state.window().id() => {
                debug!("event_loop window_id={:?} state.window().id()={:?} state.input(event)={}", window_id, state.window().id(), state.input(window_event));
                
                // if state.input(window_event) { // UPDATED!
                
                
                    match window_event {
                        WindowEvent::CursorMoved {..} => {
                            debug!("WindowEvent::CursorMoved");
                        }
                        WindowEvent::CloseRequested | WindowEvent::KeyboardInput {
                            input:
                            KeyboardInput {
                                state: ElementState::Pressed,
                                virtual_keycode: Some(VirtualKeyCode::Escape),
                                ..
                            },
                            ..
                        } => *control_flow = ControlFlow::Exit,
                        WindowEvent::Resized(physical_size) => {
                            state.resize(*physical_size);
                        }
                        WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                            state.resize(**new_inner_size);
                        }
                        _ => {}
                    }
                // }
            }
            _ => {}
        }
    });
    
}