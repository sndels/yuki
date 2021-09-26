use crate::{
    find_attr,
    materials::Material,
    math::{transforms::scale, Transform},
    parse_element,
    scene::{ply, Result},
    yuki_error, yuki_info, yuki_trace,
};

use super::transform;

use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::Arc,
};
use xml::{attribute::OwnedAttribute, name::OwnedName, reader::EventReader};

pub fn parse<T: std::io::Read>(
    dir_path: &Path,
    materials: &HashMap<String, Arc<dyn Material>>,
    attributes: &[OwnedAttribute],
    parser: &mut EventReader<T>,
    mut indent: String,
) -> Result<ply::PlyResult> {
    let data_type = find_attr!(attributes, "type").as_str();
    if data_type != "ply" {
        return Err(format!("Unexpected shape type '{}'!", data_type).into());
    }
    let mut transform = Transform::default();
    let mut ply_abspath = None;
    let mut material_id = None;
    // TODO: Parse whole shape first, load with constructed material after
    parse_element!(parser, indent, |name: &OwnedName,
                                    attributes: Vec<OwnedAttribute>,
                                    level: &mut i32,
                                    _: &mut Option<u32>|
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
                transform = transform::parse(parser, indent.clone())?;
                *level -= 1;
                indent.truncate(indent.len() - 2);
            }
            _ => return Err(format!("Unknown shape type '{}'", data_type).into()),
        }
        Ok(())
    });

    // Mitsuba's +X is to the left of +Z, ours to the right of it
    transform = &scale(-1.0, 1.0, 1.0) * &transform;

    if ply_abspath.is_none() {
        return Err("Mesh with no ply".into());
    }

    if let Some(id) = material_id {
        if let Some(material) = materials.get(&id) {
            ply::load(&ply_abspath.unwrap(), material, Some(transform))
        } else {
            Err(format!("Unknown mesh material '{}'", id).into())
        }
    } else {
        Err("Mesh with no material".into())
    }
}
