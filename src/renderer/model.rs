use std::{collections::HashMap, error::Error, path::Path};

use tobj::LoadOptions;

use super::types::Vertex;

pub struct Texture {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
}

pub struct Model {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
    pub texture: Texture,
}

impl Model {
    pub fn new(obj: &str, texture: &str, triangulate: bool) -> Result<Self, Box<dyn Error>> {
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

        let image = image::open(Path::new(texture))?;
        let pixels = image.to_rgba8().into_raw();

        Ok(Model {
            vertices,
            indices,
            texture: Texture {
                width: image.width(),
                height: image.height(),
                pixels,
            },
        })
    }
}
