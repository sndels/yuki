use chrono::{Datelike, Timelike};
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use crate::{
    camera::CameraParameters,
    film::FilmSettings,
    math::Spectrum,
    scene::{Scene, SceneLoadSettings},
    yuki_info,
};

pub fn try_load_scene(
    settings: &SceneLoadSettings,
) -> Result<(Arc<Scene>, CameraParameters, FilmSettings, f32), String> {
    if settings.path.exists() {
        match settings.path.extension() {
            Some(ext) => match ext.to_str().unwrap() {
                "ply" => match Scene::ply(&settings) {
                    Ok((scene, camera_params, film_settings, total_secs)) => {
                        yuki_info!(
                            "PLY loaded from {}",
                            settings.path.file_name().unwrap().to_str().unwrap()
                        );
                        Ok((Arc::new(scene), camera_params, film_settings, total_secs))
                    }
                    Err(why) => Err(format!("Loading PLY failed: {}", why)),
                },
                "xml" => match Scene::mitsuba(&settings) {
                    Ok((scene, camera_params, film_settings, total_secs)) => {
                        yuki_info!(
                            "Mitsuba 2.0 scene loaded from {}",
                            settings.path.file_name().unwrap().to_str().unwrap()
                        );
                        Ok((Arc::new(scene), camera_params, film_settings, total_secs))
                    }
                    Err(why) => Err(format!("Loading Mitsuba 2.0 scene failed: {}", why)),
                },
                "pbrt" => match Scene::pbrt_v3(&settings) {
                    Ok((scene, camera_params, film_settings, total_secs)) => {
                        yuki_info!(
                            "PBRT v3 scene loaded from {}",
                            settings.path.file_name().unwrap().to_str().unwrap()
                        );
                        Ok((Arc::new(scene), camera_params, film_settings, total_secs))
                    }
                    Err(why) => Err(format!("Loading PBRT v3 scene failed: {}", why)),
                },
                _ => Err(format!("Unknown extension '{}'", ext.to_str().unwrap())),
            },
            None => Err("Expected a file with an extension".into()),
        }
    } else if settings.path.as_os_str().is_empty() {
        Ok(Scene::cornell())
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
    pixels: &[Spectrum<f32>],
    path: &Path,
) -> Result<(), String> {
    yuki_info!("Writing out EXR");
    match exr::prelude::write_rgb_file(&path, width, height, |x, y| {
        let px = pixels[y * width + x];
        (px.r, px.g, px.b)
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
