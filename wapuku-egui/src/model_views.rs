use egui::Ui;
use egui_extras::{Column, TableBuilder};
use log::debug;
use wapuku_model::model::{ColumnSummaryType, Summary};
use web_sys::console::debug;

pub trait View {
    fn ui(&self, ui: &mut egui::Ui);
}

impl View for Summary {
    fn ui(&self, ui: &mut Ui) {
        let text_height = egui::TextStyle::Body.resolve(ui.style()).size;

        let mut table = TableBuilder::new(ui)
            .striped(true)
            .resizable(true)
            .cell_layout(egui::Layout::left_to_right(egui::Align::LEFT))
            .column(Column::auto().at_least(40.0).resizable(true).clip(true))
            .column(Column::auto().at_least(40.0).resizable(true).clip(true))
            .column(Column::remainder());

        table.header(20.0, |mut header| {
            header.col(|ui| {
                ui.strong("Column");
            });
            header.col(|ui| {
                ui.strong("Data");
            });
            header.col(|ui| {
                ui.strong("Actions");
            });

        }).body(|mut body| {


            body.rows(2. * text_height, self.columns().len(), |row_index, mut row| {
                let column_summary = &self.columns()[row_index];

                row.col(|ui| {
                    ui.label(column_summary.name().clone());
                });

                match column_summary.dtype() {
                    ColumnSummaryType::Numeric { data} => {

                        row.col(|ui| {
                            ui.horizontal(|ui|{
                                ui.horizontal_wrapped(|ui| {
                                    ui.add(
                                        egui::Label::new(
                                            format!("min: {}, avg: {}, max: {}", data.min(), data.avg(), data.max())
                                        ).wrap(true)
                                    );

                                });
                                ui.button(">")
                            });
                            ;
                        });

                    }
                    ColumnSummaryType::String {data}=> {
                        row.col(|ui| {
                            ui.horizontal(|ui|{
                                ui.horizontal_wrapped(|ui| {
                                    ui.add(
                                        egui::Label::new(
                                            data.unique_values(),
                                        )
                                    );

                                });
                                ui.button(">")
                            });
                           ;
                        });
                    }
                    ColumnSummaryType::Boolean => {}
                }
            })

        });


    }
}