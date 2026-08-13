use bytemuck::{Pod, Zeroable};
use rustit_geometry::{Mesh, Point3};
use std::{error::Error, io, sync::Arc};
use wgpu::util::DeviceExt;
use winit::{dpi::PhysicalSize, window::Window};

const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

pub(crate) struct Renderer {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    pipeline: wgpu::RenderPipeline,
    world_vertices: Vec<GpuVertex>,
    scene_frame: SceneFrame,
    vertex_buffer: wgpu::Buffer,
    vertex_count: u32,
    depth_view: wgpu::TextureView,
}

impl Renderer {
    pub(crate) async fn new(window: Arc<Window>, wall_mesh: &Mesh) -> Result<Self, Box<dyn Error>> {
        let size = nonzero_size(window.inner_size());
        let instance = wgpu::Instance::default();
        let surface = instance.create_surface(window)?;
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await?;
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("Rustit render device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                memory_hints: wgpu::MemoryHints::Performance,
                trace: wgpu::Trace::Off,
            })
            .await?;
        let config = surface
            .get_default_config(&adapter, size.width, size.height)
            .ok_or_else(|| io::Error::other("GPU cannot present to the Rustit window"))?;
        surface.configure(&device, &config);

        let world_vertices = scene_vertices(wall_mesh)?;
        let scene_frame = SceneFrame::from_mesh(wall_mesh);
        let vertices = project_vertices(&world_vertices, scene_frame, size);
        let vertex_count = u32::try_from(vertices.len())?;
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Rustit semantic wall vertices"),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        });

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Rustit semantic wall shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Rustit wall pipeline layout"),
            bind_group_layouts: &[],
            push_constant_ranges: &[],
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Rustit wall pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vertex_main"),
                compilation_options: Default::default(),
                buffers: &[GpuVertex::layout()],
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                cull_mode: None,
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: DEPTH_FORMAT,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: Default::default(),
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fragment_main"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview: None,
            cache: None,
        });
        let depth_view = create_depth_view(&device, size);

        Ok(Self {
            surface,
            device,
            queue,
            config,
            pipeline,
            world_vertices,
            scene_frame,
            vertex_buffer,
            vertex_count,
            depth_view,
        })
    }

    pub(crate) fn resize(&mut self, size: PhysicalSize<u32>) {
        if size.width == 0 || size.height == 0 {
            return;
        }
        self.config.width = size.width;
        self.config.height = size.height;
        self.surface.configure(&self.device, &self.config);
        self.depth_view = create_depth_view(&self.device, size);
        let vertices = project_vertices(&self.world_vertices, self.scene_frame, size);
        self.queue
            .write_buffer(&self.vertex_buffer, 0, bytemuck::cast_slice(&vertices));
    }

    pub(crate) fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        let frame = self.surface.get_current_texture()?;
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Rustit frame encoder"),
            });

        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Rustit semantic wall pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.025,
                            g: 0.035,
                            b: 0.055,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            pass.set_pipeline(&self.pipeline);
            pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            pass.draw(0..self.vertex_count, 0..1);
        }

        self.queue.submit(Some(encoder.finish()));
        frame.present();
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct SceneFrame {
    center: [f32; 3],
    half_view_height: f32,
}

impl SceneFrame {
    fn from_mesh(mesh: &Mesh) -> Self {
        let mut minimum = [f32::INFINITY; 3];
        let mut maximum = [f32::NEG_INFINITY; 3];
        for point in &mesh.vertices {
            let point = [point.x as f32, point.y as f32, point.z as f32];
            for axis in 0..3 {
                minimum[axis] = minimum[axis].min(point[axis]);
                maximum[axis] = maximum[axis].max(point[axis]);
            }
        }
        if mesh.vertices.is_empty() {
            minimum = [-1.0; 3];
            maximum = [1.0; 3];
        }
        let span = [
            maximum[0] - minimum[0],
            maximum[1] - minimum[1],
            maximum[2] - minimum[2],
        ];
        let half_view_height = (span[0] * 0.6)
            .max(span[1] * 0.6)
            .max(span[2] * 0.8)
            .max(1.0);

        Self {
            center: [
                (minimum[0] + maximum[0]) * 0.5,
                (minimum[1] + maximum[1]) * 0.5,
                (minimum[2] + maximum[2]) * 0.5,
            ],
            half_view_height,
        }
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
struct GpuVertex {
    position: [f32; 3],
    normal: [f32; 3],
    color: [f32; 3],
}

impl GpuVertex {
    const ATTRIBUTES: [wgpu::VertexAttribute; 3] = wgpu::vertex_attr_array![
        0 => Float32x3,
        1 => Float32x3,
        2 => Float32x3
    ];

    fn layout() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBUTES,
        }
    }
}

fn scene_vertices(mesh: &Mesh) -> Result<Vec<GpuVertex>, Box<dyn Error>> {
    let mut vertices = Vec::with_capacity(mesh.triangles.len() * 3 + 6);
    for triangle in &mesh.triangles {
        let point = |index: u32| {
            mesh.vertices
                .get(index as usize)
                .copied()
                .ok_or_else(|| io::Error::other("wall mesh contains an invalid triangle index"))
        };
        let points = [
            point(triangle[0])?,
            point(triangle[1])?,
            point(triangle[2])?,
        ];
        append_triangle(&mut vertices, points, [0.88, 0.38, 0.10]);
    }

    append_triangle(
        &mut vertices,
        [
            Point3::new(-2.0, -4.0, -0.01),
            Point3::new(8.0, -4.0, -0.01),
            Point3::new(8.0, 4.0, -0.01),
        ],
        [0.16, 0.19, 0.23],
    );
    append_triangle(
        &mut vertices,
        [
            Point3::new(-2.0, -4.0, -0.01),
            Point3::new(8.0, 4.0, -0.01),
            Point3::new(-2.0, 4.0, -0.01),
        ],
        [0.16, 0.19, 0.23],
    );
    Ok(vertices)
}

fn project_vertices(
    world_vertices: &[GpuVertex],
    frame: SceneFrame,
    size: PhysicalSize<u32>,
) -> Vec<GpuVertex> {
    let camera_to_target = normalize([-5.0, 10.0, -5.5]);
    let right = normalize(cross(camera_to_target, [0.0, 0.0, 1.0]));
    let up = normalize(cross(right, camera_to_target));
    let depth_axis = [
        -camera_to_target[0],
        -camera_to_target[1],
        -camera_to_target[2],
    ];
    let aspect = aspect_ratio(size);

    world_vertices
        .iter()
        .map(|vertex| {
            let centered = [
                vertex.position[0] - frame.center[0],
                vertex.position[1] - frame.center[1],
                vertex.position[2] - frame.center[2],
            ];
            GpuVertex {
                position: [
                    dot(centered, right) / (frame.half_view_height * aspect),
                    dot(centered, up) / frame.half_view_height,
                    0.5 - dot(centered, depth_axis) / 20.0,
                ],
                ..*vertex
            }
        })
        .collect()
}

fn append_triangle(vertices: &mut Vec<GpuVertex>, points: [Point3; 3], color: [f32; 3]) {
    let ab = subtract(points[1], points[0]);
    let ac = subtract(points[2], points[0]);
    let normal = normalize(cross(ab, ac));
    vertices.extend(points.map(|point| GpuVertex {
        position: [point.x as f32, point.y as f32, point.z as f32],
        normal,
        color,
    }));
}

fn subtract(a: Point3, b: Point3) -> [f32; 3] {
    [(a.x - b.x) as f32, (a.y - b.y) as f32, (a.z - b.z) as f32]
}

fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn dot(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn normalize(vector: [f32; 3]) -> [f32; 3] {
    let length = (vector[0] * vector[0] + vector[1] * vector[1] + vector[2] * vector[2]).sqrt();
    if length <= f32::EPSILON {
        return [0.0, 0.0, 1.0];
    }
    [vector[0] / length, vector[1] / length, vector[2] / length]
}

fn nonzero_size(size: PhysicalSize<u32>) -> PhysicalSize<u32> {
    PhysicalSize::new(size.width.max(1), size.height.max(1))
}

fn aspect_ratio(size: PhysicalSize<u32>) -> f32 {
    size.width as f32 / size.height.max(1) as f32
}

fn create_depth_view(device: &wgpu::Device, size: PhysicalSize<u32>) -> wgpu::TextureView {
    device
        .create_texture(&wgpu::TextureDescriptor {
            label: Some("Rustit depth texture"),
            size: wgpu::Extent3d {
                width: size.width,
                height: size.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: DEPTH_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        })
        .create_view(&wgpu::TextureViewDescriptor::default())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expands_neutral_mesh_triangles_and_adds_a_ground_plane() {
        let mesh = Mesh {
            vertices: vec![
                Point3::new(0.0, 0.0, 0.0),
                Point3::new(1.0, 0.0, 0.0),
                Point3::new(0.0, 0.0, 1.0),
            ],
            triangles: vec![[0, 1, 2]],
        };

        let vertices = scene_vertices(&mesh).expect("scene vertices");

        assert_eq!(vertices.len(), 9);
        assert!(vertices.iter().all(|vertex| {
            vertex.position.iter().all(|value| value.is_finite())
                && vertex.normal.iter().all(|value| value.is_finite())
        }));
    }

    #[test]
    fn rejects_an_invalid_neutral_mesh_index() {
        let mesh = Mesh {
            vertices: vec![Point3::new(0.0, 0.0, 0.0)],
            triangles: vec![[0, 1, 2]],
        };

        assert!(scene_vertices(&mesh).is_err());
    }

    #[test]
    fn scene_frame_centers_and_fits_the_authored_mesh() {
        let mesh = Mesh {
            vertices: vec![Point3::new(0.0, -0.1, 0.0), Point3::new(6.0, 0.1, 3.0)],
            triangles: vec![],
        };

        let scene = SceneFrame::from_mesh(&mesh);

        assert_eq!(scene.center, [3.0, 0.0, 1.5]);
        assert!((scene.half_view_height - 3.6).abs() < f32::EPSILON * 4.0);
        let center = GpuVertex {
            position: scene.center,
            normal: [0.0, 0.0, 1.0],
            color: [1.0; 3],
        };
        let projected = project_vertices(&[center], scene, PhysicalSize::new(1100, 720));
        assert_eq!(projected[0].position, [0.0, 0.0, 0.5]);
    }
}
