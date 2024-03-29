use std::collections::HashMap;
use wapuku_model::data_type::WapukuDataType;
use wapuku_model::model::{Condition, ConditionType, Corrs, SummaryColumn, SummaryColumnType};
use crate::model_views::Msg;


pub trait ValidationResult {
    fn is_error(&self)->bool;
    fn msg(&self)->&str;
}


#[derive(Debug)]
pub struct FilterNewConditionCtx {
    new_condition_column:String,

    pattern:String,
    min:String,
    max:String,
    boolean:bool,

    selected_column:Option<SummaryColumn>,

    msg:Msg,
    selected_condition: Option<*const ConditionType>
}

#[derive(Debug)]
pub struct SummaryActionsCtx {
    pub is_convret_dialog_open:Option<String>,
    pattern:String,
    to_type:WapukuDataType,
    corrs:HashMap<String, bool>,
}

impl SummaryActionsCtx {
    pub fn new() -> Self {
        Self { 
            is_convret_dialog_open:None,
            pattern: "%m/%d/%Y %T".into(),
            to_type:WapukuDataType::Datetime,
            corrs:HashMap::new(),
        }
    }

    pub fn pattern(&self) -> &String {
        &self.pattern
    }

    pub fn pattern_mut(&mut self) -> &mut String {
        &mut self.pattern
    }


    pub fn to_type(&self) -> &WapukuDataType {
        &self.to_type
    }

    pub fn get_selected_for_corr(&mut self,  column:String) -> &mut bool {
        self.corrs.entry(column).or_insert(false)
    }

    pub fn get_columns_for_corr_num(&self) -> usize {
        self.corrs.values().filter(|v|**v).count()
    }

    pub fn get_columns_for_corr(&self) -> Vec<String> {
        self.corrs.iter().filter(|kv|*kv.1).map(|v|v.0.clone()).collect()
    }
}

#[derive(Debug)]
pub enum FilterValidationResult {
    EmptyPattern,
    WrongFormat,
    LessThanMin,
    MoreThanMax,
    MinLessThanMax,
    Ok
}

impl ValidationResult for FilterValidationResult {
    fn is_error(&self) -> bool {
        match self {
            FilterValidationResult::Ok => {
                false
            },
            _=>{
                true
            }
        }
    }

    fn msg(&self) -> &str {
        match self {
            FilterValidationResult::WrongFormat => {
                "wrong format"
            }
            FilterValidationResult::LessThanMin => {
                "LessThanMin"
            }
            FilterValidationResult::MoreThanMax => {
                "MoreThanMax"
            }
            FilterValidationResult::MinLessThanMax => {
                "MinLessThanMax"
            }
            FilterValidationResult::EmptyPattern => {
                "EmptyPattern"
            }
            FilterValidationResult::Ok => {
                "Ok"
            }
        }
    }
}


impl FilterNewConditionCtx {
    pub fn new() -> Self {
        Self {
            new_condition_column: String::new(),
            min:String::new(),
            max:String::new(),
            pattern:String::new(),
            boolean:false,
            selected_column:None,
            msg:Msg::empty(),
            selected_condition: None
        }
    }

    pub fn new_condition_column(&self) -> &String {
        &self.new_condition_column
    }

    pub fn new_condition_column_mut(&mut self) -> &mut String {
        &mut self.new_condition_column
    }

    pub fn pattern(&self) -> &str {
        &self.pattern
    }

    pub fn pattern_mut(&mut self) -> &mut String {
        &mut self.pattern
    }

    pub fn boolean(&mut self) -> &mut bool  {
        &mut self.boolean
    }

    pub fn min(&self) -> &String {
        &self.min
    }

    pub fn min_mut(&mut self) -> &mut String {
        &mut self.min
    }

    pub fn max(&self) -> &String {
        &self.max
    }

    pub fn max_mut(&mut self) -> &mut String {
        &mut self.max
    }

    pub fn reset(&mut self) {
        self.pattern = "".into();
        self.min = "".into();
        self.max = "".into();
        self.boolean = false;
        self.selected_column = None;
        self.selected_condition = None;
    }

    pub fn set_selected_condition(&mut self, condition_ptr:*const ConditionType, column_condition:Option<(SummaryColumn, Condition)> ) { //only Simple conditions to init fields, Composites highlight border
        self.reset();
        self.selected_condition.replace(condition_ptr);

        if let Some((column, condition)) = column_condition {

            self.new_condition_column = column.name().clone();
            self.selected_column = Some(column);

            match condition {
                Condition::Numeric { min, max } => {
                    self.min = format!("{}", min);
                    self.max = format!("{}", max);
                }
                Condition::String { pattern } => {
                    self.pattern = pattern.clone();
                }
                Condition::Boolean { val } => {
                    self.boolean = val;
                }
            }
        }
    }

    pub fn take_selected_condition(&mut self ) -> Option<*const ConditionType> {
        self.selected_condition.take()
    }

    pub fn selected_condition(&self) -> Option<*const ConditionType> {
        self.selected_condition
    }

    pub fn init(&mut self, column: &SummaryColumn) {
        let column = column.clone();

        match column.dtype() {
            SummaryColumnType::Numeric { data } => {
                self.min = data.min().clone();
                self.max = data.max().clone();
            }
            SummaryColumnType::String { .. } => {}
            SummaryColumnType::Boolean => {}
            SummaryColumnType::Datetime { .. } => {}
        }
        self.selected_column.replace(column);
    }

    pub fn selected_column(&self) -> &Option<SummaryColumn> {
        &self.selected_column
    }

    pub fn validate(&mut self) {
        if let Some(selected_column) = self.selected_column.as_ref() {
            match selected_column.dtype() {
                SummaryColumnType::Numeric { data } => {
                    let min_r = self.min.parse::<f32>();

                    self.msg  = if min_r.is_err(){
                        FilterValidationResult::WrongFormat
                    } else {
                        let max_r = self.max.parse::<f32>();
                        if max_r.is_err() {
                            FilterValidationResult::WrongFormat
                        } else {
                            let min = min_r.unwrap();
                            let max = max_r.unwrap();

                            let column_min = data.min().parse::<f32>().unwrap_or(-f32::INFINITY);
                            let column_max = data.max().parse::<f32>().unwrap_or(f32::INFINITY);

                            if min < column_min {
                                FilterValidationResult::LessThanMin

                            } else if max > column_max {
                                FilterValidationResult::MoreThanMax

                            } else if min > max {
                                FilterValidationResult::MinLessThanMax

                            } else {
                                FilterValidationResult::Ok
                            }

                        }
                    }.into();
                }
                SummaryColumnType::String { data:_ } => {
                    self.msg =  if self.pattern.is_empty() {
                        FilterValidationResult::EmptyPattern

                    } else {
                        FilterValidationResult::Ok
                    }.into();
                }
                SummaryColumnType::Boolean => {

                }
                SummaryColumnType::Datetime { .. } => {}
            }

        }
    }

    pub fn msg(&self) -> &Msg {
        &self.msg
    }

    pub fn to_condition(&mut self) -> Option<(String, Condition)> {
        self.selected_column.take().map(|c|{
            (
                self.new_condition_column.clone(),
                match c.dtype() {

                    SummaryColumnType::Numeric { .. } => {
                        Condition::Numeric {
                            min: self.min.parse().unwrap_or(0.0),//TODO handle parse error
                            max: self.max.parse().unwrap_or(0.0),
                        }
                    }
                    SummaryColumnType::String { .. } => {
                        Condition::String {pattern: self.pattern.clone()}
                    }
                    SummaryColumnType::Boolean => {
                        Condition::Boolean {val: self.boolean}
                    }
                    SummaryColumnType::Datetime { .. } => {
                        //TODO
                        Condition::Numeric {
                            min: self.min.parse().unwrap_or(0.0),
                            max: self.max.parse().unwrap_or(0.0),
                        }
                    }
                }
            )
        })
    }
}