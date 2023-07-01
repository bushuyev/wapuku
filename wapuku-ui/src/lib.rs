#![feature(async_fn_in_trait)]

mod state;
mod resources;
mod mesh_model;
mod texture;
mod camera;
mod light;
mod workers;
pub mod visualization;


use std::cell::RefCell;
use std::collections::HashSet;
use std::mem;
use std::ops::Add;
use std::rc::Rc;
use log::{debug, trace, warn};
use winit::{event::*, event, event_loop::{ControlFlow, EventLoop}, window::WindowBuilder};
use winit::dpi::{LogicalSize, PhysicalPosition, PhysicalSize};
use winit::event_loop::EventLoopBuilder;
use wasm_bindgen::prelude::*;
use wasm_bindgen::closure::*;
use winit::platform::web::WindowExtWebSys;
use winit::window::Fullscreen;
use crate::state::State;
use wapuku_model::polars_df::*;
use wapuku_model::model::*;
use wapuku_model::test_data::*;

use futures::future::BoxFuture;
use lazy_static::lazy_static;
use std::sync::{Arc, Mutex, TryLockResult};
use rayon::*;
use rayon::iter::*;
use web_sys::*;

use std::alloc::System;
use std::pin::Pin;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{Receiver, Sender};
use std::time::Duration;
use workers::*;
use crate::visualization::VisualDataController;
use std::{hint};
use std::future::Future;
use crate::workers::interval_future::IntervalFuture;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

static POOL_PR:Mutex<Option<u32>> = Mutex::new(None);//Mutext not needed, addr

static mut TO_POOL_SENDER: Option<Box<Sender<&str>>> = None;
static mut FROM_MAIN_RECEIVER: Option<Box<Receiver<&str>>> = None;

fn worker_global_scope() -> Option<WorkerGlobalScope> {
    js_sys::global().dyn_into::<WorkerGlobalScope>().ok()
}

#[no_mangle]
pub extern "C" fn get_pool()->ThreadPool {

    let pool_addr_op:Option<u32> = if let Ok(pool_locked) = POOL_PR.try_lock() {
        log("wapuku: get_pool: got pool");
        if pool_locked.is_some() {
            log("get_pool: got pool is_some");
            let pool_addr = pool_locked.unwrap();
            
            Some(pool_addr)
        } else {
            log("wapuku: get_pool: no pool_addr");
            None
        }
    } else {
        log("wapuku: get_pool: pool_locked");
        None
    };

    log(format!("wapuku: get_pool: pool_addr_op={:?}", pool_addr_op).as_str());
    
    if let Some(pool_addr) = pool_addr_op {
        //invalidates
        **unsafe { Box::from_raw(pool_addr as *mut Box<rayon::ThreadPool>) }
    } else {
        panic!("get_pool: no pool");
    }
}

#[wasm_bindgen]
pub async fn init_pool(threads:usize) {
    log(format!("wapuku: init_pool,threads={}", threads).as_str());

    let mut counter = Arc::new(AtomicUsize::new(0));
    let  counter_clone_top = Arc::clone(&counter);
    let  counter_clone_clear = Arc::clone(&counter);

    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(threads)
        .panic_handler(|_| {
            debug!("wapuku: Panick!!!!!!!!!!!!!!")
        }).exit_handler(|n| {
        debug!("Exit {}", n);
    })
        .spawn_handler(move |thread| {
            log(format!("wapuku: wbg_rayon_PoolBuilder::build: send {:?}", thread.name()).as_str());
            let counter_clone = Arc::clone(&counter);
            
            let worker = web_sys::Worker::new("./wasm-worker.js").unwrap();
            // worker.post_message(&JsValue::from(&wasm_bindgen::memory())).expect("failed to post");
            // self.sender.send(thread).unwrap_throw();
            let msg = js_sys::Array::new();
            msg.push(&JsValue::from("init_worker"));
            msg.push(&JsValue::from(&wasm_bindgen::memory()));
            msg.push(&JsValue::from(Box::into_raw(Box::new(Box::new(|| {
                log(format!("wapuku: running thread in pool").as_str());
                thread.run()
                
            }) as Box<dyn FnOnce()>)) as u32));

            let mut closure_on_worker = Closure::wrap(Box::new(move |e: web_sys::MessageEvent| {
                debug!("worker is ready: counter_clone={:?}", counter_clone);
                counter_clone.fetch_add(1, Ordering::Relaxed);
            }) as Box<dyn FnMut(web_sys::MessageEvent)>);

            worker.set_onmessage(Some(&closure_on_worker.as_ref().unchecked_ref()));
            closure_on_worker.forget();
            
            worker.post_message(&msg).expect("failed to post");

            log(format!("wapuku: wbg_rayon_PoolBuilder::build: done").as_str());

            Ok(())
        })
        .build().unwrap();
    
    // pool.install(||());
    debug!("wapuku: init_pool: 1. counter={:?}", counter_clone_top);
    // while counter_clone_top.load(Ordering::Acquire) != threads {
        // hint::spin_loop();
    // }
   
    
    
    let pool_addr =  Box::into_raw(Box::new(Box::new(pool) as Box<rayon::ThreadPool>)) as u32;

    
    if let Ok(mut pool_locked) = POOL_PR.try_lock() {
        pool_locked.replace(pool_addr);
        debug!("wapuku: init_pool: pool_addr={}", pool_addr);
    }
    
    IntervalFuture::new(move || {
        debug!("wapuku: init_pool: 3. counter={:?}", counter_clone_clear);
        counter_clone_clear.load(Ordering::Acquire) >= threads

    }).await
}

#[wasm_bindgen]
pub fn init_worker(ptr: u32) {
    log(format!("wapuku: init_worker={}", ptr).as_str());

    // console_log::init_with_level(log::Level::Debug).expect("Couldn't initialize logger");
    
    let mut closure = unsafe { Box::from_raw(ptr as *mut Box<dyn FnOnce() + Send>) };
    (*closure)();

}

#[wasm_bindgen]
pub fn run_in_pool(ptr: u32) {
    log("wapuku: run_in_pool");
    let mut closure = unsafe { Box::from_raw(ptr as *mut Box<dyn FnOnce() + Send>) };
    (*closure)();
}

#[wasm_bindgen]
pub async fn run() {//async should be ok https://github.com/rustwasm/wasm-bindgen/issues/1904 
    
    std::panic::set_hook(Box::new(console_error_panic_hook::hook));
    console_log::init_with_level(log::Level::Debug).expect("Couldn't initialize logger");

    let (to_main, from_worker) = std::sync::mpsc::channel::<GroupsGrid>();
    debug!("wapuku: running");
  

    let workder_rc = Rc::new(web_sys::Worker::new("./wasm-worker.js").expect(format!("can't make worker for {}", "./wasm-worker.js").as_str()));
    
    // let init_pool_futrue = WorkerFuture::new(
    //     Rc::clone(&workder_rc), //"./wasm-worker.js",
    //     Box::new( move || {
    //         let msg = js_sys::Array::new();
    // 
    //         msg.push(&JsValue::from("init_pool"));
    //         msg.push(&JsValue::from(&wasm_bindgen::memory()));
    //         msg
    //     })
    // ).await;
    
    let pool_worker = PoolWorker::new();
    pool_worker.init().await;
    
    debug!("wapuku: workers started");

    let event_loop = EventLoopBuilder::<()>::with_user_event().build();
    let winit_window = WindowBuilder::new().with_resizable(true).build(&event_loop).unwrap();

    //winit sends client xy  in WindowEvent::CursorMoved, we need offset
    let mut mouse_xy:Rc<RefCell<Option<(f32, f32)>>> = Rc::new(RefCell::new(None));

    let mut pointer_xy_for_on_mousemove = Rc::clone(&mouse_xy);
    let mut pointer_xy_for_state_update = Rc::clone(&mouse_xy);

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
    let mut visual_data_controller_rc = Rc::new(RefCell::new(VisualDataController::new(&data, width, height)));
    let data_rc = Rc::new(data);


    let data_in_init = Rc::clone(&data_rc);
    let visual_data_controller_rc_worker = Rc::clone(&visual_data_controller_rc);
    
    pool_worker.run_in_pool(move || {

        log(format!("wapuku: running in pool").as_str());
        let visual_data_controller_rc_borrowed = visual_data_controller_rc_worker.borrow();

        let data_grid =  data_in_init.build_grid(
            PropertyRange::new(&*visual_data_controller_rc_borrowed.property_x, None, None),
            PropertyRange::new(&*visual_data_controller_rc_borrowed.property_y, None, None),
            visual_data_controller_rc_borrowed.groups_nr_x, visual_data_controller_rc_borrowed.groups_nr_y, "property_3",//TODO
        );

        log(format!("wapuku: got data_grid={:?}", data_grid).as_str());

        to_main.send(data_grid);

    });
    

    debug!("wapuku: web_sys::window: size: width={}, height={}", width, height);

    let mut closure_on_mousemove = Closure::wrap(Box::new( move |e: web_sys::MouseEvent| {
        debug!("wapuku: canvas.mousemove e.client_x()={:?}, e.client_y()={:?}", e.client_x(), e.client_y());

        if let Ok(mut mouse_yx_for_on_mousemove_borrowed) = pointer_xy_for_on_mousemove.try_borrow_mut() {
            mouse_yx_for_on_mousemove_borrowed.replace((e.offset_x() as f32, e.offset_y() as f32));
        }

    }) as Box<dyn FnMut(web_sys::MouseEvent)>);
    div.add_event_listener_with_callback("mousemove", &closure_on_mousemove.as_ref().unchecked_ref());
    closure_on_mousemove.forget();
            
    
    // let data:Box<dyn Data> = Box::new(PolarsData::new(parquet_scan()));
    // let data:Box<dyn Data> = Box::new(PolarsData::new(fake_df()));
    // let data:Box<dyn Data> = Box::new(TestData::new());
    
    
    // visual_data_controller.update_visuals(&data);
    
    let mut gpu_state = State::new(winit_window).await;
    let visual_data_controller_in_loop = Rc::clone(&visual_data_controller_rc);
    event_loop.run(move |event, _, control_flow| {
        let mut visual_data_controller_borrowed_mut_op = visual_data_controller_in_loop.try_borrow_mut().ok();
        
        if let Some(mut visual_data_controller_borrowed_mut) = visual_data_controller_borrowed_mut_op.as_mut() {
            if let Ok(msg) = from_worker.try_recv() {
                debug!("wapuku: event_loop got data_grid={:?}", msg);
                visual_data_controller_borrowed_mut.update_visuals(msg);
            }
        }
        // debug!("wapuku: event_loop.run={:?}", event);
    
        match event {
            event::Event::RedrawRequested(window_id) if window_id == gpu_state.window().id() => {
                if let Some(visual_updates) = visual_data_controller_borrowed_mut_op.as_mut().and_then(|mut v|v.visuals_updates()) {
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
                device_id,
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
                                    if let Ok(mut xy_ref) = pointer_xy_for_state_update.try_borrow_mut() {
                                        debug!("wapuku: event_loop::WindowEvent::MouseInput got pointer_xy_for_state_update xy_ref={:?}", xy_ref);
                                        if let Some(mut visual_data_controller_borrowed_mut) = visual_data_controller_borrowed_mut_op.as_mut() {
                                            if let Some(xy) = xy_ref.as_ref() {
                                                visual_data_controller_borrowed_mut.on_pointer_input(xy.0, xy.1);
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
                            
                            if let Ok(mut xy_ref) = pointer_xy_for_state_update.try_borrow_mut() {
                                debug!("wapuku: event_loop::WindowEvent::CursorMoved xy_ref={:?}", xy_ref);
                                if let Some(mut visual_data_controller_borrowed_mut) = visual_data_controller_borrowed_mut_op.as_mut() {
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
