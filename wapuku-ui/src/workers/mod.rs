pub mod interval_future;

use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use log::debug;
use wasm_bindgen::prelude::*;
use wasm_bindgen::closure::*;
use std::cell::RefCell;
use std::rc::Rc;


pub struct WorkerFuture {
    worker: Rc<web_sys::Worker>,
    msg: Box<dyn Fn() -> js_sys::Array>,
    result: Rc<RefCell<Option<String>>>,
}

impl WorkerFuture {
    pub fn new(worker: Rc<web_sys::Worker>, msg: Box<dyn Fn() -> js_sys::Array>) -> Self {
        Self {
            worker,
            msg,
            result: Rc::new(RefCell::new(None)),
        }
    }
    
    pub fn run_closure<F>(&mut self, closure:F) where F:FnMut(){
        
    }
}

impl Future for WorkerFuture {
    type Output = Result<String, ()>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match self.result.take() {
            None => {
                let waker = cx.waker().clone();
                let result = Rc::clone(&self.result);
                
                let mut closure_on_worker = Closure::wrap(Box::new(move |e: web_sys::MessageEvent| {
                    debug!("pool is ready: e={:?}", e);
                    result.borrow_mut().replace(String::from("done"));
                    waker.wake_by_ref();
                }) as Box<dyn FnMut(web_sys::MessageEvent)>);

                self.worker.set_onmessage(Some(&closure_on_worker.as_ref().unchecked_ref()));
                closure_on_worker.forget();

                self.worker.post_message(&(self.msg)()).expect("failed to post");

                Poll::Pending
            }
            Some(result) => {
                Poll::Ready(Ok(result.clone()))
            }
        }
    }
}

pub struct PoolWorker {
    workder_rc:Rc<web_sys::Worker>
}

impl PoolWorker {
    pub fn new() -> Self {
        Self {
            workder_rc: Rc::new(web_sys::Worker::new("./wasm-worker.js").expect(format!("can't make worker for {}", "./wasm-worker.js").as_str()))
        }
    }
    
    pub fn init(&self) ->WorkerFuture {
        WorkerFuture::new(
            Rc::clone(&self.workder_rc), //"./wasm-worker.js",
            Box::new( move || {
                let msg = js_sys::Array::new();

                msg.push(&JsValue::from("init_pool"));
                msg.push(&JsValue::from(&wasm_bindgen::memory()));
                msg
            })
        )
    }


    pub fn run_in_pool<F>(&self, cl:F) where F:FnMut(){
        let worker_param_ptr = JsValue::from(Box::into_raw(Box::new(Box::new(cl) as Box<dyn FnMut()>)) as u32);

        let msg = js_sys::Array::new();
        msg.push(&JsValue::from("run_in_pool"));
        msg.push(&worker_param_ptr);


        self.workder_rc.post_message(&msg).expect("failed to post");
    }
}
