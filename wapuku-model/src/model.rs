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
    groups: i8,
}

struct PropertyInGroup {
    property_name: String,
    size: u8,
}

trait DataGroup {
    fn size(&self) -> u8;
    fn property_groups(&self) -> Vec<&PropertyInGroup>;
}

struct SimpleDataGroup {
    size: u8,
    property_sizes: Vec<PropertyInGroup>,
}

impl SimpleDataGroup  {
    pub fn new(size: u8, property_sizes: Vec<PropertyInGroup>) -> Self {
        Self { size, property_sizes }
    }
}

impl DataGroup for SimpleDataGroup {
    fn size(&self) -> u8 {
        self.size
    }

    fn property_groups(&self) -> Vec<&PropertyInGroup> {
        self.property_sizes.iter().collect()
    }
}

struct DataVec {
    property:Box<dyn Property>,
    data: Vec<Box<dyn DataGroup>>,//column, row
}

impl DataVec {

    pub fn new(property:Box<dyn Property>, columns:usize) -> Self {
        Self {
            property,
            data: Vec::with_capacity(columns)
        }
    }

}

trait Data {
    fn all_sets(&self) -> Vec<&dyn PropertiesSet>;
    fn group_by_1(&self, property: PropertyRange) -> DataVec;
}

#[cfg(test)]
mod tests {
    use std::fmt::{Debug, Display, Formatter};
    use crate::data_type::DataType;
    use crate::model::{Data, DataVec, DataGroup, Named, PropertiesSet, Property, PropertyRange, PropertyInGroup, SimpleDataGroup};


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

        fn group_by_1(&self, property_range: PropertyRange) -> DataVec {
            // DataVec {
            //     data: vec![
            //         vec![
            //             Box::new(SimpleDataGroup::new(
            //                 8,
            //                 vec![
            //                     // PropertyInGroup {
            //                     //     property: TestProperty {
            //                     //         name: "property_1".to_string(),
            //                     //         property_type: DataType::Numeric,
            //                     //     },,
            //                     //     size: 0,
            //                     // }
            //                 ],
            //             ))
            //         ]
            //     ],
            // }
            DataVec::new(property_range.property.clone_to_box(), 10)
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
        let property_set_1 = all_sets.first().expect("no first property set");

        let mut set_1_properties = property_set_1.properties().into_iter();


        let (property_1, property_2, property_3) = (set_1_properties.next().expect("property_1"), set_1_properties.next().expect("property_2"), set_1_properties.next().expect("property_2"));

        let data_grid = wapuku_data.group_by_1(PropertyRange { property: property_1, min: None, max: None, groups: 10 });
    }
}