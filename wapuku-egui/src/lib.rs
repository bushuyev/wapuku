use std::alloc::System;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::atomic::{AtomicI64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use log::{debug, log, trace};
pub use wapuku_common_web::get_pool;
pub use wapuku_common_web::init_pool;
pub use wapuku_common_web::init_worker;
pub use wapuku_common_web::run_in_pool;
pub use wapuku_common_web::allocator::tracing::*;
use wapuku_common_web::workers::PoolWorker;
use wapuku_model::model::{Data, FrameView};
use wapuku_model::polars_df::PolarsData;
use wasm_bindgen::prelude::*;

pub use app::WapukuApp;

use crate::app::{Action, WapukuAppModel};

mod app;
mod model_views;


#[derive(Debug)]
pub enum DataMsg {
    AddFrame {name: String, frame: FrameView},
    Ok {},
    Err { msg:String}
}





#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    pub fn log(s: &str);
}

static TOTAL_MEM:f32 = 50000.0 * 65536.0;

static ALLOCATED:AtomicUsize = AtomicUsize::new(0);

#[no_mangle]
pub extern "C" fn on_alloc(size: usize, align: usize, pointer: *mut u8) {
    ALLOCATED.fetch_add(size, Ordering::Relaxed);
}

#[no_mangle]
pub extern "C" fn on_dealloc(size: usize, align: usize, pointer: *mut u8) {
    ALLOCATED.fetch_sub(size, Ordering::Relaxed);
}


#[no_mangle]
pub extern "C" fn on_alloc_zeroed(size: usize, align: usize, pointer: *mut u8) {
    ALLOCATED.fetch_add(size, Ordering::Relaxed);
}

#[no_mangle]
pub extern "C" fn on_realloc(
    old_pointer: *mut u8,
    new_pointer: *mut u8,
    old_size: usize,
    new_size: usize,
    align: usize,
) {
    ALLOCATED.fetch_add(new_size - old_size, Ordering::Relaxed);
}

#[global_allocator]
static GLOBAL_ALLOCATOR: TracingAllocator<System> = TracingAllocator(System);

#[wasm_bindgen]
pub async fn run() {
    let window = web_sys::window().unwrap();

    std::panic::set_hook(Box::new(console_error_panic_hook::hook));
    console_log::init_with_level(log::Level::Trace).expect("Couldn't initialize logger");


    let runner_rc = Rc::new(eframe::WebRunner::new());
    let runner_rc_1 = Rc::clone(&runner_rc);
    let runner_rc_2 = Rc::clone(&runner_rc);

    let (to_main, from_worker) = std::sync::mpsc::channel::<DataMsg>();
    let to_main_rc = Rc::new(to_main);
    let to_main_rc_1 = Rc::clone(&to_main_rc);
    let to_main_rc_2 = Rc::clone(&to_main_rc);
    let from_worker_rc = Rc::new(from_worker);
    let from_worker_rc1 = Rc::clone(&from_worker_rc);

    let pool_worker = PoolWorker::new();

    pool_worker.init().await.expect("pool_worker init");

    let model = WapukuAppModel::new();
    let model_box = Box::pin(model);
    debug!("wapuku: model_box_ptr={:p}", &model_box);
    model_box.debug_ptr();

    let mut wapuku_app_model = Rc::new(RefCell::new(model_box));
    let mut wapuku_app_model_rc1 = Rc::clone(&wapuku_app_model);
    let mut wapuku_app_model_rc2 = Rc::clone(&wapuku_app_model);

    let model_lock:Arc<Mutex<usize>> = Arc::new(Mutex::new(0));
    let model_lock_arc = Arc::clone(&model_lock);

    let timer_closure = Closure::wrap(Box::new(move || {

        if let Ok(mut model_borrowed) = wapuku_app_model_rc1.try_borrow_mut() {

            if let Some(action) = model_borrowed.get_next_action(){
                debug!("wapuku: got action {:?}", action);
                match action {
                    Action::LoadFile{name_ptr, data_ptr} => {

                        let to_main_rc_1_1 = Rc::clone(&to_main_rc_1);

                        pool_worker.run_in_pool( move || {

                            let data = unsafe { Box::from_raw(data_ptr as *mut Box<Vec<u8>>) };
                            let name = unsafe { Box::from_raw(name_ptr as *mut Box<String>) };
                            // let model = unsafe { Box::from_raw(0x610750 as *mut WapukuAppModel) };
                            debug!("wapuku: running in pool, load file name={:?}", name);

                            match PolarsData::load(*data, *name.clone()) {
                                Ok(frames) => {
                                    // if let Ok(lock) = model_lock_arc.try_lock() {
                                        debug!("wapuku::run_in_pool: got model");

                                        for df in frames {
                                            // model_borrowed.add_frame(df.name().clone(), Box::new(df));
                                            to_main_rc_1_1.send(DataMsg::AddFrame {
                                                name: df.name().clone(),
                                                frame: FrameView::new(
                                                    df.name().clone(),
                                                    Box::new(df)
                                                )
                                            }).expect("send");
                                        }
                                    // } else {
                                    //     debug!("wapuku::run_in_pool: model locked ");
                                    // }
                                }
                                Err(e) => {
                                    to_main_rc_1_1.send(DataMsg::Err { msg: String::from(e.to_string()) }).expect("send");
                                }
                            }


                        });
                    }
                    Action::Histogram { frame_id, name_ptr} => {
                        pool_worker.run_in_pool( || {
                            let name = unsafe { Box::from_raw(name_ptr as *mut Box<String>) };
                            debug!("wapuku: running in pool, ::ListUnique name={}", name);
                            //
                            if let Ok(lock) = model_lock_arc.try_lock() {
                                debug!("wapuku::run_in_pool: got model");
                                model_borrowed.histogram(frame_id, *name);

                            } else {
                                debug!("wapuku::run_in_pool: model locked ");
                            }

                            // to_main_rc_1.send(DataMsg::Summary{min:0., avg: 1., max:2.}).expect("send");

                        });
                    }
                }
            }

            if let Ok(data_msg) = from_worker_rc1.try_recv() {

                match data_msg {
                    DataMsg::Ok {  } => {
                    }
                    DataMsg::Err { msg } => {
                        trace!("wapuku: error={:?}", msg);
                        model_borrowed.set_error(msg);
                    }
                    DataMsg::AddFrame { name, frame } => {
                        model_borrowed.add_frame(frame);
                    }
                }
            }
            // debug!("wapuku: ALLOCATED={:?} TOTAL_MEM={:?} p={}", ALLOCATED, TOTAL_MEM, ALLOCATED.load(Ordering::Relaxed) as f32 / TOTAL_MEM);
            //
            model_borrowed.set_memory_allocated(ALLOCATED.load(Ordering::Relaxed) as f32 / TOTAL_MEM);
        }

    }) as Box<dyn FnMut()>);

    if window.set_interval_with_callback_and_timeout_and_arguments_0(&timer_closure.as_ref().unchecked_ref(), 100).is_err() {
        panic!("set_interval_with_callback_and_timeout_and_arguments_0");
    }
    timer_closure.forget();

    // eframe::WebLogger::init(log::LevelFilter::Debug).ok();

    let web_options = eframe::WebOptions::default();


    wasm_bindgen_futures::spawn_local(async move {runner_rc_1.start(
        "the_canvas_id", // hardcode it
        web_options,
        Box::new(|cc| {
            debug!("eframe::WebRunner WapukuApp::new");
            Box::new(WapukuApp::new(cc, wapuku_app_model_rc2))
        }),
    )
    .await
    .expect("failed to start eframe")});
}

