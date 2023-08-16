use egui::{Color32, emath, epaint, Frame, pos2, Rect, Stroke, Ui, vec2};
use egui_extras::{Column, TableBuilder};
use wapuku_model::data_type::WapukuDataType;
use wapuku_model::model::{ColumnSummary, Summary};

pub trait View {
    fn ui(&self, ui: &mut egui::Ui);
}

impl View for Summary {
    fn ui(&self, ui: &mut Ui) {
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


            body.rows(text_height, self.columns().len(), |row_index, mut row| {
                let column_summary = &self.columns()[row_index];

                row.col(|ui| {
                    ui.label(column_summary.name().clone());
                });

                match column_summary.dtype() {
                    WapukuDataType::Numeric => {
                        row.col(|ui| {
                            ui.label(column_summary.min().to_string());
                        });
                        row.col(|ui| {
                            ui.label(column_summary.avg().to_string());
                        });
                        row.col(|ui| {
                            ui.label(column_summary.max().to_string());
                        });
                    }
                    WapukuDataType::String => {
                        row.col(|ui| {
                            ui.label(String::from("lalalal"));
                        });
                    }
                    WapukuDataType::Boolean => {}
                }
            })

        });


    }
}


impl View for ColumnSummary {
    fn ui(&self, ui: &mut Ui) {

        match self.dtype() {
            WapukuDataType::Numeric => {}
            WapukuDataType::String => {}
            WapukuDataType::Boolean => {}
        }
    }
}