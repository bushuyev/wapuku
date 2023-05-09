mod state;
mod resources;
mod mesh_model;
mod texture;
mod camera;
mod light;


use std::collections::HashSet;
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
use wapuku_model::model::*;
use wapuku_model::test_data::*;
use wapuku_model::visualization::*;



#[wasm_bindgen]
pub async fn init_thread_pool(threads:usize){
    debug!("init_thread_pool threads={}", threads);
    wasm_bindgen_rayon::init_thread_pool(2);
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
    

    let (width, height) = web_sys::window()
        .and_then(|win| win.document())
        .and_then(|doc| {
            let div = doc.get_element_by_id("wapuku_canvas_div")?;
            let width = div.client_width();
            let height = div.client_height();
            
            let canvas = web_sys::Element::from(winit_window.canvas());
            div.append_child(&canvas).ok()?;
            
            debug!("web_sys::window: width={}, height={}", width, height);

            winit_window.set_inner_size(PhysicalSize::new(width, height));
            
            Some((width, height))
        })
        .expect("Couldn't append canvas to document body.");

    debug!("running");
    
    
    // let mut state = State::new(winit_window, Box::new(PolarsData::new())).await;
    let data:Box<dyn Data> = Box::new(TestData::new());
    let all_properties:HashSet<&dyn Property> = data.all_properties();
    

    let (property_1, property_2, property_3) = {
        let mut all_properties_iter = all_properties.into_iter();
        
        (all_properties_iter.next().expect("property_1"), all_properties_iter.next().expect("property_2"), all_properties_iter.next().expect("property_2"))
    };

    let data_grid = data.group_by_2(
        PropertyRange::new (property_1,  None, None ),
        PropertyRange::new (property_2,  None, None )
    );

    let property_x: String = property_1.name().clone();
    let property_y: String = property_2.name().clone();
    
    debug!("data_grid: {:?} property_x={} property_y={}", data_grid, property_x, property_y);
    
    // data
    let mut state = State::new(winit_window, VisualDataController::new(data, width as u32, height as u32, property_x, property_y)).await;

    event_loop.run(move |event, _, control_flow| {
        // debug!("event_loop.run={:?}", event);

        match event {
            Event::RedrawRequested(window_id) if window_id == state.window().id() => {
                state.update(/*data.visuals()*/);
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

            Event::DeviceEvent {
                event: ref device_event,
                device_id,
            } => {
                match device_event {
                    DeviceEvent::MouseMotion { delta } => {
                        debug!("event_loop::DeviceEvent::MouseMotion: delta={:?}", delta);
                    }
                    _ => {
                        
                    }
                }
            }

            Event::WindowEvent {
                event: ref window_event,
                window_id,
            } if window_id == state.window().id() => {
                // debug!("event_loop window_id={:?} state.window().id()={:?}", window_id, state.window().id());
                
                if !state.input(window_event) {
                
                    match window_event {
                        WindowEvent::MouseInput { device_id, state, button, ..} => {
                            debug!("event_loop::WindowEvent::MouseInput device_id={:?}, state={:?}, button={:?}", device_id, state, button);
                        }
                        WindowEvent::CursorMoved {position, ..} => {
                            debug!("event_loop::WindowEvent::CursorMoved: position={:?}", position);
                            state.pointer_moved(*position);
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