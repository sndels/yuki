use super::Result;
use crate::{
    materials::Material,
    math::{
        transforms::{scale, translation},
        Bounds3, Normal, Point3, Transform, Vec3,
    },
    shapes::{Mesh, Shape, Triangle},
    yuki_error, yuki_info,
};

use std::{collections::HashSet, convert::TryFrom, path::Path, sync::Arc, time::Instant};

pub struct PlyResult {
    pub mesh: Arc<Mesh>,
    pub shapes: Vec<Arc<dyn Shape>>,
}

pub fn load(
    path: &Path,
    material: &Arc<dyn Material>,
    transform: Option<Transform<f32>>,
) -> Result<PlyResult> {
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

    let vertices_start = Instant::now();
    let mut points = Vec::new();
    let mut normals = Vec::new();
    for Vertex { point, normal } in vertices {
        points.push(point);
        if let Some(n) = normal {
            normals.push(n);
        }
    }
    yuki_info!(
        "PLY: Extracted vertex attributes in {:.2}s",
        (vertices_start.elapsed().as_micros() as f32) * 1e-6
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
    let mesh = Arc::new(Mesh::new(&trfn, indices, points, normals));

    let triangles_start = Instant::now();
    let shapes: Vec<Arc<dyn Shape>> = (0..mesh.indices.len())
        .step_by(3)
        .map(|v0| {
            Arc::new(Triangle::new(Arc::clone(&mesh), v0, Arc::clone(material))) as Arc<dyn Shape>
        })
        .collect();
    yuki_info!(
        "PLY: Gathered {} triangles in {:.2}s",
        shapes.len(),
        (triangles_start.elapsed().as_micros() as f32) * 1e-6
    );

    Ok(PlyResult { mesh, shapes })
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
                content.vertex = Some(props);
            }
            "face" => {
                let mut props = HashSet::new();
                for (name, _) in &element.properties {
                    props.insert(name.clone());
                }
                content.face = Some(props);
            }
            _ => yuki_info!("PLY: Unknown element '{}'", name),
        }
    }

    let mut valid = true;

    if let Some(props) = content.vertex {
        let expected_vert_props = vec!["x", "y", "z"];
        for p in &expected_vert_props {
            if !props.contains(&(*p).to_string()) {
                yuki_error!("PLY: Element 'vertex' missing property '{}'", p);
                valid = false;
            }
        }
        for p in props.difference(
            &expected_vert_props
                .iter()
                .map(|p| (*p).to_string())
                .collect(),
        ) {
            yuki_info!("PLY: Unknown 'vertex' property '{}'", p);
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
    point: Point3<f32>,
    normal: Option<Normal<f32>>,
}

impl ply_rs::ply::PropertyAccess for Vertex {
    fn new() -> Self {
        Self {
            point: Point3::zeros(),
            normal: None,
        }
    }

    fn set_property(&mut self, key: String, property: ply_rs::ply::Property) {
        if let ply_rs::ply::Property::Float(v) = property {
            match key.as_str() {
                "x" => self.point.x = v,
                "y" => self.point.y = v,
                "z" => self.point.z = v,
                // TODO: Do relevant plys have nx first?
                "nx" => {
                    self.normal = Some(Normal::new(0.0, 0.0, 0.0));
                    self.normal.as_mut().unwrap().x = v;
                }
                "ny" => self.normal.as_mut().unwrap().y = v,
                "nz" => self.normal.as_mut().unwrap().z = v,
                _ => (),
            }
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
        match key.as_str() {
            // For some reason (Paul Bourke's example?), PLYs come with one of two different
            // names for face indices
            "vertex_index" | "vertex_indices" => match property {
                ply_rs::ply::Property::ListInt(v) => {
                    self.indices = v
                        .iter()
                        .map(|&i| usize::try_from(i).expect("Negative PLY index"))
                        .collect();
                }
                ply_rs::ply::Property::ListUInt(v) => {
                    self.indices = v.iter().map(|&i| i as usize).collect();
                }
                _ => (),
            },
            _ => (),
        }
    }
}
