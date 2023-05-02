use std::io::{BufReader, Cursor};
use log::debug;
use wgpu::{BindGroupLayout, Device, Queue};

use wgpu::util::DeviceExt;
use crate::mesh_model;
use crate::mesh_model::{Material, MeshModel};
use crate::texture::Texture;
use wapuku_resources::resources::*;
use futures::future::join_all;

fn format_url(file_name: &str) -> reqwest::Url {
    let window = web_sys::window().unwrap();
    let location = window.location();
    let mut origin = location.origin().unwrap();
    // if !origin.ends_with("learn-wgpu") {
    //     origin = format!("{}/learn-wgpu", origin);
    // }
    let base = reqwest::Url::parse(&format!("{}/", origin,)).unwrap();
    base.join(file_name).unwrap()
}

pub async fn load_string(file_name: &str) -> anyhow::Result<String> {
    let url = format_url(file_name);
    let txt = reqwest::get(url)
        .await?
        .text()
        .await?;

    Ok(txt)
}

pub async fn load_binary(file_name: &str) -> anyhow::Result<Vec<u8>> {
    let url = format_url(file_name);
    let data = reqwest::get(url)
        .await?
        .bytes()
        .await?
        .to_vec();
    
    Ok(data)
}

pub async fn load_texture(
    file_name: &str,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
) -> anyhow::Result<Texture> {
    let data = load_binary(file_name).await?;
    Texture::from_bytes(device, queue, &data, file_name)
}

pub async fn load_model(
    file_name: &str,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    layout: &wgpu::BindGroupLayout,
) -> anyhow::Result<mesh_model::MeshModel> {
    debug!("load_model: obj file_name={:?}", file_name);
    
    let obj_text = load_string(file_name).await?;
    let obj_cursor = Cursor::new(obj_text);
    let mut obj_reader = BufReader::new(obj_cursor);

    let (models, obj_materials_r) = tobj::load_obj_buf_async(
        &mut obj_reader,
        &tobj::LoadOptions {
            triangulate: true,
            single_index: true,
            ..Default::default()
        },
        |p| async move {
            debug!("material url={:?}", p);
            let p = format!("data/{}", p);
            
            let mat_text = load_string(&p).await.unwrap();
            tobj::load_mtl_buf(&mut BufReader::new(Cursor::new(mat_text)))
        },
    ).await?;

   
    let obj_materials = obj_materials_r?;
    let models_textures = models.iter()
        .map(
            |m| m.mesh.material_id.and_then(|material_id| obj_materials.get(material_id)).expect(format!("no material for {:?}", m.mesh.material_id).as_str())
        )
        .map(|m| format!("data/{}", resource_filename(m.diffuse_texture.as_str())) )
        .collect::<Vec<String>>();
    
    let meshes = join_all(models
        .into_iter()
        .zip(models_textures.iter())
        .map(|(m, texture)| async move {
            let vertices = (0..m.mesh.positions.len() / 3)
                .map(|i| mesh_model::ModelVertex {
                    position: [
                        m.mesh.positions[i * 3],
                        m.mesh.positions[i * 3 + 1],
                        m.mesh.positions[i * 3 + 2],
                    ],
                    tex_coords: [m.mesh.texcoords[i * 2], m.mesh.texcoords[i * 2 + 1]],
                    normal: [
                        m.mesh.normals[i * 3],
                        m.mesh.normals[i * 3 + 1],
                        m.mesh.normals[i * 3 + 2],
                    ],
                })
                .collect::<Vec<_>>();
    
            debug!("load_model: name={:?} vertices={:?}", m.name, vertices);
    
            let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(&format!("{:?} Vertex Buffer", file_name)),
                contents: bytemuck::cast_slice(&vertices),
                usage: wgpu::BufferUsages::VERTEX,
            });
            let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(&format!("{:?} Index Buffer", file_name)),
                contents: bytemuck::cast_slice(&m.mesh.indices),
                usage: wgpu::BufferUsages::INDEX,
            });
    
            mesh_model::Mesh {
                name: file_name.to_string(),
                vertex_buffer,
                index_buffer,
                num_elements: m.mesh.indices.len() as u32,
                material: make_material(device, queue, layout, texture).await,
            }
        })
        .collect::<Vec<_>>()).await;


    Ok(MeshModel::new (meshes, device))
}

async fn make_material(device: &Device, queue: &Queue, layout: &BindGroupLayout, file_name: &String) -> Material {
    let diffuse_texture = load_texture(file_name.as_str(), device, queue).await.unwrap();
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&diffuse_texture.view),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Sampler(&diffuse_texture.sampler),
            },
        ],
        label: None,
    });

    Material {
        name: file_name.clone(),
        diffuse_texture,
        bind_group,
    }
}

#[cfg(test)]
pub mod resources_test {
    use std::fs::File;
    use std::io::{BufReader, Cursor};
    use crate::resources::load_binary;
    use wasm_bindgen_test::*;


    #[wasm_bindgen_test]
    fn test_load_ob(){
        

    }
}