use chrono::{Datelike, Timelike};
use std::{path::PathBuf, sync::Arc};

use crate::{
    math::Vec3,
    scene::{DynamicSceneParameters, Scene, SceneLoadSettings},
    yuki_info,
};

pub fn try_load_scene(
    settings: &SceneLoadSettings,
) -> Result<(Arc<Scene>, DynamicSceneParameters, f32), String> {
    if settings.path.exists() {
        match settings.path.extension() {
            Some(ext) => match ext.to_str().unwrap() {
                "ply" => match Scene::ply(settings) {
                    Ok((scene, scene_params, total_secs)) => {
                        yuki_info!(
                            "PLY loaded from {}",
                            settings.path.file_name().unwrap().to_str().unwrap()
                        );
                        Ok((Arc::new(scene), scene_params, total_secs))
                    }
                    Err(why) => Err(format!("Loading PLY failed: {}", why)),
                },
                "xml" => match Scene::mitsuba(settings) {
                    Ok((scene, scene_params, total_secs)) => {
                        yuki_info!(
                            "Mitsuba 2.0 scene loaded from {}",
                            settings.path.file_name().unwrap().to_str().unwrap()
                        );
                        Ok((Arc::new(scene), scene_params, total_secs))
                    }
                    Err(why) => Err(format!("Loading Mitsuba 2.0 scene failed: {}", why)),
                },
                _ => Err(format!("Unknown extension '{}'", ext.to_str().unwrap())),
            },
            None => Err(format!("Expected a file with an extension")),
        }
    } else if settings.path.as_os_str().is_empty() {
        let (scene, scene_params, total_secs) = Scene::cornell();
        Ok((Arc::new(scene), scene_params, total_secs))
    } else {
        Err(format!(
            "Scene does not exist '{}'",
            settings.path.to_string_lossy()
        ))
    }
}

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
