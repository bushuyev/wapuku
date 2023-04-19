use std::collections::HashMap;
use std::marker::PhantomData;

trait PieceOfData {
    
}

pub struct UncountableProperty {
    
}

pub trait PropertyValue {

}

pub struct Range<'a, T:PartialOrd> {
    min:&'a T,
    next:Option<Box<Range<'a, T>>>
}

impl <'a, T:PartialOrd> Range<'a, T> {

    pub fn new(min: &'a T) -> Self {
        Self { min, next:None }
    }
}

pub struct Grid<'a, T:PartialOrd> {
    min:Range<'a, T>
}

impl <'a, T:PartialOrd> Grid<'a, T> {

    pub fn new(data:&'a T) -> Self {

        Self {
            min: Range::new(data),
        }
    }

    pub fn add(&mut self, data:&T) 
    where
        T:PartialOrd
    {
        
    }

}

#[cfg(test)]
mod tests {
    use crate::model::{Grid, PieceOfData, PropertyValue};
    
    struct TestItem {
        property_1:u32,
        property_2:u32,
        property_3:f32,
        property_4:f32
    }


    impl TestItem {
        pub fn new(property_1: u32, property_2: u32, property_3: f32, property_4: f32) -> Self {
            Self { property_1, property_2, property_3, property_4 }
        }
    }
    
    #[test]
    fn test_1(){
    
        let d1 = TestItem::new(1, 1, 1., 1.);
        let d2 = TestItem::new(2, 2, 2., 2.);
        let d3 = TestItem::new(3, 3, 3., 3.);
        let d4 = TestItem::new(4, 4, 4., 4.);
        
        
        let mut grid_property_1 = Grid::new(&d1.property_1);

        grid_property_1.add(&d2.property_1);
        grid_property_1.add(&d3.property_1);
        grid_property_1.add(&d4.property_1);
        
        
        
        let mut property_1_vec:Vec<(&u32, &TestItem)> = Vec::with_capacity(4);
        
        let items = vec![d1, d2, d3, d4];

        // items.iter().map(|d|(d, d.property_1))
    }

}