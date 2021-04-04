use crate::{
    bvh::{BoundingVolumeHierarchy, SplitMethod},
    lights::{light::Light, point_light::PointLight},
    math::{
        bounds::Bounds3,
        point::Point3,
        transform::{scale, translation, Transform},
        vector::Vec3,
    },
    camera::FoV,
    shapes::{mesh::Mesh, shape::Shape, sphere::Sphere, triangle::Triangle},
    yuki_error, yuki_info,
};

use ply_rs;
use std::{collections::HashSet, path::PathBuf, sync::Arc, time::Instant};

#[derive(Copy, Clone)]
pub struct SceneLoadSettings {
    pub max_shapes_in_node: u16,
}

impl SceneLoadSettings {
    pub fn default() -> Self {
        Self {
            max_shapes_in_node: 1,
        }
    }
}

pub struct DynamicSceneParameters {
    pub cam_pos: Point3<f32>,
    pub cam_target: Point3<f32>,
    pub cam_fov: FoV,
}

pub struct Scene {
    pub name: String,
    pub path: Option<PathBuf>,
    pub settings: SceneLoadSettings,
    pub meshes: Vec<Arc<Mesh>>,
    pub geometry: Arc<Vec<Arc<dyn Shape>>>,
    pub bvh: BoundingVolumeHierarchy,
    pub light: Arc<dyn Light>,
}

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

impl Scene {
    /// Loads a PLY, scales it to fit 2 units around the origin and orients the camera
    /// on it at an angle.
    ///
    /// Also returns the time it took to load in seconds.
    pub fn ply(
        path: &PathBuf,
        settings: SceneLoadSettings,
    ) -> Result<(Scene, DynamicSceneParameters, f32)> {
        let load_start = Instant::now();

        let mesh = load_ply(path)?;

        let triangles_start = Instant::now();
        let mut geometry: Vec<Arc<dyn Shape>> = Vec::new();
        for v0 in (0..mesh.indices.len()).step_by(3) {
            geometry.push(Arc::new(Triangle::new(
                mesh.clone(),
                v0,
                Vec3::new(1.0, 1.0, 1.0),
            )));
        }
        yuki_info!(
            "PLY: Gathered {} triangles in {:.2}s",
            geometry.len(),
            (triangles_start.elapsed().as_micros() as f32) * 1e-6
        );

        let meshes = vec![mesh];

        let (bvh, geometry_arc) = BoundingVolumeHierarchy::new(
            geometry,
            settings.max_shapes_in_node as usize,
            SplitMethod::Middle,
        );

        let light = Arc::new(PointLight::new(
            &translation(Vec3::new(5.0, 5.0, 0.0)),
            Vec3::from(600.0),
        ));

        let cam_pos = Point3::new(2.0, 2.0, 2.0);
        let cam_target = Point3::new(0.0, 0.0, 0.0);
        let cam_fov = FoV::X(40.0);

        let total_secs = (load_start.elapsed().as_micros() as f32) * 1e-6;

        yuki_info!("PLY: Loading took {:.2}s in total", total_secs);

        Ok((
            Self {
                name: path.file_stem().unwrap().to_str().unwrap().into(),
                path: Some(path.clone()),
                settings,
                meshes,
                geometry: geometry_arc,
                bvh: bvh,
                light,
            },
            DynamicSceneParameters {
                cam_pos,
                cam_target,
                cam_fov,
            },
            total_secs,
        ))
    }

    /// Constructs the Cornell box holding a tall box and a sphere
    // Lifted from http://www.graphics.cornell.edu/online/box/data.html
    pub fn cornell() -> (Scene, DynamicSceneParameters) {
        // Original uses a right-handed coordinate system so flip z
        let handedness_swap = Transform::new([
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, -1.0, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ]);

        // Materials
        // These are approximate as the originals are defined as spectrums
        let white = Vec3::from(180.0) / 255.0;
        let red = Vec3::new(180.0, 0.0, 0.0) / 255.0;
        let green = Vec3::new(0.0, 180.0, 0.0) / 255.0;

        let mut meshes: Vec<Arc<Mesh>> = Vec::new();
        let mut geometry: Vec<Arc<dyn Shape>> = Vec::new();

        // Walls
        {
            let wall_meshes = vec![
                // Floor
                Arc::new(Mesh::new(
                    &handedness_swap,
                    vec![0, 1, 2, 0, 2, 3],
                    vec![
                        Point3::new(0.0, 0.0, 0.0),
                        Point3::new(552.8, 0.0, 0.0),
                        Point3::new(549.6, 0.0, 559.2),
                        Point3::new(0.0, 0.0, 559.2),
                    ],
                )),
                // Ceiling
                Arc::new(Mesh::new(
                    &handedness_swap,
                    vec![0, 1, 2, 0, 2, 3],
                    vec![
                        Point3::new(0.0, 548.8, 0.0),
                        Point3::new(556.0, 548.8, 0.0),
                        Point3::new(556.0, 548.8, 559.2),
                        Point3::new(0.0, 548.8, 559.2),
                    ],
                )),
                // Back wall
                Arc::new(Mesh::new(
                    &handedness_swap,
                    vec![0, 1, 2, 0, 2, 3],
                    vec![
                        Point3::new(0.0, 0.0, 559.2),
                        Point3::new(549.6, 0.0, 559.2),
                        Point3::new(556.0, 548.8, 559.2),
                        Point3::new(0.0, 548.8, 559.2),
                    ],
                )),
                // Right wall
                Arc::new(Mesh::new(
                    &handedness_swap,
                    vec![0, 1, 2, 0, 2, 3],
                    vec![
                        Point3::new(0.0, 0.0, 0.0),
                        Point3::new(0.0, 0.0, 559.2),
                        Point3::new(0.0, 548.8, 559.2),
                        Point3::new(0.0, 548.8, 0.0),
                    ],
                )),
                // Left wall
                Arc::new(Mesh::new(
                    &handedness_swap,
                    vec![0, 1, 2, 0, 2, 3],
                    vec![
                        Point3::new(552.8, 0.0, 0.0),
                        Point3::new(549.6, 0.0, 559.2),
                        Point3::new(556.0, 548.8, 559.2),
                        Point3::new(556.0, 548.8, 0.0),
                    ],
                )),
            ];

            let materials = [white, white, white, green, red];
            for (mesh, material) in wall_meshes.iter().zip(materials.iter()) {
                for v0 in (0..mesh.indices.len()).step_by(3) {
                    geometry.push(Arc::new(Triangle::new(mesh.clone(), v0, *material)));
                }
            }
            meshes.extend(wall_meshes);
        }

        // Tall box
        {
            let mesh = Arc::new(Mesh::new(
                &handedness_swap,
                vec![
                    0, 1, 2, 0, 2, 3, 4, 0, 3, 4, 3, 5, 5, 3, 2, 5, 2, 6, 6, 2, 1, 6, 1, 7, 7, 1,
                    0, 7, 0, 4,
                ],
                vec![
                    Point3::new(423.0, 330.0, 247.0),
                    Point3::new(265.0, 330.0, 296.0),
                    Point3::new(314.0, 330.0, 456.0),
                    Point3::new(472.0, 330.0, 406.0),
                    Point3::new(423.0, 0.0, 247.0),
                    Point3::new(472.0, 0.0, 406.0),
                    Point3::new(314.0, 0.0, 456.0),
                    Point3::new(265.0, 0.0, 296.0),
                ],
            ));

            for v0 in (0..mesh.indices.len()).step_by(3) {
                geometry.push(Arc::new(Triangle::new(mesh.clone(), v0, white)));
            }
            meshes.push(mesh);
        }

        geometry.push(Arc::new(Sphere::new(
            &translation(Vec3::new(186.0, 82.5, -168.5)),
            82.5,
            white,
        )));

        let (bvh, geometry_arc) = BoundingVolumeHierarchy::new(geometry, 1, SplitMethod::Middle);

        let light = Arc::new(PointLight::new(
            &translation(Vec3::new(288.0, 547.0, -279.0)),
            Vec3::from(60000.0),
        ));

        let cam_pos = Point3::new(278.0, 273.0, 800.0);
        let cam_target = Point3::new(278.0, 273.0, -260.0);
        let cam_fov = FoV::X(40.0);

        (
            Scene {
                name: "Cornell Box".into(),
                path: None,
                settings: SceneLoadSettings::default(),
                meshes,
                geometry: geometry_arc,
                bvh: bvh,
                light,
            },
            DynamicSceneParameters {
                cam_pos,
                cam_target,
                cam_fov,
            },
        )
    }
}

fn load_ply(path: &PathBuf) -> Result<Arc<Mesh>> {
    let file = std::fs::File::open(path.to_str().unwrap())?;
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

    Ok(Arc::new(Mesh::new(
        &(&scale(mesh_scale, mesh_scale, mesh_scale) * &translation(-Vec3::from(mesh_center))),
        indices,
        points,
    )))
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
