use std::collections::{ HashSet};
use std::{error, fmt};
use std::fmt::{Debug, Display, Formatter};
use std::hash::{Hash, Hasher};

use crate::data_type::*;

///////////////Data management model////////////////
pub struct ColumnSummary {
    name:String,
    min:f32,
    avg:f32,
    max:f32
}

impl ColumnSummary {
    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn min(&self) -> f32 {
        self.min
    }
    pub fn avg(&self) -> f32 {
        self.avg
    }
    pub fn max(&self) -> f32 {
        self.max
    }
}

pub struct Summary {
    columns:Vec<ColumnSummary>
}

impl Summary {
    pub fn columns(&self) -> &Vec<ColumnSummary> {
        &self.columns
    }
}
///////////////Data view model////////////////

#[derive(Debug)]
pub enum WapukuError {
    DataFrame { msg: String }
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
    fn all_sets(&self) -> Vec<&dyn PropertiesSet>;
    fn all_properties(&self) -> HashSet<&dyn Property>;
    fn build_grid(&self, property_x: PropertyRange, property_y: PropertyRange, groups_nr_x: u8, groups_nr_y: u8, name: &str) -> GroupsGrid;
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

