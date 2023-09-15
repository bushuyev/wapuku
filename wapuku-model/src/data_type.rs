use std::fmt::Debug;



#[derive(Debug, Clone, PartialEq)]
pub enum WapukuDataType {
    Numeric,
    String,
    Boolean
}

#[derive(Debug, PartialEq)]
pub struct WapukuDataValues {
    dtype:WapukuDataType,
    name:String
}


impl WapukuDataValues {
    pub fn new(dtype: WapukuDataType, name: impl Into<String>) -> Self {
        Self { dtype, name:name.into() }
    }

    pub fn dtype(&self) -> &WapukuDataType {
        &self.dtype
    }

    pub fn name(&self) -> &str {
        &self.name
    }
}