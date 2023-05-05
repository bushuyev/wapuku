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
use winit::dpi::{LogicalSize, PhysicalSize};
use winit::event_loop::EventLoopBuilder;
use wasm_bindgen::prelude::*;
use winit::platform::web::WindowExtWebSys;
use winit::window::Fullscreen;
use crate::state::State;
use wapuku_model::polars_df::*;




#[wasm_bindgen]
pub async fn init_thread_pool(threads:usize){
    debug!("init_thread_pool threads={}", threads);
    wasm_bindgen_rayon::init_thread_pool(threads);
}

#[wasm_bindgen(start)]
pub async fn run() {//async should be ok https://github.com/rustwasm/wasm-bindgen/issues/1904 

    

    // let runtime = RuntimeEnv::default();
    // let config  = SessionConfig::default();
    
    std::panic::set_hook(Box::new(console_error_panic_hook::hook));
    console_log::init_with_level(log::Level::Debug).expect("Couldn't initialize logger");

    debug!("run");

    
    
    

    let event_loop = EventLoopBuilder::<()>::with_user_event().build();
    let winit_window = WindowBuilder::new().with_resizable(true).build(&event_loop).unwrap();

    // window.set_max_inner_size(Some(LogicalSize::new(200.0, 200.0)));
    

    web_sys::window()
        .and_then(|win| win.document())
        .and_then(|doc| {
            let div = doc.get_element_by_id("wapuku_canvas_div")?;
            let width = div.client_width() as f32;
            let height = div.client_height() as f32;
            
            let canvas = web_sys::Element::from(winit_window.canvas());
            div.append_child(&canvas).ok()?;
            
            debug!("web_sys::window: width={}, height={}", width, height);

            winit_window.set_inner_size(PhysicalSize::new(width, height));
            
            Some(())
        })
        .expect("Couldn't append canvas to document body.");

    debug!("running");
    
    
    
    let mut state = State::new(winit_window, Box::new(PolarsData::new())).await;

    event_loop.run(move |event, _, control_flow| {
        // debug!("event_loop.run={:?}", event);
        
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
                // debug!("event_loop window_id={:?} state.window().id()={:?}", window_id, state.window().id());
                
                if state.input(window_event) {
                
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
                }
            }
            _ => {}
        }
    });
    
}