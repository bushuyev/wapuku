use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;

use eframe::*;
use egui::Color32;
use egui_extras::{Column, TableBuilder};
use log::debug;
use rfd;
use wapuku_model::model::{Data, FrameView};
use crate::model_views::View;

#[derive(Debug)]
pub enum Action {
    LoadFile{name_ptr: u32, data_ptr:u32},
    Summary
}



pub struct WapukuAppModel {
    data_name:String,
    frames:Vec<FrameView>,
    pending_actions:VecDeque<Action>,
    messages:Vec<String>
}

impl WapukuAppModel {
    pub fn new() -> Self {
        Self {
            data_name:  String::from("nope"),
            pending_actions: VecDeque::new(),
            frames: vec![],
            messages: vec![]
        }
    }

    pub fn set_data_name<P>(&mut self, label: P) where P:Into<String> {
        self.data_name = label.into();
    }

    pub fn data_name(&self) -> &str {
        &self.data_name
    }

    pub fn get_next_action(&mut self) -> Option<Action> {
        self.pending_actions.pop_front()
    }

    pub fn add_frame(&mut self, name:String, data:Box<dyn Data>) {
        self.frames.push(FrameView::new(
            name,
            data
        ));
    }

    pub fn purge_frame(&mut self, frame_id:usize) {
        debug!("wapuku: purge_frame frame_id={:?}", frame_id);
        self.frames.remove(frame_id);
    }

    pub fn set_error(&mut self, msg:String){
        self.messages.push(msg);
    }

    pub fn clear_messages(&mut self) {
        self.messages.clear();
    }


    pub fn messages(&self) -> &Vec<String> {
        &self.messages
    }
}

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct WapukuApp {
    #[serde(skip)]
    model:Rc<RefCell<Box<WapukuAppModel>>>,

    #[serde(skip)]
    value: f32,
}

impl WapukuApp {
    fn bar_contents(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        egui::widgets::global_dark_light_mode_switch(ui);

        ui.separator();




        ui.separator();


    }
}

impl Default for WapukuApp {
    fn default() -> Self {
        debug!("Default for WapukuApp::default");
        Self {
            // Example stuff:
            model: Rc::new(RefCell::new(Box::new(WapukuAppModel::new()))),
            value: 2.7,
        }
    }
}

impl WapukuApp {
    /// Called once before the first frame.
    pub fn new(cc: &eframe::CreationContext<'_>, model: Rc<RefCell<Box<WapukuAppModel>>>) -> Self {
        // This is also where you can customize the look and feel of egui using
        // `cc.egui_ctx.set_visuals` and `cc.egui_ctx.set_fonts`.

        // Load previous app state (if any).
        // Note that you must enable the `persistence` feature for this to work.
        // if let Some(storage) = cc.storage {
        //     return eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default();
        // }

        Self {
            // Example stuff:
            model,
            value: 2.7,
        }

    }
}

impl eframe::App for WapukuApp {
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }

    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {

        #[cfg(not(target_arch = "wasm32"))] // no File->Quit on web pages!
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            // The top panel is often a good place for a menu bar:
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Quit").clicked() {
                        _frame.close();
                    }
                });
            });
        });

        if let Ok(mut model_borrowed) = self.model.try_borrow_mut() {
            egui::TopBottomPanel::top("wrap_app_top_bar").show(ctx, |ui| {
                egui::trace!(ui);
                ui.horizontal_wrapped(|ui| {
                    ui.visuals_mut().button_frame = false;

                    if ui.button("Load").clicked() {
                        // model_borrowed.pending_actions.push_back(Action::LoadFile)


                        let task = rfd::AsyncFileDialog::new()
                            // .add_filter("CSV files", &["csv"])
                            // .set_directory("/home/bu/dvl/rust/polars-rayon-wasm/wapuku/wapuku-egui/www/data")
                            .pick_file();

                        let model_for_file_callback = Rc::clone(&self.model);
                        wasm_bindgen_futures::spawn_local(async move {
                            let file_op = task.await;
                            if let Some(file) = file_op{
                                debug!("file=>{:?}<", file.file_name());

                                // debug!("wapuku: load size={} bytes_vec={:?}", bytes_vec.len(), bytes_vec);

                                model_for_file_callback.borrow_mut().pending_actions.push_back(
                                    Action::LoadFile{
                                        name_ptr: Box::into_raw(Box::new(Box::new(file.file_name()))) as u32,
                                        data_ptr: Box::into_raw(Box::new(Box::new(file.read().await))) as u32}
                                );
                            }

                        });
                    }

                    ui.separator();
                    ui.label("Load sample:");
                    if ui.button("Sample 1").clicked() {
                        let model_for_file_callback = Rc::clone(&self.model);
                        wasm_bindgen_futures::spawn_local(async move {
                            // debug!("wapuku: sample1 size={} bytes_vec={:?}", bytes_vec.len(), bytes_vec);

                            model_for_file_callback.borrow_mut().pending_actions.push_back(
                                    Action::LoadFile{
                                        name_ptr: Box::into_raw(Box::new(Box::new(String::from("Sample 1")))) as u32,
                                        data_ptr: Box::into_raw(Box::new(Box::new(include_bytes!("../www/data/userdata1.parquet").to_vec()))) as u32}
                                );

                        });
                    }
                    if ui.button("Sample 2").clicked() {
                        let model_for_file_callback = Rc::clone(&self.model);
                        wasm_bindgen_futures::spawn_local(async move {

                            model_for_file_callback.borrow_mut().pending_actions.push_back(
                                Action::LoadFile{
                                    name_ptr: Box::into_raw(Box::new(Box::new(String::from("Sample 2")))) as u32,
                                    data_ptr: Box::into_raw(Box::new(Box::new(include_bytes!("../www/data/userdata2.parquet").to_vec()))) as u32}
                            );

                        });
                    }
                    ui.separator();
                    let messages = model_borrowed.messages();
                    for message in messages {

                        let mut style: egui::Style = (*ctx.style()).clone();
                        style.visuals.override_text_color = Some(Color32::RED);
                        ui.set_style(style);
                        ui.label(message.clone());
                    }
                    if !messages.is_empty() {
                        if ui.button("Clear messages").clicked() {
                            model_borrowed.clear_messages();
                        }
                    }
                });
            });
            let mut frame_to_close:Option<usize> = None;
            for (frame_i, frame) in model_borrowed.frames.iter().enumerate() {
                let mut is_open = true;

                egui::Window::new(frame.name())
                    .default_width(300.)
                    .default_height(300.)
                    .vscroll(true)
                    .resizable(true)
                    .collapsible(true)
                    .default_pos([0., 0.])
                    .open(&mut is_open)
                    .show(ctx, |ui| {
                        frame.summary().ui(ui)
                    });

                if !is_open {
                    frame_to_close.replace(frame_i);
                }

            }

            if frame_to_close.is_some() {
                model_borrowed.purge_frame(frame_to_close.unwrap());
            }
        }
    }
}