use std::collections::{HashMap, HashSet};
use std::{error, fmt, iter};
use std::fmt::{Debug, Display, Formatter};
use std::hash::{Hash, Hasher};
use log::debug;
use uuid::Uuid;


use crate::data_type::*;

///////////////Data management model////////////////

pub fn wa_id() -> u128 {
    Uuid::new_v4().as_u128()
}

#[derive(Debug)]
pub enum WaModelId {
    Summary{ frame_id: u128},
    DataLump{ frame_id: u128, lump_id:u128},
    Histogram{ frame_id: u128, histogram_id: u128}
}

impl WaModelId {
    pub fn id(&self) -> &u128 {
        match self {
            WaModelId::Summary { frame_id } => {
                frame_id
            }
            WaModelId::Histogram { frame_id, histogram_id } => {
                histogram_id
            }
            WaModelId::DataLump { frame_id , lump_id} => {
                lump_id
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
        }
    }
}


#[derive(Debug)]
pub struct WaFrame {
    id:u128,
    name:String,
    summary:Summary,
    histograms:HashMap<u128, Histogram>,
    data_lumps:HashMap<u128, DataLump>
}

impl WaFrame {
    pub fn new(id: u128, name: String, summary: Summary) -> Self {
        Self {
            id: id,
            name,
            summary,
            histograms: HashMap::new(),
            data_lumps: HashMap::new()
        }
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

    pub fn add_histogram(&mut self, mut histogram:Histogram) {
        self.histograms.insert(*histogram.id(), histogram);
    }

    pub fn add_data_lump(&mut self, mut data_lump:DataLump) {
        self.data_lumps.insert(*data_lump.id(), data_lump);
    }

    pub fn histograms(&self)->impl Iterator<Item = &Histogram> {
        self.histograms.values().into_iter()
    }

    pub fn data_lumps(&self)->impl Iterator<Item = &DataLump> {
        self.data_lumps.values().into_iter()
    }

    pub fn purge(&mut self, id: WaModelId) {
        match id {
            WaModelId::Histogram{frame_id, histogram_id} => {
                self.histograms.remove(&histogram_id);
            },
            WaModelId::DataLump {frame_id, lump_id} => {
                self.data_lumps.remove(&lump_id);
            }
            _=>{}
        }
    }
}

#[derive(Debug)]
pub enum ColumnSummaryType {
    Numeric{data:NumericColumnSummary},
    String{data:StringColumnSummary},
    Boolean
}

impl From<ColumnSummaryType> for WapukuDataType {
    fn from(value: ColumnSummaryType) -> Self {
        match value {
            ColumnSummaryType::Numeric { .. } => {
                WapukuDataType::Numeric
            }
            ColumnSummaryType::String { .. } => {
                WapukuDataType::String
            }
            ColumnSummaryType::Boolean => {
                WapukuDataType::Boolean
            }
        }
    }
}

#[derive(Debug)]
pub struct ColumnSummary {
    name:String,
    dtype:ColumnSummaryType
}

impl ColumnSummary {

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn new(name: String, dtype:ColumnSummaryType) -> Self {
        Self { name, dtype }
    }

    pub fn dtype(&self) -> &ColumnSummaryType {
        &self.dtype
    }
}


#[derive(Debug)]
pub struct NumericColumnSummary {
    min:String,
    avg:String,
    max:String
}

impl NumericColumnSummary {


    pub fn min(&self) -> &str {
        &self.min
    }
    pub fn avg(&self) -> &str {
        &self.avg
    }
    pub fn max(&self) -> &str {
        &self.max
    }

    pub fn new(min: String, avg: String, max: String) -> Self {
        Self { min, avg, max }
    }
}

#[derive(Debug)]
pub struct  StringColumnSummary {
    unique_values:String
}

impl StringColumnSummary {
    pub fn new(unique_values: String) -> Self {
        Self { unique_values }
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
    columns:Vec<ColumnSummary>,
    shape:String
}

impl Summary {

    pub fn columns(&self) -> &Vec<ColumnSummary> {
        &self.columns
    }

    pub fn new(id:u128, frame_id: u128, title:String,  columns: Vec<ColumnSummary>, shape:String) -> Self {
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
}

#[derive(Debug)]
pub struct Histogram {
    id:u128,
    frame_id: u128,
    column:String,
    title: String,
    values:Vec<(String, u32)>,
}

impl Histogram {

    pub fn new(frame_id: u128, column:String, values:Vec<(String, u32)>) -> Self {
        Self {
            id: wa_id(),
            title: format!("histogram/{}", column),
            column,
            frame_id,
            values
        }
    }
    pub fn id(&self) -> &u128 {
        &self.id
    }

    pub fn column(&self) -> &str {
        &self.column
    }
    pub fn frame_id(&self) -> u128 {
        self.frame_id
    }

    pub fn _title(&self) -> &str {
        &self.title
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
    title: String,
    data:Vec<Vec<Option<String>>>,
    columns:Vec<(WapukuDataType, String)>
}

impl DataLump {

    pub fn new(frame_id: u128, rows:usize, columns:usize) -> Self {
        let column_vec = (0..columns).map(|_|None).collect::<Vec<Option<String>>>();
        Self {
            id: wa_id(),
            frame_id,
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



pub trait Data:Debug {
    fn load(data:Box<Vec<u8>>, name: Box<String>) -> Result<Vec<Self>, WapukuError> where Self: Sized;
    fn name(&self) -> String;
    fn all_sets(&self) -> Vec<&dyn PropertiesSet>;
    fn all_properties(&self) -> HashSet<&dyn Property>;
    fn build_grid(&self, property_x: PropertyRange, property_y: PropertyRange, groups_nr_x: u8, groups_nr_y: u8, name: &str) -> GroupsGrid;
    fn build_summary(&self, frame_id: u128) -> Summary;
    fn build_histogram(&self, frame_id: u128, column:String) -> Result<Histogram, WapukuError>;
    fn fetch_data(&self, frame_id: u128, offset: usize, limit: usize) -> Result<DataLump, WapukuError>;
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

