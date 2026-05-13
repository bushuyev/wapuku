pub mod interval_future;

use log::debug;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use wasm_bindgen::prelude::*;

use std::cell::RefCell;
use std::rc::Rc;

pub struct WorkerFuture {
    worker: Option<Rc<web_sys::Worker>>,
    msg: Option<Box<dyn Fn() -> js_sys::Array>>,
    result: Rc<RefCell<Option<String>>>,
}

impl WorkerFuture {
    pub fn new(worker: Rc<web_sys::Worker>, msg: Box<dyn Fn() -> js_sys::Array>) -> Self {
        Self {
            worker: Some(worker),
            msg: Some(msg),
            result: Rc::new(RefCell::new(None)),
        }
    }

    pub fn ready() -> Self {
        Self {
            worker: None,
            msg: None,
            result: Rc::new(RefCell::new(Some(String::from("done")))),
        }
    }

    pub fn run_closure<F>(&mut self, _closure: F)
    where
        F: FnMut(),
    {
    }
}

impl Future for WorkerFuture {
    type Output = Result<String, ()>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match self.result.take() {
            None => {
                let Some(worker) = self.worker.as_ref() else {
                    return Poll::Ready(Ok(String::from("done")));
                };
                let Some(msg) = self.msg.as_ref() else {
                    return Poll::Ready(Ok(String::from("done")));
                };

                let waker = cx.waker().clone();
                let result = Rc::clone(&self.result);

                let closure_on_worker = Closure::wrap(Box::new(move |e: web_sys::MessageEvent| {
                    debug!("pool is ready: e={:?}", e);
                    result.borrow_mut().replace(String::from("done"));
                    waker.wake_by_ref();
                })
                    as Box<dyn FnMut(web_sys::MessageEvent)>);

                worker.set_onmessage(Some(&closure_on_worker.as_ref().unchecked_ref()));
                closure_on_worker.forget();

                worker.post_message(&msg()).expect("failed to post");

                Poll::Pending
            }
            Some(result) => Poll::Ready(Ok(result.clone())),
        }
    }
}

enum PoolWorkerBackend {
    Worker(Rc<web_sys::Worker>),
    Inline,
}

pub struct PoolWorker {
    backend: PoolWorkerBackend,
}

impl PoolWorker {
    pub fn new() -> Self {
        if !worker_backend_available() {
            debug!("wapuku: no shareable wasm memory, running worker tasks inline");
            return Self {
                backend: PoolWorkerBackend::Inline,
            };
        }

        match web_sys::Worker::new("./wasm-worker.js") {
            Ok(worker) => Self {
                backend: PoolWorkerBackend::Worker(Rc::new(worker)),
            },
            Err(err) => {
                debug!("wapuku: can't make worker for ./wasm-worker.js: {:?}", err);
                Self {
                    backend: PoolWorkerBackend::Inline,
                }
            }
        }
    }

    pub fn init(&self) -> WorkerFuture {
        match &self.backend {
            PoolWorkerBackend::Worker(worker) => WorkerFuture::new(
                Rc::clone(worker),
                Box::new(move || {
                    let msg = js_sys::Array::new();

                    msg.push(&JsValue::from("init_pool"));
                    msg.push(&JsValue::from(&wasm_bindgen::memory()));
                    msg
                }),
            ),
            PoolWorkerBackend::Inline => WorkerFuture::ready(),
        }
    }

    pub fn run_in_pool<F>(&self, mut cl: F)
    where
        F: FnMut(),
    {
        match &self.backend {
            PoolWorkerBackend::Worker(worker) => {
                let worker_param_ptr =
                    JsValue::from(Box::into_raw(Box::new(Box::new(cl) as Box<dyn FnMut()>)) as u32);

                let msg = js_sys::Array::new();
                msg.push(&JsValue::from("run_in_pool"));
                msg.push(&worker_param_ptr);

                worker.post_message(&msg).expect("failed to post");
            }
            PoolWorkerBackend::Inline => cl(),
        }
    }
}

fn cross_origin_isolated() -> bool {
    js_sys::Reflect::get(&js_sys::global(), &JsValue::from_str("crossOriginIsolated"))
        .ok()
        .and_then(|value| value.as_bool())
        .unwrap_or(false)
}

fn worker_backend_available() -> bool {
    if !cross_origin_isolated() {
        return false;
    }

    let memory = wasm_bindgen::memory();
    let Ok(buffer) = js_sys::Reflect::get(&memory, &JsValue::from_str("buffer")) else {
        return false;
    };
    let Ok(constructor) = js_sys::Reflect::get(&buffer, &JsValue::from_str("constructor")) else {
        return false;
    };
    let Ok(name) = js_sys::Reflect::get(&constructor, &JsValue::from_str("name")) else {
        return false;
    };

    name.as_string().as_deref() == Some("SharedArrayBuffer")
}
