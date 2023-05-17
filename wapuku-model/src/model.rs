use std::collections::{HashMap, HashSet};
use std::error;
use std::fmt::{Debug, Display, Formatter};
use std::hash::{Hash, Hasher};
use std::marker::PhantomData;
use polars::error::PolarsError;

use crate::data_type::*;

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
    min: Option<f64>,
    max: Option<f64>,
}

impl PropertyRange {

    pub fn new(property: &dyn Property, min: Option<f64>, max: Option<f64>) -> Self {
        Self {
            property: property.clone_to_box(),
            min, max 
        }
    }
    
    pub fn to_range(&self, min: Option<f64>, max: Option<f64>)->Self {
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
    pub fn min(&self) -> Option<f64> {
        self.min
    }

    #[inline]
    pub fn max(&self) -> Option<f64> {
        self.max
    }
}

#[derive(Debug)]
pub struct PropertyInGroup {
    property_name: String,
    volume: u8,
}

#[derive(Debug)]
pub enum DataBounds {
    X(PropertyRange),
    XY(PropertyRange, PropertyRange),
    XYZ(PropertyRange, PropertyRange, PropertyRange),
}

pub trait DataGroup: Debug {
    fn volume(&self) -> u8;
    fn property_groups(&self) -> Vec<&PropertyInGroup>;
    fn bounds(&self)->DataBounds;
}

#[derive(Debug)]
pub struct SimpleDataGroup {
    volume: u8,
    property_sizes: Vec<PropertyInGroup>,
    bounds: DataBounds
}

impl SimpleDataGroup {

    pub fn new(volume: u8, property_sizes: Vec<PropertyInGroup>, bounds: DataBounds) -> Self {
        Self { 
            volume, 
            property_sizes,
            bounds
        }
    }
}

impl DataGroup for SimpleDataGroup {
    fn volume(&self) -> u8 {
        self.volume
    }

    fn property_groups(&self) -> Vec<&PropertyInGroup> {
        self.property_sizes.iter().collect()
    }

    fn bounds(&self) -> DataBounds {
        todo!()
    }
}

pub struct GroupsVec {
    property:Box<dyn Property>,
    data: Vec<Box<dyn DataGroup>>,
}

impl GroupsVec {

    pub fn new(property:Box<dyn Property>, data: Vec<Box<dyn DataGroup>>) -> Self {
        
        Self {
            property,
            data,
        }
    }

}

type VecX<T> = Vec<Box<T>>;
type VecY<T> = Vec<VecX<T>>;

#[derive(Debug)]
pub struct GroupsGrid {
    property_x:Box<dyn Property>,
    property_y:Box<dyn Property>,
    data: VecY<dyn DataGroup>
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
    pub fn data(&mut self) -> &mut VecY<dyn DataGroup> {
        &mut self.data
    }
}


pub trait Data {
    fn all_sets(&self) -> Vec<&dyn PropertiesSet>;
    fn all_properties(&self) -> HashSet<&dyn Property>;
    fn group_by_1(&self, property_x: PropertyRange) -> GroupsVec;
    fn group_by_2(&self, property_x: PropertyRange, property_y: PropertyRange, groups_nr_x: u8, groups_nr_y: u8) -> GroupsGrid;
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

