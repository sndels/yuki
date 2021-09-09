use super::{
    renderpasses::{find_min_max, HeatmapParams, ToneMapFilm, ToneMapType},
    util::{try_load_scene, write_exr},
    InitialSettings,
};
use crate::{
    expect,
    film::Film,
    math::Spectrum,
    renderer::{RenderResult, Renderer},
    yuki_info,
};
use glium::backend::glutin::headless::Headless;
use glutin::{dpi::PhysicalSize, event_loop::EventLoop, ContextBuilder};
use std::{
    path::Path,
    sync::{Arc, Mutex},
};

pub fn render(exr_path: &Path, settings: InitialSettings) {
    let load_settings = settings.load_settings.unwrap_or_default();

    let (scene, camera_params, _) = expect!(try_load_scene(&load_settings), "Scene loading failed");

    let film_settings = settings.film_settings.unwrap_or_default();
    let sampler = settings.sampler.unwrap_or_default();
    let scene_integrator = settings.scene_integrator.unwrap_or_default();
    let mut tone_map = settings.tone_map.unwrap_or_default();

    // TODO: Don't override film_settings if input has defined them
    let film = Arc::new(Mutex::new(Film::new(film_settings.res)));
    let mut renderer = Renderer::new();
    renderer.launch(
        scene,
        camera_params,
        Arc::clone(&film),
        sampler,
        scene_integrator,
        film_settings,
        false,
    );

    let (w, h, pixels) = match renderer.wait_result() {
        Ok(RenderResult { secs, .. }) => {
            yuki_info!("Render finished in {:.2}s", secs);

            if let ToneMapType::Raw = tone_map {
                #[allow(clippy::match_wild_err_arm)]
                // "Wild" ignore needed as err is Arc itself
                match Arc::try_unwrap(film) {
                    Ok(film) => {
                        let film =
                            expect!(film.into_inner(), "Failed to pull Film out of its Mutex");
                        (
                            film.res().x as usize,
                            film.res().y as usize,
                            film.pixels().clone(),
                        )
                    }
                    Err(_) => {
                        panic!("Failed to pull Film out of its Arc");
                    }
                }
            } else {
                let event_loop = EventLoop::new();
                let context = expect!(
                    ContextBuilder::new().build_headless(
                        &event_loop,
                        PhysicalSize::new(film_settings.res.x as u32, film_settings.res.y as u32)
                    ),
                    "Failed to create headless context"
                );
                let backend = expect!(Headless::new(context), "Failed to create headless backend");

                let mut tone_map_film = expect!(
                    ToneMapFilm::new(&backend),
                    "Failed to create tone map render pass"
                );

                if let ToneMapType::Heatmap(HeatmapParams {
                    ref mut bounds,
                    channel,
                }) = tone_map
                {
                    if bounds.is_none() {
                        *bounds = Some(expect!(
                            find_min_max(&film, channel),
                            "Failed to find film min, max"
                        ));
                    }
                }

                let tone_mapped_film = expect!(
                    tone_map_film.draw(&backend, &film, &tone_map),
                    "Failed to tone map film"
                );
                // TODO: This will explode if mapped texture format is not f32f32f32
                let pixels = unsafe {
                    tone_mapped_film.unchecked_read::<Vec<Spectrum<f32>>, Spectrum<f32>>()
                };

                (
                    tone_mapped_film.width() as usize,
                    tone_mapped_film.height() as usize,
                    pixels,
                )
            }
        }
        Err(why) => panic!("Render failed: {}", why),
    };

    expect!(write_exr(w, h, &pixels, exr_path,), "");
}
