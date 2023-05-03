use crate::model::{Data, Named};

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

pub trait VisualData {
    fn visuals(&self)->Vec<VisualInstance>;
}
//TODO https://github.com/rust-lang/rust/issues/70263
// impl <D: for<'a> Data<'a>> VisualData for D {
impl <'a, D:Data<'a>> VisualData for D {

    fn visuals(&self) -> Vec<VisualInstance> {

        vec![
            VisualInstance::new(
                cgmath::Vector3 { x: 0.0, y: 0.0, z: 0.0 },
                cgmath::Quaternion::new(1., 0., 0., 0.),
                "property_1"
            ),

            VisualInstance::new(
                cgmath::Vector3 { x: 2.0, y: 0.0, z: 0.0 },
                cgmath::Quaternion::new(1., 0., 0., 0.),
                "property_1"
            ),

            VisualInstance::new(
                cgmath::Vector3 { x: 3.0, y: 0.0, z: 0.0 },
                cgmath::Quaternion::new(1., 0., 0., 0.),
                "property_2"
            ),

            VisualInstance::new(
                cgmath::Vector3 { x: 4.0, y: 0.0, z: 0.0 },
                cgmath::Quaternion::new(1., 0., 0., 0.),
                "property_3"
            )
        ]
    }
}


#[cfg(test)]
mod tests {
    // use crate::visualization::MeshModel;

    #[test]
    pub fn test_build_instances(){
        // let model = MeshModel::new(); 
        println!("Ok")
    }
}