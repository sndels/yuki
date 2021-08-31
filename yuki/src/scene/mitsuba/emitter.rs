use super::{common::parse_rgb, transform};
use crate::{
    find_attr,
    lights::{Light, PointLight, SpotLight},
    math::{
        transforms::{scale, translation},
        Point3, Spectrum, Transform,
    },
    parse_element,
    scene::Result,
    yuki_error, yuki_info, yuki_trace,
};
use xml::{attribute::OwnedAttribute, name::OwnedName, reader::EventReader};

use std::sync::Arc;

pub enum Emitter {
    Background { color: Spectrum<f32> },
    Light { light: Arc<dyn Light> },
}

pub fn parse<T: std::io::Read>(
    attributes: &[OwnedAttribute],
    parser: &mut EventReader<T>,
    indent: String,
) -> Result<Option<Emitter>> {
    let attr_type = find_attr!(attributes, "type");
    let ret = match attr_type.as_str() {
        "constant" => Some(Emitter::Background {
            color: parse_constant_emitter(parser, indent)?,
        }),
        "point" => Some(Emitter::Light {
            light: parse_point_light(parser, indent)?,
        }),
        "spot" => Some(Emitter::Light {
            light: parse_spot_light(parser, indent)?,
        }),
        _ => None,
    };
    Ok(ret)
}

fn parse_constant_emitter<T: std::io::Read>(
    parser: &mut EventReader<T>,
    mut indent: String,
) -> Result<Spectrum<f32>> {
    let mut radiance = Spectrum::zeros();

    parse_element!(parser, indent, |name: &OwnedName,
                                    attributes: Vec<OwnedAttribute>,
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
    let mut intensity = Spectrum::zeros();

    parse_element!(parser, indent, |name: &OwnedName,
                                    attributes: Vec<OwnedAttribute>,
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

    // Mitsuba's +X is to the left of +Z, ours to the right of it
    position.x = -position.x;

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
    let mut intensity = Spectrum::zeros();
    let mut total_width_degrees = 0.0;
    let mut falloff_start_degrees = 0.0;

    parse_element!(parser, indent, |name: &OwnedName,
                                    attributes: Vec<OwnedAttribute>,
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
                light_to_world = transform::parse(parser, indent.clone())?;
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
