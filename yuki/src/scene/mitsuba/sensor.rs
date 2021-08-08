use crate::{
    camera::FoV,
    find_attr,
    math::{
        transforms::{rotation_euler, scale, translation},
        DecomposedMatrix, Point3, Transform, Vec3,
    },
    parse_element,
    scene::CameraParameters,
    yuki_error, yuki_info, yuki_trace,
};

use super::{transform, Result};

use approx::relative_eq;
use xml::{attribute::OwnedAttribute, name::OwnedName, reader::EventReader};

pub fn parse<T: std::io::Read>(
    parser: &mut EventReader<T>,
    mut indent: String,
) -> Result<CameraParameters> {
    let mut fov_axis = String::new();
    let mut fov_angle = 0.0;
    let mut transform = Transform::default();

    parse_element!(parser, indent, |name: &OwnedName,
                                    attributes: Vec<OwnedAttribute>,
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
                    "near_clip" | "far_clip" | "" => (), // TODO
                    _ => {
                        return Err(format!("Unknown sensor string element '{}'", attr_name).into())
                    }
                }
            }
            "transform" => {
                transform = transform::parse(parser, indent.clone())?;
                *level -= 1;
                indent.truncate(indent.len() - 2);
            }
            "sampler" | "film" => {
                *ignore_level = Some(0);
            }
            _ => return Err(format!("Unknown sensor data type '{}'", data_type).into()),
        }
        Ok(())
    });

    // Mitsuba's +X is to the left of +Z, ours to the right of it
    transform = &scale(-1.0, 1.0, 1.0) * &transform;

    let DecomposedMatrix {
        translation: position,
        rotation: cam_euler,
        scale: cam_scale,
    } = match transform.m().decompose() {
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
    let fov = match fov_axis.as_str() {
        "x" => FoV::X(fov_angle),
        "y" => FoV::Y(fov_angle),
        axis => {
            return Err(format!("Unknown fov axis '{}'", axis).into());
        }
    };

    // We compensate for the flipped X axis in the rotation
    let camera_to_world = &translation(position.into())
        * &rotation_euler(Vec3::new(-cam_euler.x, -cam_euler.y, cam_euler.z));
    // This should be changed to some sane distance in front of camera once we know the scene scale
    let target = &camera_to_world * Point3::new(0.0, 0.0, 1.0);

    Ok(CameraParameters {
        position,
        target,
        fov,
        ..CameraParameters::default()
    })
}
