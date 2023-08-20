use wasm_bindgen::prelude::*;
use web_sys::*;
use rayon::*;
use log::{debug};
use std::sync::{Arc, Mutex};
pub mod workers;
pub mod allocator;

use workers::interval_future::IntervalFuture;
use std::sync::atomic::{AtomicUsize, Ordering};

pub fn worker_global_scope() -> Option<WorkerGlobalScope> {
    js_sys::global().dyn_into::<WorkerGlobalScope>().ok()
}


#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    pub fn log(s: &str);
}


static POOL_PR:Mutex<Option<u32>> = Mutex::new(None);//Mutext not needed?



#[wasm_bindgen]
pub fn init_worker(ptr: u32) {
    // log(format!("wapuku: init_worker={}", ptr).as_str());

    let closure = unsafe { Box::from_raw(ptr as *mut Box<dyn FnOnce() + Send>) };
    (*closure)();
}


#[no_mangle]
#[allow(improper_ctypes_definitions)]
pub extern "C" fn get_pool()->ThreadPool {//can be called once because Box::from_raw invalidates pool_addr

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
pub async fn init_pool(threads: usize) {
    log(format!("wapuku: init_pool,threads={}", threads).as_str());

    let counter = Arc::new(AtomicUsize::new(0));
    let counter_clone_top = Arc::clone(&counter);
    let counter_clone_clear = Arc::clone(&counter);

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
            let msg = js_sys::Array::new();
            msg.push(&JsValue::from("init_worker"));
            msg.push(&JsValue::from(&wasm_bindgen::memory()));
            msg.push(&JsValue::from(Box::into_raw(Box::new(Box::new(|| {
                log(format!("wapuku: running thread in pool").as_str());
                thread.run()
            }) as Box<dyn FnOnce()>)) as u32));

            let closure_on_worker = Closure::wrap(Box::new(move |_: web_sys::MessageEvent| {
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

    debug!("wapuku: init_pool: 1. counter={:?}", counter_clone_top);
    let pool_addr = Box::into_raw(Box::new(Box::new(pool) as Box<rayon::ThreadPool>)) as u32;

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
pub fn run_in_pool(ptr: u32) {
    log("wapuku::run_in_pool");

    let closure = unsafe { Box::from_raw(ptr as *mut Box<dyn FnOnce() + Send>) };
    (*closure)();

}
