use crate::math::{Normal, Point3, Spectrum};

#[derive(Clone)]
struct ParamSetItem<T>
where
    T: Clone,
{
    name: String,
    values: Vec<T>,
}

#[derive(Clone, Default)]
pub struct ParamSet {
    floats: Vec<ParamSetItem<f32>>,
    integers: Vec<ParamSetItem<i32>>,
    points: Vec<ParamSetItem<Point3<f32>>>,
    normals: Vec<ParamSetItem<Normal<f32>>>,
    spectra: Vec<ParamSetItem<Spectrum<f32>>>,
    strings: Vec<ParamSetItem<String>>,
}

impl ParamSet {
    pub fn add_f32(&mut self, name: String, values: Vec<f32>) {
        self.floats.push(ParamSetItem { name, values });
    }

    pub fn add_i32(&mut self, name: String, values: Vec<i32>) {
        self.integers.push(ParamSetItem { name, values });
    }

    pub fn add_spectrum(&mut self, name: String, values: Vec<Spectrum<f32>>) {
        self.spectra.push(ParamSetItem { name, values });
    }

    pub fn add_point(&mut self, name: String, values: Vec<Point3<f32>>) {
        self.points.push(ParamSetItem { name, values });
    }

    pub fn add_normal(&mut self, name: String, values: Vec<Normal<f32>>) {
        self.normals.push(ParamSetItem { name, values });
    }

    pub fn add_string(&mut self, name: String, values: Vec<String>) {
        self.strings.push(ParamSetItem { name, values });
    }

    pub fn find_f32(&self, name: &str, default: f32) -> f32 {
        find_param_value(name, &self.floats, default)
    }

    pub fn find_i32(&self, name: &str, default: i32) -> i32 {
        find_param_value(name, &self.integers, default)
    }

    pub fn find_i32s<'a>(&'a self, name: &str, default: &'a [i32]) -> &'a [i32] {
        find_param_values(name, &self.integers, default)
    }

    pub fn find_spectrum(&self, name: &str, default: Spectrum<f32>) -> Spectrum<f32> {
        find_param_value(name, &self.spectra, default)
    }

    pub fn find_point(&self, name: &str, default: Point3<f32>) -> Point3<f32> {
        find_param_value(name, &self.points, default)
    }

    pub fn find_points<'a>(&'a self, name: &str, default: &'a [Point3<f32>]) -> &'a [Point3<f32>] {
        find_param_values(name, &self.points, default)
    }

    pub fn find_normals<'a>(&'a self, name: &str, default: &'a [Normal<f32>]) -> &'a [Normal<f32>] {
        find_param_values(name, &self.normals, default)
    }

    pub fn find_string<'a>(&'a self, name: &str, default: &'a str) -> &'a str {
        for param in &self.strings {
            if param.name.as_str() == name && param.values.len() == 1 {
                return &param.values[0];
            }
        }
        default
    }
}

fn find_param_value<T: Copy>(name: &str, params: &[ParamSetItem<T>], default: T) -> T {
    for param in params {
        if param.name.as_str() == name && param.values.len() == 1 {
            return param.values[0];
        }
    }
    default
}

fn find_param_values<'a, T: Clone>(
    name: &str,
    params: &'a [ParamSetItem<T>],
    default: &'a [T],
) -> &'a [T] {
    for param in params {
        if param.name.as_str() == name {
            return &param.values;
        }
    }
    default
}
