use crate::{
    find_attr,
    math::{
        transforms::{rotation, scale, translation},
        Transform, Vec3,
    },
    parse_element, try_find_attr, yuki_error, yuki_info, yuki_trace,
};

use super::Result;

use xml::{attribute::OwnedAttribute, name::OwnedName, reader::EventReader};

pub fn parse<T: std::io::Read>(
    parser: &mut EventReader<T>,
    mut indent: String,
) -> Result<Transform<f32>> {
    let mut transform = Transform::default();

    parse_element!(parser, indent, |name: &OwnedName,
                                    attributes: Vec<OwnedAttribute>,
                                    _: &mut i32,
                                    _: &mut Option<u32>|
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
                    .split(' ')
                    .map(|v| v.parse::<f32>().unwrap())
                    .collect();
                transform = &translation(Vec3::new(p[0], p[1], p[2])) * &transform;
            }
            "scale" => {
                let p_strs: Vec<&str> = find_attr!(&attributes, "value").split(' ').collect();
                let p: Vec<f32> = match p_strs.len() {
                    1 => {
                        let v = p_strs[0].parse::<f32>().unwrap();
                        vec![v, v, v]
                    }
                    3 => p_strs.iter().map(|v| v.parse::<f32>().unwrap()).collect(),
                    _ => unreachable!(),
                };
                transform = &scale(p[0], p[1], p[2]) * &transform;
            }
            "matrix" => {
                // TODO: map with ? possible?
                let values: Vec<f32> = find_attr!(&attributes, "value")
                    .split(' ')
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
