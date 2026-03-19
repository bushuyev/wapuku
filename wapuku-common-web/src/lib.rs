use wasm_bindgen::prelude::*;
pub mod workers;
pub mod allocator;


#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    pub fn log(s: &str);
}



#[wasm_bindgen]
pub fn init_worker(ptr: u32) {
    // log(format!("wapuku: init_worker={}", ptr).as_str());

    let closure = unsafe { Box::from_raw(ptr as *mut Box<dyn FnOnce() + Send>) };
    (*closure)();
}

#[wasm_bindgen]
pub async fn init_pool(_threads: usize) {
    // The app's background work currently runs through a dedicated Web Worker,
    // so this initialization hook only needs to preserve the JS-facing API.
}


#[wasm_bindgen]
pub fn run_in_pool(ptr: u32) {
    log("wapuku::run_in_pool");

    let closure = unsafe { Box::from_raw(ptr as *mut Box<dyn FnOnce() + Send>) };
    (*closure)();

}
