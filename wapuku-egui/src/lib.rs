use std::cell::RefCell;
use std::rc::Rc;

use log::{debug, trace};
pub use wapuku_common_web::get_pool;
pub use wapuku_common_web::init_pool;
pub use wapuku_common_web::init_worker;
pub use wapuku_common_web::run_in_pool;
use wapuku_common_web::workers::PoolWorker;
use wapuku_model::model::Data;
use wapuku_model::polars_df::PolarsData;
use wasm_bindgen::prelude::*;

pub use app::WapukuApp;

use crate::app::{Action, WapukuAppModel};

mod app;
mod model_views;


#[derive(Debug)]
pub enum DataMsg {
    Ok {},
    Err { msg:String}
}

#[wasm_bindgen]
pub async fn run() {
    let window = web_sys::window().unwrap();

    std::panic::set_hook(Box::new(console_error_panic_hook::hook));
    console_log::init_with_level(log::Level::Debug).expect("Couldn't initialize logger");


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

    let mut wapuku_app_model = Rc::new(RefCell::new(Box::new(WapukuAppModel::new())));
    let mut wapuku_app_model_rc1 = Rc::clone(&wapuku_app_model);
    let mut wapuku_app_model_rc2 = Rc::clone(&wapuku_app_model);

    let timer_closure = Closure::wrap(Box::new(move || {

        if let Ok(mut model_borrowed) = wapuku_app_model_rc1.try_borrow_mut() {

            if let Some(action) = model_borrowed.get_next_action(){
                debug!("wapuku: got action {:?}", action);
                match action {
                    Action::LoadFile{name_ptr, data_ptr} => {

                        pool_worker.run_in_pool( || {

                            let data = unsafe { Box::from_raw(data_ptr as *mut Box<Vec<u8>>) };
                            let name = unsafe { Box::from_raw(name_ptr as *mut Box<String>) };
                            debug!("wapuku: running in pool, load file name={:?} size={}", name, data.len());


                            match PolarsData::load(*data, *name.clone()) {
                                Ok(frames) => {
                                    for df in frames {
                                        model_borrowed.add_frame(df.name().clone(), Box::new(df));
                                    }
                                }
                                Err(e) => {
                                    to_main_rc_1.send(DataMsg::Err { msg: String::from(e.to_string()) }).expect("send");
                                }
                            }


                        });
                    }
                    Action::Summary => {
                        pool_worker.run_in_pool( || {
                            debug!("wapuku: running in pool");
                            // model_borrowed.update_summary();

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
                }
            }
        }

    }) as Box<dyn FnMut()>);

    if window.set_interval_with_callback_and_timeout_and_arguments_0(&timer_closure.as_ref().unchecked_ref(), 100).is_err() {
        panic!("set_interval_with_callback_and_timeout_and_arguments_0");
    }

    timer_closure.forget();

    eframe::WebLogger::init(log::LevelFilter::Warn).ok();

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

