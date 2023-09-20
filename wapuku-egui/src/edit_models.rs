use wapuku_model::model::{SummaryColumn, SummaryColumnType};
use crate::model_views::Msg;


pub trait ValidationResult {
    fn is_error(&self)->bool;
    fn msg(&self)->&str;
}


#[derive(Debug)]
pub struct FilterNewConditionCtx {
    new_condition_column:String,
    min:String,
    max:String,
    pattern:String,
    boolean:Option<bool>,
    selected_column:Option<SummaryColumn>,
    validation:FilterValidationResult,
    msg:Msg
}

#[derive(Debug)]
pub enum FilterValidationResult {
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
            boolean:None,
            selected_column:None,
            validation:FilterValidationResult::Ok,
            msg:Msg::empty()
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

    pub fn boolean(&self) -> Option<bool> {
        self.boolean
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

    pub fn init(&mut self, column: &SummaryColumn) {
        let column = column.clone();
        match column.dtype() {
            SummaryColumnType::Numeric { data } => {
                self.min = data.min().clone();
                self.max = data.max().clone();
            }
            SummaryColumnType::String { .. } => {}
            SummaryColumnType::Boolean => {}
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
                        FilterValidationResult::WrongFormat.into()

                    } else {
                        let max_r = self.max.parse::<f32>();
                        if max_r.is_err() {
                            FilterValidationResult::WrongFormat.into()
                        } else {
                            let min = min_r.unwrap();
                            let max = max_r.unwrap();

                            let column_min = data.min().parse::<f32>().unwrap_or(-f32::INFINITY);
                            let column_max = data.max().parse::<f32>().unwrap_or(f32::INFINITY);

                            if min < column_min {
                                FilterValidationResult::LessThanMin.into()

                            } else if max > column_max {
                                FilterValidationResult::MoreThanMax.into()

                            } else if min > max {
                                FilterValidationResult::MinLessThanMax.into()

                            } else {
                                FilterValidationResult::Ok.into()
                            }

                        }
                    };
                }
                SummaryColumnType::String { .. } => {}
                SummaryColumnType::Boolean => {}
            }

        }


    }

    pub fn msg(&self) -> &Msg {
        &self.msg
    }
}