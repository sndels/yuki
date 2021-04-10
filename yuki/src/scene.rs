use crate::{
    bvh::{BoundingVolumeHierarchy, SplitMethod},
    camera::FoV,
    lights::{light::Light, point_light::PointLight, spot_light::SpotLight},
    math::{
        bounds::Bounds3,
        point::Point3,
        transform::{rotation, rotation_y, scale, translation, Transform},
        vector::Vec3,
    },
    shapes::{mesh::Mesh, shape::Shape, sphere::Sphere, triangle::Triangle},
    yuki_error, yuki_info, yuki_trace,
};

use approx::relative_eq;
use ply_rs;
use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
    sync::Arc,
    time::Instant,
};
use xml::reader::{EventReader, XmlEvent};

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

pub enum CameraOrientation {
    Pose {
        cam_pos: Point3<f32>,
        cam_euler_deg: Vec3<f32>,
    },
    LookAt {
        cam_pos: Point3<f32>,
        cam_target: Point3<f32>,
    },
}

pub struct DynamicSceneParameters {
    pub cam_orientation: CameraOrientation,
    pub cam_fov: FoV,
}

impl DynamicSceneParameters {
    fn new() -> Self {
        Self {
            cam_orientation: CameraOrientation::LookAt {
                cam_pos: Point3::new(0.0, 0.0, 0.0),
                cam_target: Point3::new(0.0, 0.0, 0.0),
            },
            cam_fov: FoV::X(0.0),
        }
    }
}

pub struct Scene {
    pub name: String,
    pub path: Option<PathBuf>,
    pub settings: SceneLoadSettings,
    pub meshes: Vec<Arc<Mesh>>,
    pub geometry: Arc<Vec<Arc<dyn Shape>>>,
    pub bvh: BoundingVolumeHierarchy,
    pub lights: Vec<Arc<dyn Light>>,
    pub background: Vec3<f32>,
}

macro_rules! try_find_attr {
    ($attributes:expr, $name_str:expr) => {{
        let mut value = None;
        for attr in $attributes {
            if attr.name.local_name.as_str() == $name_str {
                value = Some(&attr.value);
            }
        }
        value
    }};
}

macro_rules! find_attr {
    ($attributes:expr, $name_str:expr) => {{
        match try_find_attr!($attributes, $name_str) {
            Some(v) => v,
            None => return Err(format!("Could not find element attribute '{}'", $name_str).into()),
        }
    }};
}

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

impl Scene {
    /// Loads a Mitsuba 2 scene
    ///
    /// Also returns the time it took to load in seconds.
    pub fn mitsuba(
        path: &PathBuf,
        settings: SceneLoadSettings,
    ) -> Result<(Scene, DynamicSceneParameters, f32)> {
        let load_start = Instant::now();

        let dir_path = path.parent().unwrap().to_path_buf();
        let file = std::fs::File::open(path.to_str().unwrap())?;
        let file_buf = std::io::BufReader::new(file);

        let mut meshes = Vec::new();
        let mut geometry = Vec::new();
        let mut materials: HashMap<String, Vec3<f32>> = HashMap::new();
        let mut lights: Vec<Arc<dyn Light>> = Vec::new();
        let mut background = Vec3::from(0.0);
        let mut scene_params = DynamicSceneParameters::new();
        let mut parser = EventReader::new(file_buf);
        let mut indent = String::new();
        let mut ignore_level: Option<u32> = None;
        loop {
            match parser.next() {
                Ok(evt) => match evt {
                    XmlEvent::StartDocument {
                        version,
                        encoding,
                        standalone,
                    } => yuki_trace!(
                        " Start document: XML - {}, encoding - {}, standalone {:?}",
                        version,
                        encoding,
                        standalone
                    ),
                    XmlEvent::StartElement {
                        name, attributes, ..
                    } => {
                        // Extra space to account for line number in log
                        yuki_trace!(" {}Begin: {}", indent, name);
                        indent += "  ";
                        yuki_trace!(" {}Attributes", indent);
                        indent += "  ";
                        for xml::attribute::OwnedAttribute { name, value } in &attributes {
                            // Extra space to account for line number in log
                            yuki_trace!(" {}{}: {}", indent, name, value);
                        }
                        indent.truncate(indent.len() - 2);

                        if let None = ignore_level {
                            match name.local_name.as_str() {
                                "scene" => {
                                    if find_attr!(&attributes, "version").as_str() != "2.1.0" {
                                        return Err("Scene file version is not 2.1.0".into());
                                    }
                                }
                                "default" => {
                                    // TODO
                                    ()
                                }
                                "integrator" => {
                                    ignore_level = Some(0);
                                }
                                "sensor" => {
                                    (scene_params.cam_orientation, scene_params.cam_fov) =
                                        parse_sensor(&mut parser, indent.clone())?;
                                    indent.truncate(indent.len() - 2);
                                }
                                "bsdf" => {
                                    let id = find_attr!(&attributes, "id").clone();
                                    let material = parse_material(&mut parser, indent.clone())?;
                                    indent.truncate(indent.len() - 2);
                                    materials.insert(id, material);
                                }
                                "emitter" => {
                                    let attr_type = find_attr!(&attributes, "type");
                                    match attr_type.as_str() {
                                        "constant" => {
                                            background =
                                                parse_constant_emitter(&mut parser, indent.clone())?
                                        }
                                        "point" => {
                                            lights.push(parse_point_light(
                                                &mut parser,
                                                indent.clone(),
                                            )?);
                                        }
                                        "spot" => {
                                            lights.push(parse_spot_light(
                                                &mut parser,
                                                indent.clone(),
                                            )?);
                                        }
                                        _ => ignore_level = Some(0),
                                    }
                                }
                                "shape" => {
                                    let (mesh, geom) = parse_shape(
                                        &dir_path,
                                        &materials,
                                        attributes,
                                        &mut parser,
                                        indent.clone(),
                                    )?;
                                    if let Some(m) = mesh {
                                        meshes.push(m);
                                    }
                                    geometry.extend(geom);
                                    indent.truncate(indent.len() - 2);
                                }
                                name => return Err(format!("Unknown element: '{}'", name).into()),
                            }
                        }

                        if let Some(l) = ignore_level {
                            yuki_trace!("{}Ignored", indent);
                            ignore_level = Some(l + 1);
                        }
                    }
                    XmlEvent::EndElement { name } => {
                        indent.truncate(indent.len() - 2);

                        yuki_trace!("{}End: {}", indent, name);

                        if let Some(l) = ignore_level {
                            let level_after = l - 1;
                            if level_after > 0 {
                                ignore_level = Some(l - 1);
                            } else {
                                ignore_level = None;
                            }
                        }
                    }
                    XmlEvent::ProcessingInstruction { name, .. } => {
                        return Err(format!("Unexpected processing instruction: {}", name).into())
                    }
                    XmlEvent::CData(data) => {
                        return Err(format!("Unexpected CDATA: {}", data).into())
                    }
                    XmlEvent::Comment(_) => (),
                    XmlEvent::Characters(chars) => {
                        return Err(format!("Unexpected characters outside tags: {}", chars).into())
                    }
                    XmlEvent::Whitespace(_) => (),
                    XmlEvent::EndDocument => {
                        yuki_trace!("End document");
                        break;
                    }
                },
                Err(err) => {
                    yuki_error!("XML error: {}", err);
                    break;
                }
            }
        }

        let (bvh, geometry_arc) = BoundingVolumeHierarchy::new(
            geometry,
            settings.max_shapes_in_node as usize,
            SplitMethod::Middle,
        );

        let total_secs = (load_start.elapsed().as_micros() as f32) * 1e-6;

        yuki_info!("Mitsuba 2.0: Loading took {:.2}s in total", total_secs);

        Ok((
            Scene {
                name: path.file_stem().unwrap().to_str().unwrap().into(),
                path: Some(path.clone()),
                settings: SceneLoadSettings::default(),
                meshes,
                geometry: geometry_arc,
                bvh: bvh,
                lights,
                background,
            },
            scene_params,
            total_secs,
        ))
    }

    ///
    /// Loads a PLY, scales it to fit 2 units around the origin and orients the camera
    /// on it at an angle.
    ///
    /// Also returns the time it took to load in seconds.
    pub fn ply(
        path: &PathBuf,
        settings: SceneLoadSettings,
    ) -> Result<(Scene, DynamicSceneParameters, f32)> {
        let load_start = Instant::now();

        let (mesh, geometry) = load_ply(path, Vec3::from(1.0), None)?;

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
                lights: vec![light],
                background: Vec3::from(0.0),
            },
            DynamicSceneParameters {
                cam_orientation: CameraOrientation::LookAt {
                    cam_pos,
                    cam_target,
                },
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
                lights: vec![light],
                background: Vec3::from(0.0),
            },
            DynamicSceneParameters {
                cam_orientation: CameraOrientation::LookAt {
                    cam_pos,
                    cam_target,
                },
                cam_fov,
            },
        )
    }
}

/// Pumps messages, calling start_body for each StartElement.
/// 'return's errors for unexpected data blocks.
/// Breaks when an unmatched EndElement is encountered.
///
/// start_body has a signature of (name: &OwnedName, attributes: Vec<OwnedAttribute>, ignore_level: &mut Option<u32>) -> Result<()>
/// 'ignore_level = Some(0)' can be set to skip the current element and it's children.
/// 'level' should be decremented after a recursive parser call returns to match the correct level (caller won't see EndElement)
macro_rules! parse_element {
    ($parser:ident, $indent:ident, $start_body:expr) => {
        let mut level = 0i32;
        let mut ignore_level: Option<u32> = None;
        loop {
            match $parser.next() {
                Ok(evt) => match evt {
                    XmlEvent::StartDocument { .. } => unreachable!(),
                    XmlEvent::StartElement {
                        name, attributes, ..
                    } => {
                        if let None = ignore_level {
                            yuki_trace!("{}Begin: {}", $indent, name);
                            $indent += "  ";
                            yuki_trace!("{}Attributes", $indent);
                            $indent += "  ";
                            for xml::attribute::OwnedAttribute { name, value } in &attributes {
                                yuki_trace!("{}{}: {}", $indent, name, value);
                            }
                            $indent.truncate($indent.len() - 2);
                        }

                        if let None = ignore_level {
                            $start_body(&name, attributes, &mut level, &mut ignore_level)?;
                        }

                        level += 1;

                        if let Some(l) = ignore_level {
                            if l == 0 {
                                yuki_info!("Element '{}' ignored", name);
                            }
                            ignore_level = Some(l + 1);
                        }
                    }
                    XmlEvent::EndElement { name } => {
                        if let Some(l) = ignore_level {
                            let level_after = l - 1;
                            if level_after > 0 {
                                ignore_level = Some(l - 1);
                            } else {
                                ignore_level = None;
                            }
                        }

                        if ignore_level == None || ignore_level == Some(0) {
                            $indent.truncate($indent.len() - 2);
                            yuki_trace!("{}End: {}", $indent, name);
                        }

                        level -= 1;
                        if level < 0 {
                            break;
                        }
                    }
                    XmlEvent::ProcessingInstruction { name, .. } => {
                        return Err(format!("Unexpected processing instruction: {}", name).into())
                    }
                    XmlEvent::CData(data) => {
                        return Err(format!("Unexpected CDATA: {}", data).into())
                    }
                    XmlEvent::Comment(_) => (),
                    XmlEvent::Characters(chars) => {
                        return Err(format!("Unexpected characters outside tags: {}", chars).into())
                    }
                    XmlEvent::Whitespace(_) => (),
                    XmlEvent::EndDocument => unreachable!(),
                },
                Err(err) => {
                    yuki_error!("XML error: {}", err);
                    break;
                }
            }
        }
    };
}

fn parse_sensor<T: std::io::Read>(
    parser: &mut EventReader<T>,
    mut indent: String,
) -> Result<(CameraOrientation, FoV)> {
    let mut fov_axis = String::new();
    let mut fov_angle = 0.0f32;
    let mut transform = Transform::default();

    parse_element!(parser, indent, |name: &xml::name::OwnedName,
                                    attributes: Vec<
        xml::attribute::OwnedAttribute,
    >,
                                    level: &mut i32,
                                    ignore_level: &mut Option<u32>|
     -> Result<()> {
        let data_type = name.local_name.as_str();
        match data_type {
            "string" => {
                let (attr_name, attr_value) = (
                    find_attr!(&attributes, "name").as_str(),
                    find_attr!(&attributes, "value"),
                );
                match attr_name {
                    "fov_axis" => fov_axis = attr_value.clone(),
                    _ => {
                        return Err(format!("Unknown sensor string element '{}'", attr_name).into())
                    }
                }
            }
            "float" => {
                let (attr_name, attr_value) = (
                    find_attr!(&attributes, "name").as_str(),
                    find_attr!(&attributes, "value"),
                );
                match attr_name {
                    "fov" => fov_angle = attr_value.as_str().parse()?,
                    "near_clip" => (), // TODO
                    "far_clip" => (),  // TODO
                    "" => (),          // TODO
                    _ => {
                        return Err(format!("Unknown sensor string element '{}'", attr_name).into())
                    }
                }
            }
            "transform" => {
                transform = parse_transform(parser, indent.clone())?;
                *level -= 1;
                indent.truncate(indent.len() - 2);
            }
            "sampler" => {
                *ignore_level = Some(0);
            }
            "film" => {
                *ignore_level = Some(0);
            }
            _ => return Err(format!("Unknown sensor data type '{}'", data_type).into()),
        }
        Ok(())
    });

    let (cam_pos, cam_euler, cam_scale) = match transform.m().decompose() {
        Ok(result) => result,
        Err(e) => {
            return Err(format!("Cannot decompose camera to world matrix: {}", e).into());
        }
    };
    if !relative_eq!(cam_scale, Vec3::new(1.0, 1.0, 1.0)) {
        return Err("Camera to world has scaling".into());
    }

    if fov_axis != "x" {
        return Err("Only horizontal fov is supported".into());
    }
    let cam_fov = match fov_axis.as_str() {
        "x" => FoV::X(fov_angle),
        "y" => FoV::Y(fov_angle),
        axis => {
            return Err(format!("Unknown fov axis '{}'", axis).into());
        }
    };

    Ok((
        CameraOrientation::Pose {
            cam_pos,
            cam_euler_deg: Vec3::new(
                // Seems odd that y doesn't need negation since that too is cw for us instead of mitsuba's ccw
                -cam_euler.x.to_degrees(),
                cam_euler.y.to_degrees(),
                -cam_euler.z.to_degrees(),
            ),
        },
        cam_fov,
    ))
}

fn parse_transform<T: std::io::Read>(
    parser: &mut EventReader<T>,
    mut indent: String,
) -> Result<Transform<f32>> {
    let mut transform = Transform::default();

    parse_element!(parser, indent, |name: &xml::name::OwnedName,
                                    attributes: Vec<
        xml::attribute::OwnedAttribute,
    >,
                                    level: &mut i32,
                                    ignore_level: &mut Option<u32>|
     -> Result<()> {
        let data_type = name.local_name.as_str();
        match data_type {
            "rotate" => {
                let axis = {
                    let mut axis = Vec3::new(0.0, 0.0, 0.0);
                    if let Some(v) = try_find_attr!(&attributes, "x") {
                        axis.x = v.parse()?;
                    }
                    if let Some(v) = try_find_attr!(&attributes, "y") {
                        axis.y = v.parse()?;
                    }
                    if let Some(v) = try_find_attr!(&attributes, "z") {
                        axis.z = v.parse()?;
                    }
                    axis.normalized()
                };

                let angle: f32 = find_attr!(&attributes, "angle")
                    .parse::<f32>()?
                    .to_radians();

                transform = &rotation(angle, axis) * &transform;
            }
            "translate" => {
                let p: Vec<f32> = find_attr!(&attributes, "value")
                    .split(" ")
                    .map(|v| v.parse::<f32>().unwrap())
                    .collect();
                transform = &translation(Vec3::new(p[0], p[1], p[2])) * &transform;
            }
            "scale" => {
                let p_strs: Vec<&str> = find_attr!(&attributes, "value").split(" ").collect();
                let p: Vec<f32> = match p_strs.len() {
                    1 => {
                        let v = p_strs[0].parse::<f32>().unwrap();
                        vec![v, v, v]
                    }
                    3 => p_strs.iter().map(|v| v.parse::<f32>().unwrap()).collect(),
                    _ => unreachable!(),
                };
                transform = &translation(Vec3::new(p[0], p[1], p[2])) * &transform;
            }
            "matrix" => {
                // TODO: map with ? possible?
                let values: Vec<f32> = find_attr!(&attributes, "value")
                    .split(" ")
                    .map(|v| v.parse().unwrap())
                    .collect();
                transform = &Transform::new_m(values.into()) * &transform;
            }
            _ => return Err(format!("Unknown transformation data type '{}'", data_type).into()),
        }
        Ok(())
    });

    Ok(transform)
}

fn parse_shape<T: std::io::Read>(
    dir_path: &PathBuf,
    materials: &HashMap<String, Vec3<f32>>,
    attributes: Vec<xml::attribute::OwnedAttribute>,
    parser: &mut EventReader<T>,
    mut indent: String,
) -> Result<(Option<Arc<Mesh>>, Vec<Arc<dyn Shape>>)> {
    let data_type = find_attr!(&attributes, "type").as_str();
    if data_type != "ply" {
        return Err(format!("Unexpected shape type '{}'!", data_type).into());
    }
    let mut transform = Transform::default();
    let mut ply_abspath = None;
    let mut material_id = None;
    // TODO: Parse whole shape first, load with constructed material after
    parse_element!(parser, indent, |name: &xml::name::OwnedName,
                                    attributes: Vec<
        xml::attribute::OwnedAttribute,
    >,
                                    level: &mut i32,
                                    ignore_level: &mut Option<u32>|
     -> Result<()> {
        let data_type = name.local_name.as_str();
        match data_type {
            "string" => {
                if find_attr!(&attributes, "name").as_str() != "filename" {
                    return Err("Expected 'name': 'filename' as mesh 'string' attribute".into());
                }

                let mesh_relpath =
                    PathBuf::from(find_attr!(&attributes, "value").replace("\\", "/"));
                ply_abspath = match dir_path.join(&mesh_relpath).canonicalize() {
                    Ok(p) => Some(p),
                    Err(e) => {
                        yuki_error!(
                            "Error canonicalizing absolute mesh path for '{}'",
                            mesh_relpath.to_string_lossy()
                        );
                        return Err(e.into());
                    }
                };
            }
            "ref" => {
                let ref_type = find_attr!(&attributes, "name").as_str();
                if ref_type != "bsdf" {
                    return Err(
                        format!("Expected mesh 'ref' to be 'bsdf', got '{}'", ref_type).into(),
                    );
                }
                material_id = Some(find_attr!(&attributes, "id").clone());
            }
            "transform" => {
                transform = parse_transform(parser, indent.clone())?;
                *level -= 1;
                indent.truncate(indent.len() - 2);
            }
            _ => return Err(format!("Unknown shape type '{}'", data_type).into()),
        }
        Ok(())
    });

    // Mitsuba's +X is to the left of +Z, ours to the right of it
    transform = &transform * &scale(-1.0, 1.0, 1.0);

    if let None = ply_abspath {
        return Err("Mesh with no ply".into());
    }

    if let Some(id) = material_id {
        if let Some(&material) = materials.get(&id) {
            match load_ply(&ply_abspath.unwrap(), material, Some(transform)) {
                Ok((m, g)) => Ok((Some(m), g)),
                Err(e) => Err(e),
            }
        } else {
            Err(format!("Unknown mesh material '{}'", id).into())
        }
    } else {
        Err("Mesh with no material".into())
    }
}

fn parse_constant_emitter<T: std::io::Read>(
    parser: &mut EventReader<T>,
    mut indent: String,
) -> Result<Vec3<f32>> {
    let mut radiance = Vec3::from(0.0);

    parse_element!(parser, indent, |name: &xml::name::OwnedName,
                                    attributes: Vec<
        xml::attribute::OwnedAttribute,
    >,
                                    _: &mut i32,
                                    _: &mut Option<u32>|
     -> Result<()> {
        let data_type = name.local_name.as_str();
        match data_type {
            "rgb" => {
                radiance = parse_rgb(&attributes, "radiance")?;
            }
            _ => return Err(format!("Unknown constant emitter data type '{}'", data_type).into()),
        }
        Ok(())
    });

    Ok(radiance)
}

fn parse_point_light<T: std::io::Read>(
    parser: &mut EventReader<T>,
    mut indent: String,
) -> Result<Arc<PointLight>> {
    let mut position = Point3::from(0.0);
    let mut intensity = Vec3::from(0.0);

    parse_element!(parser, indent, |name: &xml::name::OwnedName,
                                    attributes: Vec<
        xml::attribute::OwnedAttribute,
    >,
                                    _: &mut i32,
                                    _: &mut Option<u32>|
     -> Result<()> {
        let data_type = name.local_name.as_str();
        match data_type {
            "point" => {
                if find_attr!(&attributes, "name").as_str() != "position" {
                    return Err(
                        "Expected 'name': 'filename' as first mesh 'string' attribute".into(),
                    );
                }

                for axis_value in attributes.iter().skip(1) {
                    let pos_value = match axis_value.name.local_name.as_str() {
                        "x" => &mut position.x,
                        "y" => &mut position.y,
                        "z" => &mut position.z,
                        a => {
                            return Err(format!("Invalid point axis '{}'", a).into());
                        }
                    };
                    *pos_value = axis_value.value.parse()?;
                }
            }
            "rgb" => {
                intensity = parse_rgb(&attributes, "intensity")?;
            }
            _ => return Err(format!("Unknown light data type '{}'", data_type).into()),
        }
        Ok(())
    });

    Ok(Arc::new(PointLight::new(
        &translation(position.into()),
        intensity,
    )))
}

fn parse_spot_light<T: std::io::Read>(
    parser: &mut EventReader<T>,
    mut indent: String,
) -> Result<Arc<SpotLight>> {
    let mut light_to_world = Transform::default();
    let mut intensity = Vec3::from(0.0);
    let mut total_width_degrees = 0.0f32;
    let mut falloff_start_degrees = 0.0f32;

    parse_element!(parser, indent, |name: &xml::name::OwnedName,
                                    attributes: Vec<
        xml::attribute::OwnedAttribute,
    >,
                                    level: &mut i32,
                                    _: &mut Option<u32>|
     -> Result<()> {
        let data_type = name.local_name.as_str();
        match data_type {
            "float" => match find_attr!(&attributes, "name").as_str() {
                "cutoff_angle" => {
                    total_width_degrees = find_attr!(&attributes, "value").parse()?;
                }
                "beam_width" => {
                    falloff_start_degrees = find_attr!(&attributes, "value").parse()?;
                }
                n => return Err(format!("Unexpected spot light float 'name': '{}'", n).into()),
            },
            "transform" => {
                light_to_world = parse_transform(parser, indent.clone())?;
                *level -= 1;
            }
            "rgb" => {
                intensity = parse_rgb(&attributes, "intensity")?;
            }
            _ => return Err(format!("Unknown spot light data type '{}'", data_type).into()),
        }
        Ok(())
    });

    // Mitsuba's +X is to the left of +Z, ours to the right of it
    light_to_world = &scale(-1.0, 1.0, 1.0) * &light_to_world;

    Ok(Arc::new(SpotLight::new(
        &light_to_world,
        intensity,
        total_width_degrees,
        falloff_start_degrees,
    )))
}

fn parse_material<T: std::io::Read>(
    parser: &mut EventReader<T>,
    mut indent: String,
) -> Result<Vec3<f32>> {
    let mut material = Vec3::new(1.0, 0.0, 1.0);
    parse_element!(parser, indent, |name: &xml::name::OwnedName,
                                    attributes: Vec<
        xml::attribute::OwnedAttribute,
    >,
                                    level: &mut i32,
                                    _: &mut Option<u32>|
     -> Result<()> {
        let data_type = name.local_name.as_str();
        match data_type {
            "bsdf" => {
                material = parse_diffuse(parser, indent.clone())?;
                *level -= 1;
                indent.truncate(indent.len() - 2);
            }
            _ => return Err(format!("Unknown material data type '{}'", data_type).into()),
        }
        Ok(())
    });
    Ok(material)
}

fn parse_diffuse<T: std::io::Read>(
    parser: &mut EventReader<T>,
    mut indent: String,
) -> Result<Vec3<f32>> {
    let mut reflectance = Vec3::new(0.5, 0.5, 0.5);

    parse_element!(parser, indent, |name: &xml::name::OwnedName,
                                    attributes: Vec<
        xml::attribute::OwnedAttribute,
    >,
                                    _: &mut i32,
                                    _: &mut Option<u32>|
     -> Result<()> {
        let data_type = name.local_name.as_str();
        match data_type {
            "rgb" => {
                reflectance = parse_rgb(&attributes, "reflectance")?;
            }
            _ => return Err(format!("Unknown light data type '{}'", data_type).into()),
        }
        Ok(())
    });

    Ok(reflectance)
}

fn parse_rgb(
    attributes: &Vec<xml::attribute::OwnedAttribute>,
    expected_name: &str,
) -> Result<Vec3<f32>> {
    let mut v = Vec3::from(0.0);
    let name = find_attr!(attributes, "name").as_str();
    if name != expected_name {
        return Err(format!("Expected rgb to be '{}', got '{}'", expected_name, name).into());
    }
    for (i, c) in find_attr!(attributes, "value")
        .split(" ")
        .map(|c| c.parse::<f32>().unwrap())
        .enumerate()
    {
        v[i] = c;
    }
    Ok(v)
}

fn load_ply(
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
