use egui::{Color32, Ui, WidgetText};
use egui::Id;
use egui_extras::{Column, TableBuilder, TableRow};
use egui_plot::{
    Bar, BarChart,
    Plot,
};
use log::debug;
use wapuku_model::model::{ColumnSummaryType, DataLump, Histogram, Summary, WaModelId};
use wapuku_model::utils::val_or_na;

use crate::app::{ActionRq, ModelCtx};

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

        ui.horizontal(|ui| {
            ui.add(egui::Label::new("Shape:"));
            ui.add(egui::Label::new(self.shape()));

            ui.separator();
            if ui.button("Show data:").clicked() {
                ctx.queue_action(ActionRq::FetchData {
                    frame_id: self.frame_id(),
                    offset: 0,
                    limit: 100
                });

            };
        });

        let mut table = TableBuilder::new(ui)
            .striped(true)
            .resizable(true)
            .cell_layout(egui::Layout::left_to_right(egui::Align::LEFT))
            .column(Column::auto().at_least(200.0).resizable(true).clip(true))
            .column(Column::auto().at_least(200.0).resizable(true).clip(true))
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
                row.col(|ui| {
                    if ui.button(">").clicked() {
                        ctx.queue_action(ActionRq::Histogram {
                            frame_id: self.frame_id(),
                            name_ptr: Box::into_raw(Box::new(Box::new(String::from(column_summary.name())))) as u32,
                        });
                    }
                });
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


        // let max = values.iter().map(|v|v.1).max_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal)).unwrap_or(0);

        let y_ratio = 1.; // (max_height / max as f32) as f64;

        let mut bars  = values.iter().enumerate().map(|(i, (k, v))|{

            Bar::new((i as f32 * width) as f64, *v as f64 * y_ratio )
                .width(width as f64)
                .name(val_or_na(k))
        }).collect();


        let chart = BarChart::new(
            bars
        )
        .element_formatter(Box::new(move |b, c|{
                format!("{}, {}", b.name, (b.value/y_ratio) as u32)
            }))
        .color(Color32::LIGHT_BLUE)
        .name(self._title());

        let r = Plot::new("Normal Distribution Demo")

            .label_formatter(|name, value| {
                    // debug!("wapuku: name={:?}, value={:?}", name, value);

                    if !name.is_empty() {
                        name.to_owned()
                    } else {
                        "".to_owned()
                    }
                })
            .allow_zoom(true)
            .allow_drag(true)
            .custom_x_axes(vec![])
            .show(ui, |plot_ui| {
                plot_ui.bar_chart(chart);

            });

    }

    fn model_id(&self) -> WaModelId {
        WaModelId::Histogram{ frame_id: self.frame_id(), histogram_id: *self.id()}
    }
}

impl View for DataLump {
    fn title(&self) -> &str {
        self._title()
    }

    fn ui_id(&self) -> Id {
        Id::new(self.id())
    }

    fn ui(&self, ui: &mut Ui, ctx: &mut ModelCtx) {
        let text_height = egui::TextStyle::Body.resolve(ui.style()).size;

        let mut table = TableBuilder::new(ui)
            .striped(true)
            .resizable(true)
            .cell_layout(egui::Layout::left_to_right(egui::Align::LEFT));


        let columns = self.columns();

            // .column(Column::auto().at_least(200.0).resizable(true).clip(true))
            // .column(Column::auto().at_least(200.0).resizable(true).clip(true))
            // .column(Column::remainder());

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
            body.rows(2. * text_height, columns[0].len(), |row_index, mut row| {
                let column_summary = &self.columns()[row_index];

                row.col(|ui| {
                    ui.label(column_summary.name().clone());
                });
            })
        });
    }

    fn model_id(&self) -> WaModelId {
        WaModelId::DataLump{ frame_id: self.frame_id()}
    }
}


fn label_cell<'a>(mut row: &mut TableRow, label: impl Into<WidgetText>, ctx: &mut ModelCtx, frame_id:u128, name: &str) {

    row.col(|ui| {
        ui.horizontal_centered(|ui| {
            ui.add(egui::Label::new(label).wrap(true));
        });
    });
}
