use std::ops::Range;
use log::debug;
use wgpu::util::DeviceExt;
use wapuku_model::visualization::*;


impl From<&VisualInstance> for InstanceRaw {
    fn from(value: &VisualInstance) -> Self {
        //
        let mut model:[[f32; 4]; 4] = (cgmath::Matrix4::from_translation(value.position()) * cgmath::Matrix4::from(value.rotation())).into();
        model[3][0] = - model[3][0];

        InstanceRaw {
            model,
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
    instances:Range<u32>
}

impl Mesh {
    pub fn new(name: String, vertex_buffer: wgpu::Buffer, index_buffer: wgpu::Buffer, num_elements: u32, material: Material) -> Self {
        Self { 
            name, 
            vertex_buffer,
            index_buffer,
            num_elements,
            material, 
            instances: (0..0) 
        }
    }
    
    pub fn set_instances_range(&mut self, instances:Range<u32>) {
        self.instances = instances;
    }


    pub fn instances(&self) -> &Range<u32> {
        &self.instances
    }
}

pub struct MeshModel {
    meshes: Vec<Mesh>,
}

impl MeshModel {
    
    pub fn new(meshes: Vec<Mesh>, device: &wgpu::Device) -> Self {
        Self { meshes }
    }
    

    pub fn meshes(&self) -> &Vec<Mesh> {
        &self.meshes
    }
    
    pub fn mesh_by_name(&mut self, name:&str) -> Option<&mut Mesh> {
        self.meshes.iter_mut().find(|m|m.name == name)
    }
    
}

pub trait MeshInstances {
    fn mesh(&self) -> &Mesh;
    fn instances(&self) -> Range<u32>;
}

pub trait DrawModel<'a> {
    fn draw_mesh_instanced(
        &mut self,
        mesh: &'a Mesh,
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

impl<'a, 'b> DrawModel<'b> for wgpu::RenderPass<'a> where 'b: 'a {

    fn draw_mesh_instanced(
        &mut self,
        mesh: &'b Mesh,
        camera_bind_group: &'b wgpu::BindGroup,
        light_bind_group: &'a wgpu::BindGroup
    ) {
        debug!("RenderPass::draw_mesh_instanced: instances={:?}", mesh.instances);
        
        self.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
        self.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        self.set_bind_group(0, &mesh.material.bind_group, &[]);
        self.set_bind_group(1, camera_bind_group, &[]);
        self.set_bind_group(2, light_bind_group, &[]);
        self.draw_indexed(0..mesh.num_elements, 0, mesh.instances.clone());
    }

    fn draw_model_instanced(
        &mut self,
        mesh_model: &'a MeshModel,
        camera_bind_group: &'a wgpu::BindGroup,
        light_bind_group: &'a wgpu::BindGroup,
    ){

        for mesh in mesh_model.meshes.iter() {
            
            // log::warn!("materials: {}", model.materials.len());
            // let material = &model.materials[mesh.material];
            // self.draw_mesh_instanced(mesh, &mesh.material, i as u32 * 2..i as u32 * 2 + 2, camera_bind_group, light_bind_group);
            self.draw_mesh_instanced(&mesh, camera_bind_group, light_bind_group);
            // self.draw_mesh_instanced(mesh, &mesh.material, 0..4, camera_bind_group, light_bind_group);
        }
    }
}
