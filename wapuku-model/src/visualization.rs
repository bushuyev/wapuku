

pub struct Instance {
    pub(crate) position: cgmath::Vector3<f32>,
    pub(crate) rotation: cgmath::Quaternion<f32>,
}


impl Instance {

    pub fn new(position: cgmath::Vector3<f32>, rotation: cgmath::Quaternion<f32>) -> Self {
        Self { position, rotation }
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
#[cfg(test)]
mod tests {
    // use crate::visualization::MeshModel;

    #[test]
    pub fn test_build_instances(){
        // let model = MeshModel::new(); 
        println!("Ok")
    }
}