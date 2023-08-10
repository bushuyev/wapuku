use std::cell::RefCell;
use std::collections::VecDeque;
use std::future::Future;
use std::mem;
use std::rc::Rc;
use eframe::*;
use egui::{Direction, Ui};
use egui_extras::{Column, TableBuilder};
use log::debug;
use wapuku_model::model::{Data, Summary};
use rfd;

#[derive(Debug)]
pub enum Action {
    LoadFile{data_ptr:u32},
    Summary
}



pub struct WapukuAppModel {
    data_name:String,
    summary:Option<Summary>,
    pending_actions:VecDeque<Action>,
    data:Option<Box<dyn Data>>
}

impl WapukuAppModel {
    pub fn new() -> Self {
        Self {
            data_name:  String::from("nope"),
            pending_actions: VecDeque::new(),
            data: None,
            summary: None
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

    pub fn set_data(&mut self, data: Box<dyn Data>) {
        self.data.replace(data);
    }

    pub fn set_summary(&mut self, summary: Summary) {
        self.summary.replace(summary);
    }

    pub fn update_summary(&mut self) {
        self.summary = self.data.as_ref().map(|d|d.build_summary());
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
                            .add_filter("CSV files", &["csv"])
                            .set_directory("/")
                            .pick_file();

                        let model_for_file_callback = Rc::clone(&self.model);
                        wasm_bindgen_futures::spawn_local(async move {
                            let file_op = task.await;
                            if let Some(file) = file_op{
                                // debug!("file={:?}", file.file_name());
                                // debug!("content: {:?}", file.read().await);
                                // debug!("vec: {} {:?}", data.len(), data);

                                let mut data_box = Box::new(Box::new(file.read().await) as Box<Vec<u8>>);
                                model_for_file_callback.borrow_mut().pending_actions.push_back(Action::LoadFile{data_ptr: Box::into_raw(data_box) as u32});
                                // mem::forget(data_box)
                            }

                        });
                    }
                });
            });

        }

/*        egui::CentralPanel::default().show(ctx, |ui| {

            if let Ok(mut model_borrowed) = self.model.try_borrow_mut() {
                debug!("WapukuApp::update: self.model.borrow().label()={}", model_borrowed.data_name());
                ui.heading(format!("Dataframe Panel {}", model_borrowed.data_name()).as_str());

                if model_borrowed.data.is_none() {
                    if ui.button("Load").clicked() {
                        model_borrowed.pending_actions.push_back(Action::LoadFile)

                    }
                } else {
                    if ui.button("Show summary").clicked() {
                        model_borrowed.pending_actions.push_back(Action::Summary)
                    }
                }

                ui.separator();

                let text_height = egui::TextStyle::Body.resolve(ui.style()).size;

                let mut table = TableBuilder::new(ui)
                    .striped(true)
                    .resizable(true)
                    .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                    .column(Column::auto())
                    .column(Column::initial(100.0).range(40.0..=300.0))
                    .column(Column::initial(100.0).at_least(40.0).clip(true))
                    .column(Column::remainder())
                    .min_scrolled_height(0.0);

                table.header(20.0, |mut header| {
                    header.col(|ui| {
                        ui.strong("Column");
                    });
                    header.col(|ui| {
                        ui.strong("min");
                    });
                    header.col(|ui| {
                        ui.strong("max");
                    });
                    header.col(|ui| {
                        ui.strong("avg");
                    });
                }).body(|mut body| {

                    if let Some(summary) = &model_borrowed.summary {
                        body.rows(text_height, summary.columns().len(), |row_index, mut row| {
                            let column_summary = &summary.columns()[row_index];

                            row.col(|ui| {
                                ui.label(column_summary.name().clone());
                            });
                            row.col(|ui| {
                                ui.label(column_summary.min().to_string());
                            });
                            row.col(|ui| {
                                ui.label(column_summary.avg().to_string());
                            });
                            row.col(|ui| {
                                ui.label(column_summary.max().to_string());
                            });
                        })

                    }

                });

            }

        })*/;

    }
}
