use crate::{math::vector::Vec3, parse_element, yuki_error, yuki_info, yuki_trace};

use super::common::{parse_rgb, ParseResult};

use xml::{attribute::OwnedAttribute, name::OwnedName, reader::EventReader};

pub fn parse<T: std::io::Read>(
    parser: &mut EventReader<T>,
    mut indent: String,
) -> ParseResult<Vec3<f32>> {
    let mut material = Vec3::new(1.0, 0.0, 1.0);
    parse_element!(parser, indent, |name: &OwnedName,
                                    _: Vec<OwnedAttribute>,
                                    level: &mut i32,
                                    _: &mut Option<u32>|
     -> ParseResult<()> {
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
) -> ParseResult<Vec3<f32>> {
    let mut reflectance = Vec3::new(0.5, 0.5, 0.5);

    parse_element!(parser, indent, |name: &OwnedName,
                                    attributes: Vec<OwnedAttribute>,
                                    _: &mut i32,
                                    _: &mut Option<u32>|
     -> ParseResult<()> {
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
