use chrono::{Datelike, Timelike};
use std::path::PathBuf;

use crate::{math::Vec3, scene::Scene, yuki_info};

pub fn exr_path(scene: &Scene) -> Result<PathBuf, String> {
    match std::env::current_dir() {
        Ok(mut path) => {
            let now = chrono::Local::now();
            let timestamp = format!(
                "{:04}{:02}{:02}_{:02}{:02}{:02}",
                now.year(),
                now.month(),
                now.day(),
                now.hour(),
                now.minute(),
                now.second()
            );
            let filename = format!("{}_{}.exr", scene.name, timestamp);
            path.push(filename);

            Ok(path)
        }
        Err(why) => Err(format!(
            "Error getting current working directory: {:?}",
            why
        )),
    }
}

pub fn write_exr(
    width: usize,
    height: usize,
    pixels: Vec<Vec3<f32>>,
    path: PathBuf,
) -> Result<(), String> {
    yuki_info!("Writing out EXR");
    match exr::prelude::write_rgb_file(&path, width, height, |x, y| {
        let px = pixels[y * width + x];
        (px.x, px.y, px.z)
    }) {
        Ok(_) => {
            yuki_info!("EXR written to '{}'", path.to_string_lossy());
            Ok(())
        }
        Err(why) => Err(format!(
            "Error writing EXR to '{}': {:?}",
            path.to_string_lossy(),
            why
        )),
    }
}
