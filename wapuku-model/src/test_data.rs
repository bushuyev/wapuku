use std::collections::HashSet;
use std::marker::PhantomData;
use crate::data_type::WapukuDataType;
use crate::model::{Data, DataBounds, DataGroup, DataProperty, GroupsGrid, GroupsVec, PropertiesSet, Property, PropertyRange, SimpleDataGroup, SimplePropertiesSet};

#[derive(Debug)]
pub struct TestData {
    property_sets: Vec<SimplePropertiesSet>,
}

impl  TestData {

    pub fn new() -> Self {
        Self { 
            property_sets:vec![
                SimplePropertiesSet::new(

                vec![
                    DataProperty::new(
                        WapukuDataType::Numeric,
                        "property_1".to_string(),

                    ),
                    DataProperty::new(
                        WapukuDataType::Numeric,
                        "property_2".to_string(),

                    ),
                    DataProperty::new(
                        WapukuDataType::Numeric,
                        "property_3".to_string(),

                    ),
                ],
                "item_1",
            )], 
        }
    }

}

impl  Data for TestData {
    
    fn all_sets(&self) -> Vec<&dyn PropertiesSet> {
        self.property_sets.iter().fold(vec![], |mut props, p| {
            props.push(p);

            props
        })
    }

    fn all_properties(&self) -> HashSet<&dyn Property> {
        self.property_sets.iter().flat_map(|property_set|property_set.properties().into_iter()).collect()
    }

    fn build_grid(&self, property_x: PropertyRange, property_y: PropertyRange, groups_nr_x: u8, groups_nr_y: u8, name: &str) -> GroupsGrid {
        

        GroupsGrid::new(
            property_x.property().clone_to_box(),
            property_y.property().clone_to_box(),
            
                (0..groups_nr_y).map(|y|
                    (0..groups_nr_x).map(|x|
                        Some(Box::<dyn DataGroup>::from(Box::new(SimpleDataGroup::new((x+y) as usize, vec![],
                             DataBounds::XY(
                                 property_x.to_range(Some((x as f64 * 10.0).ceil() as i64), Some((x as f64 * 10.0 + 10.0).ceil() as i64)),
                                 property_y.to_range(Some((x as f64 * 10.0).ceil() as i64), Some((x as f64 * 10.0 + 10.0).ceil() as i64)),
                             ),
                        ))))
                    ).collect::<Vec<Option<Box<dyn DataGroup>>>>()

            ).collect::<Vec<Vec<Option<Box<dyn DataGroup>>>>>()
            
        )
    }

   
}


#[cfg(test)]
mod tests {
    use std::fmt::{Debug, Display, Formatter};
    use std::marker::PhantomData;
    use std::mem;
    use log::debug;
    use crate::data_type::WapukuDataType;
    use crate::model::{Data, GroupsVec, DataGroup, Named, PropertiesSet, Property, PropertyRange, PropertyInGroup, SimpleDataGroup, GroupsGrid, DataBounds, DataProperty, SimplePropertiesSet};
    use crate::test_data::TestData;


    #[test]
    fn test_data_init() {
        // let wapuku_data = TestData::new();
        // 
        // let all_properties = wapuku_data.all_properties();
        // 
        // debug!("wapuku: all_properties: {:?}", all_properties);
        // 
        // let all_sets = wapuku_data.all_sets();
        // let property_set_1 = all_sets.first().expect("no first property se");
        // 
        // let mut set_1_properties = property_set_1.properties().into_iter();
        // 
        // 
        // let (property_1, property_2, property_3) = (set_1_properties.next().expect("property_1"), set_1_properties.next().expect("property_2"), set_1_properties.next().expect("property_2"));
        // 
        // let data_vec = wapuku_data.group_by_1(PropertyRange::new (property_1,  None, None ));
        // 
        // let mut data_grid = wapuku_data.build_grid(
        //     PropertyRange::new (property_1,  None, None ),
        //     PropertyRange::new (property_2,  None, None ),
        //     3, 3, "property_3"
        // );
        // 
        // if let Some(group) = data_grid.data().first().and_then(|first_row|first_row.first()) {
        // 
        //     let data_grid_0_0 = wapuku_data.build_grid(
        //         PropertyRange::new (property_1,  None, None ),
        //         PropertyRange::new (property_2,  None, None ),
        //         3, 3, "property_3"
        //     );
        // }

    }
}