use std::any::Any;
use egui::{Color32, Frame, InnerResponse, Sense, Stroke, Ui, WidgetText};
use egui::Id;
use egui_extras::{Column, TableBuilder, TableRow};
use egui_plot::{
    Bar, BarChart,
    Plot,
};
use log::debug;
use wapuku_model::messages::OK;
use wapuku_model::model::{CompositeType, Condition, ConditionType, DataLump, Filter, Histogram, Summary, SummaryColumnType, WaModelId};
use wapuku_model::utils::val_or_na;

use crate::app::{ActionRq, ModelCtx, UIAction};
use crate::edit_models::ValidationResult;

#[derive(Debug)]
pub struct Msg {
    text:String,
    color:Color32
}

impl Msg {
    pub fn new(text: &str, color: Color32) -> Self {
        Self { text:text.to_string(), color }
    }

    pub fn empty() -> Self {
        Self {
            text:String::from(OK),
            color: Color32::BLACK,
        }
    }

    pub fn text(&self) -> &String {
        &self.text
    }
    pub fn color(&self) -> &Color32 {
        &self.color
    }
}


impl<T:ValidationResult> From<T> for Msg {
    fn from(value: T) -> Self {

        Msg::new(
            value.msg(),
            if value.is_error() { Color32::RED } else { Color32::BLACK }
        )
    }
}

#[derive(Debug)]
pub enum LayoutRequest {
    Center
}

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
            if ui.button("Data").clicked() {
                ctx.queue_action(ActionRq::DataLump {
                    frame_id: self.frame_id(),
                    offset: 0,
                    limit: 100
                });

            };

            if ui.button("Filter").clicked() {
                let frame_id = self.frame_id();

                ctx.ui_action(
                    UIAction::WaFrame{frame_id : frame_id, action: Box::new(|mut frame|{
                        frame.add_filter();
                    })}
                );

                // ctx.ui_action(UIAction::WaFrame{frame_id : frame_id, action: Box::new(|mut summary|{
                //     summary.add_filter();
                // })));

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
                    SummaryColumnType::Numeric { data} => {

                        label_cell(&mut row, format!("min: {}, avg: {}, max: {}", data.min(), data.avg(), data.max()), ctx, self.frame_id(), column_summary.name());

                    }
                    SummaryColumnType::String {data}=> {
                        label_cell(&mut row, data.unique_values(), ctx, self.frame_id(), column_summary.name());
                    }

                    SummaryColumnType::Boolean => {

                    }
                }
                row.col(|ui| {
                    if ui.button("H").clicked() {
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

impl View for Filter {
    fn title(&self) -> &str {
        self._title()
    }

    fn ui_id(&self) -> Id {
        Id::new(self.id())
    }

    fn ui(&self, ui: &mut Ui, ctx: &mut ModelCtx) {
        Frame::canvas(ui.style()).show(ui, |ui| {

            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    if let InnerResponse { inner: Some(r), response } = egui::ComboBox::from_label(">>")
                        .selected_text(ctx.filter_new_condition_ctx().new_condition_column())
                        .show_ui(ui, |ui| {
                            ui.style_mut().wrap = Some(false);
                            ui.set_min_width(60.0);

                            for c in self.columns() {
                                if ui.selectable_value(ctx.filter_new_condition_ctx_mut().new_condition_column_mut(), c.name().clone(), c.name()).clicked() {
                                    debug!("combo Selected");
                                    if let Some(selected_column) = self.columns().iter().find(|c| c.name().eq(ctx.filter_new_condition_ctx().new_condition_column())) {
                                        ctx.filter_new_condition_ctx_mut().init(selected_column);
                                    }
                                }
                            }
                        }) {
                        // debug!("combo result={:?} response={:?}", r, response);
                    };

                    if ui.button("Apply").clicked() {

                        ctx.queue_action(ActionRq::ApplyFilter {
                            frame_id: self.frame_id(),
                            filter: self.clone()
                        });
                    }

                    if let Some(selected_column) = ctx.filter_new_condition_ctx().selected_column() {
                        match selected_column.dtype() {
                            SummaryColumnType::Numeric { data } => {
                                let msg_color = ctx.filter_new_condition_ctx().msg().color().clone();

                                ui.vertical(|ui| {
                                    ui.horizontal(|ui| {
                                        if egui::TextEdit::singleline(ctx.filter_new_condition_ctx_mut().min_mut())
                                            .hint_text("min")
                                            .text_color(msg_color)
                                            .show(ui).response.changed() {
                                            debug!("min changed");
                                            ctx.filter_new_condition_ctx_mut().validate();
                                        }

                                        if egui::TextEdit::singleline(ctx.filter_new_condition_ctx_mut().max_mut())
                                            .hint_text("max")
                                            .text_color(msg_color)
                                            .show(ui).response.changed() {
                                            debug!("max changed");
                                            ctx.filter_new_condition_ctx_mut().validate();
                                        }
                                    });
                                    ui.label(ctx.filter_new_condition_ctx_mut().msg().text().clone())/*.text_color(msg_color)*/;
                                });
                            }
                            SummaryColumnType::String { data } => {
                                let msg_color = ctx.filter_new_condition_ctx().msg().color().clone();

                                ui.vertical(|ui| {
                                    if egui::TextEdit::singleline(ctx.filter_new_condition_ctx_mut().pattern_mut())
                                        .hint_text("pattern")
                                        .text_color(msg_color)
                                        .show(ui).response.changed() {
                                        debug!("pattern changed");
                                        ctx.filter_new_condition_ctx_mut().validate();
                                    };
                                    ui.label(ctx.filter_new_condition_ctx_mut().msg().text().clone())/*.text_color(msg_color)*/;
                                });
                            }
                            SummaryColumnType::Boolean => {
                                ui.checkbox(ctx.filter_new_condition_ctx_mut().boolean(), "");
                            }
                        }

                        if ui.button("Add").clicked() {
                            debug!("add filter: {:?}", ctx.filter_new_condition_ctx());
                            if let Some((column_name, condition)) = ctx.filter_new_condition_ctx_mut().to_condition().take() {
                                let selected_condition = ctx.selected_condition();

                                ctx.ui_action(
                                    UIAction::WaFrame {
                                        frame_id: self.frame_id(),
                                        action: Box::new(move |mut frame| {

                                            frame.add_filter_condition(ConditionType::Single { column_name, condition }, selected_condition);
                                        }),
                                    }
                                );
                            }
                        }
                    }

                });

                if let Some(conditions) = self.conditions() {
                    add_conditions(conditions, ui, ctx, self.frame_id(), self.ui_id())
                }
            });
        });

    }

    fn model_id(&self) -> WaModelId {
        WaModelId::Filter{ frame_id: self.frame_id(), filter_id: *self.id()}
    }
}


fn add_conditions(condition_type: &ConditionType, ui: &mut Ui, ctx: &mut ModelCtx, frame_id:u128, ui_id:Id) {
    let current_condition = condition_type as *const _;

    let style = ui.style();

    let condition_frame = if ctx.selected_condition().map(|c|c == current_condition ).unwrap_or(false) {
        Frame::group(style).stroke(egui::Stroke {
            width: 2.,
            color: egui::Color32::GREEN,
        })
    } else {
        Frame::group(style)
    };

    let _ =  condition_frame.show(ui, |ui|{

        match condition_type {
            ConditionType::Single { column_name, condition } => {
                ui.horizontal(|ui| {
                    ui.label(column_name);
                    match condition {
                        Condition::Numeric { min, max } => {
                            ui.label(format!("min: {}", min));
                            ui.label(format!("max: {}", max));
                        }
                        Condition::String { pattern } => {
                            ui.label(format!("pattern: {}", pattern));
                        }
                        Condition::Boolean { val } => {
                            ui.label(format!("val: {}", val));
                        }
                    }
                    if ui.button("-").clicked() {
                        ctx.ui_action(
                            UIAction::WaFrame {
                                frame_id,
                                action: Box::new(move |mut frame| {
                                    frame.remove_filter_condition(current_condition);
                                }),
                            }
                        );
                    }

                });
            }
            ConditionType::Compoiste { conditions, ctype } => {
                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        if ui.button(">>").clicked() {
                            debug!("condition_type AND {:?} clicked", current_condition);
                            ctx.set_selected_condition(current_condition);
                        }

                        if ui.button(
                            match ctype {
                                CompositeType::AND => {
                                    "AND"
                                }
                                CompositeType::OR => {
                                    "OR"
                                }
                            }
                        ).clicked() {
                            ctx.ui_action(
                                UIAction::WaFrame {
                                    frame_id,
                                    action: Box::new(move |mut frame| {
                                        frame.change_condition_type(current_condition);
                                    }),
                                }
                            );
                        }
                        if ui.button("AND+").clicked() {

                            ctx.ui_action(
                                UIAction::WaFrame {
                                    frame_id,
                                    action: Box::new(move |mut frame| {
                                        frame.add_filter_condition(ConditionType::Compoiste {conditions: vec![], ctype: CompositeType::AND}, Some(current_condition));
                                    }),
                                }
                            );
                        }
                        if ui.button("OR+").clicked() {
                            ctx.ui_action(
                                UIAction::WaFrame {
                                    frame_id,
                                    action: Box::new(move |mut frame| {
                                        frame.add_filter_condition(ConditionType::Compoiste {conditions: vec![], ctype: CompositeType::OR}, Some(current_condition));
                                    }),
                                }
                            );
                        }
                        if ui.button("-").clicked() {
                            ctx.ui_action(
                                UIAction::WaFrame {
                                    frame_id,
                                    action: Box::new(move |mut frame| {
                                        frame.remove_filter_condition(current_condition);
                                    }),
                                }
                            );
                        }
                    });

                    for condition in conditions {
                        add_conditions(condition, ui, ctx, frame_id, ui_id);
                    }
                });
            }
        }
    }).response;


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

        ui.horizontal(|ui| {

            if self.offset() > &0 {

                if ui.button("Prev").clicked() {

                    ctx.queue_action(ActionRq::DataLump {
                        frame_id: *self.frame_id(),
                        offset: if self.offset() > &100 {*self.offset() - 100} else {0},
                        limit: 100
                    });
                };
            }

            ui.add(egui::Label::new(format!("{}-{}", self.offset(), self.offset()+self.data().len())));

            if ui.button("Next").clicked() {

                ctx.queue_action(ActionRq::DataLump {
                    frame_id: *self.frame_id(),
                    offset: *self.offset() + 100,
                    limit: 100
                });

            };
        });

        let mut table = TableBuilder::new(ui)
            .striped(true)
            .resizable(true)
            .cell_layout(egui::Layout::left_to_right(egui::Align::LEFT));


        let columns = self.columns();

        table = columns.iter().fold(table, |t, c|{
            t.column(Column::auto().at_least(20.0).resizable(true).clip(true))
        });
            // .column(Column::auto().at_least(200.0).resizable(true).clip(true))
            // .column(Column::auto().at_least(200.0).resizable(true).clip(true))
            // .column(Column::remainder());

        table.header(20.0, |mut header| {

            columns.iter().for_each(|c|{
                header.col(|ui| {
                    ui.strong(c.1.clone());
                });
            });


        }).body(|mut body| {
            let mut data_iter = self.data().iter();

            body.rows(2. * text_height, self.data().len(), |row_index, mut row| {
                // let column_summary = &self.data()[row_index];

                if let Some(row_data) = data_iter.next(){
                    row_data.iter().for_each(|col_data|{
                        row.col(|ui| {
                            ui.label(col_data.as_ref().map(|v|v.clone()).unwrap_or(String::from("n/a")));
                        });
                    });
                }


            })
        });
    }

    fn model_id(&self) -> WaModelId {
        WaModelId::DataLump{ frame_id: *self.frame_id(), lump_id: *self.id() }
    }
}


fn label_cell<'a>(mut row: &mut TableRow, label: impl Into<WidgetText>, ctx: &mut ModelCtx, frame_id:u128, name: &str) {

    row.col(|ui| {
        ui.horizontal_centered(|ui| {
            ui.add(egui::Label::new(label).wrap(true));
        });
    });
}
