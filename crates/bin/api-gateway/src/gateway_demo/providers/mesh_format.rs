use serde::Deserialize;

use super::super::artifacts::FetchedMesh;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Document {
    asset: Asset,
    buffers: [Buffer; 1],
    buffer_views: Vec<BufferView>,
    accessors: Vec<Accessor>,
    meshes: Vec<Mesh>,
    nodes: Vec<Node>,
    scenes: Vec<Scene>,
    scene: usize,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Asset {
    version: String,
    #[serde(rename = "generator")]
    _generator: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Buffer {
    byte_length: usize,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct BufferView {
    buffer: usize,
    #[serde(default)]
    byte_offset: usize,
    byte_length: usize,
    #[serde(rename = "target")]
    _target: Option<u32>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Accessor {
    buffer_view: usize,
    #[serde(default)]
    byte_offset: usize,
    component_type: u32,
    count: usize,
    #[serde(rename = "type")]
    element_type: String,
    #[serde(rename = "min")]
    _min: Option<Vec<f32>>,
    #[serde(rename = "max")]
    _max: Option<Vec<f32>>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Mesh {
    primitives: Vec<Primitive>,
    #[serde(rename = "name")]
    _name: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Primitive {
    attributes: Attributes,
    indices: Option<usize>,
    mode: Option<u8>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Attributes {
    #[serde(rename = "POSITION")]
    position: usize,
    #[serde(rename = "NORMAL")]
    normal: Option<usize>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Node {
    mesh: usize,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Scene {
    nodes: Vec<usize>,
}

pub(super) fn parse(bytes: Vec<u8>) -> Option<FetchedMesh> {
    if bytes.get(..4)? != b"glTF"
        || le_u32(bytes.get(4..8)?)? != 2
        || le_u32(bytes.get(8..12)?)? as usize != bytes.len()
    {
        return None;
    }
    let (json, remaining) = chunk(&bytes[12..], b"JSON")?;
    let (binary, remaining) = chunk(remaining, b"BIN\0")?;
    if !remaining.is_empty() || json.len() > 256 * 1024 {
        return None;
    }
    let doc: Document = serde_json::from_slice(json).ok()?;
    let buffer_length = doc.buffers[0].byte_length;
    let padding = binary.get(buffer_length..)?;
    if doc.asset.version != "2.0"
        || buffer_length == 0
        || padding.len() > 3
        || padding.iter().any(|byte| *byte != 0)
        || doc.meshes.is_empty()
        || doc.scenes.get(doc.scene)?.nodes.is_empty()
        || doc.nodes.iter().any(|node| node.mesh >= doc.meshes.len())
        || doc
            .scenes
            .iter()
            .flat_map(|scene| &scene.nodes)
            .any(|node| *node >= doc.nodes.len())
        || doc.buffer_views.iter().any(|view| {
            view.buffer != 0
                || view.byte_length == 0
                || view.byte_offset.saturating_add(view.byte_length) > buffer_length
        })
    {
        return None;
    }
    let mut primitive_count = 0;
    let mut index_budget = buffer_length;
    for mesh in &doc.meshes {
        if mesh.primitives.is_empty() {
            return None;
        }
        for primitive in &mesh.primitives {
            let positions = doc.vector_count(primitive.attributes.position)?;
            if primitive.mode.unwrap_or(4) != 4 || positions < 3 {
                return None;
            }
            if let Some(normal) = primitive.attributes.normal {
                if doc.vector_count(normal)? != positions {
                    return None;
                }
            }
            let corners = match primitive.indices {
                Some(index) => {
                    let accessor = doc.accessors.get(index)?;
                    let width = match accessor.component_type {
                        5121 => 1,
                        5123 => 2,
                        5125 => 4,
                        _ => return None,
                    };
                    if accessor.element_type != "SCALAR" || !doc.fits(accessor, width, width) {
                        return None;
                    }
                    let offset = doc.buffer_views.get(accessor.buffer_view)?.byte_offset
                        + accessor.byte_offset;
                    let length = accessor.count * width;
                    index_budget = index_budget.checked_sub(length)?;
                    if !binary
                        .get(offset..offset + length)?
                        .chunks_exact(width)
                        .all(|value| {
                            let mut index = [0; 4];
                            index[..width].copy_from_slice(value);
                            (u32::from_le_bytes(index) as usize) < positions
                        })
                    {
                        return None;
                    }
                    accessor.count
                }
                None => positions,
            };
            if corners < 3 || corners % 3 != 0 {
                return None;
            }
            primitive_count += 1;
        }
    }
    Some(FetchedMesh {
        mesh_count: doc.meshes.len(),
        primitive_count,
        bytes,
    })
}

impl Document {
    fn vector_count(&self, index: usize) -> Option<usize> {
        let accessor = self.accessors.get(index)?;
        (accessor.component_type == 5126
            && accessor.element_type == "VEC3"
            && self.fits(accessor, 4, 12))
        .then_some(accessor.count)
    }

    fn fits(&self, accessor: &Accessor, alignment: usize, element_size: usize) -> bool {
        self.buffer_views
            .get(accessor.buffer_view)
            .is_some_and(|view| {
                accessor.byte_offset % alignment == 0
                    && view.byte_offset % alignment == 0
                    && accessor
                        .byte_offset
                        .saturating_add(accessor.count.saturating_mul(element_size))
                        <= view.byte_length
            })
    }
}

fn chunk<'a>(bytes: &'a [u8], kind: &[u8; 4]) -> Option<(&'a [u8], &'a [u8])> {
    let length = le_u32(bytes.get(..4)?)? as usize;
    if length % 4 != 0 || bytes.get(4..8)? != kind {
        return None;
    }
    bytes.get(8..)?.split_at_checked(length)
}

fn le_u32(bytes: &[u8]) -> Option<u32> {
    Some(u32::from_le_bytes(bytes.try_into().ok()?))
}
