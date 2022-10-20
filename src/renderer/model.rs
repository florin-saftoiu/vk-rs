use std::{collections::HashMap, error::Error, path::Path};

use ash::vk;
use cgmath::Point3;
use tobj::LoadOptions;

use super::{types::Vertex, Renderer};

pub struct Texture {
    width: u32,
    height: u32,
    pixels: Vec<u8>,
}

impl Texture {
    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn pixels(&self) -> &[u8] {
        &self.pixels
    }
}

pub struct Model {
    _vertices: Vec<Vertex>,
    indices: Vec<u32>,
    vertex_buffer: vk::Buffer,
    vertex_buffer_memory: vk::DeviceMemory,
    index_buffer: vk::Buffer,
    index_buffer_memory: vk::DeviceMemory,
    _texture: Texture,
    texture_image: vk::Image,
    texture_image_memory: vk::DeviceMemory,
    texture_image_view: vk::ImageView,
    uniform_buffers: Vec<vk::Buffer>,
    uniform_buffers_memory: Vec<vk::DeviceMemory>,
    descriptor_sets: Vec<vk::DescriptorSet>,
    pub position: Point3<f32>,
    pub theta: f32,
}

impl Model {
    pub fn _vertices(&self) -> &[Vertex] {
        &self._vertices
    }

    pub fn indices(&self) -> &[u32] {
        &self.indices
    }

    pub fn vertex_buffer(&self) -> vk::Buffer {
        self.vertex_buffer
    }

    pub fn vertex_buffer_memory(&self) -> vk::DeviceMemory {
        self.vertex_buffer_memory
    }

    pub fn index_buffer(&self) -> vk::Buffer {
        self.index_buffer
    }

    pub fn index_buffer_memory(&self) -> vk::DeviceMemory {
        self.index_buffer_memory
    }

    pub fn _texture(&self) -> &Texture {
        &self._texture
    }

    pub fn texture_image(&self) -> vk::Image {
        self.texture_image
    }

    pub fn texture_image_memory(&self) -> vk::DeviceMemory {
        self.texture_image_memory
    }

    pub fn texture_image_view(&self) -> vk::ImageView {
        self.texture_image_view
    }

    pub fn descriptor_sets(&self) -> &[vk::DescriptorSet] {
        &self.descriptor_sets
    }

    pub fn uniform_buffers(&self) -> &[vk::Buffer] {
        &self.uniform_buffers
    }

    pub fn uniform_buffers_memory(&self) -> &[vk::DeviceMemory] {
        &self.uniform_buffers_memory
    }

    pub fn new(
        renderer: &Renderer,
        obj: &str,
        texture: &str,
        triangulate: bool,
    ) -> Result<Self, Box<dyn Error>> {
        let mut vertices = vec![];
        let mut indices = vec![];
        let mut unique_vertices = HashMap::new();

        let load_options = LoadOptions {
            triangulate, // enable if model is not composed of triangles only
            ..Default::default()
        };
        let (models, _) = tobj::load_obj(obj, &load_options)?;
        for model in models.iter() {
            let mesh = &model.mesh;
            for (&index, &tex_index) in mesh.indices.iter().zip(mesh.texcoord_indices.iter()) {
                let vertex = Vertex {
                    pos: [
                        mesh.positions[3 * index as usize + 0],
                        mesh.positions[3 * index as usize + 1],
                        mesh.positions[3 * index as usize + 2],
                    ],
                    color: [1.0, 1.0, 1.0],
                    tex_coord: [
                        mesh.texcoords[2 * tex_index as usize + 0],
                        1.0 - mesh.texcoords[2 * tex_index as usize + 1],
                    ],
                };
                if let Some(i) = unique_vertices.get(&vertex) {
                    indices.push(*i as u32);
                } else {
                    let i = vertices.len();
                    unique_vertices.insert(vertex, i);
                    vertices.push(vertex);
                    indices.push(i as u32)
                }
            }
        }

        let (vertex_buffer, vertex_buffer_memory) = renderer.create_vertex_buffer(&vertices)?;
        let (index_buffer, index_buffer_memory) = renderer.create_index_buffer(&indices)?;

        let image = image::open(Path::new(texture))?;
        let pixels = image.to_rgba8().into_raw();

        let texture = Texture {
            width: image.width(),
            height: image.height(),
            pixels,
        };

        let (texture_image, texture_image_memory) = renderer.create_texture_image(&texture)?;
        let texture_image_view = renderer.create_texture_image_view(texture_image)?;
        let (uniform_buffers, uniform_buffers_memory) = renderer.create_model_uniform_buffers()?;
        let descriptor_sets =
            renderer.create_model_descriptor_sets(&uniform_buffers, texture_image_view)?;

        Ok(Model {
            _vertices: vertices,
            indices,
            vertex_buffer,
            vertex_buffer_memory,
            index_buffer,
            index_buffer_memory,
            _texture: texture,
            texture_image,
            texture_image_memory,
            texture_image_view,
            uniform_buffers,
            uniform_buffers_memory,
            descriptor_sets,
            position: Point3 {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            theta: 0.0,
        })
    }
}
