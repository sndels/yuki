use super::{
    renderpasses::ToneMapFilm,
    util::{try_load_scene, write_exr},
    InitialSettings,
};
use crate::{
    expect,
    film::Film,
    math::Vec3,
    renderer::{RenderResult, Renderer},
    yuki_info,
};
use glium::backend::glutin::headless::Headless;
use glutin::{dpi::PhysicalSize, event_loop::EventLoop, ContextBuilder};
use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};

pub fn render(exr_path: PathBuf, mut settings: InitialSettings) {
    let (scene, scene_params, _) = expect!(
        try_load_scene(&settings.load_settings),
        "Scene loading failed"
    );
    let film = Arc::new(Mutex::new(Film::new(settings.film_settings.res)));
    let mut renderer = Renderer::new();
    renderer.launch(
        scene,
        &scene_params,
        film.clone(),
        settings.sampler_settings,
        settings.scene_integrator,
        settings.film_settings,
        true,
    );

    let (w, h, pixels) = match renderer.wait_result() {
        Ok(RenderResult { secs, .. }) => {
            yuki_info!("Render finished in {:.2}s", secs);

            let event_loop = EventLoop::new();
            let context = expect!(
                ContextBuilder::new().build_headless(
                    &event_loop,
                    PhysicalSize::new(
                        settings.film_settings.res.x as u32,
                        settings.film_settings.res.y as u32
                    )
                ),
                "Failed to create headless context"
            );
            let backend = expect!(Headless::new(context), "Failed to create headless backend");

            let mut tone_map_film = expect!(
                ToneMapFilm::new(&backend),
                "Failed to create tone map render pass"
            );
            let tone_mapped_film = expect!(
                tone_map_film.draw(&backend, &film, &mut settings.tone_map),
                "Failed to tone map film"
            );
            // TODO: This will explode if mapped texture format is not f32f32f32
            let pixels = unsafe { tone_mapped_film.unchecked_read::<Vec<Vec3<f32>>, Vec3<f32>>() };

            (
                tone_mapped_film.width() as usize,
                tone_mapped_film.height() as usize,
                pixels,
            )

            // match Arc::try_unwrap(film) {
            //     Ok(film) => {
            //         expect!(film.into_inner(), "Failed to pull Film out of its Mutex")
            //     }
            //     Err(_) => {
            //         panic!("Failed to pull Film out of its Arc");
            //     }
            // }
        }
        Err(why) => panic!("Render failed: {}", why),
    };

    expect!(write_exr(w, h, &pixels, exr_path,), "");
}
