use std::cell::RefCell;
use std::rc::Rc;
use log::debug;
pub use wapuku_common_web::get_pool;
pub use wapuku_common_web::init_pool;
pub use wapuku_common_web::init_worker;
pub use wapuku_common_web::run_in_pool;
use wapuku_common_web::workers::PoolWorker;
use wapuku_model::model::Data;
use wapuku_model::polars_df::{fake_df, from_csv, PolarsData};
use wasm_bindgen::prelude::*;

pub use app::WapukuApp;
use crate::app::{Action, WapukuAppModel};
use crate::DataMsg::Summary;

mod app;

#[derive(Debug)]
pub enum DataMsg {
    FrameLoaded{name:String, data: Box<dyn Data>},
    Summary {min:f32, avg:f32, max:f32}
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
                    Action::LoadFile{data_ptr} => {
                        // let to_main_rc_1_1 = Rc::clone(&to_main_rc_1);
                        // let data = unsafe { Box::from_raw(data_ptr as *mut Box<Vec<u8>>) };
                        // debug!("wapuku: running in pool, load file (*data).len()={:?} data={:?}", (*data).len(), data);
                        debug!("wapuku: 1. data_ptr={}", data_ptr);
                        pool_worker.run_in_pool( || {
                            debug!("wapuku: 2. data_ptr={}", data_ptr);
                            let data = unsafe { Box::from_raw(data_ptr as *mut Box<Vec<u8>>) };
                            debug!("wapuku: running in pool, load file (*data).len()={:?} data={:?}", (*data).len(), String::from_utf8(**data));

                            // debug!("wapuku: running in pool, load file data={:?}", data_ptr);
                            //
                            // // let data = *unsafe { Box::from_raw(data_ptr as *mut Box<Vec<u8>>) };
                            // let data = unsafe { Box::from_raw(data_ptr as *mut Box<Vec<u8>>) };
                            //
                            // to_main_rc_1.send(DataMsg::FrameLoaded {name: String::from("Fake data"), data: Box::new(PolarsData::new(from_csv(*data)))}).expect("send");

                        });
                    }
                    Action::Summary => {
                        pool_worker.run_in_pool( || {
                            debug!("wapuku: running in pool");
                            model_borrowed.update_summary();

                            to_main_rc_1.send(DataMsg::Summary{min:0., avg: 1., max:2.}).expect("send");

                        });
                    }
                }
            }

            if let Ok(data_msg) = from_worker_rc1.try_recv() {

                match data_msg {
                    DataMsg::FrameLoaded { name, data } => {
                        model_borrowed.set_data_name(name);
                        // debug!("wapuku: event_loop got data={:?}", data);

                        model_borrowed.set_data(data);

                        model_borrowed.update_summary();

                    }
                    DataMsg::Summary{min, avg, max} => {
                        // model_borrowed.set_summary(wapuku_model::model::Summary::new (vec![]))
                    }
                }
            }
        }

    }) as Box<dyn FnMut()>);

    if window.set_interval_with_callback_and_timeout_and_arguments_0(&timer_closure.as_ref().unchecked_ref(), 100).is_err() {
        panic!("set_interval_with_callback_and_timeout_and_arguments_0");
    }

    timer_closure.forget();

    eframe::WebLogger::init(log::LevelFilter::Debug).ok();

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
