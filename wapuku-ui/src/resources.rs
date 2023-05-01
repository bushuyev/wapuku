use std::io::{BufReader, Cursor};
use log::debug;

use wgpu::util::DeviceExt;
use crate::mesh_model;
use crate::mesh_model::{Instance, MeshModel};
use crate::texture::Texture;

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

    let (models, obj_materials) = tobj::load_obj_buf_async(
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

    let mut materials = Vec::new();
    for m in obj_materials? {
        // let diffuse_texture = load_texture(&m.diffuse_texture, device, queue).await?;
        let diffuse_texture = load_texture(&String::from("data/wapuku_purple_1024.jpg"), device, queue).await?;
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

        materials.push(mesh_model::Material {
            name: m.name,
            diffuse_texture,
            bind_group,
        })
    }
    debug!("load_model: models.len()={:?}", models.len());
    
    let meshes = models
        .into_iter()
        .map(|m| {
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
                material: m.mesh.material_id.unwrap_or(0),
            }
        })
        .collect::<Vec<_>>();
    
    debug!("load_model: meshes={:?}", meshes);

    let instances = vec![
        Instance {
            position: cgmath::Vector3 { x: 0.0, y: 0.0, z: 0.0 },
            rotation: cgmath::Quaternion::new(1., 0., 0., 0.),
        }
    ];


    let instance_data = instances.iter().map(Instance::to_raw).collect::<Vec<_>>();

    let instance_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Instance Buffer"),
        contents: bytemuck::cast_slice(&instance_data),
        usage: wgpu::BufferUsages::VERTEX,
    });


    Ok(mesh_model::MeshModel {
        meshes,
        materials,
        instances,
        instance_buffer
    })
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