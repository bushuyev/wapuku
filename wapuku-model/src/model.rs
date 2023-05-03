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

trait PropertiesSet: Named + Debug {
    fn properties(&self) -> Vec<&dyn Property>;
}

pub trait Property: Named + Debug {
    fn get_type(&self) -> &DataType;
    fn clone_to_box(&self) -> Box<dyn Property>;
}

struct PropertyRange<'a> {
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
}

struct PropertyInGroup {
    property_name: String,
    volume: u8,
}

enum DataBounds<'a> {
    X(PropertyRange<'a>),
    XY(PropertyRange<'a>, PropertyRange<'a>),
    XYZ(PropertyRange<'a>, PropertyRange<'a>, PropertyRange<'a>),
}

trait DataGroup {
    fn volume(&self) -> u8;
    fn property_groups(&self) -> Vec<&PropertyInGroup>;
    fn min_value(&self) -> f64;
    fn max_value(&self) -> f64;
}

struct SimpleDataGroup<'a> {
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

struct GroupsVec {
    property:Box<dyn Property>,
    data: Vec<Box<dyn DataGroup>>,//column, row
}

impl GroupsVec {

    pub fn new(property:Box<dyn Property>, data: Vec<Box<dyn DataGroup>>) -> Self {
        
        Self {
            property,
            data,
        }
    }

}

type VecX = Vec<Box<dyn DataGroup>>;
type VecY = Vec<VecX>;

struct GroupsGrid {
    property_x:Box<dyn Property>,
    property_y:Box<dyn Property>,
    data: VecY
}

impl GroupsGrid {
    pub fn new(property_x: Box<dyn Property>, property_y: Box<dyn Property>, data: Vec<Vec<Box<dyn DataGroup>>>) -> Self {
        Self { property_x, property_y, data }
    }
}


trait Data {
    fn all_sets(&self) -> Vec<&dyn PropertiesSet>;
    fn group_by_1(&self, property_x: PropertyRange<'static>) -> GroupsVec;
    fn group_by_2(&self, property_x: PropertyRange<'static>, property_y: PropertyRange<'static>) -> GroupsGrid;
}

#[cfg(test)]
mod tests {
    use std::fmt::{Debug, Display, Formatter};
    use std::mem;
    use crate::data_type::DataType;
    use crate::model::{Data, GroupsVec, DataGroup, Named, PropertiesSet, Property, PropertyRange, PropertyInGroup, SimpleDataGroup, GroupsGrid, DataBounds};


    #[derive(Debug)]
    struct TestProperty {
        property_type: DataType,
        name: String,
    }

    impl Display for TestProperty {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            write!(f, "{}", self.name)
        }
    }

    impl Named for TestProperty {
        fn get_name(&self) -> &String {
            &self.name
        }
    }

    impl Property for TestProperty {
        fn get_type(&self) -> &DataType {
            &self.property_type
        }

        fn clone_to_box(&self) -> Box<dyn Property> {
            Box::new(TestProperty {
                property_type: self.property_type.clone(),
                name: self.name.clone(),
            })
        }
    }

    #[derive(Debug)]
    struct TestPropertiesSet {
        properties: Vec<TestProperty>,
        name: String,
    }

    impl Named for TestPropertiesSet {
        fn get_name(&self) -> &String {
            &self.name
        }
    }

    impl PropertiesSet for TestPropertiesSet {
        fn properties(&self) -> Vec<&dyn Property> {
            self.properties.iter().fold(vec![], |mut props, p| {
                props.push(p);

                props
            })
        }
    }


    struct TestData {
        property_sets: Vec<TestPropertiesSet>,
    }

    impl Data for TestData {
        
        fn all_sets(&self) -> Vec<&dyn PropertiesSet> {
            self.property_sets.iter().fold(vec![], |mut props, p| {
                props.push(p);

                props
            })
        }

        fn group_by_1(&self, property_range: PropertyRange<'static>) -> GroupsVec {

            GroupsVec::new(property_range.property.clone_to_box(), vec![
                Box::new(SimpleDataGroup::new(10, vec![], DataBounds::X(property_range.to_range(Some(0.0),Some(10.0)))))
            ])
        }

        fn group_by_2(&self, property_x: PropertyRange<'static>, property_y: PropertyRange<'static>) -> GroupsGrid {

            GroupsGrid::new(
                property_x.property.clone_to_box(),
                property_y.property.clone_to_box(),
                vec![
                    (0..10).map(|i|Box::<dyn DataGroup>::from(Box::new(
                        SimpleDataGroup::new(10, vec![], DataBounds::X(property_x.to_range(Some(i as f64 * 10.0), Some(i as f64 * 10.0 + 10.0))))
                    ))).collect()
                ]
            )
        }

    }

    #[test]
    fn test_data_init() {
        let wapuku_data = TestData {
            property_sets: vec![TestPropertiesSet {
                name: "item_1".to_string(),
                properties: vec![
                    TestProperty {
                        name: "property_1".to_string(),
                        property_type: DataType::Numeric,
                    },
                    TestProperty {
                        name: "property_2".to_string(),
                        property_type: DataType::Numeric,
                    },
                    TestProperty {
                        name: "property_3".to_string(),
                        property_type: DataType::Numeric,
                    },
                ],

            }],
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