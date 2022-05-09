mod render_manager;
mod render_worker;

use render_manager::RenderManager;

use serde::{Deserialize, Serialize};
use std::sync::{
    mpsc::{channel, SendError, TryRecvError},
    Arc, Mutex,
};

use crate::{
    camera::CameraParameters,
    film::{Film, FilmSettings},
    integrators::IntegratorType,
    sampling::SamplerType,
    scene::Scene,
    yuki_debug,
};

pub enum RenderStatus {
    Progress {
        active_threads: usize,
        tiles_done: usize,
        tiles_total: usize,
        approx_remaining_s: f32,
        current_rays_per_s: f32,
    },
    Finished {
        ray_count: usize,
    },
}

#[derive(Debug, Default, Copy, Clone, Deserialize, Serialize)]
pub struct RenderSettings {
    pub mark_tiles: bool,
    pub use_single_render_thread: bool,
}

pub struct Renderer {
    manager: Option<RenderManager>,
    render_in_progress: bool,
    render_id: usize,
}

impl Renderer {
    pub fn new() -> Self {
        Self {
            manager: None,
            render_in_progress: false,
            render_id: 0,
        }
    }

    /// Checks if the render task is active.
    pub fn is_active(&self) -> bool {
        self.render_in_progress
    }

    /// Returns the `RenderResult` if the task has finished.
    pub fn check_status(&mut self) -> Option<RenderStatus> {
        let mut ret = None;
        if self.manager.is_some() && self.render_in_progress {
            loop {
                match self.manager.as_ref().unwrap().rx.try_recv() {
                    Ok(msg) => match msg {
                        render_manager::Message::Finished {
                            render_id,
                            ray_count,
                        } => {
                            if render_id == self.render_id {
                                yuki_debug!("check_status: Render job has finished");
                                self.render_in_progress = false;
                                ret = Some(RenderStatus::Finished { ray_count });
                                break;
                            }
                            yuki_debug!("check_status: Stale render job has finished");
                            break;
                        }
                        render_manager::Message::Progress {
                            render_id,
                            active_threads,
                            tiles_done,
                            tiles_total,
                            approx_remaining_s,
                            current_rays_per_s,
                        } => {
                            if render_id == self.render_id {
                                yuki_debug!("check_status: Render job has progressed");
                                ret = Some(RenderStatus::Progress {
                                    active_threads,
                                    tiles_done,
                                    tiles_total,
                                    approx_remaining_s,
                                    current_rays_per_s,
                                });
                            } else {
                                yuki_debug!("check_status: Stale render job has progressed");
                            }
                        }
                    },
                    Err(why) => match why {
                        TryRecvError::Empty => {
                            yuki_debug!("check_status: Render job still running");
                            break;
                        }
                        TryRecvError::Disconnected => {
                            panic!("check_status: Render manager has been terminated");
                        }
                    },
                }
            }
        }
        ret
    }

    pub fn kill(&mut self) {
        if let Some(RenderManager { tx, handle, .. }) = self.manager.take() {
            drop(tx.send(None));
            handle.join().unwrap();
        }
    }

    /// Launch a new render task, overriding the previous one if one is already running.
    pub fn launch(
        &mut self,
        scene: Arc<Scene>,
        camera_params: CameraParameters,
        film: Arc<Mutex<Film>>,
        sampler: SamplerType,
        integrator: IntegratorType,
        film_settings: FilmSettings,
        render_settings: RenderSettings,
    ) {
        self.render_id += 1;

        if self.manager.is_none() {
            let (tx, manager_rx) = channel();
            let (manager_tx, rx) = channel();

            let handle = render_manager::launch(manager_tx, manager_rx);

            self.manager = Some(RenderManager { tx, rx, handle });
        }
        let manager = self.manager.as_ref().unwrap();

        yuki_debug!("launch: Sending new payload");
        match manager.tx.send(Some(render_manager::Payload {
            render_id: self.render_id,
            scene,
            camera_params,
            film,
            sampler,
            integrator,
            film_settings,
            render_settings,
        })) {
            Ok(_) => {
                self.render_in_progress = true;
            }
            Err(SendError(_)) => {
                panic!("launch: Render manager has been terminated");
            }
        }
    }
}

impl Drop for Renderer {
    fn drop(&mut self) {
        self.kill();
    }
}
