use std::cell::RefCell;
use std::collections::HashSet;
use std::rc::Rc;
use std::sync::Arc;
use log::{debug, trace};
use winit::{
    event::*,
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};
use winit::dpi::{LogicalSize, PhysicalPosition, PhysicalSize};
use winit::event_loop::EventLoopBuilder;
use wasm_bindgen::prelude::*;
use winit::platform::web::WindowExtWebSys;
use winit::window::Fullscreen;
use wapuku_model::polars_df::*;
use wapuku_model::model::*;
use wapuku_model::test_data::*;
use wapuku_model::visualization::*;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

#[wasm_bindgen(start)]
pub async fn run() {
    log("worker run");
}

#[wasm_bindgen]
pub fn worker_entry_point(arg: i32) {
    log(format!("worker_entry_point: {:?}", arg).as_str());
}