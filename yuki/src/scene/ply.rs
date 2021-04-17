use super::Result;
use crate::{
    math::{
        transform::{scale, translation},
        Bounds3, Point3, Transform, Vec3,
    },
    shapes::{Mesh, Shape, Triangle},
    yuki_error, yuki_info,
};

use ply_rs;
use std::{collections::HashSet, path::PathBuf, sync::Arc, time::Instant};

pub fn load(
    path: &PathBuf,
    albedo: Vec3<f32>,
    transform: Option<Transform<f32>>,
) -> Result<(Arc<Mesh>, Vec<Arc<dyn Shape>>)> {
    let file = match std::fs::File::open(path.to_str().unwrap()) {
        Ok(f) => f,
        Err(e) => {
            yuki_error!("Could not open '{}'", path.to_string_lossy());
            return Err(e.into());
        }
    };
    let mut file_buf = std::io::BufReader::new(file);

    let header =
        ply_rs::parser::Parser::<ply_rs::ply::DefaultElement>::new().read_header(&mut file_buf)?;

    if !is_valid(&header) {
        return Err("PLY: Unsupported content".into());
    }

    let vertices_start = Instant::now();
    let vertex_parser = ply_rs::parser::Parser::<Vertex>::new();
    let vertices = vertex_parser.read_payload_for_element(
        &mut file_buf,
        &header.elements["vertex"],
        &header,
    )?;
    yuki_info!(
        "PLY: Parsed {} vertices in {:.2}s",
        vertices.len(),
        (vertices_start.elapsed().as_micros() as f32) * 1e-6
    );

    let faces_start = Instant::now();
    let face_parser = ply_rs::parser::Parser::<Face>::new();
    let faces =
        face_parser.read_payload_for_element(&mut file_buf, &header.elements["face"], &header)?;
    yuki_info!(
        "PLY: Parsed {} faces in {:.2}s",
        faces.len(),
        (faces_start.elapsed().as_micros() as f32) * 1e-6
    );

    let points_start = Instant::now();
    let points: Vec<Point3<f32>> = vertices
        .iter()
        .map(|&Vertex { x, y, z }| Point3::new(x, y, z))
        .collect();
    yuki_info!(
        "PLY: Converted vertices to points in {:.2}s",
        (points_start.elapsed().as_micros() as f32) * 1e-6
    );

    let indices_start = Instant::now();
    let mut indices = Vec::new();
    for f in faces {
        let v0 = f.indices[0];
        let mut is = f.indices.iter().skip(1).peekable();
        while let Some(&v1) = is.next() {
            if let Some(&&v2) = is.peek() {
                indices.push(v0);
                indices.push(v1);
                indices.push(v2);
            }
        }
    }
    yuki_info!(
        "PLY: Converted faces to an index buffer in {:.2}s",
        (indices_start.elapsed().as_micros() as f32) * 1e-6
    );

    // Find bounds and transform to fit in (-1,-1,-1),(1,1,1) in world space
    let bb = points
        .iter()
        .fold(Bounds3::default(), |bb, &p| bb.union_p(p));
    let mesh_center = bb.p_min + bb.diagonal() / 2.0;
    let mesh_scale = 1.0 / bb.diagonal().max_comp();

    let trfn = transform.unwrap_or(
        &scale(mesh_scale, mesh_scale, mesh_scale) * &translation(-Vec3::from(mesh_center)),
    );
    let mesh = Arc::new(Mesh::new(&trfn, indices, points));

    let triangles_start = Instant::now();
    let mut geometry: Vec<Arc<dyn Shape>> = Vec::new();
    for v0 in (0..mesh.indices.len()).step_by(3) {
        geometry.push(Arc::new(Triangle::new(mesh.clone(), v0, albedo)));
    }
    yuki_info!(
        "PLY: Gathered {} triangles in {:.2}s",
        geometry.len(),
        (triangles_start.elapsed().as_micros() as f32) * 1e-6
    );

    return Ok((mesh, geometry));
}

struct PlyContent {
    vertex: Option<HashSet<String>>,
    face: Option<HashSet<String>>,
}

impl PlyContent {
    fn new() -> Self {
        Self {
            vertex: None,
            face: None,
        }
    }
}

fn is_valid(header: &ply_rs::ply::Header) -> bool {
    let mut content = PlyContent::new();
    for (name, element) in &header.elements {
        match name.as_str() {
            "vertex" => {
                let mut props = HashSet::new();
                for (name, _) in &element.properties {
                    props.insert(name.clone());
                }
                content.vertex = Some(props)
            }
            "face" => {
                let mut props = HashSet::new();
                for (name, _) in &element.properties {
                    props.insert(name.clone());
                }
                content.face = Some(props)
            }
            _ => yuki_info!("PLY: Unknown element '{}'", name),
        }
    }

    let mut valid = true;

    if let Some(props) = content.vertex {
        let expected_vert_props = vec!["x", "y", "z"];
        for p in &expected_vert_props {
            if !props.contains(&p.to_string()) {
                yuki_error!("PLY: Element 'vertex' missing property '{}'", p);
                valid = false;
            }
        }
        for p in props.difference(&expected_vert_props.iter().map(|p| p.to_string()).collect()) {
            yuki_info!("PLY: Unknown 'vertex' property '{}'", p)
        }
    } else {
        yuki_error!("PLY: Missing element 'vertex'");
        valid = false;
    }

    if let Some(props) = content.face {
        // For some reason (Paul Bourke's example?), PLYs come with one of two different names
        // for face indices
        if !props.contains(&String::from("vertex_index"))
            && !props.contains(&String::from("vertex_indices"))
        {
            yuki_error!(
                "PLY: Elemnent 'face' should have either 'vertex_index' or 'vertex_indices'"
            );
            valid = false;
        }
        for p in props {
            match p.as_str() {
                "vertex_index" | "vertex_indices" => (),
                _ => yuki_info!("PLY: Unknown 'face' property '{}'", p),
            }
        }
    } else {
        yuki_error!("PLY: Missing element 'face'");
        valid = false;
    }

    valid
}

struct Vertex {
    x: f32,
    y: f32,
    z: f32,
}

impl ply_rs::ply::PropertyAccess for Vertex {
    fn new() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        }
    }

    fn set_property(&mut self, key: String, property: ply_rs::ply::Property) {
        match property {
            ply_rs::ply::Property::Float(v) => match key.as_str() {
                "x" => self.x = v,
                "y" => self.y = v,
                "z" => self.z = v,
                _ => (),
            },
            _ => (),
        }
    }
}

struct Face {
    indices: Vec<usize>,
}

impl ply_rs::ply::PropertyAccess for Face {
    fn new() -> Self {
        Self {
            indices: Vec::new(),
        }
    }

    fn set_property(&mut self, key: String, property: ply_rs::ply::Property) {
        match property {
            ply_rs::ply::Property::ListInt(v) => match key.as_str() {
                // For some reason (Paul Bourke's example?), PLYs come with one of two different
                // names for face indices
                "vertex_index" | "vertex_indices" => {
                    self.indices = v.iter().map(|&i| i as usize).collect()
                }
                _ => (),
            },
            _ => (),
        }
    }
}
