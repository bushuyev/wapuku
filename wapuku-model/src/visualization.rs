use std::collections::{HashMap, HashSet};
use std::marker::PhantomData;
use std::ops::{Div, Mul};
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, AtomicUsize, Ordering};
use cgmath::{ElementWise, MetricSpace, Quaternion, Vector3, Vector4, Zero};
use log::debug;
use crate::model::{Data, DataBounds, DataGroup, GroupsGrid, Named, Property, PropertyRange};

#[derive(Debug)]
pub struct VisualBounds {
    x_left_top: f32,
    y_left_top: f32,
    x_right_bottom: f32,
    y_right_bottom: f32,
}

pub const V_LEFT_TOP: Vector4<f32> = Vector4::new(-3., 3., 0., 1.);
pub const V_RIGHT_BOTTOM: Vector4<f32> = Vector4::new(3., -3., 0., 1.);

trait Animation {
    fn tick(&mut self, visual_instance: &mut VisualInstance) -> AnimationState;
}

#[derive(PartialEq)]
enum AnimationState {
    Running,
    Done,
}



struct ScaleDown {
    target: f32,
    d: f32,
    what_to_scale: Box<dyn Fn(&mut VisualInstance)->&mut f32>
}

impl ScaleDown {

    pub fn new(target: f32, d:f32, what_to_scale:impl Fn(&mut VisualInstance)->&mut f32 + 'static) -> Self {
        Self { 
            target, 
            d,
            what_to_scale: Box::new(what_to_scale)
        }
    }
}

impl Animation for ScaleDown {

    fn tick(&mut self, visual_instance: &mut VisualInstance) -> AnimationState {

        *(self.what_to_scale)(visual_instance) -= self.d;

        
        if *(self.what_to_scale)(visual_instance) <= self.target {
            AnimationState::Done
        } else {
            AnimationState::Running
        }
    }
}


struct SimultaneousAnimations {
    animations:Vec<Box<dyn Animation>>
}

impl SimultaneousAnimations {
    pub fn new(animations: Vec<Box<dyn Animation>>) -> Self {
        Self { animations }
    }
}

impl Animation for SimultaneousAnimations {
    fn tick(&mut self, visual_instance: &mut VisualInstance) -> AnimationState {
       let mut state  = AnimationState::Done;

       for animation in self.animations.iter_mut() {
           if animation.tick(visual_instance) == AnimationState::Running {
               state = AnimationState::Running;
           }
       }

       state
    }
}

struct ConsecutiveAnimations {
    animations:Vec<Box<dyn Animation>>
}

impl ConsecutiveAnimations {
    pub fn new(animations: Vec<Box<dyn Animation>>) -> Self {
        let mut animations = animations;
        animations.reverse();

        Self { 
            animations
        }
    }
}


impl Animation for ConsecutiveAnimations {
    fn tick(&mut self, visual_instance: &mut VisualInstance) -> AnimationState {
        if let Some(mut animation) = self.animations.pop() {
            animation.tick(visual_instance)

        } else {
            AnimationState::Done
        }
    }
}



#[cfg(test)]
mod animation_tests {
    use cgmath::{Quaternion, Vector3, Zero};
    use crate::visualization::{Animation, MoveTo, ScaleDown, VisualInstance, VisualInstanceData};

    #[test]
    fn test_scale_xy() {
        // let scale_down = ScaleDown::new(Vector3::new())
        let mut scale_down = ScaleDown::new(0.0, 0.01, |v:&mut VisualInstance| &mut v.scale.x);
        let mut vi = VisualInstance::new(Vector3::zero(), Quaternion::zero(), "tst", VisualInstanceData::Empty);

        scale_down.tick(&mut vi);
        
        assert_eq!(vi.scale.x, 0.99);
    }
    
    #[test]
    fn test_move_from_to(){
        let mut vi = VisualInstance::new(Vector3::new(10., 10., 10.), Quaternion::zero(), "tst", VisualInstanceData::Empty);
        let mut move_to = MoveTo::from_to_in_steps(vi.position, Vector3::new(110., 110., 110.), 10);

        move_to.tick(&mut vi);

        assert_eq!(vi.position.x, 20.0);
    }
}

struct MoveTo {
    target: Vector3<f32>,
    d: Vector3<f32>,
}


impl MoveTo {

    pub fn from_to_in_steps(from:Vector3<f32>, to:Vector3<f32>, steps:u32) -> Self {
        debug!("MoveTo::from_to_in_steps: from={:?}, to={:?}, steps={:?}", from, to, steps);

        Self {
            target: to,
            d: (to - from).div(steps as f32),
        }
    }
}

impl Animation for MoveTo {
    fn tick(&mut self, visual_instance: &mut VisualInstance) -> AnimationState {
        visual_instance.position += self.d;

        if visual_instance.position.distance2(self.target) < 0.1 {
            AnimationState::Done
        } else {
            AnimationState::Running
        }
    }
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

    pub fn contain(&self, x: f32, y: f32) -> bool {
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
    Empty,
}

#[derive(Debug)]
pub struct VisualInstance {
    id:u32,
    position: Vector3<f32>,
    rotation: Quaternion<f32>,
    scale: Vector3<f32>,
    name: String,
    visual_bounds: VisualBounds,
    data: VisualInstanceData,
}

static VISUAL_INSTANCE_ID: AtomicU32 = AtomicU32::new(0);

fn get_next_id() -> u32 {
    let v = VISUAL_INSTANCE_ID.load(Ordering::Relaxed) + 1;
    VISUAL_INSTANCE_ID.store( v, Ordering::Relaxed);

    v
}

impl VisualInstance {

    pub fn new<S: Into<String>>(position: Vector3<f32>, rotation: Quaternion<f32>, name: S, data: VisualInstanceData) -> Self {

        Self {
            id: get_next_id(),
            position,
            rotation,
            scale: Vector3::new(1., 1., 1.),
            name: name.into(),
            visual_bounds: VisualBounds::default(),
            data,
        }
    }

    #[inline]
    pub fn position(&self) -> Vector3<f32> {
        self.position
    }

    #[inline]
    pub fn rotation(&self) -> Quaternion<f32> {
        self.rotation
    }

    pub fn bounds_mut(&mut self) -> &mut VisualBounds {
        &mut self.visual_bounds
    }

    #[inline]
    pub fn bounds(&self) -> &VisualBounds {
        &self.visual_bounds
    }

    #[inline]
    pub fn scale(&self) -> &Vector3<f32> {
        &self.scale
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn set_scale(&mut self, x: Option<f32>, y: Option<f32>, z: Option<f32>) {
        self.scale.x = x.unwrap_or(self.scale.x);
        self.scale.y = y.unwrap_or(self.scale.y);
        self.scale.z = z.unwrap_or(self.scale.z);
    }
 
}

impl Named for VisualInstance {
    fn get_name(&self) -> &String {
        &self.name
    }
}

pub struct VisualDataController {
    property_x: Box<dyn Property>,
    property_y: Box<dyn Property>,
    data: Box<dyn Data>,
    current_grid: GroupsGrid,
    visuals: Option<HashMap<String, Vec<VisualInstance>>>,
    has_updates: bool,
    animations: HashMap<u32, Box<dyn Animation>>,
    width:i32, height:i32
}

impl VisualDataController {

    pub fn new(data: Box<dyn Data>, property_x_name: String, property_y_name: String, width:i32, height:i32) -> Self {
        let property_x = data.all_properties().into_iter().find(|p| p.name() == &property_x_name).expect(format!("property_x {} not found", property_x_name).as_str());
        let property_y = data.all_properties().into_iter().find(|p| p.name() == &property_y_name).expect(format!("property_x {} not found", property_y_name).as_str());

        let groups_nr_x = 3;
        let groups_nr_y = 2;
        let mut data_grid = data.group_by_2(
            PropertyRange::new(property_x, None, None),
            PropertyRange::new(property_y, None, None),
            groups_nr_x, groups_nr_y,
        );

        let step = 9.;
        let d_property = step / 5.;
        let min_x = ((groups_nr_x as f32 - 1.0) / -2.) * step;
        let min_y = ((groups_nr_y as f32 - 1.0) / 2.) * step;
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
                let plate_y = (min_y - y as f32 * step) as f32;

                debug!("VisualDataController::new x={}, y={}  plate_x={}, plate_y={}", x, y, plate_x, plate_y);

                plates.push(
                  VisualInstance::new(
                      cgmath::Vector3 { x: plate_x, y: plate_y, z: plate_z },
                      cgmath::Quaternion::new(1., 0., 0., 0.),
                      format!("plate: x={} y={}", x, y),
                      VisualInstanceData::DataGroup(group)
                  )
                );

               /* let mut plates = h.entry(String::from("property_1")).or_insert(vec![]);
                
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
                        "property_3",
                        VisualInstanceData::Empty
                    )
                );
                
                let mut plates = h.entry(String::from("property_4")).or_insert(vec![]);
                
                plates.push(
                    VisualInstance::new(
                        cgmath::Vector3 { x: plate_x + d_property, y: plate_y - d_property, z: properties_z },
                        cgmath::Quaternion::new(1., 0., 0., 0.),
                        "property_4",
                        VisualInstanceData::Empty
                    )
                );*/
                
                h
        });

        // let mut visuals:HashMap<String, Vec<VisualInstance>> = HashMap::new();
        
        // visuals.insert(String::from("plate"), vec![
        //     VisualInstance::new(
        //         cgmath::Vector3 { x: 5.0, y:  0.0, z: 1.0 },
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
        // "property_1" => "Sphere",
        // "property_2" => "Cone",
        // "property_3" => "Cube",
        // "property_4" => "Cylinder",
        // visuals.insert(String::from("property_1"), vec![
        //     VisualInstance::new(
        //         cgmath::Vector3 { x: -5.0, y:  5.0, z: 0.0 },
        //         cgmath::Quaternion::new(1., 0., 0., 0.),
        //         "property_1",
        //         VisualInstanceData::Empty
        //     )
        // ]);
        // 
        // visuals.insert(String::from("property_2"), vec![
        //     VisualInstance::new(
        //         cgmath::Vector3 { x: -5.0, y:  -5.0, z: 0.0 },
        //         cgmath::Quaternion::new(1., 0., 0., 0.),
        //         "property_2",
        //         VisualInstanceData::Empty
        //     )
        // ]);
        // 
        // visuals.insert(String::from("property_3"), vec![
        //     VisualInstance::new(
        //         cgmath::Vector3 { x: 5.0, y:  5.0, z: 0.0 },
        //         cgmath::Quaternion::new(1., 0., 0., 0.),
        //         "property_3",
        //         VisualInstanceData::Empty
        //     )
        // ]);
        // 
        // visuals.insert(String::from("property_4"), vec![
        //     VisualInstance::new(
        //         cgmath::Vector3 { x: 5.0, y:  -5.0, z: 0.0 },
        //         cgmath::Quaternion::new(1., 0., 0., 0.),
        //         "property_4",
        //         VisualInstanceData::Empty
        //     )
        // ]);

        Self { 
            property_x: property_x.clone_to_box(),
            property_y: property_y.clone_to_box(),
            data,
            current_grid: data_grid,
            visuals: Some(visuals),
            has_updates: true,
            animations: HashMap::new(),
            width, height
        }
    }

    pub fn visuals(&mut self) -> Option<&HashMap<String, Vec<VisualInstance>>> {
        self.visuals.as_ref()
    }

    pub fn visuals_updates(&mut self) -> Option<&mut HashMap<String, Vec<VisualInstance>>> {
        // for visual_instance in visuals.and_then(|visuals| visuals.values_mut().flat_map(|visuals_vec|visuals_vec.iter_mut())) {
        //     if let Some(animation) = self.animations.get(visual_instance.id) {
        // 
        //     }
        // }

        if let Some(visuals) = self.visuals.as_mut().map(|visuals| visuals.values_mut().flat_map(|visuals_vec|visuals_vec.iter_mut())) {
            for visual_instance in visuals {
                debug!("visuals_updates: visual_instance={:?}", visual_instance);
                if let Some(animation) = self.animations.get_mut(&visual_instance.id) {
                    self.has_updates = true;
                    debug!("visuals_updates: animation for id={:?}", visual_instance.id);
                    
                    if animation.tick(visual_instance) == AnimationState::Done {
                        self.animations.remove(&visual_instance.id);

                        debug!("visuals_updates: removed animation for id={:?}", visual_instance.id);
                    }
                }
            }
        }
        
        if self.has_updates {
            self.has_updates = false;
            
            self.visuals.as_mut()
        } else {
            None
        }
    }

    pub fn on_pointer_moved(&mut self, x:f32, y:f32){
        debug!("on_pointer_moved: x={}, y={}", x, y);

        if let Some(visuals_iter_mut) = Self::flat_visuals(self.visuals.as_mut()) {
            for visual in visuals_iter_mut {
                if visual.bounds().contain(x, y) {
                    visual.set_scale(Some(1.1), Some(1.1), None);
                } else {
                    visual.set_scale(Some(1.0), Some(1.0), None);
                }
            }

            self.has_updates = true;
        }
    }

    fn flat_visuals(visuals: Option<&mut HashMap<String, Vec<VisualInstance>>>) -> Option<impl Iterator<Item = &mut VisualInstance>> {
        visuals.map(|visuals|
            visuals
                .values_mut()
                .flat_map(|visuals_vec| visuals_vec.iter_mut())
        )
    }

    fn find_group_by_xy(x: f32, y: f32, visuals: Option<&mut HashMap<String, Vec<VisualInstance>>>, on_each: impl FnMut(&mut VisualInstance) -> &mut VisualInstance) -> Option<&mut VisualInstance> {

        Self::flat_visuals(visuals).and_then(|mut visuals_iter|visuals_iter.find(|v| v.bounds().contain(x, y)))
    }

    pub fn on_pointer_input(&mut self, x: f32, y: f32) {

        if let Some(visual_under_pointer) = Self::flat_visuals(self.visuals.as_mut()).and_then(|mut visuals_iter_mut|visuals_iter_mut.find(|v|v.bounds().contain(x, y))) {
            
            if let Some(visuals_iter_mut) = Self::flat_visuals(self.visuals.as_mut()) {
                for visual in visuals_iter_mut {
                    if visual.bounds().contain(x, y) {
                        self.animations.insert(visual.id, Box::new(MoveTo::from_to_in_steps(visual.position, Vector3::new(0.0, 0.0, visual.position.z), 100)));

                    } else {
                        

                        self.animations.insert(
                            visual.id,
                            Box::new(SimultaneousAnimations::new(vec![
                                Box::new(ScaleDown::new(0.0, 0.01, |v:&mut VisualInstance| &mut v.scale.y)),
                                Box::new(ScaleDown::new(0.0, 0.01, |v:&mut VisualInstance| &mut v.scale.x))
                            ]))
                        );

                    }
                }
            }
        } 
    }
}



#[cfg(test)]
mod tests {
    // use crate::visualization::MeshModel;

    use std::ops::Mul;
    use cgmath::{SquareMatrix, Vector4};

    #[test]
    pub fn test_build_instances() {
        // let model = MeshModel::new(); 

        let m_0 = [
            [-0.72450477, 0.0, 0.0, 0.0],
            [0.0, 2.4142134, 0.0, 0.0],
            [0.0, 0.0, 1.001001, 1.0],
            [0.0, 0.0, 9.90991, 10.0]
        ];

        let m = cgmath::Matrix4::new(
            m_0[0][0], m_0[0][1], m_0[0][2], m_0[0][3],
            m_0[1][0], m_0[1][1], m_0[1][2], m_0[1][3],
            m_0[2][0], m_0[2][1], m_0[2][2], m_0[2][3],
            m_0[3][0], m_0[3][1], m_0[3][2], m_0[3][3],
        );


        let size = Vector4::new(452.0, 107.0, -10., 1.);


        let size_in_world = m.invert().map(|mi| mi.mul(size));
        // let size_in_world = m.mul(size);

        println!("m={:?} size={:?}, size_in_world={:?}", m, size, size_in_world);
    }
}