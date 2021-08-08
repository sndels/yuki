use std::{
    collections::{HashMap, VecDeque},
    sync::{
        mpsc::{channel, Receiver, Sender, TryRecvError},
        Arc, Mutex,
    },
    thread::JoinHandle,
    time::Instant,
};

use crate::{
    camera::{Camera, CameraParameters},
    film::{film_tiles, Film, FilmSettings, FilmTile},
    integrators::IntegratorType,
    math::Vec3,
    samplers::{create_sampler, Sampler, SamplerSettings},
    scene::Scene,
    yuki_debug, yuki_error, yuki_info, yuki_trace, yuki_warn,
};

#[derive(Copy, Clone)]
pub struct RenderResult {
    pub secs: f32,
    pub ray_count: usize,
}

struct RenderTask {
    pub tx_task: Option<Sender<usize>>,
    pub rx_task: Receiver<RenderResult>,
    pub handle: JoinHandle<()>,
}

impl RenderTask {
    pub fn new(
        tx_task: Option<Sender<usize>>,
        rx_task: Receiver<RenderResult>,
        handle: JoinHandle<()>,
    ) -> Self {
        Self {
            tx_task,
            rx_task,
            handle,
        }
    }
}

pub struct Renderer {
    render_task: Option<RenderTask>,
    task_ending: bool,
}

impl Renderer {
    pub fn new() -> Self {
        Self {
            render_task: None,
            task_ending: false,
        }
    }

    /// Waits for the render task to end and returns its result
    pub fn wait_result(&mut self) -> Result<RenderResult, String> {
        let task = std::mem::replace(&mut self.render_task, None);
        self.task_ending = false;

        if let Some(RenderTask {
            rx_task, handle, ..
        }) = task
        {
            if let Ok(result) = rx_task.recv() {
                yuki_trace!("wait_result: Waiting for the finished render job to exit");
                handle.join().unwrap();
                yuki_debug!("wait_result: Render job has finished");
                Ok(result)
            } else {
                handle.join().unwrap();
                Err("Render task disconnected without notifying".into())
            }
        } else {
            Err("No render task active".into())
        }
    }

    /// Checks if the render task is active.
    pub fn is_active(&self) -> bool {
        self.render_task.is_some()
    }

    /// Checks if the render task has been killed and is in the process of winding down.
    pub fn is_winding_down(&self) -> bool {
        self.task_ending
    }

    /// Returns the `RenderResult` if the task has finished.
    pub fn check_result(&mut self) -> Option<RenderResult> {
        let mut ret = None;
        let task = std::mem::replace(&mut self.render_task, None);
        if let Some(RenderTask {
            tx_task,
            rx_task,
            handle,
        }) = task
        {
            match rx_task.try_recv() {
                Ok(result) => {
                    yuki_trace!("check_result: Waiting for the finished render job to exit");
                    handle.join().unwrap();
                    yuki_debug!("check_result: Render job has finished");
                    ret = Some(result);
                    self.task_ending = false;
                }
                Err(why) => match why {
                    TryRecvError::Empty => {
                        yuki_debug!("check_result: Render job still running");
                        self.render_task = Some(RenderTask::new(tx_task, rx_task, handle));
                    }
                    TryRecvError::Disconnected => {
                        yuki_warn!("check_result: Render disconnected without notifying");
                        handle.join().unwrap();
                    }
                },
            }
        }
        ret
    }

    /// Checks if the render task has finished and kills it if it has not.
    /// Returns `true` if the task has finished, `false` if it is winding down after being killed.
    pub fn has_finished_or_kill(&mut self) -> bool {
        let task = std::mem::replace(&mut self.render_task, None);
        if let Some(RenderTask {
            tx_task,
            rx_task,
            handle,
        }) = task
        {
            yuki_trace!("has_finished_or_kill: Checking if the render job has finished");
            // See if the task has completed
            match rx_task.try_recv() {
                Ok(_) => {
                    yuki_trace!(
                        "has_finished_or_kill: Waiting for the finished render job to exit"
                    );
                    handle.join().unwrap();
                    yuki_debug!("has_finished_or_kill: Render job has finished");
                    self.task_ending = false;
                    true
                }
                Err(why) => {
                    // Task is either still running or has disconnected without notifying us
                    match why {
                        TryRecvError::Empty => {
                            yuki_debug!("has_finished_or_kill: Render job still running");
                            // Only send the kill command once
                            if let Some(tx) = tx_task {
                                yuki_debug!(
                                    "has_finished_or_kill: Sending kill command to the render job"
                                );
                                let _ = tx.send(0);
                            }
                            // Keep handles to continue polling until the thread has stopped
                            self.render_task = Some(RenderTask::new(None, rx_task, handle));
                            self.task_ending = true;
                            false
                        }
                        TryRecvError::Disconnected => {
                            yuki_warn!(
                                "has_finished_or_kill: Render disconnected without notifying"
                            );
                            handle.join().unwrap();
                            true
                        }
                    }
                }
            }
        } else {
            yuki_debug!("has_finished_or_kill: No existing render job");
            true
        }
    }

    /// Launch render task.
    pub fn launch(
        &mut self,
        scene: Arc<Scene>,
        camera_params: CameraParameters,
        film: Arc<Mutex<Film>>,
        sampler_settings: SamplerSettings,
        integrator: IntegratorType,
        film_settings: FilmSettings,
        match_logical_cores: bool,
    ) {
        let (to_render, render_rx) = channel();
        let (render_tx, from_render) = channel();

        yuki_trace!("launch: Launching render job");

        let render_thread = launch_render(
            render_tx,
            render_rx,
            scene,
            camera_params,
            film,
            integrator,
            create_sampler(sampler_settings),
            film_settings,
            match_logical_cores,
        );

        yuki_info!("launch: Render job launched");

        self.render_task = Some(RenderTask::new(Some(to_render), from_render, render_thread));
    }
}

impl Drop for Renderer {
    fn drop(&mut self) {
        let task = std::mem::replace(&mut self.render_task, None);
        if let Some(RenderTask {
            tx_task, handle, ..
        }) = task
        {
            if let Some(tx) = tx_task {
                let _ = tx.send(0);
            }
            handle.join().unwrap();
        }
    }
}

fn launch_render(
    to_parent: Sender<RenderResult>,
    from_parent: Receiver<usize>,
    scene: Arc<Scene>,
    camera_params: CameraParameters,
    mut film: Arc<Mutex<Film>>,
    integrator: IntegratorType,
    sampler: Arc<dyn Sampler>,
    film_settings: FilmSettings,
    match_logical_cores: bool,
) -> JoinHandle<()> {
    let camera = Camera::new(camera_params, film_settings);

    std::thread::spawn(move || {
        yuki_debug!("Render: Begin");
        yuki_trace!("Render: Getting tiles");
        // Get tiles, resizes film if necessary
        let tiles = Arc::new(Mutex::new(film_tiles(&mut film, film_settings)));

        yuki_trace!("Render: Launch threads");
        let render_start = Instant::now();
        let thread_count = if match_logical_cores {
            num_cpus::get()
        } else {
            num_cpus::get_physical()
        };
        let (child_send, from_children) = channel();
        let mut children: HashMap<usize, (Sender<usize>, JoinHandle<_>)> = (0..thread_count)
            .map(|i| {
                let (to_child, child_receive) = channel();
                let child_send = child_send.clone();
                let tiles = Arc::clone(&tiles);
                let camera = camera.clone();
                let scene = Arc::clone(&scene);
                let film = Arc::clone(&film);
                let sampler = Arc::clone(&sampler);
                (
                    i,
                    (
                        to_child,
                        std::thread::spawn(move || {
                            render(
                                i,
                                &child_send,
                                &child_receive,
                                &tiles,
                                &scene,
                                &camera,
                                integrator,
                                &sampler,
                                &film,
                            );
                        }),
                    ),
                )
            })
            .collect();

        // Wait for children to finish
        let mut ray_count = 0;
        while !children.is_empty() {
            if from_parent.try_recv().is_ok() {
                yuki_debug!("Render: Killed by parent");
                break;
            }

            if let Ok((thread_id, rays)) = from_children.try_recv() {
                yuki_trace!("Render: Join {}", thread_id);
                let (_, child) = children.remove(&thread_id).unwrap();
                child.join().unwrap();
                yuki_trace!("Render: {} terminated", thread_id);
                ray_count += rays;
            } else {
                std::thread::sleep(std::time::Duration::from_millis(5));
            }
        }

        // Kill children after being killed
        if !children.is_empty() {
            // Kill everyone first
            for (tx, _) in children.values() {
                // No need to check for error, child having disconnected, since that's our goal
                let _ = tx.send(0);
            }
            // Wait for everyone to end
            for (thread_id, (_, child)) in children {
                // This message might not be from the same child, but we don't really care as long as
                // every child notifies us
                from_children.recv().unwrap();
                child.join().unwrap();
                yuki_debug!("Render: {} terminated", thread_id);
            }
        }

        yuki_trace!("Render: Report back");
        let secs = (render_start.elapsed().as_micros() as f32) * 1e-6;
        if let Err(why) = to_parent.send(RenderResult { secs, ray_count }) {
            yuki_error!("Render: Error notifying parent: {}", why);
        };
        yuki_debug!("Render: End");
    })
}

fn render(
    thread_id: usize,
    to_parent: &Sender<(usize, usize)>,
    from_parent: &Receiver<usize>,
    tiles: &Arc<Mutex<VecDeque<FilmTile>>>,
    scene: &Arc<Scene>,
    camera: &Camera,
    integrator_type: IntegratorType,
    sampler: &Arc<dyn Sampler>,
    film: &Arc<Mutex<Film>>,
) {
    yuki_debug!("Render thread {}: Begin", thread_id);

    let mut rays = 0;
    'work: loop {
        if from_parent.try_recv().is_ok() {
            yuki_debug!("Render thread {}: Killed by parent", thread_id);
            break 'work;
        }

        let tile = {
            let mut tiles = tiles.lock().unwrap();
            tiles.pop_front()
        };
        if tile.is_none() {
            break;
        }
        let mut tile = tile.unwrap();
        yuki_trace!("Render thread {}: Mark tile {:?}", thread_id, tile.bb);
        {
            yuki_trace!("Render thread {}: Waiting for lock on film", thread_id);
            let mut film = film.lock().unwrap();
            yuki_trace!("Render thread {}: Acquired film", thread_id);

            film.mark(&tile, Vec3::new(1.0, 0.0, 1.0));

            yuki_trace!("Render thread {}: Releasing film", thread_id);
        }

        yuki_trace!("Render thread {}: Render tile {:?}", thread_id, tile.bb);
        let mut terminated_early = false;
        let integrator = integrator_type.instantiate();
        rays += integrator.render(scene, camera, sampler, &mut tile, &mut || {
            // Let's have low latency kills for more interactive view
            if from_parent.try_recv().is_ok() {
                yuki_debug!("Render thread {}: Killed by parent", thread_id);
                terminated_early = true;
            }
            terminated_early
        });
        if terminated_early {
            break 'work;
        }

        yuki_trace!("Render thread {}: Update tile {:?}", thread_id, tile.bb);
        {
            yuki_trace!("Render thread {}: Waiting for lock on film", thread_id);
            let mut film = film.lock().unwrap();
            yuki_trace!("Render thread {}: Acquired film", thread_id);

            film.update_tile(&tile);

            yuki_trace!("Render thread {}: Releasing film", thread_id);
        }
    }

    yuki_trace!("Render thread {}: Signal end", thread_id);
    if let Err(why) = to_parent.send((thread_id, rays)) {
        yuki_error!("Render thread {}: Error: {}", thread_id, why);
    };
    yuki_debug!("Render thread {}: End", thread_id);
}
