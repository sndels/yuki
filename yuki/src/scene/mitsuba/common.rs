use crate::{find_attr, math::Spectrum, scene::Result};
use xml::attribute::OwnedAttribute;

pub fn parse_rgb(attributes: &[OwnedAttribute], expected_name: &str) -> Result<Spectrum<f32>> {
    let mut v = Spectrum::zeros();
    let name = find_attr!(attributes, "name").as_str();
    if name != expected_name {
        return Err(format!("Expected rgb to be '{}', got '{}'", expected_name, name).into());
    }
    for (i, c) in find_attr!(attributes, "value")
        .split(' ')
        .map(|c| c.parse::<f32>().unwrap())
        .enumerate()
    {
        v[i] = c;
    }
    Ok(v)
}
