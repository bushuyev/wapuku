use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;
use std::sync::mpsc::Receiver;
use eframe::*;
use egui::Direction;
use egui_extras::{Column, TableBuilder};
use log::debug;
use wapuku_model::model::Data;
use wapuku_model::polars_df::PolarsData;
use crate::DataMsg;

#[derive(Debug)]
pub enum Action {
    LoadFile,
    Test1
}

pub struct WapukuAppModel {
    label:String,
    pending_actions:VecDeque<Action>,
    data:Option<Box<dyn Data>>
}

impl WapukuAppModel {
    pub fn new() -> Self {
        Self {
            label:  String::from("nope"),
            pending_actions: VecDeque::new(),
            data: None
        }
    }

    pub fn set_label<P>(&mut self, label: P) where P:Into<String> {
        self.label = label.into();
    }

    pub fn label(&self) -> &str {
        &self.label
    }

    pub fn get_next_action(&mut self) -> Option<Action> {
        self.pending_actions.pop_front()
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
    /// Called by the frame work to save state before shutdown.
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }

    /// Called each time the UI needs repainting, which may be many times per second.
    /// Put your widgets into a `SidePanel`, `TopPanel`, `CentralPanel`, `Window` or `Area`.
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {

        // Examples of how to create different panels and windows.
        // Pick whichever suits you.
        // Tip: a good default choice is to just keep the `CentralPanel`.
        // For inspiration and more examples, go to https://emilk.github.io/egui

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

        egui::CentralPanel::default().show(ctx, |ui| {
            debug!("WapukuApp::update: self.model.borrow().label()={}", self.model.borrow().label());
            ui.heading(format!("Dataframe Panel {}", self.model.borrow().label()).as_str());

            if ui.button("Load").clicked() {
                debug!("LoadLoadLoadLoad");
                if let Ok(mut model_borrowed) = self.model.try_borrow_mut() {
                    model_borrowed.pending_actions.push_back(Action::LoadFile)
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
                    ui.strong("Row");
                });
                header.col(|ui| {
                    ui.strong("Expanding content");
                });
                header.col(|ui| {
                    ui.strong("Clipped text");
                });
                header.col(|ui| {
                    ui.strong("Content");
                });
            }).body(|mut body| {
                body.rows(text_height, 3, |row_index, mut row| {
                    row.col(|ui| {
                        ui.label(row_index.to_string());
                    });
                    row.col(|ui| {
                        ui.label("c1");
                    });
                    row.col(|ui| {
                        ui.label("c2");
                    });
                    row.col(|ui| {
                        ui.add(
                            egui::Label::new("Thousands of rows of even height").wrap(false),
                        );
                    });
                })
            });

            ui.with_layout(egui::Layout::centered_and_justified(Direction::TopDown), |ui| {
                ui.vertical(|ui| {
                    ui.label("label1");
                    ui.label("label2");
                });
            });
        });

    }
}
