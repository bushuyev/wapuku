use std::collections::HashMap;
use std::error;
use std::fmt::{Debug, Display, Formatter};
use std::marker::PhantomData;
use polars::error::PolarsError;

use crate::data_type::*;

#[derive(Debug)]
pub(crate) enum WapukuError {
    DataFrame { msg: String }
}

impl Display for WapukuError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "({:?})", self)
    }
}

impl error::Error for WapukuError {}

pub(crate) type WapukuResult<T> = Result<T, WapukuError>;

pub trait Named {
    fn get_name(&self) -> &String;
}

pub trait PropertiesSet: Named + Debug {
    fn properties(&self) -> Vec<&dyn Property>;
}

pub trait Property: Named + Debug {
    fn get_type(&self) -> &WapukuDataType;
    fn clone_to_box(&self) -> Box<dyn Property>;
    fn name(&self)->&String;
}

pub struct PropertyRange<'a> {
    property: &'a dyn Property,
    min: Option<f64>,
    max: Option<f64>,
}

impl <'a> PropertyRange <'a> {

    pub fn new(property: &'a dyn Property, min: Option<f64>, max: Option<f64>) -> Self {
        Self {
            property,
            min, max 
        }
    }
    
    pub fn to_range(&self, min: Option<f64>, max: Option<f64>)->Self {
        Self {
            property: self.property, 
            min, max 
        }
    }

    #[inline]
    pub fn property(&self) -> &'a dyn Property {
        self.property
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

pub struct PropertyInGroup {
    property_name: String,
    volume: u8,
}

pub(crate) enum DataBounds<'a> {
    X(PropertyRange<'a>),
    XY(PropertyRange<'a>, PropertyRange<'a>),
    XYZ(PropertyRange<'a>, PropertyRange<'a>, PropertyRange<'a>),
}

pub trait DataGroup {
    fn volume(&self) -> u8;
    fn property_groups(&self) -> Vec<&PropertyInGroup>;
    fn min_value(&self) -> f64;
    fn max_value(&self) -> f64;
}

pub(crate) struct SimpleDataGroup<'a> {
    volume: u8,
    property_sizes: Vec<PropertyInGroup>,
    bounds: DataBounds<'a>
}

impl <'a> SimpleDataGroup<'a>  {

    pub fn new(size: u8, property_sizes: Vec<PropertyInGroup>, bounds: DataBounds<'a>) -> Self {
        Self { 
            volume: size, 
            property_sizes,
            bounds
        }
    }
}

impl <'a> DataGroup for SimpleDataGroup<'a> {
    fn volume(&self) -> u8 {
        self.volume
    }

    fn property_groups(&self) -> Vec<&PropertyInGroup> {
        self.property_sizes.iter().collect()
    }

    fn min_value(&self) -> f64 {
        todo!()
    }

    fn max_value(&self) -> f64 {
        todo!()
    }
}

pub struct GroupsVec<'a, T:DataGroup> {
    property:&'a dyn Property,
    data: Vec<T>,//column, row
}

impl <'a, T:DataGroup> GroupsVec<'a, T> {

    pub fn new(property:&'a dyn Property, data: Vec<T>) -> Self {
        
        Self {
            property,
            data,
        }
    }

}

type VecX<T> = Vec<T>;
type VecY<T> = Vec<VecX<T>>;

pub struct GroupsGrid<'a, T:DataGroup> {
    property_x:&'a dyn Property,
    property_y:&'a dyn Property,
    data: VecY<T>
}

impl <'a, T:DataGroup> GroupsGrid <'a, T> {
    pub fn new(property_x: &'a dyn Property, property_y: &'a dyn Property, data: Vec<Vec<T>>) -> Self {
        Self { property_x, property_y, data }
    }
}


pub(crate) trait Data<'a> {
    type DataGroupType:DataGroup;
    fn all_sets(&self) -> Vec<&dyn PropertiesSet>;
    fn group_by_1(&self, property_x: PropertyRange<'a>) -> GroupsVec<Self::DataGroupType>;
    fn group_by_2(&self, property_x: PropertyRange<'a>, property_y: PropertyRange<'a>) -> GroupsGrid<Self::DataGroupType>;
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


#[cfg(test)]
mod tests {
    use std::fmt::{Debug, Display, Formatter};
    use std::marker::PhantomData;
    use std::mem;
    use crate::data_type::WapukuDataType;
    use crate::model::{Data, GroupsVec, DataGroup, Named, PropertiesSet, Property, PropertyRange, PropertyInGroup, SimpleDataGroup, GroupsGrid, DataBounds, DataProperty, SimplePropertiesSet};

    struct TestData<'a> {
        property_sets: Vec<SimplePropertiesSet>,
        pd: PhantomData<&'a SimplePropertiesSet>,
    }

    impl <'a> Data<'a> for TestData<'a> {
        type DataGroupType = SimpleDataGroup<'a>;
        
        fn all_sets(&self) -> Vec<&dyn PropertiesSet> {
            self.property_sets.iter().fold(vec![], |mut props, p| {
                props.push(p);

                props
            })
        }

        fn group_by_1(&self, property_range: PropertyRange<'a>) -> GroupsVec<SimpleDataGroup<'a>> {

            GroupsVec::new(property_range.property, vec![
                SimpleDataGroup::new(10, vec![], DataBounds::X(property_range.to_range(Some(0.0),Some(10.0))))
            ])
        }

        fn group_by_2(&self, property_x: PropertyRange<'a>, property_y: PropertyRange<'a>) -> GroupsGrid<Self::DataGroupType> {

            GroupsGrid::new(
                property_x.property,
                property_y.property,
                vec![
                    (0..10).map(|i|/*Box::<dyn DataGroup>::from(Box::new(*/
                        SimpleDataGroup::new(10, vec![],
                     DataBounds::XY(
                                property_x.to_range(Some(i as f64 * 10.0), Some(i as f64 * 10.0 + 10.0)),
                                property_y.to_range(Some(i as f64 * 10.0), Some(i as f64 * 10.0 + 10.0))
                             )
                        )
                    ).collect()
                ]
            )
        }

    }

    #[test]
    fn test_data_init() {
        let wapuku_data = TestData {
            property_sets: vec![SimplePropertiesSet {
                name: "item_1".to_string(),
                properties: vec![
                    DataProperty {
                        name: "property_1".to_string(),
                        property_type: WapukuDataType::Numeric,
                    },
                    DataProperty {
                        name: "property_2".to_string(),
                        property_type: WapukuDataType::Numeric,
                    },
                    DataProperty {
                        name: "property_3".to_string(),
                        property_type: WapukuDataType::Numeric,
                    },
                ],

            }],
            pd: PhantomData
        };

        let all_sets = wapuku_data.all_sets();
        let property_set_1 = all_sets.first().expect("no first property se");

        let mut set_1_properties = property_set_1.properties().into_iter();


        let (property_1, property_2, property_3) = (set_1_properties.next().expect("property_1"), set_1_properties.next().expect("property_2"), set_1_properties.next().expect("property_2"));

        let data_vec = wapuku_data.group_by_1(PropertyRange::new (property_1,  None, None ));

        let data_grid = wapuku_data.group_by_2(
            PropertyRange::new (property_1,  None, None ),
            PropertyRange::new (property_2,  None, None )
        );

        if let Some(group) = data_grid.data.first().and_then(|first_row|first_row.first()) {

            let data_grid_0_0 = wapuku_data.group_by_2(
                PropertyRange::new (property_1,  None, None ),
                PropertyRange::new (property_2,  None, None )
            );
        }
        
    }
}