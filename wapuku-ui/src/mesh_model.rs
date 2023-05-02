use std::ops::Range;
use log::debug;
use wgpu::util::DeviceExt;
use wapuku_model::visualization::*;


impl From<&Instance> for InstanceRaw {
    fn from(value: &Instance) -> Self {

        InstanceRaw {
            model: (cgmath::Matrix4::from_translation(value.position()) * cgmath::Matrix4::from(value.rotation())).into(),
        }
    }
}




#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct InstanceRaw {
    #[allow(dead_code)]
    model: [[f32; 4]; 4],
}

impl InstanceRaw {
    pub(crate) fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        use std::mem;
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<InstanceRaw>() as wgpu::BufferAddress,
            // We need to switch from using a step mode of Vertex to Instance
            // This means that our shaders will only change to use the next
            // instance when the shader starts processing a new instance
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 5,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 4]>() as wgpu::BufferAddress,
                    shader_location: 6,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 8]>() as wgpu::BufferAddress,
                    shader_location: 7,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 12]>() as wgpu::BufferAddress,
                    shader_location: 8,
                    format: wgpu::VertexFormat::Float32x4,
                },
            ],
        }
    }
}

use crate::texture;

pub trait Vertex {
    fn desc<'a>() -> wgpu::VertexBufferLayout<'a>;
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ModelVertex {
    pub position: [f32; 3],
    pub tex_coords: [f32; 2],
    pub normal: [f32; 3],
}

impl Vertex for ModelVertex {
    fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        use std::mem;
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<ModelVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x2,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 5]>() as wgpu::BufferAddress,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Float32x3,
                },
            ],
        }
    }
}

#[derive(Debug)]
pub struct Material {
    pub name: String,
    pub diffuse_texture: texture::Texture,
    pub bind_group: wgpu::BindGroup,
}

#[derive(Debug)]
pub struct Mesh {
    pub name: String,
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub num_elements: u32,
    pub material: Material,
}

pub struct MeshModel {
    meshes: Vec<Mesh>,
    instance_buffer: wgpu::Buffer,
    instances: Vec<Instance>,
}

impl MeshModel {
    
    pub fn tick(&mut self){
        // self.instance_buffer.
    }
    
    pub fn new(meshes: Vec<Mesh>, device: &wgpu::Device) -> Self {
        let instances = vec![
            Instance::new(
                cgmath::Vector3 { x: 0.0, y: 0.0, z: 0.0 },
                cgmath::Quaternion::new(1., 0., 0., 0.)
            ),

            Instance::new(
                cgmath::Vector3 { x: 2.0, y: 0.0, z: 0.0 },
                cgmath::Quaternion::new(1., 0., 0., 0.)
            ),

            Instance::new(
                cgmath::Vector3 { x: 3.0, y: 0.0, z: 0.0 },
                cgmath::Quaternion::new(1., 0., 0., 0.)
            ),

            Instance::new(
                cgmath::Vector3 { x: 4.0, y: 0.0, z: 0.0 },
                cgmath::Quaternion::new(1., 0., 0., 0.)
            )
        ];


        let instance_data:Vec<InstanceRaw> = instances.iter().map(|i|i.into()).collect::<Vec<_>>();

        let instance_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Instance Buffer"),
            contents: bytemuck::cast_slice(&instance_data),
            usage: wgpu::BufferUsages::VERTEX,
        });
        
        Self { meshes, instance_buffer, instances }
    }

    pub fn meshes(&self) -> &Vec<Mesh> {
        &self.meshes
    }
    pub fn instance_buffer(&self) -> &wgpu::Buffer {
        &self.instance_buffer
    }
    pub fn instances(&self) -> &Vec<Instance> {
        &self.instances
    }
    
    pub fn build_instances(&mut self){
        
    }
}

pub trait DrawModel<'a> {
    fn draw_mesh_instanced(
        &mut self,
        mesh: &'a Mesh,
        material: &'a Material,
        instances: Range<u32>,
        camera_bind_group: &'a wgpu::BindGroup,
        light_bind_group: &'a wgpu::BindGroup,
    );


    fn draw_model_instanced(
        &mut self,
        model: &'a MeshModel,
        camera_bind_group: &'a wgpu::BindGroup,
        light_bind_group: &'a wgpu::BindGroup,
    );
}

impl<'a, 'b> DrawModel<'b> for wgpu::RenderPass<'a>
    where
        'b: 'a,
{

    fn draw_mesh_instanced(
        &mut self,
        mesh: &'b Mesh,
        material: &'b Material,
        instances: Range<u32>,
        camera_bind_group: &'b wgpu::BindGroup,
        light_bind_group: &'a wgpu::BindGroup
    ) {
        debug!("RenderPass::draw_mesh_instanced: instances={:?}", instances);
        
        self.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
        self.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        self.set_bind_group(0, &material.bind_group, &[]);
        self.set_bind_group(1, camera_bind_group, &[]);
        self.set_bind_group(2, light_bind_group, &[]);
        self.draw_indexed(0..mesh.num_elements, 0, instances);
    }


    fn draw_model_instanced(
        &mut self,
        model: &'a MeshModel,
        camera_bind_group: &'a wgpu::BindGroup,
        light_bind_group: &'a wgpu::BindGroup
    ) {
        
        model.meshes.iter().enumerate().for_each(|(i, mesh)| {
            // log::warn!("materials: {}", model.materials.len());
            // let material = &model.materials[mesh.material];
            self.draw_mesh_instanced(mesh, &mesh.material, i as u32 * 2..i as u32 * 2 + 2, camera_bind_group, light_bind_group);
            // self.draw_mesh_instanced(mesh, &mesh.material, 0..4, camera_bind_group, light_bind_group);
        });
    }
}
