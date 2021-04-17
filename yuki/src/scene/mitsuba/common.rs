use crate::{find_attr, math::Vec3};
use xml::attribute::OwnedAttribute;

pub type ParseResult<T> = std::result::Result<T, Box<dyn std::error::Error>>;

pub fn parse_rgb(attributes: &Vec<OwnedAttribute>, expected_name: &str) -> ParseResult<Vec3<f32>> {
    let mut v = Vec3::from(0.0);
    let name = find_attr!(attributes, "name").as_str();
    if name != expected_name {
        return Err(format!("Expected rgb to be '{}', got '{}'", expected_name, name).into());
    }
    for (i, c) in find_attr!(attributes, "value")
        .split(" ")
        .map(|c| c.parse::<f32>().unwrap())
        .enumerate()
    {
        v[i] = c;
    }
    Ok(v)
}
