use std::collections::{HashMap, HashSet};
use std::marker::PhantomData;
use log::debug;
use crate::model::{Data, DataBounds, DataGroup, GroupsGrid, Named, Property, PropertyRange};

#[derive(Debug)]
pub struct VisualInstance {
    position: cgmath::Vector3<f32>,
    rotation: cgmath::Quaternion<f32>,
    name: String
}


impl VisualInstance {

    pub fn new<S: Into<String>>(position: cgmath::Vector3<f32>, rotation: cgmath::Quaternion<f32>, name: S) -> Self {
        Self {
            position, 
            rotation,
            name: name.into()
        }
    }

    #[inline]
    pub fn position(&self) -> cgmath::Vector3<f32> {
        self.position
    }

    #[inline]
    pub fn rotation(&self) -> cgmath::Quaternion<f32> {
        self.rotation
    }
   
}

impl Named for VisualInstance {
    fn get_name(&self) -> &String {
        &self.name
    }
}

pub struct VisualDataController {
    width: u32,
    height: u32,
    property_x:Box<dyn Property>,
    property_y:Box<dyn Property>,
    data:Box<dyn Data>,
    current_grid:GroupsGrid,
    visuals:Option<HashMap<String, Vec<VisualInstance>>>
}

impl VisualDataController {
    
    pub fn new(data: Box<dyn Data>, width: u32, height: u32, property_x_name: String, property_y_name: String) -> Self {
        

        let property_x = data.all_properties().into_iter().find(|p| p.name() == &property_x_name).expect(format!("property_x {} not found", property_x_name).as_str());
        let property_y = data.all_properties().into_iter().find(|p| p.name() == &property_y_name).expect(format!("property_x {} not found", property_y_name).as_str());

        let data_grid = data.group_by_2(
            PropertyRange::new (property_x,  None, None ),
            PropertyRange::new (property_y,  None, None )
        );

        let visuals:HashMap<String, Vec<VisualInstance>> = data_grid.data()
            .iter().enumerate()
            .flat_map(
                |(y, vec_x)| vec_x.iter().enumerate().map(move |(x, group)| (x, y, group))
            )
            .fold(HashMap::new(), |mut h:HashMap<String, Vec<VisualInstance>>, (x, y, group)|{
               
                let mut property_groups = h.entry(String::from("property_1")).or_insert(vec![]);

                debug!("VisualDataController::new x={}, y={}", x, y);

                property_groups.push(
                  VisualInstance::new(
                      cgmath::Vector3 { x: -5.0 + x as f32, y:  y as f32, z: 0.0 },
                      cgmath::Quaternion::new(1., 0., 0., 0.),
                      "property_1"
                  )
                );
                         
                h
        });

        Self { 
            width, height,
            property_x: property_x.clone_to_box(),
            property_y: property_y.clone_to_box(),
            data,
            current_grid: data_grid,
            visuals: Some(visuals)
        }
    }

    pub fn visuals(&mut self) -> Option<HashMap<String, Vec<VisualInstance>>> {
        self.visuals.take()
        // let mut h = HashMap::new();
        // 
        // // self.group_by_2()
        // 
        // h.insert(String::from("property_3"),
        //          vec![
        //              VisualInstance::new(
        //                  cgmath::Vector3 { x: 0.0, y: 0.0, z: 0.0 },
        //                  cgmath::Quaternion::new(1., 0., 0., 0.),
        //                  "property_3"
        //              ),
        // 
        //          ]
        // );
        // 
        // h.insert(String::from("property_1"),
        //          vec![
        //              VisualInstance::new(
        //                  cgmath::Vector3 { x: 2.0, y: 0.0, z: 0.0 },
        //                  cgmath::Quaternion::new(1., 0., 0., 0.),
        //                  "property_1"
        //              ),
        // 
        //              VisualInstance::new(
        //                  cgmath::Vector3 { x: 6.0, y: 0.0, z: 0.0 },
        //                  cgmath::Quaternion::new(1., 0., 0., 0.),
        //                  "property_1"
        //              ),
        //          ]
        // );
        // 
        // h.insert(String::from("property_2"),
        //          vec![
        //              VisualInstance::new(
        //                  cgmath::Vector3 { x: 4.0, y: 0.0, z: 0.0 },
        //                  cgmath::Quaternion::new(1., 0., 0., 0.),
        //                  "property_2"
        //              ),
        // 
        //              VisualInstance::new(
        //                  cgmath::Vector3 { x: 8.0, y: 0.0, z: 0.0 },
        //                  cgmath::Quaternion::new(1., 0., 0., 0.),
        //                  "property_2"
        //              ),
        //          ]
        // );
        // 
        // h
    }

}



#[cfg(test)]
mod tests {
    // use crate::visualization::MeshModel;

    use std::ops::Mul;
    use cgmath::{SquareMatrix, Vector4};

    #[test]
    pub fn test_build_instances(){
        // let model = MeshModel::new(); 

        let m_0 = [
            [-0.72450477,   0.0,        0.0,        0.0],
            [0.0,           2.4142134,  0.0,        0.0],
            [0.0,           0.0,        1.001001,   1.0],
            [0.0,           0.0,        9.90991,    10.0]
        ];

        let m = cgmath::Matrix4::new(
            m_0[0][ 0],  m_0[0][ 1], m_0[0][ 2], m_0[0][ 3],
            m_0[1][ 0],  m_0[1][ 1], m_0[1][ 2], m_0[1][ 3],
            m_0[2][ 0],  m_0[2][ 1], m_0[2][ 2], m_0[2][ 3],
            m_0[3][ 0],  m_0[3][ 1], m_0[3][ 2], m_0[3][ 3],
        );
            
       

        let size = Vector4::new(452.0, 107.0, -10., 1.,);

        

        let size_in_world = m.invert().map(|mi|mi.mul(size));
        // let size_in_world = m.mul(size);

        println!("m={:?} size={:?}, size_in_world={:?}", m, size, size_in_world);

        
    }
}