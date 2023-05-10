use std::collections::{HashMap, HashSet};
use std::marker::PhantomData;
use log::debug;
use crate::model::{Data, DataBounds, DataGroup, GroupsGrid, Named, Property, PropertyRange};

#[derive(Debug)]
pub struct VisualBounds {
    x_left_top: f32,
    y_left_top: f32,
    x_right_bottom: f32,
    y_right_bottom: f32
}

impl VisualBounds {
    
    pub fn new(x_left_top: f32, y_left_top: f32, x_right_bottom: f32, y_right_bottom: f32) -> Self {
        Self { x_left_top, y_left_top, x_right_bottom, y_right_bottom }
    }
    
    pub fn update(&mut self, x_left_top: f32, y_left_top: f32, x_right_bottom: f32, y_right_bottom: f32) {
        self.x_left_top = x_left_top;
        self.y_left_top = y_left_top;
        self.x_right_bottom = x_right_bottom;
        self.y_right_bottom = y_right_bottom;
    }
    
    pub fn contain(&self, x:f32, y:f32) -> bool {
        let b_x = self.x_left_top <= x && x <= self.x_right_bottom;
        let b_y = self.y_left_top >= y && y >= self.y_right_bottom;

        debug!("VisualBounds::contain:  b_x={} b_y={} self={:?}, x={:?}, y={:?} ", b_x, b_y, self, x, y);
        
        b_x && b_y
    }
}
impl Default for VisualBounds {
    fn default() -> Self {
        VisualBounds::new(-1., 1., 1., -1.)
    }
}

#[derive(Debug)]
pub enum VisualInstanceData {
    DataGroup(Box<dyn DataGroup>),
    Empty
}

#[derive(Debug)]
pub struct VisualInstance {
    position: cgmath::Vector3<f32>,
    rotation: cgmath::Quaternion<f32>,
    name: String,
    visual_bounds: VisualBounds,
    data:VisualInstanceData
}


impl VisualInstance {

    pub fn new<S: Into<String>>(position: cgmath::Vector3<f32>, rotation: cgmath::Quaternion<f32>, name: S, data:VisualInstanceData) -> Self {
        Self {
            position, 
            rotation,
            name: name.into(),
            visual_bounds: VisualBounds::default(),
            data
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
    
    pub fn bounds_mut(&mut self) -> &mut VisualBounds {
        &mut self.visual_bounds
    }

    pub fn bounds(&self) -> &VisualBounds {
        &self.visual_bounds
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
    visuals:Option<HashMap<String, Vec<VisualInstance>>>,
    has_updates:bool
}

impl VisualDataController {
    
    pub fn new(data: Box<dyn Data>, width: u32, height: u32, property_x_name: String, property_y_name: String) -> Self {
        

        let property_x = data.all_properties().into_iter().find(|p| p.name() == &property_x_name).expect(format!("property_x {} not found", property_x_name).as_str());
        let property_y = data.all_properties().into_iter().find(|p| p.name() == &property_y_name).expect(format!("property_x {} not found", property_y_name).as_str());

        let groups_nr_x = 3;
        let groups_nr_y = 2;
        let mut data_grid = data.group_by_2(
            PropertyRange::new (property_x,  None, None ),
            PropertyRange::new (property_y,  None, None ),
            groups_nr_x, groups_nr_y
        );

        let step = 9.;
        let d_property = step/5.;
        let min_x = ((groups_nr_x as f32 - 1.0) / -2.) * step;
        let min_y = ((groups_nr_y as f32 - 1.0) / -2.) * step;
        let plate_z = 1.0;
        let properties_z = 0.0;
        

        //TODO layout
        let visuals:HashMap<String, Vec<VisualInstance>> = data_grid.data()
            .drain(..).enumerate()
            .flat_map(
                move |(y, mut vec_x)| vec_x.drain(..).collect::<Vec<Box<dyn DataGroup>>>().into_iter().enumerate().map(move |(x, group)| (x, y, group))
            )
            .fold(HashMap::new(), move |mut h:HashMap<String, Vec<VisualInstance>>, (x, y, group)|{

                
                
                let mut plates = h.entry(String::from("plate")).or_insert(vec![]);

                let plate_x = (min_x + x as f32 * step) as f32;
                let plate_y = (min_y + y as f32 * step) as f32;

                debug!("VisualDataController::new x={}, y={}  plate_x={}, plate_y={}", x, y, plate_x, plate_y);
                
                plates.push(
                  VisualInstance::new(
                      cgmath::Vector3 { x: plate_x, y: plate_y, z: plate_z },
                      cgmath::Quaternion::new(1., 0., 0., 0.),
                      "plate",
                      VisualInstanceData::DataGroup(group)
                  )
                );

                let mut plates = h.entry(String::from("property_1")).or_insert(vec![]);

                plates.push(
                    VisualInstance::new(
                        cgmath::Vector3 { x: plate_x - d_property, y: plate_y - d_property, z: properties_z },
                        cgmath::Quaternion::new(1., 0., 0., 0.),
                        "property_1",
                        VisualInstanceData::Empty
                    )
                );

                let mut plates = h.entry(String::from("property_2")).or_insert(vec![]);

                plates.push(
                    VisualInstance::new(
                        cgmath::Vector3 { x: plate_x - d_property, y: plate_y + d_property, z: properties_z },
                        cgmath::Quaternion::new(1., 0., 0., 0.),
                        "property_2",
                        VisualInstanceData::Empty
                    )
                );

                let mut plates = h.entry(String::from("property_3")).or_insert(vec![]);

                plates.push(
                    VisualInstance::new(
                        cgmath::Vector3 { x: plate_x + d_property, y: plate_y + d_property, z: properties_z },
                        cgmath::Quaternion::new(1., 0., 0., 0.),
                        "property_2",
                        VisualInstanceData::Empty
                    )
                );

                let mut plates = h.entry(String::from("property_4")).or_insert(vec![]);

                plates.push(
                    VisualInstance::new(
                        cgmath::Vector3 { x: plate_x + d_property, y: plate_y - d_property, z: properties_z },
                        cgmath::Quaternion::new(1., 0., 0., 0.),
                        "property_2",
                        VisualInstanceData::Empty
                    )
                );
                
                h
        });

        // let mut visuals:HashMap<String, Vec<VisualInstance>> = HashMap::new();
        // 
        // visuals.insert(String::from("plate"), vec![
        //     VisualInstance::new(
        //         cgmath::Vector3 { x: 0.0, y:  0.0, z: 1.0 },
        //         cgmath::Quaternion::new(1., 0., 0., 0.),
        //         "plate",
        //         VisualInstanceData::Empty
        //     )
        // ]);
        // 
       /* visuals.insert(String::from("plate"), vec![
            VisualInstance::new(
                cgmath::Vector3 { x: -5.0, y:  0.0, z: 0.0 },
                cgmath::Quaternion::new(1., 0., 0., 0.),
                "plate",
                VisualInstanceData::Empty
            ),
            VisualInstance::new(
                cgmath::Vector3 { x: -0.0, y:  -5.0, z: 0.0 },
                cgmath::Quaternion::new(1., 0., 0., 0.),
                "plate",
                VisualInstanceData::Empty
            ),
            VisualInstance::new(
                cgmath::Vector3 { x: 0.0, y:  0.0, z: 0.0 },
                cgmath::Quaternion::new(1., 0., 0., 0.),
                "plate",
                VisualInstanceData::Empty
            ),
            VisualInstance::new(
                cgmath::Vector3 { x: 5.0, y:  0.0, z: 0.0 },
                cgmath::Quaternion::new(1., 0., 0., 0.),
                "plate",
                VisualInstanceData::Empty
            ),
           VisualInstance::new(
                cgmath::Vector3 { x: 0.0, y:  5.0, z: 0.0 },
                cgmath::Quaternion::new(1., 0., 0., 0.),
                "plate",
                VisualInstanceData::Empty
            ),
        ]);*/
        // 
        // visuals.insert(String::from("property_2"), vec![
        //     VisualInstance::new(
        //         cgmath::Vector3 { x: 1.0, y:  1.0, z: 0.0 },
        //         cgmath::Quaternion::new(1., 0., 0., 0.),
        //         "property_2",
        //         VisualInstanceData::Empty
        //     )
        // ]);
        // 
        // visuals.insert(String::from("property_3"), vec![
        //     VisualInstance::new(
        //         cgmath::Vector3 { x: -1.0, y:  -1.0, z: 0.0 },
        //         cgmath::Quaternion::new(1., 0., 0., 0.),
        //         "property_3",
        //         VisualInstanceData::Empty
        //     )
        // ]);
        // 
        // visuals.insert(String::from("property_4"), vec![
        //     VisualInstance::new(
        //         cgmath::Vector3 { x: 1.0, y:  -1.0, z: 0.0 },
        //         cgmath::Quaternion::new(1., 0., 0., 0.),
        //         "property_4",
        //         VisualInstanceData::Empty
        //     )
        // ]);

        Self { 
            width, height,
            property_x: property_x.clone_to_box(),
            property_y: property_y.clone_to_box(),
            data,
            current_grid: data_grid,
            visuals: Some(visuals),
            has_updates: true
        }
    }

    pub fn visuals(&mut self) -> Option<&HashMap<String, Vec<VisualInstance>>> {
        self.visuals.as_ref()
    }

    pub fn visuals_updates(&mut self) -> Option<&mut HashMap<String, Vec<VisualInstance>>> {
        if self.has_updates {
            self.has_updates = false;
            
            self.visuals.as_mut()
        } else {
            None
        }
        
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