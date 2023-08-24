use std::cell::{RefCell, RefMut};
use std::cmp::Ordering;
use std::rc::Rc;
use egui::{Color32, Ui, WidgetText};
use egui::Id;
use egui::plot::{
    Arrows, AxisBools, /*AxisHints,*/ Bar, BarChart, BoxElem, BoxPlot, BoxSpread, CoordinatesFormatter,
    Corner, GridInput, GridMark, HLine, Legend, Line, LineStyle, MarkerShape, Plot, PlotImage,
    PlotPoint, PlotPoints, PlotResponse, Points, Polygon, Text, VLine,
};
use egui_extras::{Column, TableBuilder, TableRow};
use log::debug;
use wapuku_model::model::{ColumnSummaryType, Histogram, HistogramValues, Summary, WaModelId};
use crate::app::{ActionRq, ModelCtx, WapukuAppModel};

pub trait View {
    fn title(&self) -> &str;
    fn ui_id(&self) -> Id;
    fn ui(&self, ui: &mut egui::Ui, ctx: &mut ModelCtx);

    fn model_id(&self) -> WaModelId;
}

impl View for Summary {
    fn title(&self) -> &str {
        self._title()
    }

    fn ui_id(&self) -> Id {
        Id::new(self.id())
    }

    fn ui(&self, ui: &mut Ui, ctx: &mut ModelCtx){
        let text_height = egui::TextStyle::Body.resolve(ui.style()).size;

        let mut table = TableBuilder::new(ui)
            .striped(true)
            .resizable(true)
            .cell_layout(egui::Layout::left_to_right(egui::Align::LEFT))
            .column(Column::auto().at_least(100.0).resizable(true).clip(true))
            .column(Column::auto().at_least(100.0).resizable(true).clip(true))
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

    fn model_id(&self) -> WaModelId {
        WaModelId::Summary{ frame_id: self.frame_id()}
    }
}

impl View for Histogram {

    fn title(&self) -> &str {
        self._title()
    }

    fn ui_id(&self) -> Id {
        Id::new(self.id())
    }

    fn ui(&self, ui: &mut Ui, ctx: &mut ModelCtx) {

        let max_height = ui.available_height() * 0.8;
        let max_width = ui.available_width() * 0.8;

        let values = self.values();
        let width = max_width/ values.len() as f32;

        let mut bars = match values {
            HistogramValues::Numeric { .. } => {
                vec![]
            }
            HistogramValues::Categoric { y } => {

                let max = y.iter().map(|v|v.1).max_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal)).unwrap_or(0.);

                y.iter().enumerate().map(|(i, (k, v))|{
                    Bar::new((i as f32 * width) as f64, (max_height * v/ max ) as f64)
                        .width(width as f64)
                        .name(k)
                }).collect()
            }
        };


        let chart = BarChart::new(
            bars
        )
        .color(Color32::LIGHT_BLUE)

        .name(self._title());

        let r = Plot::new("Normal Distribution Demo")
            // .legend(Legend::default())
            // .show_grid(false)
            .label_formatter(|name, value| {
                    if !name.is_empty() {
                        name.to_owned()
                    } else {
                        "_".to_owned()
                    }
                })
            .show_x(false)
            // .show_y(false)
            .show_axes([false, false])
            // .legend(Legend::default())
            .clamp_grid(true)
            // .y_axis_width(3)
            .allow_zoom(true)
            .allow_drag(true)
            .show(ui, |plot_ui| {
                plot_ui.bar_chart(chart);

            });

    }

    fn model_id(&self) -> WaModelId {
        WaModelId::Histogram{ frame_id: self.frame_id(), histogram_id: *self.id()}
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
