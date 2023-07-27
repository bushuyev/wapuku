pub use wapuku_common_web::get_pool;
pub use wapuku_common_web::init_pool;
pub use wapuku_common_web::init_worker;
pub use wapuku_common_web::run_in_pool;
use wasm_bindgen::prelude::*;

pub use app::WapukuApp;

mod app;

#[wasm_bindgen]
pub async fn run() {
    eframe::WebLogger::init(log::LevelFilter::Debug).ok();

    let web_options = eframe::WebOptions::default();

    wasm_bindgen_futures::spawn_local(async {
        eframe::WebRunner::new()
            .start(
                "the_canvas_id", // hardcode it
                web_options,
                Box::new(|cc| Box::new(app::WapukuApp::new(cc))),
            )
            .await
            .expect("failed to start eframe");
    });
}
