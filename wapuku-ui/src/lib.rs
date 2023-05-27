mod state;
mod resources;
mod mesh_model;
mod texture;
mod camera;
mod light;


use std::cell::RefCell;
use std::collections::HashSet;
use std::mem;
use std::ops::Add;
use std::rc::Rc;
use log::{debug, trace};
use winit::{
    event::*,
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};
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
use wapuku_model::visualization::*;
use futures::future::BoxFuture;
use lazy_static::lazy_static;
use std::sync::{Arc, Mutex, TryLockResult};
use rayon::*;
use rayon::iter::*;
use web_sys::*;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}




lazy_static! {
    static ref POOL_PR:Mutex<Option<u32>> = Mutex::new(None);
}

#[no_mangle]
pub extern "C" fn get_pool()->ThreadPool {
    if let Ok(pool_locked) = POOL_PR.try_lock() {
        debug!("get_pool: got pool");
        if pool_locked.is_some() {
            debug!("get_pool: got pool is_some");
            let pool_addr = pool_locked.unwrap();
            
            **unsafe { Box::from_raw(pool_addr as *mut Box<rayon::ThreadPool>) }
            
        } else {
            panic!("get_pool: no pool");
        }
    } else {
        panic!("get_pool: pool_locked");
    }
}

#[wasm_bindgen]
pub fn init_pool(threads:u32){
    log(format!("wapuku: init_pool,threads={}", threads).as_str());

    // POOL_PR = 1u32;

    // debug!("run_closure ok: ptr={}", ptr);

    // JsValue::from(Box::into_raw(Box::new(Box::new(move || {
    //     debug!("click: In Worker");
    // 
    //     // Box::pin(async {
    //     //     trace!("click: In Worker async");
    //     // 
    //     //     ()
    //     // }) as BoxFuture<'static, ()>
    // 
    // }) as Box<dyn FnOnce()>)) as u32)

    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(3)
        .panic_handler(|_| {
            debug!("Panick!!!!!!!!!!!!!!")
        }).exit_handler(|n| {
        debug!("Exit {}", n);
    })
        .spawn_handler(move |thread| {
            log(format!("wbg_rayon_PoolBuilder::build: send {:?}", thread.name()).as_str());

            let worker = web_sys::Worker::new("./wasm-worker.js").unwrap();
            // worker.post_message(&JsValue::from(&wasm_bindgen::memory())).expect("failed to post");
            // self.sender.send(thread).unwrap_throw();
            let msg = js_sys::Array::new();
            msg.push(&JsValue::from(&wasm_bindgen::memory()));
            msg.push(&JsValue::from("init_worker"));
            msg.push(&JsValue::from(Box::into_raw(Box::new(Box::new(|| thread.run()) as Box<dyn FnOnce()>)) as u32));
            worker.post_message(&msg).expect("failed to post");

            Ok(())
        })
        .build().unwrap();
    // pool.install(||());
    
    let pool_addr =  Box::into_raw(Box::new(Box::new(pool) as Box<rayon::ThreadPool>)) as u32;

    if let Ok(mut pool_locked) = POOL_PR.try_lock() {
        pool_locked.replace(pool_addr);
        debug!("POOL_PR: {:?}", pool_locked);
    }
    
   
}


#[wasm_bindgen]
pub fn run_closure(ptr: u32) {
    let mut closure = unsafe { Box::from_raw(ptr as *mut Box<dyn FnOnce() + Send>) };
    (*closure)();
}

#[wasm_bindgen]
pub fn run_in_pool(ptr: u32) {
    let mut closure = unsafe { Box::from_raw(ptr as *mut Box<dyn FnOnce() + Send>) };

    // POOL.install(closure);
    if let Ok(pool_locked) = POOL_PR.try_lock() {
        debug!("run_in_pool: got pool");
        if pool_locked.is_some() {
            debug!("run_in_pool: got pool is_some");
            
            let pool_addr = pool_locked.unwrap();
            let mut pool = unsafe { Box::from_raw(pool_addr as *mut Box<rayon::ThreadPool>) };
    
            pool.install(closure);
            
            mem::forget(pool);

            debug!("run_in_pool: pool.install done");
        }
    } else {
        debug!("run_in_pool: pool_locked")
    }
    // (*closure)();
}

#[wasm_bindgen]
pub async fn run() {//async should be ok https://github.com/rustwasm/wasm-bindgen/issues/1904 

    std::panic::set_hook(Box::new(console_error_panic_hook::hook));
    console_log::init_with_level(log::Level::Debug).expect("Couldn't initialize logger");

    debug!("wapuku: running");

    //Atomics.wait cannot be called in this context
    
    let pool_worker = web_sys::Worker::new("./wasm-worker.js").unwrap();
    // worker.post_message(&JsValue::from(&wasm_bindgen::memory())).expect("failed to post");

    let msg = js_sys::Array::new();
    
    msg.push(&JsValue::from(&wasm_bindgen::memory()));
    msg.push(&JsValue::from("init_pool"));
    pool_worker.post_message(&msg).expect("failed to post");
    
    let pool_worker_rc = Rc::new(pool_worker);
    let pool_worker_1 = Rc::clone(&pool_worker_rc);
    let pool_worker_2 = Rc::clone(&pool_worker_rc);
    let pool_worker_3 = Rc::clone(&pool_worker_rc);
    
    let mut closure_on_worker = Closure::wrap(Box::new( move |e: web_sys::MessageEvent| {
        debug!("pool is ready: e={:?}", e);
        let data:Box<dyn Data> = Box::new(PolarsData::new(fake_df()));

        let all_properties:HashSet<&dyn Property> = data.all_properties();


        let (property_1, property_2, property_3) = {
            let mut all_properties_iter = all_properties.into_iter().collect::<Vec<&dyn Property>>();
            all_properties_iter.sort_by(|p1, p2| p1.name().cmp(p2.name()));

            (*all_properties_iter.get(0).expect("property_1"), *all_properties_iter.get(1).expect("property_2"), *all_properties_iter.get(2).expect("property_3"))
        };

        let property_x: String = property_1.name().clone();
        let property_y: String = property_2.name().clone();

        debug!("wapuku: property_x={} property_y={}",  property_x, property_y);

        

        let msg = js_sys::Array::new();

        let worker_param_ptr = JsValue::from(Box::into_raw(Box::new(Box::new(move || {
            debug!("click1: VisualDataController");

            VisualDataController::new(data, property_x, property_y, 100, 100);
            debug!("click2: VisualDataController");

           

        }) as Box<dyn FnOnce()>)) as u32);

        msg.push(&JsValue::from("run_in_pool"));
        msg.push(&worker_param_ptr);

        // pool_worker_1.post_message(&msg).expect("failed to post");
        
    }) as Box<dyn FnMut(web_sys::MessageEvent)>);
    pool_worker_2.set_onmessage(Some(&closure_on_worker.as_ref().unchecked_ref()));
    closure_on_worker.forget();





    debug!("workers started");

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

            debug!("wapuku: web_sys::window: size: width={}, height={}", width, height);

            let mut closure_on_mousemove = Closure::wrap(Box::new( move |e: web_sys::MouseEvent| {
                // debug!("wapuku: canvas.mousemove e.client_x()={:?}, e.client_y()={:?}", e.client_x(), e.client_y());
                // debug!("wapuku: canvas.mousemove e.offset_x()={:?}, e.offset_y()={:?}", e.offset_x(), e.offset_y());
                // 
                // if let Ok(mut mouse_yx_for_on_mousemove_borrowed) = pointer_xy_for_on_mousemove.try_borrow_mut() {
                //     mouse_yx_for_on_mousemove_borrowed.replace((e.offset_x() as f32, e.offset_y() as f32));
                // }
                debug!("wapuku: click");
               /* if let Ok(pool_locked) = POOL_PR.try_lock() {
                    if pool_locked.is_some() {
                        let pool_addr = pool_locked.unwrap();
                        let mut pool = unsafe { Box::from_raw(pool_addr as *mut Box<rayon::ThreadPool>) };

                        pool.install(||debug!("click click click"));
                    }
                }*/
                let msg = js_sys::Array::new();

                let worker_param_ptr = JsValue::from(Box::into_raw(Box::new(Box::new(move || {
                    

                    let v: Vec<_> = (0..10_000_000).collect();
                    let x = v.par_iter().sum::<i32>();

                    debug!("click: In Worker: x={}", x);
                    
                    // Box::pin(async {
                    //     trace!("click: In Worker async");
                    // 
                    //     ()
                    // }) as BoxFuture<'static, ()>
                
                }) as Box<dyn FnOnce()>)) as u32);
                
                msg.push(&JsValue::from("run_in_pool"));
                msg.push(&worker_param_ptr);

                pool_worker_3.post_message(&msg).expect("failed to post");

            }) as Box<dyn FnMut(web_sys::MouseEvent)>);

            div.add_event_listener_with_callback("click", &closure_on_mousemove.as_ref().unchecked_ref());

            closure_on_mousemove.forget();
            
            Some((width, height))
        })
        .expect("Couldn't append canvas to document body.");

    
    // let data:Box<dyn Data> = Box::new(PolarsData::new(parquet_scan()));
   /* let data:Box<dyn Data> = Box::new(PolarsData::new(fake_df()));
    // let data:Box<dyn Data> = Box::new(TestData::new());
    
    let all_properties:HashSet<&dyn Property> = data.all_properties();
    

    let (property_1, property_2, property_3) = {
        let mut all_properties_iter = all_properties.into_iter().collect::<Vec<&dyn Property>>();
        all_properties_iter.sort_by(|p1, p2| p1.name().cmp(p2.name()));

        (*all_properties_iter.get(0).expect("property_1"), *all_properties_iter.get(1).expect("property_2"), *all_properties_iter.get(2).expect("property_3"))
    };

    let property_x: String = property_1.name().clone();
    let property_y: String = property_2.name().clone();
    
    debug!("wapuku: property_x={} property_y={}",  property_x, property_y);

    VisualDataController::new(data, property_x, property_y, width, height);*/
    // data


/*
    let mut gpu_state = State::new(winit_window, VisualDataController::new(data, property_x, property_y, width, height)).await;
    
    event_loop.run(move |event, _, control_flow| {

        // debug!("wapuku: event_loop.run={:?}", event);

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
                        debug!("wapuku: event_loop::DeviceEvent::MouseMotion: delta={:?}", delta);
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
                            debug!("wapuku: event_loop::WindowEvent::MouseInput device_id={:?}, state={:?}, button={:?}", device_id, state, button);
                            match state {
                                ElementState::Pressed => {}
                                ElementState::Released => {
                                    if let Ok(mut xy_ref) = pointer_xy_for_state_update.try_borrow_mut() {
                                        debug!("wapuku: event_loop::WindowEvent::MouseInput got pointer_xy_for_state_update xy_ref={:?}", xy_ref);

                                        if let Some(xy) = xy_ref.as_ref() {
                                            gpu_state.pointer_input(xy.0, xy.1);
                                        }
                                    } else {
                                        debug!("wapuku: event_loop::WindowEvent::MouseInput can't get pointer_xy_for_state_update ");
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
    })*/;
    
}