use std::cell::{RefCell, RefMut};
use std::rc::Rc;
use egui::{Ui, WidgetText};
use egui_extras::{Column, TableBuilder, TableRow};
use wapuku_model::model::{ColumnSummaryType, Summary};
use crate::app::{ActionRq, ModelCtx, WapukuAppModel};

pub trait View {
    fn ui(&self, ui: &mut egui::Ui, ctx: &mut ModelCtx);
}

impl View for Summary {
    fn ui(&self, ui: &mut Ui, ctx: &mut ModelCtx){
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

                        label_cell(&mut row, format!("min: {}, avg: {}, max: {}", data.min(), data.avg(), data.max()), ctx, self.frame_id(), column_summary.name());

                    }
                    ColumnSummaryType::String {data}=> {
                        label_cell(&mut row, data.unique_values(), ctx, self.frame_id(), column_summary.name());
                    }
                    ColumnSummaryType::Boolean => {

                    }
                }
            })

        });

    }
}

fn label_cell<'a>(mut row: &mut TableRow, label: impl Into<WidgetText>, ctx: &mut ModelCtx, frame_id:u128, name: &str) {

    row.col(|ui| {
        ui.horizontal_centered(|ui| {

            ui.add(egui::Label::new(label).wrap(true));

            if ui.button(">").clicked() {

                ctx.queue_action(ActionRq::Histogram {
                    frame_id,
                    name_ptr: Box::into_raw(Box::new(Box::new(String::from(name)))) as u32,
                });

            }
        });
    });
}
