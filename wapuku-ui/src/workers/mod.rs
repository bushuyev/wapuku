use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use log::debug;
use wasm_bindgen::prelude::*;
use wasm_bindgen::closure::*;
use std::cell::RefCell;
use std::rc::Rc;


pub struct WorkerFuture {
    worker: Rc<RefCell<web_sys::Worker>>,
    msg: Box<dyn Fn() -> js_sys::Array>,
    result: Rc<RefCell<Option<String>>>,
}

impl WorkerFuture {
    pub fn new(worker: Rc<RefCell<web_sys::Worker>>, msg: Box<dyn Fn() -> js_sys::Array>) -> Self {
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

                self.worker.borrow_mut().set_onmessage(Some(&closure_on_worker.as_ref().unchecked_ref()));
                closure_on_worker.forget();

                self.worker.borrow_mut().post_message(&(self.msg)()).expect("failed to post");

                Poll::Pending
            }
            Some(result) => {
                Poll::Ready(Ok(result.clone()))
            }
        }
    }
}