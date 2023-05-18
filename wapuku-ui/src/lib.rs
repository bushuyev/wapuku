mod state;
mod resources;
mod mesh_model;
mod texture;
mod camera;
mod light;


use std::cell::RefCell;
use std::collections::HashSet;
use std::rc::Rc;
use std::sync::Arc;
use log::{debug, trace};
use winit::{
    event::*,
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};
use winit::dpi::{LogicalSize, PhysicalPosition, PhysicalSize};
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

    debug!("running");

    let event_loop = EventLoopBuilder::<()>::with_user_event().build();
    let winit_window = WindowBuilder::new().with_resizable(true).build(&event_loop).unwrap();

    //winit sends client xy  in WindowEvent::CursorMoved, we need offset
    let mut mouse_xy:Rc<RefCell<Option<(f32, f32)>>> = Rc::new(RefCell::new(None));

    let mut pointer_xy_for_on_mousemove = Rc::clone(&mouse_xy);
    let mut pointer_xy_for_state_update = Rc::clone(&mouse_xy);

    let (width, height) = web_sys::window()
        .and_then(|win| win.document())
        .and_then(|doc| {
            let div = doc.get_element_by_id("wapuku_canvas_div")?;
            let width = div.client_width();
            let height = div.client_height();
            
            let canvas = web_sys::Element::from(winit_window.canvas());
            // let canvas = doc.get_element_by_id("wapuku_canvas")?; // winit fails with Canvas is not found
            div.append_child(&canvas).ok()?;
            

            winit_window.set_inner_size(PhysicalSize::new(width, height));

            debug!("web_sys::window: size: width={}, height={}", width, height);

            let mut closure_on_mousemove = Closure::wrap(Box::new( move |e: web_sys::MouseEvent| {
                debug!("canvas.mousemove e.client_x()={:?}, e.client_y()={:?}", e.client_x(), e.client_y());
                debug!("canvas.mousemove e.offset_x()={:?}, e.offset_y()={:?}", e.offset_x(), e.offset_y());

                if let Ok(mut mouse_yx_for_on_mousemove_borrowed) = pointer_xy_for_on_mousemove.try_borrow_mut() {
                    mouse_yx_for_on_mousemove_borrowed.replace((e.offset_x() as f32, e.offset_y() as f32));
                }

            }) as Box<dyn FnMut(web_sys::MouseEvent)>);

            div.add_event_listener_with_callback("mousemove", &closure_on_mousemove.as_ref().unchecked_ref());

            closure_on_mousemove.forget();
            
            Some((width, height))
        })
        .expect("Couldn't append canvas to document body.");

    
    // let data:Box<dyn Data> = Box::new(PolarsData::new(parquet_scan()));
    let data:Box<dyn Data> = Box::new(TestData::new());
    
    let all_properties:HashSet<&dyn Property> = data.all_properties();
    

    let (property_1, property_2, property_3) = {
        let mut all_properties_iter = all_properties.into_iter();
        
        (all_properties_iter.next().expect("property_1"), all_properties_iter.next().expect("property_2"), all_properties_iter.next().expect("property_3"))
    };

    let data_grid = data.group_by_2(
        PropertyRange::new (property_1,  None, None ),
        PropertyRange::new (property_2,  None, None ),
        3, 3
    );

    let property_x: String = property_1.name().clone();
    let property_y: String = property_2.name().clone();
    
    debug!("data_grid: {:?} property_x={} property_y={}", data_grid, property_x, property_y);
    
    // data
    let mut gpu_state = State::new(winit_window, VisualDataController::new(data, property_x, property_y, width, height)).await;

    event_loop.run(move |event, _, control_flow| {
        // debug!("event_loop.run={:?}", event);

        match event {
            Event::RedrawRequested(window_id) if window_id == gpu_state.window().id() => {
                gpu_state.update(/*data.visuals()*/);
                match gpu_state.render() {
                    Ok(_) => {}
                    // Reconfigure the surface if lost
                    Err(wgpu::SurfaceError::Lost) => gpu_state.resize(gpu_state.size),
                    // The system is out of memory, we should probably quit
                    Err(wgpu::SurfaceError::OutOfMemory) => *control_flow = ControlFlow::Exit,
                    // All other errors (Outdated, Timeout) should be resolved by the next frame
                    Err(e) => eprintln!("{:?}", e),
                }
            }

            Event::MainEventsCleared => {
                // RedrawRequested will only trigger once, unless we manually
                // request it.
                gpu_state.window().request_redraw();
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
            } if window_id == gpu_state.window().id() => {
                
                if !gpu_state.input(window_event) {
                
                    match window_event {
                        WindowEvent::MouseInput { device_id, state, button, ..} => {
                            debug!("event_loop::WindowEvent::MouseInput device_id={:?}, state={:?}, button={:?}", device_id, state, button);
                            match state {
                                ElementState::Pressed => {}
                                ElementState::Released => {
                                    if let Ok(mut xy_ref) = pointer_xy_for_state_update.try_borrow_mut() {
                                        debug!("event_loop::WindowEvent::MouseInput got pointer_xy_for_state_update xy_ref={:?}", xy_ref);

                                        if let Some(xy) = xy_ref.as_ref() {
                                            gpu_state.pointer_input(xy.0, xy.1);
                                        }
                                    } else {
                                        debug!("event_loop::WindowEvent::MouseInput can't get pointer_xy_for_state_update ");
                                    }

                                }
                            }
                        }
                        WindowEvent::CursorMoved {..} => {
                            
                            if let Ok(mut xy_ref) = pointer_xy_for_state_update.try_borrow_mut() {
                                if let Some(xy) = xy_ref.as_ref() {
                                    gpu_state.pointer_moved(xy.0, xy.1);
                                }
                            }
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
                            gpu_state.resize(*physical_size);
                        }
                        WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                            gpu_state.resize(**new_inner_size);
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    });
    
}