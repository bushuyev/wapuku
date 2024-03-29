//moved from wapuku-ui to run tests
use std::collections::{HashMap, HashSet};
use std::f32::consts::PI;
use std::fmt::Debug;
use std::ops::{Add, AddAssign, Div, Sub};
use std::sync::atomic::{AtomicU32, Ordering};

use cgmath::{InnerSpace, MetricSpace, Quaternion, Vector2, Vector3, Vector4, Zero};
use log::debug;

use wapuku_model::model::{Data, DataGroup, GroupsGrid, Named, Property};

#[derive(Debug)]
pub struct VisualBounds {
    x_left_top: f32,//TODO to Vector3
    y_left_top: f32,
    x_right_bottom: f32,
    y_right_bottom: f32,
}

pub const V_LEFT_TOP: Vector4<f32> = Vector4::new(-3., 3., 0., 1.);
pub const V_RIGHT_BOTTOM: Vector4<f32> = Vector4::new(3., -3., 0., 1.);

trait Animation {
    type V;
    fn tick(&mut self, visual_instance: &mut Self::V) -> AnimationState;
}

#[derive(PartialEq, Debug)]
enum AnimationState {
    Running,
    Done,
}


struct SimultaneousAnimations<V> {
    animations:Vec<Box<dyn Animation<V=V>>>
}

impl <V> SimultaneousAnimations<V> {
    pub fn new(animations: Vec<Box<dyn Animation<V=V>>>) -> Self {
        Self { animations }
    }
}

impl <V> Animation for SimultaneousAnimations<V> {
    type V = V;

    fn tick(&mut self, visual_instance: &mut V) -> AnimationState {
       let mut state  = AnimationState::Done;

       for animation in self.animations.iter_mut() {
           if animation.tick(visual_instance) == AnimationState::Running {
               state = AnimationState::Running;
           }
       }

       state
    }
}

struct ConsecutiveAnimations<V> {
    animations:Vec<Box<dyn Animation<V=V>>>
}

impl <V> ConsecutiveAnimations<V> {
    pub fn new(animations: Vec<Box<dyn Animation<V=V>>>) -> Self {
        let mut animations = animations;
        animations.reverse();

        Self { 
            animations
        }
    }
}


impl <V> Animation for ConsecutiveAnimations<V> {
    type V = V;

    fn tick(&mut self, visual_instance: &mut V) -> AnimationState {
        debug!("wapuku: Animation for ConsecutiveAnimations:tick self.animations.len()={:?}", self.animations.len());
        
        if let Some(animation) = self.animations.last_mut() {

            match animation.tick(visual_instance) {
                AnimationState::Running => {
                    AnimationState::Running
                }
                AnimationState::Done => {
                    self.animations.pop();

                    if self.animations.is_empty() {
                        AnimationState::Done
                    } else {
                        AnimationState::Running
                    }
                }
            }

        } else {
            
            AnimationState::Done
        }
    }
}



#[cfg(test)]
mod animation_tests {
    use cgmath::{Quaternion, Vector3, Zero};

    use wapuku_ui::visualization::{Animation, AnimationState, ConsecutiveAnimations, Lerp, VisualInstance, VisualInstanceData};

    use crate::visualization::{Animation, AnimationState, Lerp, VisualInstance, VisualInstanceData};

    #[test]
    fn test_scale_xy() {
        // let scale_down = ScaleDown::new(Vector3::new())
        let mut scale_down = Lerp::from_to_in_steps(1.0, 0.98, |v: &VisualInstance| &v.scale.x, |v: &mut VisualInstance, x| v.scale.x = x, 2, 0.001);
        let mut vi = VisualInstance::new(Vector3::zero(), Quaternion::zero(), "tst", VisualInstanceData::Empty);

        let state = scale_down.tick(&mut vi);
        
        assert_eq!(vi.scale.x, 0.99);
        assert_eq!(AnimationState::Running, state);

        let state = scale_down.tick(&mut vi);

        assert_eq!(vi.scale.x, 0.98);
        assert_eq!(AnimationState::Done, state);
    }

    #[test]
    fn test_scale_v3() {
        // let scale_down = ScaleDown::new(Vector3::new())
        let scale_111 = Vector3::new(1., 1., 1.);
        let mut scale_down = Lerp::from_to_in_steps(
            scale_111,
            Vector3::zero(),
            VisualInstance::scale,
            VisualInstance::set_scale,
            100, 0.001
        );
        let mut vi = VisualInstance::new(Vector3::zero(), Quaternion::zero(), "tst", VisualInstanceData::Empty);
        assert_eq!(vi.scale, scale_111);
        
        let mut state = None;
        (0..100).for_each(|i|{
            state = Some(scale_down.tick(&mut vi));
        });

        assert_eq!(vi.scale, Vector3::zero());
        assert_eq!(state, Some(AnimationState::Done));
    }
    
    #[test]
    fn test_move_from_to(){
        let base = Vector3::new(10., 10., 10.);
        let mut vi = VisualInstance::new(base, Quaternion::zero(), "tst", VisualInstanceData::Empty);
        let target = Vector3::new(110., 110., 110.);
        let mut move_to = Lerp::from_to_in_steps(vi.position, target, |v: &VisualInstance| &v.position, |v: &mut VisualInstance, p| v.position = p, 10, 0.1);

        (0..10).for_each(|i|{
            move_to.tick(&mut vi);
        });

        assert_eq!(vi.position, target);

        let mut move_to = Lerp::from_to_in_steps(target, base, |v: &VisualInstance| &v.position, |v: &mut VisualInstance, p| v.position = p, 10, 0.1);

        (0..10).for_each(|i|{
            move_to.tick(&mut vi);
        });

        assert_eq!(vi.position, base);
    }

    #[test]
    fn test_consecutive_animations(){
        let mut vi = VisualInstance::new(Vector3::new(10., 10., 10.), Quaternion::zero(), "tst", VisualInstanceData::Empty);

        let target = Vector3::new(210., 210., 210.);
        let mut animations = ConsecutiveAnimations::new( 
            vec![
                Box::new(Lerp::from_to_in_steps(vi.position,                              Vector3::new(110., 110., 110.), |v: &VisualInstance| &v.position, |v: &mut VisualInstance, p| v.position = p, 10, 0.1)),
                Box::new(Lerp::from_to_in_steps(Vector3::new(110., 110., 110.), target, |v: &VisualInstance| &v.position, |v: &mut VisualInstance, p| v.position = p, 10, 0.1)),
            ]
        );

        (0..19).for_each(|i|{
            let state = animations.tick(&mut vi);
            assert_eq!(AnimationState::Running, state);
        });

        let state = animations.tick(&mut vi);
        assert_eq!(AnimationState::Done, state);
        assert_eq!(vi.position, target);
    }
    
}

trait Lerpable {
    type T;
    fn in_steps(&self, steps:u32) -> Self::T;
    fn is_done(&self, to:Self::T, d:Self::T, e:f32) -> bool;
}


impl Lerpable for f32 {
    type T = f32;
    
    fn in_steps(&self, steps:u32) -> f32 {
        self/ (steps as f32)
    }
    
    fn is_done(&self, to:f32, d:f32,  _e:f32) -> bool {
        if d > 0. {
            self >= &to
        } else {
            self <= &to
        }
    }
}

impl Lerpable for Vector3<f32> {
    type T = Vector3<f32>;

    fn in_steps(&self, steps:u32) -> Vector3<f32> {
        self.div(steps as f32)
    }

    fn is_done(&self, to:Vector3<f32>, d:Vector3<f32>, e:f32) -> bool {
        let dot = (to - *self).dot(d);
        let x = dot <= 0.;
        if !x {
            debug!("wapuku: Lerpable for Vector3: is_done, self={:?} to={:?} self.distance2(to)={:?} e={:?} dot={:?}", self, to, self.distance2(to), e, dot);
        }
        x
    }
}

struct Lerp<T: Add<Output=T> + AddAssign + Sub<Output=T> + Copy + Debug + Lerpable<T=T>, V> {
    to: T,
    d: T,
    getter: Box<dyn Fn(&V)->&T>,
    setter: Box<dyn Fn(&mut V, T)>,
    e: f32
}

impl <T:Add<Output=T> + AddAssign + Sub<Output=T>  + Copy + Debug + Lerpable<T=T>, V> Lerp<T, V> {

    pub fn from_to_in_steps(//TODO pass V and delay instantiation
        from: T, 
        to: T,
        getter:impl Fn(&V)->&T + 'static,
        setter:impl Fn(&mut V, T) + 'static, 
        steps: u32, 
        e:f32
    ) -> Self {
        

        let d = (to - (from)).in_steps(steps);

        debug!("wapuku: ScaleDown::from_to_in_steps: from={:?}, to={:?}, steps={:?},  d={:?}, e={:?}", from, to, steps, d, e);
        
        Self {
            to,
            d,
            getter: Box::new(getter),
            setter: Box::new(setter),
            e
        }
    }
}

#[cfg(test)]
mod test_lerpables {
    use cgmath::Vector3;

    use wapuku_ui::visualization::Lerpable;

    use crate::visualization::Lerpable;

    #[test]
    fn test_f32(){
        let to:f32 = 1.0;
        let d:f32 = 0.1;
        
        assert_eq!(false, (-1.5f32).is_done(to, d, 0.01));
        assert_eq!(false, (-1.0f32).is_done(to, d, 0.01));
        assert_eq!(false, 0.5f32.is_done(to, d, 0.01));
        assert_eq!(true, 1.0f32.is_done(to, d, 0.01));
        assert_eq!(true, 1.5f32.is_done(to, d, 0.01));

        let to:f32 = -1.0;
        let d:f32 = -0.1;

        assert_eq!(false, 1.0f32.is_done(to, d, 0.01));
        assert_eq!(false, 0.5f32.is_done(to, d, 0.01));
        assert_eq!(false, (-0.5f32).is_done(to, d, 0.01));
        assert_eq!(true, (-1.0f32).is_done(to, d, 0.01));
        assert_eq!(true, (-1.5f32).is_done(to, d, 0.01));
    }

    #[test]
    fn test_vectorf32(){
        let to = Vector3::new(1., 0., 0.);
        let d = Vector3::new(0.1, 0.1, 0.1);

        assert_eq!(false, Vector3::new(-1.5f32, 0., 0.).is_done(to, d, 0.01));
        assert_eq!(false, Vector3::new(-1.0f32, 0., 0.).is_done(to, d, 0.01));
        assert_eq!(false, Vector3::new(0.5f32, 0., 0.).is_done(to, d, 0.01));
        assert_eq!(true, Vector3::new(1.0f32, 0., 0.).is_done(to, d, 0.01));
        assert_eq!(true, Vector3::new(1.5f32, 0., 0.).is_done(to, d, 0.01));


        let to = Vector3::new(-1.0, 0., 0.);
        let d = Vector3::new(-0.1, 0., 0.);

        assert_eq!(false, Vector3::new(1.0f32, 0., 0.).is_done(to, d, 0.01));
        assert_eq!(false, Vector3::new(0.5f32, 0., 0.).is_done(to, d, 0.01));
        assert_eq!(false, Vector3::new(-0.5f32, 0., 0.).is_done(to, d, 0.01));
        assert_eq!(true, Vector3::new(-1.0f32, 0., 0.).is_done(to, d, 0.01));
        assert_eq!(true, Vector3::new(-1.5f32, 0., 0.).is_done(to, d, 0.01));
    }
}

impl <T:Add<Output=T> + AddAssign + Sub<Output=T> + Copy + Debug + Lerpable<T=T>, V> Animation for Lerp<T, V> {
    type V = V;

    fn tick(&mut self, visual_instance: &mut V) -> AnimationState {


        if (self.getter)(visual_instance).is_done(self.to, self.d, self.e) {
            AnimationState::Done

        } else {
            (self.setter)(visual_instance, *(self.getter)(visual_instance) + self.d);

            if (self.getter)(visual_instance).is_done(self.to, self.d, self.e) {
                (self.setter)(visual_instance, self.to);

                AnimationState::Done
            } else {
                AnimationState::Running
            }
        }

    }
}


impl VisualBounds {

    pub fn new(x_left_top: f32, y_left_top: f32, x_right_bottom: f32, y_right_bottom: f32) -> Self {
        let bounds = Self {
            x_left_top,
            y_left_top,
            x_right_bottom,
            y_right_bottom
        };
        debug!("wapuku: VisualBounds::new {:?}", bounds);
        bounds
    }

    fn from_width_heigh(width:f32, height:f32) -> Self {
        VisualBounds::new(-width / 2., height / 2., width / 2., -height / 2.)
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

        debug!("wapuku: VisualBounds::contain:  b_x={} b_y={} self={:?}, x={:?}, y={:?} ", b_x, b_y, self, x, y);

        b_x && b_y
    }

    // fn move_to(&mut self, p:&Vector3<f32>) {
    //     self.x_right_bottom += p.x;
    //     self.x_left_top += p.x;
    // 
    //     self.y_right_bottom += p.y;
    //     self.y_left_top += p.y;
    // }
    
    fn center(&self) -> Vector2<f32> {
        Vector2::new(
            self.x_left_top - (self.x_left_top - self.x_right_bottom)/2.,
            self.y_left_top - (self.y_left_top - self.y_right_bottom)/2.
        )
    }

    fn width_hgith(&self) -> Vector2<f32> {
        Vector2::new(
             self.x_right_bottom - self.x_left_top,
            self.y_left_top - self.y_right_bottom
        )
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
pub enum ChildrenLayout {
    Circle,
    Line
}


impl ChildrenLayout {

    fn layout<'a> (&self, mut positions:Vec<&'a mut Vector3<f32>>, bounds:&VisualBounds){
        
        match self {
            ChildrenLayout::Circle => {
                let center = bounds.center();
                let r = bounds.width_hgith().x / 2.;
                
                debug!("wapuku: ChildrenLayout::Circle:layout center={:?}, r={:?}", center, r);
                
                if positions.len() == 1 {
                    let position_0 = positions.get_mut(0).unwrap();
                    position_0.x = center.x;
                    position_0.y = center.y;
                    
                } else {
                    let step = 2. * PI /  (positions.len() as f32);
                    let mut φ:f32 = 0.;

                    positions.into_iter().for_each(|p|{
                        p.x = r * φ.cos();
                        p.y = r * φ.sin();

                        φ += step;
                    });
                    
                }
            }

            ChildrenLayout::Line => {
                let mut x = bounds.x_left_top;
                let     y = bounds.y_left_top - (bounds.y_left_top - bounds.y_right_bottom)/2.;
                
                let step = (bounds.x_right_bottom - bounds.x_left_top) / (positions.len() as f32);

                positions.into_iter().for_each(|p|{
                    debug!("wapuku: ChildrenLayout::Line:layout x={}", x);
                    p.x = x;
                    p.y = y;
                    x += step;
                });
                
            }
        }
    }
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
    children: Option<Vec<Box<VisualInstance>>>,
    #[allow(dead_code)]
    children_layout:Option<ChildrenLayout>
}

static VISUAL_INSTANCE_ID: AtomicU32 = AtomicU32::new(0);

fn get_next_id() -> u32 {
    let v = VISUAL_INSTANCE_ID.load(Ordering::Relaxed) + 1;
    VISUAL_INSTANCE_ID.store( v, Ordering::Relaxed);

    v
}
const DEFAULT_LAYOUT:ChildrenLayout = ChildrenLayout::Line;

impl VisualInstance {

    pub fn new<S: Into<String>>(position: Vector3<f32>, rotation: Quaternion<f32>, name: S, data: VisualInstanceData) -> Self {
        Self::_new_with_children(position, rotation, name, data, None, None, VisualBounds::default())
    }


    pub fn new_with_children<S: Into<String>>(position: Vector3<f32>, rotation: Quaternion<f32>, name: S, data: VisualInstanceData, children:Vec<Box<VisualInstance>>, children_layout:ChildrenLayout, bounds:(f32, f32)) -> Self {
        let bounds = VisualBounds::from_width_heigh(bounds.0, bounds.1);
        // bounds.move_to(&position);

        Self::_new_with_children(position, rotation, name, data, Some(children), Some(children_layout), bounds)
    }
    
    fn _new_with_children<S: Into<String>>(position: Vector3<f32>, rotation: Quaternion<f32>, name: S, data: VisualInstanceData, children_op:Option<Vec<Box<VisualInstance>>>, children_layout: Option<ChildrenLayout>, visual_bounds: VisualBounds) -> Self {
        let mut children_op_mut = children_op;

        if let Some(children)  = children_op_mut.as_mut() {
            children_layout.as_ref().unwrap_or(&DEFAULT_LAYOUT).layout(  children.iter_mut().map(|vi|vi.position_mut()).collect(), &visual_bounds);
            children.iter_mut().for_each(|c|{
                c.position.x += position.x;
                c.position.y += position.y;
            })
        }

        Self {
            id: get_next_id(),
            position,
            rotation,
            scale: Vector3::new(1., 1., 1.),
            name: name.into(),
            visual_bounds,//TODO here relative, will be updated to screen in State::update
            data,
            children: children_op_mut,
            children_layout
        }
    }

    #[inline]
    pub fn position(&self) -> &Vector3<f32> {
        &self.position
    }

    #[inline]
    pub fn set_position(&mut self, position:Vector3<f32>) {
        let d_position = position - self.position;
        
        self.children.as_mut().map(|children|children.iter_mut().for_each(|child|{
            child.set_position(child.position() + d_position);
        }));
        
        self.position = position
        
    }

    #[inline]
    pub fn position_mut(&mut self) -> &mut Vector3<f32> {
        &mut self.position
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

    pub fn set_scale(&mut self, scale:Vector3<f32>) {
        self.scale = scale;

        self.children.as_mut().map(|children|children.iter_mut().for_each(|child|{
            child.scale = self.scale
        }));
    }
 
    pub fn with_children(&self)->Vec<&VisualInstance> {
        let mut with_children = vec![];
        with_children.push(self);

        if let Some(children) = self.children.as_ref() {
            for child in children.iter() {
                with_children.push(&*child);
            }
        }

        with_children
    }
}

impl Named for VisualInstance {
    fn get_name(&self) -> &String {
        &self.name
    }
}

pub struct VisualDataController {
    //TODO pubs
    pub(crate) property_x: Box<dyn Property>,
    pub(crate) property_y: Box<dyn Property>,
    pub(crate) groups_nr_x: u8,
    pub(crate) groups_nr_y: u8,
    // data: Box<dyn Data>,
    visuals: Vec<VisualInstance>,
    visual_id_under_pointer_op: Option<u32>,
    has_updates: bool,
    animations: HashMap<u32, Box<dyn Animation<V=VisualInstance>>>
}

impl VisualDataController {
    

    pub fn new(data: &dyn Data, _width:i32, _height:i32) -> Self {
        let all_properties:HashSet<&dyn Property> = data.all_properties();


        let (property_1, property_2, _property_3) = {
            let mut all_properties_iter = all_properties.into_iter().collect::<Vec<&dyn Property>>();
            all_properties_iter.sort_by(|p1, p2| p1.name().cmp(p2.name()));

            (*all_properties_iter.get(0).expect("property_1"), *all_properties_iter.get(1).expect("property_2"), *all_properties_iter.get(2).expect("property_3"))
        };

        let property_x_name: String = property_1.name().clone();
        let property_y_name: String = property_2.name().clone();

        debug!("wapuku: property_x={} property_y={}",  property_x_name, property_y_name);
        
        let property_x = data.all_properties().into_iter().find(|p| p.name() == &property_x_name).expect(format!("property_x {} not found", property_x_name).as_str());
        let property_y = data.all_properties().into_iter().find(|p| p.name() == &property_y_name).expect(format!("property_y {} not found", property_y_name).as_str());


        Self { 
            property_x: property_x.clone_to_box(),
            property_y: property_y.clone_to_box(),
            groups_nr_x: 3,
            groups_nr_y: 3,
            // data,
            visuals:vec![],
            has_updates: true,
            animations: HashMap::new(),
            visual_id_under_pointer_op: None,
        }
    }

    pub fn update_visuals(&mut self, data_grid: GroupsGrid) {
        // let mut data_grid = data.build_grid(
        //     PropertyRange::new(&*self.property_x, None, None),
        //     PropertyRange::new(&*self.property_y, None, None),
        //     self.groups_nr_x, self.groups_nr_y, "property_3",//TODO
        // );
        
        // let mut data_grid = GroupsGrid::new(
        //     self.property_x.clone_to_box(),
        //     self.property_y.clone_to_box(),
        //     vec![]
        // );


        let step = 9.;
        let d_property = step / 5.;
        let min_x = ((self.groups_nr_x as f32 - 1.0) / -2.) * step;
        let min_y = ((self.groups_nr_y as f32 - 1.0) / 2.) * step;
        let plate_z = 1.0;
        let _properties_z = 0.0;

        debug!("wapuku: VisualDataController::new: data_grid={:?}", data_grid);

        //TODO layout
        let visuals: Vec<VisualInstance> = data_grid.data()
            .drain(..).enumerate()
            .flat_map(
                move |(y, mut vec_x)| vec_x.drain(..).collect::<Vec<Option<Box<dyn DataGroup>>>>().into_iter().enumerate().map(move |(x, group)| (x, y, group))
            )
            .fold(vec![], move |mut h: Vec<VisualInstance>, (x, y, group)| {
                // if group.volume() > 0 {
                //     return h;
                // }


                let plate_x = (min_x + x as f32 * step) as f32;
                let plate_y = (min_y - y as f32 * step) as f32;

                debug!("VisualDataController::new x={}, y={}  plate_x={}, plate_y={}", x, y, plate_x, plate_y);

                if let Some(group) = group {
                    h.push(
                        VisualInstance::new_with_children(
                            cgmath::Vector3 { x: plate_x, y: plate_y, z: plate_z },
                            cgmath::Quaternion::new(1., 0., 0., 0.),
                            // format!("plate: x={} y={}", x, y),
                            format!("plate"),
                            VisualInstanceData::DataGroup(group),
                            vec![
                                Box::new(VisualInstance::new(
                                    // cgmath::Vector3 { x: plate_x - d_property, y: plate_y - d_property, z: properties_z },
                                    cgmath::Vector3::zero(),
                                    cgmath::Quaternion::new(1., 0., 0., 0.),
                                    "property_1",
                                    VisualInstanceData::Empty
                                )),
                                Box::new(VisualInstance::new(
                                    // cgmath::Vector3 { x: plate_x - d_property, y: plate_y + d_property, z: properties_z },
                                    cgmath::Vector3::zero(),
                                    cgmath::Quaternion::new(1., 0., 0., 0.),
                                    "property_2",
                                    VisualInstanceData::Empty
                                )),
                                Box::new(VisualInstance::new(
                                    // cgmath::Vector3 { x: plate_x + d_property, y: plate_y + d_property, z: properties_z },
                                    cgmath::Vector3::zero(),
                                    cgmath::Quaternion::new(1., 0., 0., 0.),
                                    "property_3",
                                    VisualInstanceData::Empty
                                )),
                                Box::new(VisualInstance::new(
                                    // cgmath::Vector3 { x: plate_x + d_property, y: plate_y - d_property, z: properties_z },
                                    cgmath::Vector3::zero(),
                                    cgmath::Quaternion::new(1., 0., 0., 0.),
                                    "property_4",
                                    VisualInstanceData::Empty
                                ))
                            ],
                            ChildrenLayout::Circle,
                            (2. * d_property, 2. * d_property)
                        )
                    )
                };

                h
            });
        self.visuals = visuals;
    }


    pub fn visuals_updates(&mut self) -> Option<&mut Vec<VisualInstance>> {
        
        for visual_instance in self.visuals.iter_mut() {
            // debug!("wapuku: visuals_updates: visual_instance={:?}", visual_instance);
            if let Some(animation) = self.animations.get_mut(&visual_instance.id) {
                self.has_updates = true;
                // debug!("wapuku: visuals_updates: animation for id={:?}", visual_instance.id);
                
                if animation.tick(visual_instance) == AnimationState::Done {
                    self.animations.remove(&visual_instance.id);
                    // debug!("wapuku: visuals_updates: removed animation for id={:?}", visual_instance.id);
                }
            }
        }
        
        
        if self.has_updates {
            self.has_updates = false;
            
            // self.visuals.as_mut()
            Some(&mut self.visuals)
        } else {
            None
        }
    }

    pub fn on_pointer_moved(&mut self, x:f32, y:f32){
        debug!("wapuku: on_pointer_moved: x={}, y={}", x, y);

        let visual_under_pointer_op = self.visuals.iter().find(|v|v.bounds().contain(x, y));

        
        match visual_under_pointer_op {
            None => {
                debug!("wapuku: on_pointer_moved: no visual_under_pointer_op");
                self.clear_prev_visual_under_pointer(None);
            }
            Some(visual_under_pointer) => {
                if self.visual_id_under_pointer_op.map(|visual_id_under_pointer| visual_under_pointer.id != visual_id_under_pointer).unwrap_or(true) {
                    debug!("wapuku: on_pointer_moved: visual_under_pointer_op");

                    if !self.animations.contains_key(&visual_under_pointer.id) {
                        self.animations.insert(
                            visual_under_pointer.id,
                            Box::new(SimultaneousAnimations::new(vec![
                                Box::new(Lerp::from_to_in_steps(visual_under_pointer.scale.y, 1.1, |v: &VisualInstance| &v.scale.y, |v: &mut VisualInstance, y| v.scale.y = y, 10, 0.01)),
                                Box::new(Lerp::from_to_in_steps(visual_under_pointer.scale.x, 1.1, |v: &VisualInstance| &v.scale.x, |v: &mut VisualInstance, x| v.scale.x = x, 10, 0.01))
                            ]))
                        );

                        self.clear_prev_visual_under_pointer(Some(visual_under_pointer.id));
                    }
                }
            }
        }

        self.has_updates = true;
    }

    fn clear_prev_visual_under_pointer(&mut self, current_visual_id_under_pointer_op:Option<u32>) {
        debug!("wapuku: clear_prev_visual_under_pointer: self.visual_id_under_pointer_op={:?}", self.visual_id_under_pointer_op);
        
        if let Some(prev_visual_under_pointer) = self.visual_id_under_pointer_op.take().and_then(|prev_visual_id_under_pointer| self.visuals.iter().find(|v| v.id == prev_visual_id_under_pointer)) {
            if !self.animations.contains_key(&prev_visual_under_pointer.id) {
                self.animations.insert(
                    prev_visual_under_pointer.id,
                    Box::new(SimultaneousAnimations::new(vec![
                        Box::new(Lerp::from_to_in_steps(prev_visual_under_pointer.scale.y, 1.0, |v: &VisualInstance| &v.scale.y, |v: &mut VisualInstance, y| v.scale.y = y, 10, 0.01)),
                        Box::new(Lerp::from_to_in_steps(prev_visual_under_pointer.scale.x, 1.0, |v: &VisualInstance| &v.scale.x, |v: &mut VisualInstance, x| v.scale.x = x, 10, 0.01))
                    ]))
                );
            }
        }

        self.visual_id_under_pointer_op = current_visual_id_under_pointer_op;
    }

    #[allow(dead_code)]
    fn flat_visuals(visuals: Option<&mut HashMap<String, Vec<VisualInstance>>>) -> Option<impl Iterator<Item = &mut VisualInstance>> {
        visuals.map(|visuals|
            visuals
                .values_mut()
                .flat_map(|visuals_vec| visuals_vec.iter_mut())
        )
    }

    #[allow(dead_code)]
    fn find_group_by_xy(x: f32, y: f32, visuals: Option<&mut HashMap<String, Vec<VisualInstance>>>, _on_each: impl FnMut(&mut VisualInstance) -> &mut VisualInstance) -> Option<&mut VisualInstance> {

        Self::flat_visuals(visuals).and_then(|mut visuals_iter|visuals_iter.find(|v| v.bounds().contain(x, y)))
    }

    pub fn get_visual_under_pointer(&mut self, x: f32, y: f32) -> Option<&Box<dyn DataGroup>> {
        debug!("wapuku: on_pointer_input:  x={}, y={}", x, y);

        self.visuals.iter().find(|v|v.bounds().contain(x, y)).and_then(|visual_under_pointer|
            match &visual_under_pointer.data {
                VisualInstanceData::DataGroup(data_group) => {
                    Some(data_group)
                    // match data_group.bounds() {
                    //     DataBounds::XY(property_x, property_y) => Some({
                    //         self.data.build_grid(property_x, property_y, 3, 3, "property_3")
                    // None
                    },
                    _ => None

                },

        )

    }

    pub fn on_pointer_input(&mut self, x: f32, y: f32) {
        debug!("wapuku: on_pointer_input:  x={}, y={}", x, y);
        
        if let Some(_visual_under_pointer) = self.visuals.iter().find(|v|v.bounds().contain(x, y)) {
            
/*            match &visual_under_pointer.data {
                VisualInstanceData::DataGroup(data_group) => {
                    match data_group.bounds() {
                        DataBounds::XY(property_x, property_y) => {
                            let grid = self.data.build_grid(property_x, property_y, 3, 3, "property_3");
                        }
                        _ => {}
                    }
                }
                VisualInstanceData::Empty => {}
            }
*/            

            for visual in self.visuals.iter_mut() {
                let position = visual.position;
                
                debug!("wapuku: on_pointer_input: visual={:?}", visual);
                
                if visual.bounds().contain(x, y) {
                    self.animations.insert(
                        visual.id,
                        Box::new(ConsecutiveAnimations::new(vec![
                                Box::new(SimultaneousAnimations::new(vec![
                                    Box::new(Lerp::from_to_in_steps(
                                        visual.position, 
                                        Vector3::new(0.0, 0.0, visual.position.z), 
                                        VisualInstance::position, 
                                        VisualInstance::set_position, 
                                        100, 0.001
                                    )),
                                    Box::new(Lerp::from_to_in_steps(
                                        Vector3::new(1.0, 1.0, 1.0),
                                        Vector3::new(0.2, 0.2, 0.2),
                                        VisualInstance::scale,
                                        VisualInstance::set_scale,
                                        100, 0.003,
                                    ))
                                ])),
                                Box::new(SimultaneousAnimations::new(vec![
                                    Box::new(Lerp::from_to_in_steps(
                                        Vector3::new(0.0, 0.0, visual.position.z),
                                        position,
                                        VisualInstance::position,
                                        VisualInstance::set_position,
                                        100, 0.001,
                                    )),
                                    Box::new(Lerp::from_to_in_steps(
                                        Vector3::new(0.2, 0.2, 0.2),
                                        Vector3::new(1.0, 1.0, 1.0),
                                        VisualInstance::scale,
                                        VisualInstance::set_scale,
                                        100, 0.001,
                                    ))
                                ]))
                            ]))
                    );

                } else {
                    
                    self.animations.insert(
                        visual.id,
                        Box::new(ConsecutiveAnimations::new(vec![
                            Box::new(Lerp::from_to_in_steps(
                                visual.scale,
                                Vector3::zero(),
                                VisualInstance::scale,
                                VisualInstance::set_scale,
                                99, 0.001,
                            )),
                            Box::new(Lerp::from_to_in_steps(
                                visual.position,
                                Vector3::new(0.0, 0.0, visual.position.z),
                                
                                VisualInstance::position,
                                VisualInstance::set_position,
                                1, 0.001,
                            )),
                            Box::new(SimultaneousAnimations::new(vec![
                                Box::new(Lerp::from_to_in_steps(
                                    Vector3::new(0.0, 0.0, visual.position.z),
                                    position,
                                    VisualInstance::position,
                                    VisualInstance::set_position,
                                    100, 0.001,
                                )),
                                Box::new(Lerp::from_to_in_steps(
                                    Vector3::zero(),
                                    Vector3::new(1.0, 1.0, 1.0),
                                    VisualInstance::scale,
                                    VisualInstance::set_scale,
                                    100, 0.001,
                                )),
                            ]))
                            
                        ])),
                    );
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