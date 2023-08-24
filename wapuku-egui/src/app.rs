use std::cell::RefCell;
use std::collections::VecDeque;
use std::pin::Pin;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

use log::debug;
use rfd;
use wapuku_model::model::{Data, WaFrame, Histogram, WaModelId};
use crate::model_views::View;
use egui::{Align, Align2, Color32, emath, epaint, Frame, Id, Layout, Pos2, Rect, Stroke, Ui, Vec2};
use std::collections::HashMap;
use std::io::Read;

#[derive(Debug)]
pub enum ActionRq {
    LoadFrame { name_ptr: u32, data_ptr: u32 },
    Histogram { frame_id:u128, name_ptr: u32 },
}

#[derive(Debug)]
pub enum ActionRs {
    LoadFrame {frame: WaFrame},
    Histogram {frame_id:u128, histogram:Histogram},
    Err { msg:String}
}


pub struct ModelCtx {
    pending_actions: VecDeque<ActionRq>,
}

impl ModelCtx {
    pub fn new() -> Self {
        Self {
            pending_actions: VecDeque::new()
        }
    }

    pub fn queue_action(&mut self, action: ActionRq) {
        self.pending_actions.push_back(action)
    }
}

static mut model_counter:usize = 0;




pub struct WapukuAppModel {
    data_name: String,
    frames: HashMap<u128, WaFrame>,
    ctx: ModelCtx,
    messages: Vec<String>,
    memory_allocated:f32,

    lock:Arc<Mutex<usize>>
}

unsafe impl Sync for WapukuAppModel {}

impl WapukuAppModel {
    pub fn new() -> Self {
        debug!("wapuku: WapukuAppModel::new");
        let test = unsafe {
            model_counter = model_counter + 1;
            String::from(format!("aaa: {}", model_counter))
        };

        Self {
            data_name: String::from("nope"),
            ctx: ModelCtx::new(),
            frames: HashMap::new(),
            messages: vec![],
            memory_allocated: 0.0,
            lock: Arc::new(Mutex::new(0))
        }
    }

    pub fn set_data_name<P>(&mut self, label: P) where P: Into<String> {
        self.data_name = label.into();
    }

    pub fn data_name(&self) -> &str {
        &self.data_name
    }

    pub fn get_next_action(&mut self) -> Option<ActionRq> {
        self.ctx.pending_actions.pop_front()
    }

    pub fn add_frame(&mut self, frame:WaFrame) {
        debug!("wapuku: add_frame name={:?}", frame.name());
        let model_lock_arc = Arc::clone(&self.lock);
        let result = model_lock_arc.try_lock();

        if let Ok(lock) = result {
            self.frames.insert(frame.id(), frame);

            debug!("wapuku:add_frame: Ok");
        } else {
            debug!("wapuku::add_frame: model locked ");
        }
    }

    pub fn add_histogram(&mut self, frame_id:u128, historgam:Histogram) {
        if let Some(frame) = self.frames.get_mut(&frame_id) {
            frame.add_histogram(historgam);
        } else {
            debug!("wapuku: no frame_id={}", frame_id); //TODO err msg
        }
    }

    pub fn debug_ptr(&self) {
        debug!("wapuku:debug_ptr: self_ptr={:p}",  self);
    }

    pub fn purge(&mut self, id: WaModelId) {
        debug!("wapuku: purge_frame frame_id={:?}", id);
        match id {
            WaModelId::Summary{frame_id} => {
                self.frames.remove(&frame_id);
            }
            WaModelId::Histogram{frame_id, histogram_id} => {
                if let Some(frame) = self.frames.get_mut(&frame_id) {
                    frame.purge(id);
                }
            }
        }

        // mem::drop(self.frames.remove(frame_id));
    }

    pub fn set_error(&mut self, msg: String) {
        self.messages.push(msg);
    }

    pub fn clear_messages(&mut self) {
        self.messages.clear();
    }


    pub fn messages(&self) -> &Vec<String> {
        &self.messages
    }

    pub fn queue_action(&mut self, action: ActionRq) {
        self.ctx.pending_actions.push_back(action)
    }

    pub fn on_each_frame<F>(&mut self, mut f: F) where F: FnMut(&mut ModelCtx, usize, &dyn View) {

        self.frames.values().for_each(|frame| {
            (f)(&mut self.ctx, 0, frame.summary());

            for hist in frame.histograms() {
                (f)(&mut self.ctx, 0, hist);
            }

        })
    }


    pub fn set_memory_allocated(&mut self, memory_allocated: f32) {
        self.memory_allocated = memory_allocated;
    }


    pub fn memory_allocated(&self) -> f32 {
        self.memory_allocated
    }

}

// #[derive(serde::Deserialize, serde::Serialize)]
// #[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct WapukuApp {
    // #[serde(skip)]
    model: Rc<RefCell<Pin<Box<WapukuAppModel>>>>,
}

impl WapukuApp {
    fn bar_contents(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        egui::widgets::global_dark_light_mode_switch(ui);

        ui.separator();


        ui.separator();
    }
}

// impl Default for WapukuApp {
//     fn default() -> Self {
//         debug!("Default for WapukuApp::default");
//         Self {
//             // Example stuff:
//             model: Rc::new(RefCell::new(Box::new(WapukuAppModel::new()))),
//         }
//     }
// }

impl WapukuApp {
    pub fn new(cc: &eframe::CreationContext<'_>, model: Rc<RefCell<Pin<Box<WapukuAppModel>>>>) -> Self {

        Self {
            model
        }
    }
}

impl eframe::App for WapukuApp {
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        // eframe::set_value(storage, eframe::APP_KEY, self);
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


        egui::TopBottomPanel::top("wrap_app_top_bar").show(ctx, |ui| {
            egui::trace!(ui);
            ui.horizontal(|ui| {

            });
            ui.horizontal_wrapped(|ui| {
                ui.visuals_mut().button_frame = false;

                if ui.button("Load").clicked() {
                    let task = rfd::AsyncFileDialog::new()
                        .add_filter("CSV, parquet files", &["csv", "parquet", "zip"])
                        .pick_file();

                    let model_for_file_callback = Rc::clone(&self.model);
                    wasm_bindgen_futures::spawn_local(async move {
                        let file_op = task.await;
                        if let Some(file) = file_op {
                            debug!("file=>{:?}<", file.file_name());

                            // debug!("wapuku: load size={} bytes_vec={:?}", bytes_vec.len(), bytes_vec);

                            model_for_file_callback.borrow_mut().queue_action(
                                ActionRq::LoadFrame {
                                    name_ptr: Box::into_raw(Box::new(Box::new(file.file_name()))) as u32,
                                    data_ptr: Box::into_raw(Box::new(Box::new(file.read().await))) as u32,
                                }
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

                        if let Ok(mut model_borrowed_mut) = model_for_file_callback.try_borrow_mut() {
                            model_borrowed_mut.queue_action(
                                ActionRq::LoadFrame {
                                    name_ptr: Box::into_raw(Box::new(Box::new(String::from("Sample 1.parquet")))) as u32,
                                    data_ptr: Box::into_raw(Box::new(Box::new(include_bytes!("../www/data/userdata1.parquet").to_vec()))) as u32,
                                });
                        }
                    });
                }
                if ui.button("Sample 2").clicked() {
                    let model_for_file_callback = Rc::clone(&self.model);
                    wasm_bindgen_futures::spawn_local(async move {
                        if let Ok(mut model_borrowed_mut) = model_for_file_callback.try_borrow_mut() {
                            model_borrowed_mut.queue_action(
                                ActionRq::LoadFrame {
                                    name_ptr: Box::into_raw(Box::new(Box::new(String::from("Sample 2.parquet")))) as u32,
                                    data_ptr: Box::into_raw(Box::new(Box::new(include_bytes!("../www/data/userdata2.parquet").to_vec()))) as u32,
                                }
                            );
                        }
                    });
                }
                ui.separator();
                if let Ok(mut model_borrowed_mut) = self.model.try_borrow_mut() {
                    let messages = model_borrowed_mut.messages();
                    for message in messages {
                        let mut style: egui::Style = (*ctx.style()).clone();
                        style.visuals.override_text_color = Some(Color32::RED);
                        ui.set_style(style);
                        ui.label(message.clone());
                    }
                    if !messages.is_empty() {
                        if ui.button("Clear messages").clicked() {
                            model_borrowed_mut.clear_messages();
                        }
                    }

                    let progress_bar = egui::ProgressBar::new(model_borrowed_mut.memory_allocated());
                    // let progress_bar = egui::ProgressBar::new(0.5).animate(false);

                    ui.with_layout(Layout::right_to_left(Align::Center),|ui| {
                        ui.add_sized([100.0, 20.0], progress_bar);
                        ui.label("memory:");
                    });
                }


            });
        });

        let mut connections:HashMap<u128, Vec<Pos2>> = HashMap::new();

        if let Ok(mut model_borrowed_mut) = self.model.try_borrow_mut() {
            let mut frame_to_close: Option<WaModelId> = None;

            model_borrowed_mut.on_each_frame( |model_ctx, frame_i, view| {
                let mut is_open = true;

                let frame = egui::Window::new(view.title())
                    .id(view.ui_id())
                    .default_width(300.)
                    .default_height(300.)
                    .vscroll(true)
                    .resizable(true)
                    .collapsible(true)
                    .default_pos([0., 0.])
                    .open(&mut is_open)
                    .show(ctx, |ui| {
                        view.ui(ui, model_ctx)
                    }).expect("show frame");


                // connections.push(frame.response.rect.min);
                connections.insert(*view.model_id().id(), vec![frame.response.rect.min]);
                if let Some(parent_connections) = view.model_id().parent_id().and_then(|parent_id|connections.get_mut(parent_id)) {
                    parent_connections.push(frame.response.rect.min);
                }


                if !is_open {
                    frame_to_close.replace(view.model_id());
                }
            });

            if frame_to_close.is_some() {
                model_borrowed_mut.purge(frame_to_close.unwrap());
            }
        }

        egui::Area::new("connections")
            .anchor(Align2::LEFT_TOP, Vec2::new(0., 0.))
            // .default_size(Vec2::new(ctx.available_rect().width(), ctx.available_rect().height()))
            .movable(false)
            .interactable(false)
            .show(ctx, |ui| {
                Frame::canvas(ui.style()).show(ui, |ui| {
                    // ui.with_layout(egui::Layout::top_down(Align::Min));
                    ui.ctx().request_repaint();

                    let (_id, rect) = ui.allocate_space(Vec2::new(ui.available_width(), ui.available_height()));

                    let to_screen = emath::RectTransform::from_to(Rect::from_x_y_ranges(0.0..=1.0, -1.0..=1.0), rect);

                    // let connections_in_screen = connections.iter().map(|p| to_screen * *p).collect::<Vec<Pos2>>();

                    // debug!("connections={:?} connections_in_screen={:?}", connections.clone(), connections_in_screen);

                    connections.into_values().filter(|v|v.len() > 1).for_each(|mut connection| {
                        let parent_point = connection.remove(0);
                        connection.into_iter().for_each(|endpoint|{
                            ui.painter().extend(vec![epaint::Shape::line(vec![parent_point, endpoint], Stroke::new(2.0, Color32::GREEN))]);
                        });

                    });
                    //
                    // ui.painter().extend(vec![epaint::Shape::line(vec![Pos2::new(0., 0., ), Pos2::new(100., 100. )], Stroke::new(2.0, Color32::GREEN))]);
                });

            });

        // debug!("wapuku: data: {:?}", ctx.data());

    }
}

