use crate::{
    materials::{Material, Matte},
    math::Vec3,
    parse_element,
    scene::Result,
    yuki_error, yuki_info, yuki_trace,
};

use super::common::parse_rgb;

use std::sync::Arc;
use xml::{attribute::OwnedAttribute, name::OwnedName, reader::EventReader};

pub fn parse<T: std::io::Read>(
    parser: &mut EventReader<T>,
    mut indent: String,
) -> Result<Arc<dyn Material>> {
    let mut material = Arc::new(Matte::new(Vec3::new(1.0, 0.0, 1.0)));

    parse_element!(parser, indent, |name: &OwnedName,
                                    attributes: Vec<OwnedAttribute>,
                                    level: &mut i32,
                                    _: &mut Option<u32>|
     -> Result<()> {
        let data_type = name.local_name.as_str();
        match data_type {
            "bsdf" => {
                material = Arc::new(Matte::new(parse_diffuse(parser, indent.clone())?));
                *level -= 1;
                indent.truncate(indent.len() - 2);
            }
            // TODO: Proper parsing for twosided and similar "recursive" bsdfs so this can come from parse_diffuse
            "rgb" => {
                material = Arc::new(Matte::new(parse_rgb(&attributes, "reflectance")?));
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

    parse_element!(parser, indent, |name: &OwnedName,
                                    attributes: Vec<OwnedAttribute>,
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
