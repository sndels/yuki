use crate::{
    camera::FoV,
    find_attr,
    math::{transform::Transform, vector::Vec3},
    parse_element,
    scene::CameraOrientation,
    yuki_error, yuki_info, yuki_trace,
};

use super::{common::ParseResult, transform};

use approx::relative_eq;
use xml::{attribute::OwnedAttribute, name::OwnedName, reader::EventReader};

pub fn parse<T: std::io::Read>(
    parser: &mut EventReader<T>,
    mut indent: String,
) -> ParseResult<(CameraOrientation, FoV)> {
    let mut fov_axis = String::new();
    let mut fov_angle = 0.0f32;
    let mut transform = Transform::default();

    parse_element!(parser, indent, |name: &OwnedName,
                                    attributes: Vec<OwnedAttribute>,
                                    level: &mut i32,
                                    ignore_level: &mut Option<u32>|
     -> ParseResult<()> {
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
                transform = transform::parse(parser, indent.clone())?;
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