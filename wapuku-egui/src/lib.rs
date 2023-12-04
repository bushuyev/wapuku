use std::alloc::System;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::atomic::{AtomicUsize, Ordering};

use log::{debug, trace};

pub use wapuku_common_web::allocator::tracing::*;
pub use wapuku_common_web::get_pool;
pub use wapuku_common_web::init_pool;
pub use wapuku_common_web::init_worker;
pub use wapuku_common_web::run_in_pool;
use wapuku_common_web::workers::PoolWorker;
use wapuku_model::data_type::WapukuDataType;
use wapuku_model::model::{Data, wa_id, WaFrame};
use wapuku_model::polars_df::PolarsData;
use wasm_bindgen::prelude::*;

use app::ActionRs;
pub use app::WapukuApp;

use crate::app::{ActionRq, WapukuAppModel};

mod app;
mod model_views;
mod edit_models;


#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    pub fn log(s: &str);
}

static TOTAL_MEM:f32 = 50000.0 * 65536.0;

static ALLOCATED:AtomicUsize = AtomicUsize::new(0);

#[no_mangle]
#[allow(unused)]
pub extern "C" fn on_alloc(size: usize, align: usize, pointer: *mut u8) {
    ALLOCATED.fetch_add(size, Ordering::Relaxed);
}

#[no_mangle]
#[allow(unused)]
pub extern "C" fn on_dealloc(size: usize, align: usize, pointer: *mut u8) {
    ALLOCATED.fetch_sub(size, Ordering::Relaxed);
}


#[no_mangle]
#[allow(unused)]
pub extern "C" fn on_alloc_zeroed(size: usize, align: usize, pointer: *mut u8) {
    ALLOCATED.fetch_add(size, Ordering::Relaxed);
}

#[no_mangle]
#[allow(unused)]
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


    let (to_main, from_worker) = std::sync::mpsc::channel::<ActionRs>();
    let to_main_rc = Rc::new(to_main);
    let to_main_rc_1 = Rc::clone(&to_main_rc);

    let from_worker_rc = Rc::new(from_worker);
    let from_worker_rc1 = Rc::clone(&from_worker_rc);

    let pool_worker = PoolWorker::new();

    pool_worker.init().await.expect("pool_worker init");

    let model = WapukuAppModel::new();
    let model_box = Box::pin(model);

    debug!("wapuku: model_box_ptr={:p}", &model_box);
    model_box.debug_ptr();

    let wapuku_app_model = Rc::new(RefCell::new(model_box));
    let wapuku_app_model_rc1 = Rc::clone(&wapuku_app_model);
    let wapuku_app_model_rc2 = Rc::clone(&wapuku_app_model);

    let data_map:HashMap<u128, Box<dyn Data>> = HashMap::new();
    let data_map_rc = Rc::new(RefCell::new(data_map));


    let timer_closure = Closure::wrap(Box::new(move || {

        if let Ok(mut model_borrowed) = wapuku_app_model_rc1.try_borrow_mut() {

            if let Some(action) = model_borrowed.get_next_action(){
                debug!("wapuku: got action {:?}", action);
                let to_main_rc_1_1 = Rc::clone(&to_main_rc_1);
                let data_map_rc_1 = Rc::clone(&data_map_rc);

                match action {
                    ActionRq::LoadFrame {name_ptr, data_ptr} => {

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
                                            let frame_id = wa_id();

                                            to_main_rc_1_1.send(ActionRs::LoadFrame {
                                                frame: WaFrame::new(
                                                    frame_id,
                                                    df.name().clone(),
                                                    df.build_summary(frame_id, None),
                                                )
                                            }).expect("send");

                                            data_map_rc_1.borrow_mut().insert(frame_id, Box::new(df));
                                        }
                                }
                                Err(e) => {
                                    to_main_rc_1_1.send(ActionRs::Err { msg: String::from(e.to_string()) }).expect("send");
                                }
                            }


                        });
                    }
                    ActionRq::Histogram { frame_id, name_ptr} => {
                       pool_worker.run_in_pool( move || {
                            let name = **unsafe { Box::from_raw(name_ptr as *mut Box<String>) };
                            debug!("wapuku: running in pool, ::ListUnique name={}", name);

                            let result = data_map_rc_1.borrow().get(&frame_id).expect(format!("no data for frame_id={}", frame_id).as_str()).build_histogram(frame_id, name, None);
                            match result {
                                Ok(histogram) => {
                                    to_main_rc_1_1.send(ActionRs::Histogram {
                                        frame_id,
                                        histogram,
                                    }).expect("send");
                                }
                                Err(e) => {
                                    to_main_rc_1_1.send(ActionRs::Err { msg: String::from(e.to_string()) }).expect("send");
                                }
                            }
                        });
                    }
                    ActionRq::Convert { frame_id, name_ptr, pattern_ptr, to_type_ptr} => {

                        pool_worker.run_in_pool( move || {
                            let name = **unsafe { Box::from_raw(name_ptr as *mut Box<String>) };
                            let pattern  = **unsafe { Box::from_raw(pattern_ptr as *mut Box<String>) };
                            let to_type  = **unsafe { Box::from_raw(to_type_ptr as *mut Box<WapukuDataType>) };
                            debug!("wapuku: running in pool, ::ListUnique name={} to_type={:?}", name, to_type);
                            //"%m/%d/%Y"
                            let result = data_map_rc_1.borrow_mut().get_mut(&frame_id).expect(format!("no data for frame_id={}", frame_id).as_str())
                                .convert_column(frame_id, name.clone(), pattern.into());
                            match result {
                                Ok(new_type) => {
                                    to_main_rc_1_1.send(ActionRs::Convert {
                                        frame_id,
                                        name:name,
                                        new_type,
                                    }).expect("send ActionRs::Convert");
                                    // to_main_rc_1_1.send(ActionRs::Err { msg: "zzzz".into() }).expect("send");
                                }
                                Err(e) => {
                                    to_main_rc_1_1.send(ActionRs::Err { msg: e.to_string() }).expect("send");
                                }
                            }
                        });
                    }
                    ActionRq::DataLump { frame_id , offset, limit} => {

                        pool_worker.run_in_pool( move || {
                            let result = data_map_rc_1.borrow().get(&frame_id).expect(format!("no data for frame_id={}", frame_id).as_str()).fetch_data(frame_id, offset, limit);
                            match result {
                                Ok(data_lump) => {
                                    debug!("wapuku: running in pool, sending data lump");

                                    to_main_rc_1_1.send(ActionRs::DataLump {
                                        frame_id,
                                        lump: data_lump,
                                    }).expect("ActionRs::DataLump");
                                }
                                Err(e) => {
                                    to_main_rc_1_1.send(ActionRs::Err { msg: String::from(e.to_string()) }).expect("send");
                                }
                            }
                        });
                    }
                    ActionRq::ApplyFilter { frame_id, filter } => {

                        pool_worker.run_in_pool( move || {

                            let result = data_map_rc_1.borrow().get(&frame_id).expect(format!("no data for frame_id={}", frame_id).as_str()).apply_filter(frame_id, filter.clone());
                            match result {
                                Ok(wiltered_fame) => {
                                    debug!("wapuku: running in pool, sending data lump");

                                    let frame_id = wa_id();

                                    to_main_rc_1_1.send(ActionRs::LoadFrame {
                                        frame: WaFrame::new(
                                            frame_id,
                                            format!("{} filtered", wiltered_fame.data().name()),
                                            wiltered_fame.data().build_summary(frame_id, None),
                                        )
                                    }).expect("ActionRs::LoadFrame");

                                    data_map_rc_1.borrow_mut().insert(frame_id, wiltered_fame.into());
                                }
                                Err(e) => {
                                    to_main_rc_1_1.send(ActionRs::Err { msg: String::from(e.to_string()) }).expect("send");
                                }
                            }
                        });
                    }
                    ActionRq::Corr { frame_id, column_vec_ptr } => {


                        pool_worker.run_in_pool( move || {

                            let names = **unsafe { Box::from_raw(column_vec_ptr as *mut Box<Vec<String>>) };

                            debug!("ActionRq::Corr: names in pool: {:?}", names);

                            let result = data_map_rc_1.borrow().get(&frame_id).expect(format!("no data for frame_id={}", frame_id).as_str()).clc_corrs(frame_id, names);
                            match result {
                                Ok(corrs) => {
                                    debug!("wapuku: running in pool, sending data lump");

                                    to_main_rc_1_1.send(ActionRs::Corr {
                                        frame_id,
                                        corrs,
                                    }).expect("ActionRs::DataLump");
                                }
                                Err(e) => {
                                    to_main_rc_1_1.send(ActionRs::Err { msg: String::from(e.to_string()) }).expect("send");
                                }
                            }
                        });
                    }
                }
            }
            model_borrowed.run_ui_actions();

            if let Ok(data_msg) = from_worker_rc1.try_recv() {
                // debug!("wapuku: try_recv: data_msg={:?}", data_msg);

                match data_msg {

                    ActionRs::LoadFrame {  frame } => {
                        model_borrowed.add_frame(frame);
                    }

                    ActionRs::Histogram { frame_id, histogram } => {
                        model_borrowed.add_histogram(frame_id, histogram);
                    }

                    ActionRs::DataLump { frame_id, lump } => {
                        debug!("wapuku: running in pool, got data lump");

                        model_borrowed.add_data_lump(frame_id, lump);
                    }

                    ActionRs::Convert { frame_id, name, new_type } => {
                        debug!("wapuku: ActionRs::Convert frame_id={:?} name={:?} new_type={:?}", frame_id, name, new_type );
                        model_borrowed.change_column_type(frame_id, name, new_type);
                    }

                    ActionRs::Corr { frame_id, corrs } => {
                        debug!("wapuku: ActionRs::Corr frame_id={:?} corrs={:?}", frame_id, corrs );
                    }

                    ActionRs::Err { msg } => {
                        debug!("wapuku: error={:?}", msg);
                        model_borrowed.set_error(msg);
                    }
                }
            }

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

