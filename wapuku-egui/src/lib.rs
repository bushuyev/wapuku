use std::cell::RefCell;
use std::rc::Rc;
use eframe::web::request_animation_frame;
use log::debug;
pub use wapuku_common_web::get_pool;
pub use wapuku_common_web::init_pool;
pub use wapuku_common_web::init_worker;
pub use wapuku_common_web::run_in_pool;
use wapuku_common_web::workers::PoolWorker;
use wapuku_model::model::Data;
use wapuku_model::polars_df::{fake_df, PolarsData};
use wasm_bindgen::prelude::*;

pub use app::WapukuApp;
use crate::app::WapukuAppModel;

mod app;

#[derive(Debug)]
pub struct DataMsg {

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

    let data = PolarsData::new(fake_df());
    let data_rc = Rc::new(data);
    let data_in_init = Rc::clone(&data_rc);

    pool_worker.run_in_pool(move || {
        debug!("wapuku: running in pool");
        let all_props = data_in_init.all_properties();
        debug!("wapuku: all_props={:?}", all_props);

        to_main_rc_1.send(DataMsg{}).expect("send");

    });

    let mut wapuku_app_model = Rc::new(RefCell::new(Box::new(WapukuAppModel::new())));
    let mut wapuku_app_model_rc1 = Rc::clone(&wapuku_app_model);

    let timer_closure = Closure::wrap(Box::new(move || {
        if let Ok(msg) = from_worker_rc1.try_recv() {
            wapuku_app_model.borrow_mut().set_label("aaaaaaaaaaaaaaaaa");
            debug!("wapuku: event_loop got data_grid={:?}", msg);
            // runner_rc_2.
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
            Box::new(app::WapukuApp::new(cc, wapuku_app_model_rc1))
        }),
    )
        .await
        .expect("failed to start eframe")});
}
