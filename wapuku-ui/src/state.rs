use std::{f32, iter};
use std::collections::HashMap;
use std::ops::{Mul, Range};
use log::{debug, trace};
use wgpu::{BindGroupLayout, Color, Device, PipelineLayout, RenderPipeline, ShaderModule, SurfaceConfiguration, Texture, TextureFormat, VertexBufferLayout};
use wgpu::util::DeviceExt;
use winit::event::WindowEvent;
use winit::window::Window;
use crate::mesh_model::{DrawModel, InstanceRaw, Mesh, MeshInstances, MeshModel, Vertex};
use crate::{mesh_model, resources, texture};
use cgmath::prelude::*;
use cgmath::{Matrix4, Point3, Vector2, Vector3, Vector4};
use winit::dpi::PhysicalSize;
use crate::camera::{Camera, CameraController, CameraUniform};
use crate::light::{DrawLight, LightUniform};
use wapuku_model::model::*;
use wapuku_model::visualization::*;


pub struct State {
    surface: wgpu::Surface,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    pub(crate)  size: winit::dpi::PhysicalSize<u32>,
    pub(crate)  window: Window,

    vis_ctrl:VisualDataController,//TODO rename
    mesh_model: MeshModel,
    instance_buffer: wgpu::Buffer,
    color:Color,
    light_render_pipeline: wgpu::RenderPipeline,
    render_pipeline: wgpu::RenderPipeline,
    
    camera_bind_group: wgpu::BindGroup,
    camera_buffer: wgpu::Buffer,
    camera_controller: CameraController,
    camera_uniform:CameraUniform,
    camera: Camera,
    
    depth_texture: texture::Texture,

    light_uniform:LightUniform,
    light_buffer: wgpu::Buffer,
    light_bind_group: wgpu::BindGroup,

    multisampled_framebuffer: wgpu::TextureView,

    projection: Matrix4<f32>

}

pub const SAMPLE_COUNT: u32 = 1; //4;

impl State {

    pub async fn new(window: Window, model:VisualDataController) -> Self {
        let size = window.inner_size();
        
        debug!("State::new: size={:?}", size);

        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            dx12_shader_compiler: Default::default(),
        });

        
        let surface = unsafe { instance.create_surface(&window) }.unwrap();

        let adapter = instance.request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .unwrap();

        let (device, queue) = adapter.request_device(
                &wgpu::DeviceDescriptor {
                    label: None,
                    features: wgpu::Features::empty(),
                    // WebGL doesn't support all of wgpu's features, so if
                    // we're building for the web we'll have to disable some.
                    limits: if cfg!(target_arch = "wasm32") {
                        wgpu::Limits::downlevel_webgl2_defaults()
                    } else {
                        wgpu::Limits::default()
                    },
                },
                // Some(&std::path::Path::new("trace")), // Trace path
                None,
            )
            .await
            .unwrap();

        let surface_caps = surface.get_capabilities(&adapter);
        // Shader code in this tutorial assumes an Srgb surface texture. Using a different
        // one will result all the colors comming out darker. If you want to support non
        // Srgb surfaces, you'll need to account for that when drawing to the frame.
        let surface_format = surface_caps.formats.iter()
            .copied()
            .filter(|f| f.is_srgb())
            .next()
            .unwrap_or(surface_caps.formats[0]);

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: surface_caps.present_modes[0],
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
        };
        surface.configure(&device, &config);


        let texture_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
                label: Some("texture_bind_group_layout"),
            });



        let camera = Camera {
            eye: (0.0, 0.0, -20.0).into(),
            target: (0.0, 0.0, 0.0).into(),
            up: cgmath::Vector3::new(0., 1., 0.),
            aspect: config.width as f32 / config.height as f32,
            fovy: 45.0,
            znear: 0.1,
            zfar: 100.0,
        };
        

        let mut camera_uniform = CameraUniform::new();
        let inverse_proj = camera_uniform.update_view_proj(&camera);



        let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Camera Buffer"),
            contents: bytemuck::cast_slice(&[camera_uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let camera_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
                label: Some("camera_bind_group_layout"),
            });

        //////////////////////////light
        let light_uniform = LightUniform {
            position: [0.0, 10.0, -10.0],
            _padding: 0,
            color: [1.0, 1.0, 1.0],
            _padding2: 0,
        };

        // We'll want to update our lights position, so we use COPY_DST
        let light_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Light VB"),
                contents: bytemuck::cast_slice(&[light_uniform]),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            }
        );

        let light_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
                label: None,
            });


        


        Self {
            surface,
            size,
            window,
            
            light_render_pipeline: {
                Self::create_render_pipeline(
                    &device,
                    &device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                        label: Some("Light Pipeline Layout"),
                        bind_group_layouts: &[&camera_bind_group_layout, &light_bind_group_layout],
                        push_constant_ranges: &[],
                    }),
                    config.format,
                    &[mesh_model::ModelVertex::desc()],
                    &device.create_shader_module(wgpu::ShaderModuleDescriptor {
                        label: Some("Light Shader"),
                        source: wgpu::ShaderSource::Wgsl(include_str!("light.wgsl").into()),
                    }), 1
                )
            },
            
            render_pipeline: Self::render_pipeline(&device, &config, &texture_bind_group_layout, &camera_bind_group_layout, &light_bind_group_layout),
            camera_bind_group: device.create_bind_group(&wgpu::BindGroupDescriptor {
                layout: &camera_bind_group_layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: camera_buffer.as_entire_binding(),
                }],
                label: Some("camera_bind_group"),
            }),
            camera_buffer,
            camera_uniform,
            camera_controller: CameraController::new(0.2),
            camera,

            vis_ctrl: model,
            mesh_model: resources::load_model(
                "data/wapuku.obj",
                &device,
                &queue,
                &texture_bind_group_layout,
            ).await.unwrap(),

            instance_buffer: device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Instance Buffer"),
                contents: &[0; 10000], //TODO resize?
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            }),

            light_uniform,
            light_bind_group: device.create_bind_group(&wgpu::BindGroupDescriptor {
                layout: &light_bind_group_layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: light_buffer.as_entire_binding(),
                }],
                label: None,
            }),
            light_buffer,
            
            color: Color {
                r: 0.1,
                g: 1.2,
                b: 0.3,
                a: 1.0,
            },
            depth_texture: texture::Texture::create_depth_texture(&device, &config, "depth_texture"),
            multisampled_framebuffer: Self::create_multisampled_framebuffer(&device, &config,SAMPLE_COUNT),
            
            config,
            device,
            queue,
            projection: inverse_proj
        }
        
    }

    fn create_multisampled_framebuffer(
        device: &wgpu::Device,
        sc_desc: &SurfaceConfiguration,
        sample_count: u32,
    ) -> wgpu::TextureView {
        
        let multisampled_frame_descriptor = &wgpu::TextureDescriptor {
            size: wgpu::Extent3d {
                width: sc_desc.width,
                height: sc_desc.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count,
            dimension: wgpu::TextureDimension::D2,
            format: sc_desc.format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            label: None,
            view_formats: &[sc_desc.format],
        };

        device
            .create_texture(multisampled_frame_descriptor)
            .create_view(&wgpu::TextureViewDescriptor::default())
        
    }

    fn render_pipeline(
        device: &Device, 
        config: &SurfaceConfiguration, 
        texture_bind_group_layout: &BindGroupLayout, 
        camera_bind_group_layout: &BindGroupLayout,
        light_bind_group_layout: &BindGroupLayout
    ) -> RenderPipeline {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
        });

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[
                    texture_bind_group_layout, 
                    camera_bind_group_layout,
                    light_bind_group_layout
                ],
                push_constant_ranges: &[],
            });

        Self::create_render_pipeline(device, &render_pipeline_layout, config.format, &[mesh_model::ModelVertex::desc(), InstanceRaw::desc()], &shader, SAMPLE_COUNT)
    }

    fn create_render_pipeline(device: &Device, render_pipeline_layout: &PipelineLayout, format: wgpu::TextureFormat, buffers: &[wgpu::VertexBufferLayout], shader: &wgpu::ShaderModule, multisample_count: u32) -> RenderPipeline {
        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: shader,
                entry_point: "vs_main",
                buffers,
            },
            fragment: Some(wgpu::FragmentState {
                module: shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: format,
                    blend: Some(wgpu::BlendState {
                        color: wgpu::BlendComponent::REPLACE,
                        alpha: wgpu::BlendComponent::REPLACE,
                    }),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                // Setting this to anything other than Fill requires Features::POLYGON_MODE_LINE
                // or Features::POLYGON_MODE_POINT
                polygon_mode: wgpu::PolygonMode::Fill,
                // Requires Features::DEPTH_CLIP_CONTROL
                unclipped_depth: false,
                // Requires Features::CONSERVATIVE_RASTERIZATION
                conservative: false,
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: texture::Texture::DEPTH_FORMAT,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState {
                count: multisample_count,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            // If the pipeline will be used with a multiview render pass, this
            // indicates how many array layers the attachments will have.
            multiview: None,
        });
        render_pipeline
    }
    pub fn window(&self) -> &Window {
        &self.window
    }

    //https://github.com/rust-windowing/winit/issues/1661
    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        debug!("State::resize: new_size={:?}", new_size);

        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);
        }
    }

    pub fn input(&mut self, event: &WindowEvent) -> bool {
        self.camera_controller.process_events(event)
    }

    pub fn pointer_moved(&mut self, position: winit::dpi::PhysicalPosition<f64>) {
        debug!("pointer_moved: location={:?} self.size={:?} self.camera_uniform.view_proj()={:?} self.inverse_proj={:?}", position, self.size, self.camera_uniform.view_proj(), self.projection);

        if let Some(visuals) = self.vis_ctrl.visuals() {
            let visuals:&HashMap<String, Vec<VisualInstance>> = visuals;

            let found = visuals.values().flat_map(|vv| vv.iter()).find(|v: &&VisualInstance| {
                v.bounds().contain(position.x as f32, position.y as f32)
            });
            
            debug!("pointer_moved, found={:?}", found)
        }

        // let clip_x = position.x as f32 / self.size.width as f32 *  2. - 1.;
        // let clip_y = position.y as f32 / self.size.height as f32 * -2. + 1.;
        // 
        // 
        // let clip_pos = Vector4::<f32>::new(clip_x, clip_y  as f32, -10., 1.,);
        // 
        // let p = Point3::new(0., 0., 0.);
        // // clip_pos.
        // // p.mul(self.inverse_proj);
        // 
        // let pos_in_world = self.projection.mul(clip_pos);
        // 
        // debug!("pointer_moved: clip_pos={:?} pos_in_world={:?}", clip_pos, pos_in_world);
    }

    pub fn update(&mut self/*, visuals:HashMap<String, Vec<VisualInstance>>*/) {
        self.camera_controller.update_camera(&mut self.camera);
        self.projection = self.camera_uniform.update_view_proj(&self.camera);
        self.queue.write_buffer(&self.camera_buffer, 0, bytemuck::cast_slice(&[self.camera_uniform]));

        if let Some(visuals) = self.vis_ctrl.visuals_updates() {
            // debug!("State::update: self.config.width={:?}, self.config.height={:?}, self.camera.build_view_projection_matrix()={:?}", self.config.width, self.config.height, self.camera.build_view_projection_matrix());

            for visual_instance in visuals.values_mut().flat_map(|vv| vv.iter_mut()) {
                let v_left_top = Vector4::new(-1., 1., 0., 1.);
                let v_right_bottom = Vector4::new(1., -1., 0., 1.);

                let instance = visual_instance.position();
                let model_matrix = Matrix4::from_translation(instance);

                let (x_left_top, y_left_top) = Self::to_screen_xy(v_left_top, model_matrix, &self.projection, &self.size);
                let (x_right_bottom, y_right_bottom) = Self::to_screen_xy(v_right_bottom, model_matrix, &self.projection, &self.size);

                visual_instance.bounds_mut().update(x_left_top, y_left_top, x_right_bottom, y_right_bottom);

                debug!("State::update: v={:?}  x_left_top={}, y_left_top={}, x_right_bottom={}, y_right_bottom={}", visual_instance, x_left_top, y_left_top, x_right_bottom, y_right_bottom);
            }

            let instance_data = Self::visuals_to_raw(visuals, &mut self.mesh_model);

            self.queue.write_buffer(&self.instance_buffer, 0, bytemuck::cast_slice(&instance_data));
        }
      
        self.light_uniform.position = self.camera.eye.into(); //[self.camera.eye.x, self.camera.eye.y, self.camera.eye.z];
        self.queue.write_buffer(&self.light_buffer, 0, bytemuck::cast_slice(&[self.light_uniform]));

    }

    fn to_screen_xy(v_left_top: Vector4<f32>, model_matrix: Matrix4<f32>, projection: &Matrix4<f32>, size: &PhysicalSize<u32>) -> (f32, f32) {
        let world_position = model_matrix.mul(v_left_top);

        let v_clip = projection.mul(world_position);
        let v_ndc = Vector3::new(v_clip.x / v_clip.w, v_clip.y / v_clip.w, v_clip.z / v_clip.w);

        let x_left_top = (v_ndc.x + 1.) * (size.width as f32 / 2.);
        let y_left_top = (v_ndc.y + 1.) * (size.height as f32 / 2.);
        (x_left_top, y_left_top)
    }

    //TODO move out
    fn visuals_to_raw(visuals:&HashMap<String, Vec<VisualInstance>>, mesh_model: &mut MeshModel) -> Vec<InstanceRaw> {
    
        let mut prev_mesh_range = 0u32;


        let instance_data: Vec<InstanceRaw> = visuals.iter().flat_map(|(name, m)| {
            let mesh_name = match name.as_str() {
                "property_1" => "Sphere",
                "property_2" => "Cone",
                "property_3" => "Cube",
                "property_4" => "Cylinder",
                "plate" => "Torus",
                &_ => "Torus"
            };

            debug!("State::visualis_to_raw: mesh_name={:?} name={:?}", mesh_name, name);

            let mesh_op = mesh_model.mesh_by_name(mesh_name);

            if let Some(mesh) = mesh_op {
                let mesh_range = prev_mesh_range + m.len() as u32;

                mesh.set_instances_range((prev_mesh_range..mesh_range));

                prev_mesh_range = mesh_range;
            }

            m.iter()
        }).map(|i| i.into()).collect::<Vec<InstanceRaw>>();

        debug!("State::visuals_to_raw: visuals={:?} instance_data.len()={}", visuals, instance_data.len());

        instance_data
        
    }

    pub fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        let output = self.surface.get_current_texture()?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view, //&self.multisampled_framebuffer,
                    resolve_target: None, //Some(&view),
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::WHITE),
                        store: true,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_texture.view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: true,
                    }),
                    stencil_ops: None,
                }),
            });

            render_pass.set_vertex_buffer(1, self.instance_buffer.slice(..));

            render_pass.set_pipeline(&self.light_render_pipeline);
            render_pass.draw_light_model(
                &self.mesh_model,
                &self.camera_bind_group,
                &self.light_bind_group,
            );
            
            render_pass.set_pipeline(&self.render_pipeline);
            

            render_pass.draw_model_instanced(
                &self.mesh_model,
                &self.camera_bind_group,
                &self.light_bind_group
            );
        }

        self.queue.submit(iter::once(encoder.finish()));
        output.present();

        Ok(())
    }
}
 