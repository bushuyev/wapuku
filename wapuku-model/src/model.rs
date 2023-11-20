use std::collections::{HashMap, HashSet};
use std::{error, fmt};
use std::fmt::{Debug, Display, Formatter};
use std::hash::{Hash, Hasher};
use log::{debug, error, warn};
use uuid::Uuid;


use crate::data_type::*;


///////////////Data management model////////////////

pub fn wa_id() -> u128 {
    Uuid::new_v4().as_u128()
}



#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum WaModelId {
    Summary{ frame_id: u128},
    Filter{ frame_id: u128, filter_id:u128},
    DataLump{ frame_id: u128, lump_id:u128},
    Histogram{ frame_id: u128, histogram_id: u128}
}

impl WaModelId {
    pub fn id(&self) -> &u128 {
        match self {
            WaModelId::Summary { frame_id } => {
                frame_id
            }
            WaModelId::Histogram { frame_id:_, histogram_id } => {
                histogram_id
            }
            WaModelId::DataLump { frame_id:_ , lump_id} => {
                lump_id
            }
            WaModelId::Filter { frame_id:_, filter_id } => {
                filter_id
            }
        }
    }

    pub fn parent_id(&self)->Option<&u128> {
        match self {
            WaModelId::Summary { .. } => {
                None
            }
            WaModelId::Histogram { frame_id, .. } => {
                Some(frame_id)
            }
            WaModelId::DataLump { frame_id, .. } => {
                Some(frame_id)
            }
            WaModelId::Filter { frame_id, .. } => {
                Some(frame_id)
            }
        }
    }
}


#[derive(Debug)]
pub struct WaFrame {
    id:u128,
    name:String,
    summary:Summary,
    histograms:HashMap<u128, Histogram>,
    data_lump:Option<DataLump>,
    filter:Option<Filter>
}

impl WaFrame {
    pub fn new(id: u128, name: String, summary: Summary) -> Self {
        Self {
            id: id,
            name,
            summary,
            histograms: HashMap::new(),
            data_lump: None,
            filter: None
        }
    }

    pub fn add_filter(&mut self) -> u128 {

        let new_filter = Filter::empty(
            self.id,
            self.summary.columns().iter().map(|cs| cs.clone()).collect()
            // self.summary.columns().iter().map(|cs| cs.into()).collect()
        );
        let filter_id = new_filter.id;
        self.filter.replace(new_filter);

        filter_id
    }

    pub fn add_filter_condition(&mut self,  new_condition:ConditionType, target_condition:Option<*const ConditionType>) {
        if let Some(filter) = self.filter.as_mut() {
            filter.add_condition(new_condition, target_condition);
        } else {
            error!("Not filter for condition {:?} in frame {:?}", new_condition, self)
        }
    }

    pub fn remove_filter_condition(&mut self,  condition_to_remove:*const ConditionType) {
        if let Some(filter) = self.filter.as_mut() {
            filter.remove_condition(condition_to_remove);
        }
    }

    pub fn change_condition_type(&mut self, target_condition:*const ConditionType) {
        if let Some(filter) = self.filter.as_mut() {
            filter.change_condition_type(target_condition);
        } else {
            error!("Not filter for target_condition {:?} in frame {:?}", target_condition, self)
        }
    }

    pub fn filter(&self) -> Option<&Filter> {
        self.filter.as_ref()
    }

    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn summary(&self) -> &Summary {
        &self.summary
    }

    pub fn id(&self) -> u128 {
        self.id
    }

    pub fn add_histogram(&mut self, histogram:Histogram) {
        self.histograms.insert(*histogram.id(), histogram);
    }

    pub fn add_data_lump(&mut self, data_lump:DataLump) {
        if let Some(lump) = self.data_lump.as_mut() {
            lump.replace_data(data_lump);
        } else {
            self.data_lump.replace(data_lump);
        }
    }

    pub fn histograms(&self)->impl Iterator<Item = &Histogram> {
        self.histograms.values().into_iter()
    }

    pub fn data_lump(&self)->Option<&DataLump> {
        self.data_lump.as_ref()
    }

    pub fn purge(&mut self, id: WaModelId) {
        match id {
            WaModelId::Histogram{frame_id:_, histogram_id} => {
                self.histograms.remove(&histogram_id);
            },
            WaModelId::DataLump {frame_id:_, lump_id:_} => {
                self.data_lump.take();
            },
            WaModelId::Filter {frame_id:_, filter_id:_} => {
                self.filter.take();
            },
            _=>{}
        }
    }

    pub fn change_column_type(&mut self, column_name:String, dtype:SummaryColumn) {
        self.summary.change_column_type(column_name, dtype);
    }
}

#[derive(Debug, Clone)]
pub enum SummaryColumnType {
    Numeric{data:NumericColumnSummary},
    String{data:StringColumnSummary},
    Datetime{data:NumericColumnSummary},
    Boolean
}

impl From<SummaryColumnType> for WapukuDataType {
    fn from(value: SummaryColumnType) -> Self {
        match value {
            SummaryColumnType::Numeric { .. } => {
                WapukuDataType::Numeric
            }
            SummaryColumnType::String { .. } => {
                WapukuDataType::String
            }
            SummaryColumnType::Boolean => {
                WapukuDataType::Boolean
            }
            SummaryColumnType::Datetime { .. } => {
                WapukuDataType::Datetime
            }
        }
    }
}

impl From<&SummaryColumnType> for WapukuDataType {
    fn from(value: &SummaryColumnType) -> Self {
        match value {
            SummaryColumnType::Numeric { .. } => {
                WapukuDataType::Numeric
            }
            SummaryColumnType::String { .. } => {
                WapukuDataType::String
            }
            SummaryColumnType::Boolean => {
                WapukuDataType::Boolean
            }
            SummaryColumnType::Datetime { .. } => {
                WapukuDataType::Datetime
            }
        }
    }
}



#[derive(Debug, Clone)]
pub struct SummaryColumn {
    name:String,
    dtype: SummaryColumnType
}

impl SummaryColumn {

    pub fn name(&self) -> &String {
        &self.name
    }

    pub fn new(name: impl Into<String>, dtype: SummaryColumnType) -> Self {
        Self { name:name.into(), dtype }
    }

    pub fn dtype(&self) -> &SummaryColumnType {
        &self.dtype
    }
}


#[derive(Debug, Clone)]
pub struct NumericColumnSummary {
    min:String,
    avg:String,
    max:String
}

impl NumericColumnSummary {

    pub fn min(&self) -> &String {
        &self.min
    }
    pub fn avg(&self) -> &String {
        &self.avg
    }
    pub fn max(&self) -> &String {
        &self.max
    }
    pub fn new(min: impl Into<String>, avg: impl Into<String>, max: impl Into<String>) -> Self {
        Self { min:min.into(), avg:avg.into(), max:max.into() }
    }
}

#[derive(Debug, Clone)]
pub struct  StringColumnSummary {
    unique_values:String
}

impl StringColumnSummary {
    pub fn new(unique_values: impl Into<String>) -> Self {
        Self { unique_values:unique_values.into() }
    }

    pub fn unique_values(&self) -> &str {
        &self.unique_values
    }
}

#[derive(Debug)]
pub struct Summary {
    id:u128,
    frame_id: u128,
    _title:String,
    columns:Vec<SummaryColumn>,
    shape:String,
}

impl Summary {

    pub fn new(id:u128, frame_id: u128, title:String, columns: Vec<SummaryColumn>, shape:String) -> Self {
        Self {
            _title: title,
            id,
            frame_id,
            columns,
            shape
        }
    }

    pub fn frame_id(&self) -> u128 {
        self.frame_id
    }

    pub fn _title(&self) -> &str {
        &self._title
    }

    pub fn id(&self) -> u128 {
        self.id
    }

    pub fn shape(&self) -> &str {
        &self.shape
    }

    pub fn columns(&self) -> &Vec<SummaryColumn> {
        &self.columns
    }

    pub fn change_column_type(&mut self, column_name:String, dtype:SummaryColumn) {
        if let Some(column) = self.columns.iter_mut().find(|c|c.name.eq(&column_name)) {
            column.dtype = dtype.dtype
        } else {
            error!("change_column_type: no column_name={}", column_name)
        }
    }
}

#[derive(Debug)]
pub struct Histogram {
    id:u128,
    frame_id: u128,
    title: String,
    column:String,
    values:Vec<(String, u32)>,
}

impl Histogram {

    pub fn new(frame_id: u128, column:String, values:Vec<(String, u32)>) -> Self {
        Self {
            id: wa_id(),
            frame_id,
            title: format!("histogram/{}", column),
            column,
            values
        }
    }

    pub fn id(&self) -> &u128 {
        &self.id
    }

    pub fn frame_id(&self) -> u128 {
        self.frame_id
    }

    pub fn _title(&self) -> &str {
        &self.title
    }

    pub fn column(&self) -> &str {
        &self.column
    }


    pub fn values(&self) -> &Vec<(String, u32)> {
        &self.values
    }
}

/////////////////////////
#[derive(Debug)]
pub struct DataLump {
    id:u128,
    frame_id: u128,
    offset:usize,
    title: String,
    data:Vec<Vec<Option<String>>>,
    columns:Vec<(WapukuDataType, String)>
}

impl DataLump {

    pub fn new(frame_id: u128, offset:usize, rows:usize, columns:usize) -> Self {
        let column_vec = (0..columns).map(|_|None).collect::<Vec<Option<String>>>();
        Self {
            id: wa_id(),
            frame_id,
            offset,
            title: format!("data"),
            data: (0..rows).map(|_|column_vec.clone()).collect(),
            columns: vec![]
        }
    }

    pub fn add_column(&mut self,  dtype:WapukuDataType, name:impl Into<String>) {
        self.columns.push((dtype, name.into()));
    }

    pub fn set_value(&mut self, row:usize, col:usize, val:String) {
        self.data[row][col].replace(val);
    }

    pub fn data(&self) ->&Vec<Vec<Option<String>>> {
        &self.data
    }



    pub fn id(&self) -> &u128 {
        &self.id
    }



    pub fn _title(&self) -> &str {
        &self.title
    }

    pub fn columns(&self) -> &Vec<(WapukuDataType, String)> {
        &self.columns
    }
    pub fn frame_id(&self) -> &u128 {
        &self.frame_id
    }
    pub fn offset(&self) -> &usize {
        &self.offset
    }
    pub fn replace_data(&mut self, other:DataLump) {
        self.offset = other.offset;
        self.title = other.title;
        self.data = other.data;
    }
}
/////////////////////////
#[derive(Debug)]
pub struct Correlations {
    id:u128,
    frame_id: u128,
    title: String,
    columns:Vec<String>,
}

impl Correlations {
    pub fn new(frame_id: u128) -> Self {
        Self {
            id: wa_id(),
            frame_id,
            title: format!("Correlations"),
            columns:vec![]
        }
    }
}

///////////////Data view model////////////////

#[derive(Debug)]
pub enum WapukuError {
    DataLoad { msg: String },
    DataFrame { msg: String },
    General {msg: String},
    ToDo
}

impl WapukuError {
    pub fn msg(&self) -> &str {
        match self {
            WapukuError::DataLoad { msg } => {
                msg.as_ref()
            }
            WapukuError::DataFrame { msg } => {
                msg.as_ref()
            }
            WapukuError::General { msg } => {
                msg.as_ref()
            }
            WapukuError::ToDo => {
                "todo"
            }
        }
    }
}

impl Display for WapukuError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "({:?})", self)
    }
}

impl error::Error for WapukuError {}

pub type WapukuResult<T> = Result<T, WapukuError>;

pub trait Named {
    fn get_name(&self) -> &String;
}

pub trait PropertiesSet: Named + Debug {
    fn properties(&self) -> Vec<&dyn Property>;
}

pub trait Property: Named + Debug  {
    fn get_type(&self) -> &WapukuDataType;
    fn clone_to_box(&self) -> Box<dyn Property>;
    fn name(&self)->&String;
}

impl Hash for &dyn Property {
    fn hash<H>(&self, state: &mut H) where H: Hasher {
        self.name().hash(state)
    }
}

impl PartialEq for &dyn Property {
    fn eq(&self, other: &&dyn Property) -> bool {
        self.name() == other.name()
    }
}

impl Eq for &dyn Property {}



#[derive(Debug)]
pub struct PropertyRange {
    property: Box<dyn Property>,
    min: Option<i64>,
    max: Option<i64>,
}

impl PropertyRange {

    pub fn new(property: &dyn Property, min: Option<i64>, max: Option<i64>) -> Self {
        Self {
            property: property.clone_to_box(),
            min, max 
        }
    }
    
    pub fn to_range(&self, min: Option<i64>, max: Option<i64>)->Self {
        Self {
            property: self.property.clone_to_box(), 
            min, max 
        }
    }

    #[inline]
    pub fn property(&self) -> &Box<dyn Property> {
        &self.property
    }

    #[inline]
    pub fn min(&self) -> Option<i64> {
        self.min
    }

    #[inline]
    pub fn max(&self) -> Option<i64> {
        self.max
    }

    pub fn clone_to_box(&self) -> PropertyRange {
        PropertyRange {
            property: self.property.clone_to_box(),
            min: self.min.clone(),
            max: self.max.clone(),
        }
    }

}

#[derive(Debug)]
pub struct PropertyInGroup {
    #[allow(dead_code)]
    property_name: String,
    #[allow(dead_code)]
    volume: u8,
}

#[derive(Debug)]
pub enum DataBounds {
    X(PropertyRange),
    XY(PropertyRange, PropertyRange),
    XYZ(PropertyRange, PropertyRange, PropertyRange),
}

pub trait DataGroup: Debug {
    fn volume(&self) -> usize;
    fn property_groups(&self) -> Vec<&PropertyInGroup>;
    fn bounds(&self)->&DataBounds;
}

pub struct SimpleDataGroup {
    volume: usize,
    property_sizes: Vec<PropertyInGroup>,
    bounds: DataBounds
}

impl SimpleDataGroup {

    pub fn new(volume: usize, property_sizes: Vec<PropertyInGroup>, bounds: DataBounds) -> Self {
        Self { 
            volume, 
            property_sizes,
            bounds
        }
    }
}

impl Debug for SimpleDataGroup {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("")
            .field("volume", &self.volume)
            .field("property_sizes", &self.property_sizes)
            .finish()
    }}

impl DataGroup for SimpleDataGroup {
    fn volume(&self) -> usize {
        self.volume
    }

    fn property_groups(&self) -> Vec<&PropertyInGroup> {
        self.property_sizes.iter().collect()
    }

    fn bounds(&self) -> &DataBounds {
        &self.bounds
    }
}

pub type VecX<T> = Vec<Option<Box<T>>>;
pub type VecY<T> = Vec<VecX<T>>;

// #[derive(Debug)]
pub struct GroupsGrid {
    property_x:Box<dyn Property>,
    property_y:Box<dyn Property>,
    data: VecY<dyn DataGroup>
}

impl Debug for GroupsGrid {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {

        write!(f, "GroupsGrid:").expect("write");
        write!(f, "property_x: {:?}\r\n", self.property_x).expect("write");
        write!(f, "property_y: {:?}\r\n", self.property_y).expect("write");
        /*.field(&self.data);*/
        
        self.data.iter().enumerate().for_each(|(_i, r)|{
            write!(f, "data: row={:?}\r\n", r).expect("write");
        });
       
        Ok(())

    }
}

impl  GroupsGrid {

    pub fn new(property_x: Box<dyn Property>, property_y: Box<dyn Property>, data: VecY<dyn DataGroup>) -> Self {
        Self { property_x, property_y, data }
    }

    pub fn property_x(&self) -> &Box<dyn Property> {
        &self.property_x
    }
    pub fn property_y(&self) -> &Box<dyn Property> {
        &self.property_y
    }
    
    pub fn data(self) -> VecY<dyn DataGroup> {
        self.data
    }
    
    pub fn group_at(&self, x:usize, y:usize) -> Option<&Box<dyn DataGroup>> {
       self.data.get(y).and_then(|row|row.get(x).and_then(|v|v.as_ref()))
    }
}


pub struct FilteredFame {
    data:Box<dyn Data>
}

impl FilteredFame {
    pub fn new(data: Box<dyn Data>) -> Self {
        Self { data }
    }

    pub fn into(self) -> Box<dyn Data> {
        self.data
    }

    pub fn data(&self) -> &Box<dyn Data> {
        &self.data
    }
}


pub trait Data:Debug {
    fn load(data:Box<Vec<u8>>, name: Box<String>) -> Result<Vec<Self>, WapukuError> where Self: Sized;
    fn name(&self) -> String;
    fn all_sets(&self) -> Vec<&dyn PropertiesSet>;
    fn all_properties(&self) -> HashSet<&dyn Property>;
    fn build_grid(&self, property_x: PropertyRange, property_y: PropertyRange, groups_nr_x: u8, groups_nr_y: u8, name: &str) -> GroupsGrid;
    fn build_summary(&self, frame_id: u128, column: Option<String>) -> Summary;
    fn build_histogram(&self, frame_id: u128, column:String, bins: Option<usize>) -> Result<Histogram, WapukuError>;
    fn fetch_data(&self, frame_id: u128, offset: usize, limit: usize) -> Result<DataLump, WapukuError>;
    fn apply_filter(&self, frame_id: u128, filter:Filter) -> Result<FilteredFame, WapukuError>;
    fn convert_column(&mut self, frame_id: u128, column:String, pattern:String) -> Result<SummaryColumn, WapukuError>;
}

#[derive(Debug)]
pub struct DataProperty {
    property_type: WapukuDataType,
    name: String,
}

impl DataProperty {
    pub fn new<S:Into<String>>(property_type: WapukuDataType, name: S) -> Self {
        Self { property_type, name: name.into() }
    }
}

impl Display for DataProperty {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name)
    }
}

impl Named for DataProperty {
    fn get_name(&self) -> &String {
        &self.name
    }
}

impl Property for DataProperty {
    fn get_type(&self) -> &WapukuDataType {
        &self.property_type
    }

    fn clone_to_box(&self) -> Box<dyn Property> {
        Box::new(DataProperty {
            property_type: self.property_type.clone(),
            name: self.name.clone(),
        })
    }

    fn name(&self) -> &String {
        &self.name
    }
}


#[derive(Debug)]
pub struct SimplePropertiesSet {
    properties: Vec<DataProperty>,
    name: String,
}

impl SimplePropertiesSet {

    pub fn new<S:Into<String>>(properties: Vec<DataProperty>, name: S) -> Self {
        Self { 
            properties, 
            name: name.into()
        }
    }
}

impl Named for SimplePropertiesSet {
    fn get_name(&self) -> &String {
        &self.name
    }
}

impl PropertiesSet for SimplePropertiesSet {
    fn properties(&self) -> Vec<&dyn Property> {
        self.properties.iter().fold(vec![], |mut props, p| {
            props.push(p);

            props
        })
    }
}

//////////////////////////////////

#[derive(Debug)]
pub enum FilterColumnType {
    Numeric{min:f32, max:f32},
    String{pattern:String},
    Boolean{val:bool}
}

#[derive(Debug)]
pub struct FilterColumn {
    name:String,
    dtype: WapukuDataType
}

impl FilterColumn {
    pub fn new(name: String, dtype: WapukuDataType) -> Self {
        Self { name, dtype }
    }


    pub fn name(&self) -> &String {
        &self.name
    }
    pub fn dtype(&self) -> &WapukuDataType {
        &self.dtype
    }
}


#[derive(Debug, Clone)]
pub struct Filter {
    id:u128,
    frame_id: u128,
    title: String,
    columns:Vec<SummaryColumn>,
    conditions:Option<ConditionType>
}

impl From<&SummaryColumn> for FilterColumn {
    fn from(value: &SummaryColumn) -> Self {
        FilterColumn::new(value.name().to_string(), value.dtype().into())
    }
}

impl Filter {

    pub fn empty(frame_id: u128, columns:Vec<SummaryColumn>,)-> Self {
        Self {
            id: wa_id(),
            frame_id,
            columns,
            title: format!("filter/{}", "some"),//TODO name
            conditions:None
        }
    }

    pub fn id(&self) -> &u128 {
        &self.id
    }

    pub fn frame_id(&self) -> u128 {
        self.frame_id
    }

    pub fn _title(&self) -> &str {
        &self.title
    }


    pub fn columns(&self) -> &Vec<SummaryColumn> {
        &self.columns
    }


    pub fn conditions(&self) -> Option<&ConditionType> {
        self.conditions.as_ref()
    }

    pub fn change_condition_type(&mut self, target_condition:*const ConditionType) {
        if let Some(condition) = self.conditions.as_mut() {
            Self::in_sub_conditions_mut(target_condition, condition);
        }
    }

    fn in_sub_conditions_mut(target_condition: *const ConditionType, condition: &mut ConditionType) {
        let current_addr = condition as *const _;

        match condition {
            ConditionType::Compoiste { conditions, ctype } => {

                if current_addr == target_condition {
                    *ctype = match ctype {
                        CompositeType::AND => {
                            CompositeType::OR
                        }
                        CompositeType::OR => {
                            CompositeType::AND
                        }
                    };
                } else {
                    for child_condition in conditions {
                        Self::in_sub_conditions_mut(target_condition, child_condition);
                    }

                }
            }
            _ => {}
        }
    }

    #[cfg(test)]
    pub fn top_condition_ptr(&self)->Option<*const ConditionType> {
        self.conditions.as_ref().map(|c| c as * const _)
    }

    pub fn add_condition(&mut self, new_condition:ConditionType, target_condition:Option<*const ConditionType>) {
        debug!("add_condition 1 target_condition={:?} new_condition_ptr={:?}", target_condition, &new_condition as * const _);
        let old_addr  =  self.conditions.as_ref().map(|c| c as * const _);

        match self.conditions.take() {
            None => {
                self.conditions.replace(new_condition);
            }
            Some(mut condition_type) => {
                let parent_addr = &condition_type as *const _;


                match condition_type {
                    ConditionType::Single { .. } => {
                        debug!("add_condition replace single parent target_condition={:?} old_addr={:?}", target_condition, old_addr);

                        if target_condition.eq(&old_addr) {
                            self.conditions.replace(new_condition);

                        } else {
                            self.conditions.replace(ConditionType::Compoiste { conditions: vec![condition_type, new_condition], ctype: CompositeType::AND });
                        }
                    }
                    ConditionType::Compoiste { ref mut conditions, .. } => {
                        debug!("add_condition Compoiste target_condition={:?} parent_addr={:?}", target_condition, parent_addr);

                        if target_condition.is_none() || target_condition.eq(&old_addr) {
                            debug!("add_condition 1");

                            conditions.push(new_condition);
                        } else {
                            debug!("add_condition 2");

                            if !Self::push_child_condition(new_condition.clone(), conditions, target_condition.expect("target_condition")) {
                                debug!("add_condition 3");

                                conditions.push(new_condition);
                            }
                        }


                        self.conditions.replace(condition_type);
                    }
                }

            }
        }
    }

    pub fn remove_condition(&mut self, condition_to_remove: *const ConditionType) {

        if self.conditions.is_some(){

            if self.conditions.as_ref().map(|c| c as *const _ == condition_to_remove).unwrap_or(false) {
                self.conditions.take();
            } else {
                Self::remove_child_condition(self.conditions.as_mut().expect("self.conditions"), condition_to_remove)

            }

        } else {
            warn!("No top condition for remove_condition ")
        }

    }

    fn remove_child_condition(parent_condition:&mut ConditionType, condition_to_remove: *const ConditionType){
        match parent_condition {
            ConditionType::Single { .. } => {
                warn!("Parent condition can't be single")
            }
            ConditionType::Compoiste { conditions, .. } => {

                #[allow(unused)]
                if let Some((i, _c)) = conditions.iter().enumerate().find(|(i, c)| *c as *const _ == condition_to_remove) {
                    conditions.remove(i);
                } else {
                    for c in conditions {
                        Self::remove_child_condition(c, condition_to_remove);
                    }
                }
            }
        }
    }

    fn push_child_condition(new_condition: ConditionType, parent_conditions: &mut Vec<ConditionType>, target_addr:*const ConditionType) -> bool{
        for condition in parent_conditions {
            let parent_addr = condition as *const _;
            match condition {
                ConditionType::Single { condition, .. } => {
                    debug!("push_child_condition: 2 parent_addr={:?} target_addr={:?}", parent_addr, target_addr);
                    if parent_addr == target_addr {
                        let found_condition = condition;
                        match new_condition {
                            ConditionType::Single { ref condition, .. } => {
                                debug!("push_child_condition: 3");

                                *found_condition = condition.clone();
                            }
                            ConditionType::Compoiste { .. } => {
                                warn!("trying to replace single condition with composite");
                            }
                        }
                        return true;
                    }

                }
                ConditionType::Compoiste {  ref mut conditions, .. } => {
                    debug!("push_child_condition: 3 parent_addr={:?} target_addr={:?}", parent_addr, target_addr);
                    if parent_addr == target_addr {
                        conditions.push(new_condition);
                        return true;
                    } else {
                        return Self::push_child_condition(new_condition.clone(), conditions, target_addr);
                    }

                }
            }
        }

        return false;

    }

}

#[derive(Debug, Clone)]
pub enum ConditionType {
    Single{column_name:String, condition:Condition},
    Compoiste {conditions:Vec<ConditionType>, ctype:CompositeType},
}

#[derive(Debug, Clone)]
pub enum CompositeType {
    AND,
    OR
}


#[derive(Debug, Clone)]
pub enum  Condition {
    Numeric{min:f32, max:f32},
    String{pattern:String},
    Boolean{val:bool}
}