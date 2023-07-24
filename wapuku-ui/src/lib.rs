#![feature(async_fn_in_trait)]
#[allow(mixed_script_confusables)]

mod state;
mod resources;
mod mesh_model;
mod texture;
mod camera;
mod light;

pub mod visualization;


use std::cell::RefCell;



use std::rc::Rc;
use log::{debug};
use winit::{event::*, event, event_loop::{ControlFlow}, window::WindowBuilder};
use winit::dpi::{PhysicalSize};
use winit::event_loop::EventLoopBuilder;
use rayon::*;

use wasm_bindgen::prelude::*;
use winit::platform::web::WindowExtWebSys;

use crate::state::State;
use wapuku_model::polars_df::*;
use wapuku_model::model::*;

use crate::visualization::VisualDataController;
use wapuku_common_web::workers::*;

pub use wapuku_common_web::init_worker;
pub use wapuku_common_web::get_pool;
pub use wapuku_common_web::init_pool;
pub use wapuku_common_web::run_in_pool;
use wapuku_common_web::log;


#[wasm_bindgen]
pub async fn run() {//async should be ok https://github.com/rustwasm/wasm-bindgen/issues/1904 
    
    std::panic::set_hook(Box::new(console_error_panic_hook::hook));
    console_log::init_with_level(log::Level::Debug).expect("Couldn't initialize logger");

    let (to_main, from_worker) = std::sync::mpsc::channel::<GroupsGrid>();
    let to_main_rc = Rc::new(to_main);
    let to_main_rc_1 = Rc::clone(&to_main_rc);
    let to_main_rc_2 = Rc::clone(&to_main_rc);
    debug!("wapuku: running");
  

    let pool_worker = PoolWorker::new();
    pool_worker.init().await.expect("pool_worker init");
    
    debug!("wapuku: workers started");

    let event_loop = EventLoopBuilder::<()>::with_user_event().build();
    let winit_window = WindowBuilder::new().with_resizable(true).build(&event_loop).unwrap();

    //winit sends client xy  in WindowEvent::CursorMoved, we need offset
    let mouse_xy:Rc<RefCell<Option<(f32, f32)>>> = Rc::new(RefCell::new(None));

    let pointer_xy_for_on_mousemove = Rc::clone(&mouse_xy);
    let pointer_xy_for_state_update = Rc::clone(&mouse_xy);

    let win = web_sys::window().expect("window");
    let doc = win.document().expect("document");

 
    let div = doc.get_element_by_id("wapuku_canvas_div").expect("wapuku_canvas_div");
    let width = div.client_width();
    let height = div.client_height();
    
    let canvas = web_sys::Element::from(winit_window.canvas());
    // let canvas = doc.get_element_by_id("wapuku_canvas")?; // winit fails with Canvas is not found
    div.append_child(&canvas).ok().expect("canvas");
    

    winit_window.set_inner_size(PhysicalSize::new(width, height));

    let data = PolarsData::new(fake_df());
    let visual_data_controller_rc = Rc::new(RefCell::new(VisualDataController::new(&data, width, height)));
    let data_rc = Rc::new(data);


    let data_in_init = Rc::clone(&data_rc);
    let data_in_on_pointer = Rc::clone(&data_rc);
    let visual_data_controller_rc_worker_1 = Rc::clone(&visual_data_controller_rc);


    pool_worker.run_in_pool(move || {

        log(format!("wapuku: running in pool").as_str());
        let visual_data_controller_rc_borrowed = visual_data_controller_rc_worker_1.borrow();

        let data_grid =  data_in_init.build_grid(
            PropertyRange::new(&*visual_data_controller_rc_borrowed.property_x, None, None),
            PropertyRange::new(&*visual_data_controller_rc_borrowed.property_y, None, None),
            visual_data_controller_rc_borrowed.groups_nr_x, visual_data_controller_rc_borrowed.groups_nr_y, "property_3",//TODO
        );

        log(format!("wapuku: got data_grid={:?}", data_grid).as_str());

        to_main_rc_1.send(data_grid).expect("send");

    });
    

    debug!("wapuku: web_sys::window: size: width={}, height={}", width, height);

    let closure_on_mousemove = Closure::wrap(Box::new( move |e: web_sys::MouseEvent| {
        // debug!("wapuku: canvas.mousemove e.client_x()={:?}, e.client_y()={:?}", e.client_x(), e.client_y());

        if let Ok(mut mouse_yx_for_on_mousemove_borrowed) = pointer_xy_for_on_mousemove.try_borrow_mut() {
            mouse_yx_for_on_mousemove_borrowed.replace((e.offset_x() as f32, e.offset_y() as f32));
        }

    }) as Box<dyn FnMut(web_sys::MouseEvent)>);
    div.add_event_listener_with_callback("mousemove", &closure_on_mousemove.as_ref().unchecked_ref()).expect("mousemove");
    closure_on_mousemove.forget();


    let mut gpu_state = State::new(winit_window).await;
    let visual_data_controller_in_loop = Rc::clone(&visual_data_controller_rc);
    event_loop.run(move |event, _, control_flow| {
        let mut visual_data_controller_borrowed_mut_op = visual_data_controller_in_loop.try_borrow_mut().ok();
        
        if let Some(visual_data_controller_borrowed_mut) = visual_data_controller_borrowed_mut_op.as_mut() {
            if let Ok(msg) = from_worker.try_recv() {
                debug!("wapuku: event_loop got data_grid={:?}", msg);
                visual_data_controller_borrowed_mut.update_visuals(msg);
            }
        }
        // debug!("wapuku: event_loop.run={:?}", event);
    
        match event {
            event::Event::RedrawRequested(window_id) if window_id == gpu_state.window().id() => {
                if let Some(visual_updates) = visual_data_controller_borrowed_mut_op.as_mut().and_then(|v|v.visuals_updates()) {
                    gpu_state.update(visual_updates);
                }

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
    
            event::Event::MainEventsCleared => {
                // RedrawRequested will only trigger once, unless we manually
                // request it.
                gpu_state.window().request_redraw();
            }
    
            event::Event::DeviceEvent {
                event: ref device_event,
                device_id: _,
            } => {
                match device_event {
                    DeviceEvent::MouseMotion { delta } => {
                        debug!("wapuku: event_loop::DeviceEvent::MouseMotion: delta={:?}", delta);
                    }
                    _ => {
                        
                    }
                }
            }
    
            event::Event::WindowEvent {
                event: ref window_event,
                window_id,
            } if window_id == gpu_state.window().id() => {

                if !gpu_state.input(window_event) {
                
                    match window_event {
                        WindowEvent::MouseInput { device_id, state, button, ..} => {
                            debug!("wapuku: event_loop::WindowEvent::MouseInput device_id={:?}, state={:?}, button={:?}", device_id, state, button);
                            match state {
                                ElementState::Pressed => {}
                                ElementState::Released => {
                                    if let Ok(xy_ref) = pointer_xy_for_state_update.try_borrow_mut() {
                                        debug!("wapuku: event_loop::WindowEvent::MouseInput got pointer_xy_for_state_update xy_ref={:?}", xy_ref);
                                        if let Some(visual_data_controller_borrowed_mut) = visual_data_controller_borrowed_mut_op.as_mut() {
                                            if let Some(xy) = xy_ref.as_ref() {
                                                // visual_data_controller_borrowed_mut.on_pointer_input(xy.0, xy.1);
                                                let nr_x = visual_data_controller_borrowed_mut.groups_nr_x;
                                                let nr_y = visual_data_controller_borrowed_mut.groups_nr_y;
                                                 if let Some(bounds) = visual_data_controller_borrowed_mut.get_visual_under_pointer(xy.0, xy.1).map(|g|g.bounds()) {
                                                     match bounds {
                                                         DataBounds::XY(property_x, property_y) => {
                                                             let to_main_rc_2_moved = Rc::clone(&to_main_rc_2);
                                                             let data_in_on_pointer_moved = Rc::clone(&data_in_on_pointer);

                                                             // let visual_data_controller_rc_borrowed = visual_data_controller_rc_worker_2.borrow();

                                                             pool_worker.run_in_pool(move || {

                                                                 log(format!("wapuku: running in pool").as_str());


                                                                 let data_grid =  data_in_on_pointer_moved.build_grid(
                                                                     property_x.clone_to_box(),
                                                                     property_y.clone_to_box(),
                                                                     nr_x, nr_y, "property_3",//TODO
                                                                 );

                                                                 log(format!("wapuku: got data_grid={:?}", data_grid).as_str());

                                                                 to_main_rc_2_moved.send(data_grid).expect("send to main");

                                                             });
                                                         }
                                                         _ => {
                                                             //TODO
                                                         }
                                                     }
                                                 }
                                            }
                                        }
                                    } else {
                                        debug!("wapuku: event_loop::WindowEvent::MouseInput can't get pointer_xy_for_state_update ");
                                    }
    
                                }
                            }
                        }
                        WindowEvent::CursorMoved {..} => {
                            debug!("wapuku: event_loop::WindowEvent::CursorMoved pointer_xy_for_state_update={:?}", pointer_xy_for_state_update);
                            
                            if let Ok(xy_ref) = pointer_xy_for_state_update.try_borrow_mut() {
                                debug!("wapuku: event_loop::WindowEvent::CursorMoved xy_ref={:?}", xy_ref);
                                if let Some(visual_data_controller_borrowed_mut) = visual_data_controller_borrowed_mut_op.as_mut() {
                                    if let Some(xy) = xy_ref.as_ref() {
                                        visual_data_controller_borrowed_mut.on_pointer_moved(xy.0, xy.1);
                                    }
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
