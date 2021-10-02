use crate::{
    materials::{Glass, Material, Matte},
    math::Spectrum,
    parse_element,
    scene::Result,
    textures::ConstantTexture,
    yuki_error, yuki_info, yuki_trace,
};

use super::{common::parse_rgb, find_attr};

use approx::abs_diff_eq;
use std::sync::Arc;
use xml::{attribute::OwnedAttribute, name::OwnedName, reader::EventReader};

// TODO: This does not support glass now. Make "bsdf" -parsing generic, move it out of mod.rs
pub fn parse_twosided<T: std::io::Read>(
    parser: &mut EventReader<T>,
    mut indent: String,
) -> Result<Arc<dyn Material>> {
    let mut material: Arc<dyn Material> = Arc::new(Matte::new(
        Arc::new(ConstantTexture::new(Spectrum::ones())),
        Arc::new(ConstantTexture::new(0.0)),
    ));

    parse_element!(parser, indent, |name: &OwnedName,
                                    attributes: Vec<OwnedAttribute>,
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
            "rgb" => {
                material = Arc::new(Matte::new(
                    Arc::new(ConstantTexture::new(parse_rgb(&attributes, "reflectance")?)),
                    Arc::new(ConstantTexture::new(0.0)),
                ));
            }
            _ => return Err(format!("Unknown material data type '{}'", data_type).into()),
        }
        Ok(())
    });
    Ok(material)
}

pub fn parse_diffuse<T: std::io::Read>(
    parser: &mut EventReader<T>,
    mut indent: String,
) -> Result<Arc<dyn Material>> {
    let mut reflectance = Arc::new(ConstantTexture::new(Spectrum::new(0.5, 0.5, 0.5)));

    parse_element!(parser, indent, |name: &OwnedName,
                                    attributes: Vec<OwnedAttribute>,
                                    _: &mut i32,
                                    _: &mut Option<u32>|
     -> Result<()> {
        let data_type = name.local_name.as_str();
        match data_type {
            "rgb" => {
                reflectance =
                    Arc::new(ConstantTexture::new(parse_rgb(&attributes, "reflectance")?));
            }
            _ => return Err(format!("Unknown light data type '{}'", data_type).into()),
        }
        Ok(())
    });

    Ok(Arc::new(Matte::new(
        reflectance,
        Arc::new(ConstantTexture::new(0.0)),
    )))
}

const BK7_GLASS_IOR: f32 = 1.5046;
const AIR_IOR: f32 = 1.000_277;

pub fn parse_dielectric<T: std::io::Read>(
    parser: &mut EventReader<T>,
    mut indent: String,
) -> Result<Arc<dyn Material>> {
    let mut int_ior = BK7_GLASS_IOR;
    let mut ext_ior = AIR_IOR;
    let mut specular_reflectance = Arc::new(ConstantTexture::new(Spectrum::ones()));
    let mut specular_transmittance = Arc::new(ConstantTexture::new(Spectrum::ones()));

    parse_element!(parser, indent, |name: &OwnedName,
                                    attributes: Vec<OwnedAttribute>,
                                    _: &mut i32,
                                    _: &mut Option<u32>|
     -> Result<()> {
        let data_type = name.local_name.as_str();
        match data_type {
            "rgb" => {
                if let Ok(v) = parse_rgb(&attributes, "specular_reflectance") {
                    specular_reflectance = Arc::new(ConstantTexture::new(v));
                } else if let Ok(v) = parse_rgb(&attributes, "specular_transmittance") {
                    specular_transmittance = Arc::new(ConstantTexture::new(v));
                } else {
                    return Err(format!(
                        "Unknown dielectric rgb data '{}'",
                        find_attr!(&attributes, "name")
                    )
                    .into());
                }
            }
            "float" => {
                let (attr_name, attr_value) = (
                    find_attr!(&attributes, "name").as_str(),
                    find_attr!(&attributes, "value").as_str().parse::<f32>()?,
                );
                match attr_name {
                    "int_ior" => int_ior = attr_value,
                    "ext_ior" => ext_ior = attr_value,
                    _ => {
                        return Err(format!("Unknown dielectric float data '{}'", attr_name).into())
                    }
                }
            }
            _ => return Err(format!("Unknown dielectric data type '{}'", data_type).into()),
        }
        Ok(())
    });

    if !abs_diff_eq!(ext_ior, AIR_IOR, epsilon = 0.001) {
        return Err(format!(
            "Only air supported for external IoR not supported but received '{}'",
            ext_ior
        )
        .into());
    }

    Ok(Arc::new(Glass::new(
        specular_reflectance,
        specular_transmittance,
        int_ior,
    )))
}
