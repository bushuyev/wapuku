use std::ops::RangeInclusive;
use egui::{Color32, Context, FontId, Frame, InnerResponse, RichText, Ui, WidgetText};
use egui::Id;
use egui_extras::{Column, TableBuilder, TableRow};
use egui_plot::{AxisHints, Bar, BarChart, Plot, PlotPoints, Polygon};
use log::debug;
use wapuku_model::data_type::WapukuDataType;
use wapuku_model::messages::OK;
use wapuku_model::model::{CompositeType, Condition, ConditionType, Corrs, DataLump, Filter, Histogram, Summary, SummaryColumn, SummaryColumnType, WaModelId};
use wapuku_model::utils::val_or_na;

use crate::app::{ActionRq, ModelCtx, UIAction};
use crate::edit_models::ValidationResult;

#[derive(Debug)]
pub struct Msg {
    text:String,
    color:Color32
}

const ICON_FONT:FontId = FontId::proportional(30.0);

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
    fn ui(&self, ui: &mut egui::Ui, ctx: &Context, model_ctx: &mut ModelCtx);

    fn model_id(&self) -> WaModelId;
}

impl View for Summary {
    fn title(&self) -> &str {
        self._title()
    }

    fn ui_id(&self) -> Id {
        Id::new(self.id())
    }

    fn ui(&self, ui: &mut egui::Ui, ctx: &Context, model_ctx: &mut ModelCtx){
        let text_height = egui::TextStyle::Body.resolve(ui.style()).size;

        ui.horizontal(|ui| {
            ui.add(egui::Label::new("Shape:"));
            ui.add(egui::Label::new(self.shape()));

            ui.separator();
            if ui.button("Data").clicked() {
                model_ctx.queue_action(ActionRq::DataLump {
                    frame_id: self.frame_id(),
                    offset: 0,
                    limit: 100
                });

            };

            if ui.button("Filter").clicked() {
                let frame_id = self.frame_id();

                model_ctx.ui_action(
                    UIAction::WaFrame{frame_id : frame_id, action: Box::new(|frame|{
                       Some(UIAction::Layout {frame_id: WaModelId::Filter {frame_id:frame.id(), filter_id:frame.add_filter()}, request: LayoutRequest::Center })
                    })}
                );

            };
        });

        let table = TableBuilder::new(ui)
            .striped(true)
            .resizable(true)
            .cell_layout(egui::Layout::left_to_right(egui::Align::LEFT))
            .column(Column::auto().at_least(200.0).resizable(true).clip(true))
            .column(Column::auto().at_least(10.0).resizable(false).clip(true))
            .column(Column::auto().at_least(200.0).resizable(true).clip(true))
            .column(Column::remainder());

        table.header(20.0, |mut header| {
            header.col(|ui| {
                ui.strong("Column");
            });
            header.col(|ui| {
                ui.strong("Type");
            });
            header.col(|ui| {
                ui.strong("Data");
            });

            header.col(|ui| {
                // ui.vertical(|ui|{

                    ui.horizontal(|ui|{
                        ui.strong("Actions");
                        ui.add_space(20.0);//TODO
                        ui.set_enabled(model_ctx.summary_actions_ctx().get_columns_for_corr_num() >=2);

                        if ui.button("➡").clicked() {
                            debug!("➡➡➡➡");
                            model_ctx.queue_action(ActionRq::Corr {
                                frame_id: self.frame_id(),
                                column_vec_ptr: Box::into_raw(Box::new(Box::<Vec<String>>::new(model_ctx.summary_actions_ctx().get_columns_for_corr()))) as u32,
                            });
                        }
                    });
                // });

            });

        }).body(|body| {

            body.rows(4. * text_height, self.columns().len(), | mut row| {
                let column_summary = &self.columns()[row.index()];

                row.col(|ui| {
                    ui.label(column_summary.name().clone());

                });
                row.col(|ui| {
                    match column_summary.dtype(){
                        SummaryColumnType::Numeric { .. } => {
                            ui.label(RichText::new("🔢").font(ICON_FONT));
                        }
                        SummaryColumnType::String { .. } => {
                            ui.horizontal(|ui| {
                                ui.label(RichText::new("🔠").font(ICON_FONT));

                                if ui.button("➡").clicked() {
                                    model_ctx.summary_actions_ctx_mut().is_convret_dialog_open = Some(column_summary.name().clone());
                                }
                                if model_ctx.summary_actions_ctx().is_convret_dialog_open.as_ref().map(|c|c.eq(column_summary.name())).unwrap_or(false) {
                                    egui::Window::new("Convert to").current_pos(ui.clip_rect().center())/*.open(&mut model_ctx.summary_actions_ctx_mut().is_convret_dialog_open)*/.show(ctx, |ui| {
                                        ui.horizontal(|ui| {
                                            ui.label("Pattern:");
                                            ui.add(egui::TextEdit::singleline(model_ctx.summary_actions_ctx_mut().pattern_mut()).hint_text("pattern"));
                                            if ui.button("Cancel").clicked() {
                                                model_ctx.summary_actions_ctx_mut().is_convret_dialog_open = None;
                                            }
                                            if ui.button("Convert").clicked() {
                                                model_ctx.summary_actions_ctx_mut().is_convret_dialog_open = None;

                                                model_ctx.queue_action(ActionRq::Convert {
                                                    frame_id: self.frame_id(),
                                                    name_ptr: Box::into_raw(Box::new(Box::new(String::from(column_summary.name())))) as u32,
                                                    pattern_ptr: Box::into_raw(Box::new(Box::new(String::from(model_ctx.summary_actions_ctx().pattern())))) as u32,
                                                    to_type_ptr: Box::into_raw(Box::new(Box::new(model_ctx.summary_actions_ctx().to_type().clone()))) as u32//TODO other
                                                });
                                            }
                                        });
                                    });
                                }
                            });
                        }
                        SummaryColumnType::Datetime { .. } => {
                            ui.label(RichText::new("📆").font(ICON_FONT));
                        }
                        SummaryColumnType::Boolean => {
                            ui.label(RichText::new("🌓").font(ICON_FONT));
                        }
                    }
                });
                row.col(|ui| {
                    match column_summary.dtype() {
                        SummaryColumnType::Numeric { data } => {
                            let label = format!("min: {}, avg: {}, max: {}", data.min(), data.avg(), data.max());
                            let _name = column_summary.name();
                            ui.horizontal_centered(|ui| {
                                ui.add(egui::Label::new(label).wrap(true));
                            });
                        }
                        SummaryColumnType::String { data } => {
                            ui.add(egui::Label::new(data.unique_values()).wrap(true));
                        }

                        SummaryColumnType::Boolean => {}
                        SummaryColumnType::Datetime { .. } => {}
                    }
                });
                row.col(|ui| {
                    ui.horizontal(|ui|{
                        if ui.button("H").clicked() {
                            model_ctx.queue_action(ActionRq::Histogram {
                                frame_id: self.frame_id(),
                                name_ptr: Box::into_raw(Box::new(Box::<String>::new(column_summary.name().into()))) as u32,
                            });
                        }
                        if ui.checkbox(&mut model_ctx.summary_actions_ctx_mut().get_selected_for_corr(column_summary.name().into()), "C").clicked() {
                            // debug!("Correlations clicked");
                            // if model_ctx.summary_actions_ctx().get_columns_for_corr_num() >=2 {
                            //
                            //     model_ctx.queue_action(ActionRq::Corr {
                            //         frame_id: self.frame_id(),
                            //         column_vec_ptr: Box::into_raw(Box::new(Box::<Vec<String>>::new(model_ctx.summary_actions_ctx().get_columns_for_corr()))) as u32,
                            //     });
                            // }
                        }
                    });

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

    fn ui(&self, ui: &mut egui::Ui, ctx: &Context, model_ctx: &mut ModelCtx) {
        Frame::canvas(ui.style()).show(ui, |ui| {

            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    if let InnerResponse { inner: Some(_), response:_ } = egui::ComboBox::from_label(":")
                        .selected_text(model_ctx.filter_new_condition_ctx().new_condition_column())
                        .show_ui(ui, |ui| {
                            ui.style_mut().wrap = Some(false);
                            ui.set_min_width(60.0);

                            for c in self.columns() {
                                if ui.selectable_value(model_ctx.filter_new_condition_ctx_mut().new_condition_column_mut(), c.name().clone(), c.name()).clicked() {
                                    debug!("combo Selected");
                                    if let Some(selected_column) = self.columns().iter().find(|c| c.name().eq(model_ctx.filter_new_condition_ctx().new_condition_column())) {
                                        model_ctx.filter_new_condition_ctx_mut().init(selected_column);
                                    }
                                }
                            }
                        }) {
                        // debug!("combo result={:?} response={:?}", r, response);
                    };


                    if let Some(selected_column) = model_ctx.filter_new_condition_ctx().selected_column() {
                        match selected_column.dtype() {
                            SummaryColumnType::Numeric { data:_ } => {
                                let msg_color = model_ctx.filter_new_condition_ctx().msg().color().clone();

                                ui.vertical(|ui| {
                                    ui.horizontal(|ui| {
                                        if egui::TextEdit::singleline(model_ctx.filter_new_condition_ctx_mut().min_mut())
                                            .hint_text("min")
                                            .text_color(msg_color)
                                            .show(ui).response.changed() {
                                            debug!("min changed");
                                            model_ctx.filter_new_condition_ctx_mut().validate();
                                        }

                                        if egui::TextEdit::singleline(model_ctx.filter_new_condition_ctx_mut().max_mut())
                                            .hint_text("max")
                                            .text_color(msg_color)
                                            .show(ui).response.changed() {
                                            debug!("max changed");
                                            model_ctx.filter_new_condition_ctx_mut().validate();
                                        }
                                    });
                                    ui.label(model_ctx.filter_new_condition_ctx_mut().msg().text().clone())/*.text_color(msg_color)*/;
                                });
                            }
                            SummaryColumnType::String { data:_ } => {
                                let msg_color = model_ctx.filter_new_condition_ctx().msg().color().clone();

                                ui.vertical(|ui| {
                                    if egui::TextEdit::singleline(model_ctx.filter_new_condition_ctx_mut().pattern_mut())
                                        .hint_text("pattern")
                                        .text_color(msg_color)
                                        .show(ui).response.changed() {
                                        debug!("pattern changed");
                                        model_ctx.filter_new_condition_ctx_mut().validate();
                                    };
                                    ui.label(model_ctx.filter_new_condition_ctx_mut().msg().text().clone())/*.text_color(msg_color)*/;
                                });
                            }
                            SummaryColumnType::Boolean => {
                                ui.checkbox(model_ctx.filter_new_condition_ctx_mut().boolean(), "");
                            }
                            SummaryColumnType::Datetime { .. } => {}
                        }

                        let selected_condition = model_ctx.filter_new_condition_ctx().selected_condition();

                        if ui.button( if selected_condition.is_some() {"Save"} else { "Add"}).clicked() {

                            debug!("add filter: {:?}", model_ctx.filter_new_condition_ctx());

                            if let Some((column_name, condition)) = model_ctx.filter_new_condition_ctx_mut().to_condition().take() {

                                model_ctx.filter_new_condition_ctx_mut().reset();
                                model_ctx.ui_action(
                                    UIAction::WaFrame {
                                        frame_id: self.frame_id(),
                                        action: Box::new(move |frame| {
                                            frame.add_filter_condition(ConditionType::Single { column_name, condition }, selected_condition);

                                            None
                                        }),
                                    }
                                );
                            }
                        }
                    }

                });

                if ui.button("Apply filter").clicked() {

                    model_ctx.queue_action(ActionRq::ApplyFilter {
                        frame_id: self.frame_id(),
                        filter: self.clone()
                    });
                }

                if let Some(conditions) = self.conditions() {
                    add_conditions(conditions, ui, model_ctx, self.frame_id(), self.ui_id(), self.columns())
                }
            });
        });

    }

    fn model_id(&self) -> WaModelId {
        WaModelId::Filter{ frame_id: self.frame_id(), filter_id: *self.id()}
    }
}


fn add_conditions(condition_type: &ConditionType, ui: &mut Ui, ctx: &mut ModelCtx, frame_id:u128, ui_id:Id, columns:&Vec<SummaryColumn>) {
    let current_condition = condition_type as *const _;

    let style = ui.style();

    let condition_frame = if ctx.filter_new_condition_ctx().selected_condition().map(|c|c == current_condition ).unwrap_or(false) {
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

                    if ui.button(">>").clicked() {
                        debug!("condition_type Single {:?} clicked", current_condition);
                        ctx.filter_new_condition_ctx_mut().set_selected_condition(
                            current_condition,
                            columns.iter().find(|c|c.name().eq(column_name)).map(|c|(c.clone(), condition.clone()))
                        );
                    }
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
                                action: Box::new(move |frame| {
                                    frame.remove_filter_condition(current_condition);

                                    None
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
                            ctx.filter_new_condition_ctx_mut().set_selected_condition(current_condition, None);
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
                                    action: Box::new(move |frame| {
                                        frame.change_condition_type(current_condition);

                                        None
                                    }),
                                }
                            );
                        }
                        if ui.button("AND+").clicked() {

                            ctx.ui_action(
                                UIAction::WaFrame {
                                    frame_id,
                                    action: Box::new(move |frame| {
                                        frame.add_filter_condition(ConditionType::Compoiste {conditions: vec![], ctype: CompositeType::AND}, Some(current_condition));

                                        None
                                    }),
                                }
                            );
                        }
                        if ui.button("OR+").clicked() {
                            ctx.ui_action(
                                UIAction::WaFrame {
                                    frame_id,
                                    action: Box::new(move |frame| {
                                        frame.add_filter_condition(ConditionType::Compoiste {conditions: vec![], ctype: CompositeType::OR}, Some(current_condition));

                                        None
                                    }),
                                }
                            );
                        }
                        if ui.button("-").clicked() {
                            ctx.ui_action(
                                UIAction::WaFrame {
                                    frame_id,
                                    action: Box::new(move |frame| {
                                        frame.remove_filter_condition(current_condition);

                                        None
                                    }),
                                }
                            );
                        }
                    });

                    for condition in conditions {
                        add_conditions(condition, ui, ctx, frame_id, ui_id, columns);
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

    fn ui(&self, ui: &mut egui::Ui, ctx: &Context, model_ctx: &mut ModelCtx) {

        let _max_height = ui.available_height() * 0.8;
        let max_width = ui.available_width() * 0.8;

        let values = self.values();
        let width = max_width/ values.len() as f32;
        // let max = values.iter().map(|v|v.1).max_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal)).unwrap_or(0);

        let y_ratio = 1.; // (max_height / max as f32) as f64;

        let bars  = values.iter().enumerate().map(|(i, (k, v))|{
            Bar::new((i as f32 * width) as f64, *v as f64 * y_ratio )
                .width(width as f64)
                .name(val_or_na(k))
        }).collect();


        let chart = BarChart::new(
            bars
        )
        .element_formatter(Box::new(move |b, _c|{
                format!("{}, {}", b.name, (b.value/y_ratio) as u32)
            }))
        .color(Color32::LIGHT_BLUE)
        .name(self._title());

        Plot::new("Histogram")

            .label_formatter(|name, _value| {
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

impl View for Corrs {
    fn title(&self) -> &str {
        self._title()
    }

    fn ui_id(&self) -> Id {
        Id::new(self.id())
    }

    fn ui(&self, ui: &mut Ui, ctx: &Context, model_ctx: &mut ModelCtx) {
        ui.add(egui::Label::new(self.title()));

        let _columns = self.columns().clone();
        debug!("_columns={:?}", _columns);

        let x_fmt = move |x:f64, _digits, _range: &RangeInclusive<f64>| {
            let x_1 = x * 10.;
            if x >= 0.0 && (x_1).fract() == 0.0 {
                _columns.get(x_1 as usize).map(|v|v.clone()).unwrap_or(String::from(""))
            } else {
                "".into()
            }

        };

        let _columns = self.columns().clone();

        let y_fmt = move |y:f64, _digits, _range: &RangeInclusive<f64>| {
            let y_1 = y * 10.;
            if y >= 0.0 && (y_1).fract() == 0.0 {
                _columns.get(y_1 as usize).map(|v|v.clone()).unwrap_or(String::from(""))
            } else {
                "".into()
            }

        };


        Plot::new("Correlations")

            .label_formatter(|name, _value| {
                // debug!("wapuku: name={:?}, value={:?}", name, value);

                if !name.is_empty() {
                    name.to_owned()
                } else {
                    "".to_owned()
                }

            })
            .allow_zoom(true)
            .allow_drag(true)
            .custom_x_axes(vec![
                AxisHints::default()
                    .formatter(x_fmt)
                    .max_digits(4),
            ])
            .custom_y_axes(vec![
                AxisHints::default()
                    .formatter(y_fmt)
                    .max_digits(4),
            ])
            .show(ui, |plot_ui| {
                let bounds = plot_ui.plot_bounds();
                // debug!(">>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>..... bounds={:?}", bounds);
                // plot_ui.polygon()
                // plot_ui.polygon(Polygon::new(PlotPoints::Owned(vec![[-10., -10.].into(), [-10., 10.].into(), [10., 10.].into(), [10., -10.].into() ])).fill_color(Color32::LIGHT_YELLOW))
            });

    }

    fn model_id(&self) -> WaModelId {
        WaModelId::DataLump{ frame_id: *self.frame_id(), lump_id: *self.id() }
    }
}

impl View for DataLump {
    fn title(&self) -> &str {
        self._title()
    }

    fn ui_id(&self) -> Id {
        Id::new(self.id())
    }

    fn ui(&self, ui: &mut egui::Ui, ctx: &Context, model_ctx: &mut ModelCtx) {
        let text_height = egui::TextStyle::Body.resolve(ui.style()).size;

        ui.horizontal(|ui| {

            if self.offset() > &0 {

                if ui.button("Prev").clicked() {

                    model_ctx.queue_action(ActionRq::DataLump {
                        frame_id: *self.frame_id(),
                        offset: if self.offset() > &100 {*self.offset() - 100} else {0},
                        limit: 100
                    });
                };
            }

            ui.add(egui::Label::new(format!("{}-{}", self.offset(), self.offset()+self.data().len())));

            if ui.button("Next").clicked() {

                model_ctx.queue_action(ActionRq::DataLump {
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

        table = columns.iter().fold(table, |t, _c|{
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


        }).body(|body| {
            let mut data_iter = self.data().iter();

            body.rows(2. * text_height, self.data().len(), |mut row| {
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
