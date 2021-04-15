mod common;
mod emitter;
mod macros;
mod material;
mod sensor;
mod shape;
mod transform;

use crate::{
    bvh::{BoundingVolumeHierarchy, SplitMethod},
    find_attr,
    lights::light::Light,
    math::vector::Vec3,
    scene::{DynamicSceneParameters, Scene, SceneLoadSettings},
    yuki_error, yuki_trace,
};

use self::{common::ParseResult, emitter::Emitter};

use std::{collections::HashMap, path::PathBuf, sync::Arc};
use xml::{
    attribute::OwnedAttribute,
    reader::{EventReader, XmlEvent},
};

pub fn load(
    path: &PathBuf,
    settings: SceneLoadSettings,
) -> ParseResult<(Scene, DynamicSceneParameters)> {
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
                    for OwnedAttribute { name, value } in &attributes {
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
                                    sensor::parse(&mut parser, indent.clone())?;
                                indent.truncate(indent.len() - 2);
                            }
                            "bsdf" => {
                                let id = find_attr!(&attributes, "id").clone();
                                let material = material::parse(&mut parser, indent.clone())?;
                                indent.truncate(indent.len() - 2);
                                materials.insert(id, material);
                            }
                            "emitter" => {
                                match emitter::parse(&attributes, &mut parser, indent.clone())? {
                                    Some(e) => match e {
                                        Emitter::Background { color } => background = color,
                                        Emitter::Light { light } => lights.push(light),
                                    },
                                    None => ignore_level = Some(0),
                                }
                            }
                            "shape" => {
                                let (mesh, geom) = shape::parse(
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
                XmlEvent::CData(data) => return Err(format!("Unexpected CDATA: {}", data).into()),
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
    ))
}