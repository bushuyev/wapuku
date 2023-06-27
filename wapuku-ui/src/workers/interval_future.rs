use std::cell::RefCell;
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Context, Poll};
use log::debug;
use web_sys::*;
use wasm_bindgen::prelude::*;
use wasm_bindgen::closure::*;

pub struct IntervalFuture {
    stop_cl: Rc<Box<dyn Fn()->bool>>
}

impl IntervalFuture {
    pub fn new<F>(stop_cl:F) -> Self where F:Fn()->bool + 'static{
        Self {
            stop_cl: Rc::new(Box::new(stop_cl))
        }
    }
}

impl Future for IntervalFuture {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if (self.stop_cl)() {
            Poll::Ready(())
        } else {
            let waker = cx.waker().clone();
            let worker_global_scope = js_sys::global().dyn_into::<WorkerGlobalScope>().ok().unwrap();

            let interval_id:Rc<RefCell<Option<i32>>> = Rc::new(RefCell::new(None));
            let interval_id_rc = Rc::clone(&interval_id);
            let stop_cl_rc = Rc::clone(&self.stop_cl);
            let closure: Closure<dyn FnMut()> = Closure::new( move || {
                debug!("wapuku: IntervalFuture:poll: interval_id={:?}", interval_id_rc);
                if (stop_cl_rc)() {
                    debug!("wapuku: IntervalFuture:poll: 4.  interval_id={:?}", interval_id_rc);
                    waker.wake_by_ref();
                    js_sys::global().dyn_into::<WorkerGlobalScope>().ok().unwrap().clear_interval_with_handle(interval_id_rc.borrow().unwrap());
                }
            });
            let timer_handler = worker_global_scope.set_interval_with_callback_and_timeout_and_arguments_0(closure.as_ref().unchecked_ref(), 1000);
            debug!("wapuku: IntervalFuture:poll: 2. timer_handler={:?}", timer_handler);
            interval_id.borrow_mut().replace(timer_handler.unwrap());
            closure.forget();
            Poll::Pending
        }
        
    }
}