use std::{
    collections::{HashMap, VecDeque},
    ops::{Deref, DerefMut},
    sync::{
        mpsc::{channel, Receiver, RecvError, SendError, Sender, TryRecvError},
        Arc, Mutex,
    },
    thread::JoinHandle,
    time::Instant,
};

use crate::{
    camera::{Camera, CameraParameters},
    film::{film_tiles, Film, FilmSettings, FilmTile},
    integrators::IntegratorType,
    math::Spectrum,
    sampling::{Sampler, SamplerType},
    scene::Scene,
    yuki_debug, yuki_error, yuki_trace,
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
                        RenderManagerMessage::Finished {
                            render_id,
                            ray_count,
                        } => {
                            if render_id == self.render_id {
                                yuki_debug!("check_status: Render job has finished");
                                self.render_in_progress = false;
                                ret = Some(RenderStatus::Finished { ray_count });
                                break;
                            } else {
                                yuki_debug!("check_status: Stale render job has finished");
                                break;
                            }
                        }
                        RenderManagerMessage::Progress {
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
        mark_tiles: bool,
    ) {
        self.render_id += 1;

        if self.manager.is_none() {
            let (tx, manager_rx) = channel();
            let (manager_tx, rx) = channel();

            let handle = launch_manager(manager_tx, manager_rx);

            self.manager = Some(RenderManager { tx, rx, handle });
        }
        let manager = self.manager.as_ref().unwrap();

        yuki_debug!("launch: Sending new payload");
        match manager.tx.send(Some(RenderManagerPayload {
            render_id: self.render_id,
            scene,
            camera_params,
            film,
            sampler,
            integrator,
            film_settings,
            mark_tiles,
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

fn launch_manager(
    to_parent: Sender<RenderManagerMessage>,
    from_parent: Receiver<Option<RenderManagerPayload>>,
) -> JoinHandle<()> {
    std::thread::spawn(move || {
        yuki_trace!("Render manager: Launch threads");
        // TODO: Keep track of how physical vs logical behaves with optimizations
        let thread_count = num_cpus::get();
        let (child_send, from_children) = channel();
        let children: HashMap<usize, (Sender<Option<RenderThreadPayload>>, JoinHandle<_>)> = (0
            ..thread_count)
            .map(|thread| {
                let (to_child, child_receive) = channel();
                let child_send = child_send.clone();
                (
                    thread,
                    (
                        to_child,
                        std::thread::spawn(move || {
                            launch_worker(thread, &child_send, &child_receive);
                        }),
                    ),
                )
            })
            .collect();

        // Wait for children to finish
        'thread: loop {
            let mut active_tiles_total = 0;
            let mut active_tiles_done = 0;
            let mut active_render_id = 0;
            let mut active_children = 0;
            let mut ray_count = 0;
            let avg_tile_window = 2 * thread_count;
            let mut tile_infos = VecDeque::new();

            // Blocking recv to avoid spinlock when there is no need to message the parent
            let mut previous_message = match from_parent.recv() {
                Ok(msg) => Some(Ok(msg)),
                Err(RecvError {}) => {
                    panic!("Render manager: Receive channel disconnected")
                }
            };
            'work: loop {
                if previous_message.is_none() {
                    previous_message = Some(from_parent.try_recv());
                }
                let payload = match previous_message.take().unwrap() {
                    Ok(Some(payload)) => {
                        yuki_debug!("Render manager: Received new payload");
                        Some(payload)
                    }
                    Ok(None) => {
                        yuki_debug!("Render manager: Killed by parent");
                        break 'thread;
                    }
                    Err(TryRecvError::Disconnected) => {
                        panic!("Render manager: Receive channel disconnected")
                    }
                    Err(TryRecvError::Empty) => None,
                };

                if let Some(mut payload) = payload {
                    let camera = Camera::new(payload.camera_params, payload.film_settings);
                    let sampler = payload.sampler.instantiate(
                        1 + payload.integrator.n_sampled_dimensions(), // Camera sample and whatever the integrator needs
                    );

                    // TODO: This would be faster as a proper batch queues with work stealing,
                    //       though the gains are likely only seen on interactive "frame rates"
                    //       since sync only happens on tile pop (and film write).
                    //       Visible rendering order could be retained by distributing the
                    //       batches as interleaved (tiles i, 2*i, 3*i, ...)
                    let (tiles, tile_count) = {
                        let tiles = film_tiles(&mut payload.film, payload.film_settings);
                        let tile_count = tiles.len();
                        (Arc::new(Mutex::new(tiles)), tile_count)
                    };

                    active_render_id = payload.render_id;

                    active_tiles_done = 0;
                    active_tiles_total = tile_count;
                    ray_count = 0;

                    for (tx, _) in children.values() {
                        let payload = RenderThreadPayload {
                            render_id: active_render_id,
                            tiles: Arc::clone(&tiles),
                            camera: camera.clone(),
                            scene: Arc::clone(&payload.scene),
                            integrator_type: payload.integrator,
                            sampler: Arc::clone(&sampler),
                            film: Arc::clone(&payload.film),
                            mark_tiles: payload.mark_tiles,
                        };

                        if let Err(SendError { .. }) = tx.send(Some(payload)) {
                            panic!("launch: Worker has been terminated");
                        }
                    }

                    active_children = children.len();
                } else {
                    let prev_active_children = active_children;

                    while let Ok(msg) = from_children.try_recv() {
                        match msg {
                            RenderThreadMessage::Finished(ThreadInfo {
                                thread_id,
                                render_id,
                            }) => {
                                if render_id == active_render_id {
                                    yuki_trace!("Render manager: Worker {} finished", thread_id);
                                    active_children -= 1;
                                } else {
                                    yuki_trace!(
                                        "Render manager: Worker {} finished stale work",
                                        thread_id
                                    );
                                }
                            }
                            RenderThreadMessage::TileDone {
                                info,
                                ray_count: rays,
                                elapsed_s,
                            } => {
                                if info.render_id == active_render_id {
                                    ray_count += rays;

                                    if tile_infos.len() >= avg_tile_window {
                                        tile_infos.pop_front();
                                    }
                                    tile_infos.push_back((elapsed_s, rays));

                                    active_tiles_done += 1;

                                    let avg_s_per_tile =
                                        tile_infos.iter().map(|(s, _)| s).sum::<f32>()
                                            / (tile_infos.len() as f32);

                                    let approx_remaining_s = avg_s_per_tile
                                        * ((active_tiles_total - active_tiles_done) as f32)
                                        / (active_children as f32);

                                    let current_rays_per_s = tile_infos
                                        .iter()
                                        // Sum of averages to downplay overtly expensive threads
                                        .map(|&(s, r)| (r as f32) / s)
                                        .sum::<f32>()
                                        / (tile_infos.len() as f32)
                                        * (active_children as f32);

                                    if let Err(why) =
                                        to_parent.send(RenderManagerMessage::Progress {
                                            render_id: active_render_id,
                                            active_threads: active_children,
                                            tiles_done: active_tiles_done,
                                            tiles_total: active_tiles_total,
                                            approx_remaining_s,
                                            current_rays_per_s,
                                        })
                                    {
                                        yuki_error!(
                                            "Render manager: Error sending progress to parent: {}",
                                            why
                                        );
                                    }
                                }
                            }
                        }
                    }

                    let task_finished = prev_active_children > 0 && active_children == 0;

                    if task_finished {
                        yuki_trace!("Render manager: Report back");
                        if let Err(why) = to_parent.send(RenderManagerMessage::Finished {
                            render_id: active_render_id,
                            ray_count,
                        }) {
                            yuki_error!(
                                "Render manager: Error notifying parent on finish: {}",
                                why
                            );
                        };
                        break 'work;
                    }
                }
            }
        }

        // Kill children after being killed
        if !children.is_empty() {
            // Kill everyone first
            for (tx, _) in children.values() {
                // No need to check for error, child having disconnected, since that's our goal
                drop(tx.send(None));
            }
        }

        yuki_debug!("Render manager: End");
    })
}

fn launch_worker(
    thread_id: usize,
    to_parent: &Sender<RenderThreadMessage>,
    from_parent: &Receiver<Option<RenderThreadPayload>>,
) {
    yuki_debug!("Render thread {}: Begin", thread_id);

    'thread: loop {
        let mut thread_info = ThreadInfo {
            render_id: 0,
            thread_id,
        };
        let mut payload: Option<RenderThreadPayload> = None;

        // Blocking recv to avoid spinlock when there is no need to message to parent
        let mut newest_msg = match from_parent.recv() {
            Ok(msg) => Some(Ok(msg)),
            Err(RecvError {}) => {
                panic!("Render thread {}: Receive channel disconnected", thread_id)
            }
        };
        'work: loop {
            // Receive new payload before getting next tile to ensure the tile we get
            // matches the held film
            let tile = {
                // We try_recv both here and inside render kernel so we can interrupt render mid-kernel
                if newest_msg.is_none() {
                    newest_msg = Some(from_parent.try_recv());
                }

                match newest_msg.take().unwrap() {
                    Ok(Some(new_payload)) => {
                        yuki_debug!("Render thread {}: Received new payload", thread_id);
                        thread_info.render_id = new_payload.render_id;
                        payload = Some(new_payload);
                    }
                    Ok(None) => {
                        yuki_debug!("Render thread {}: Killed by parent", thread_id);
                        break 'thread;
                    }
                    Err(TryRecvError::Disconnected) => {
                        panic!("Render thread {}: Receive channel disconnected", thread_id);
                    }
                    Err(TryRecvError::Empty) => (),
                }

                let tile = payload.as_deref_mut().and_then(|payload| {
                    let mut tiles = payload.tiles.lock().unwrap();
                    tiles.pop_front()
                });

                if payload.is_some() && tile.is_none() {
                    yuki_trace!("Render thread {}: Signal done", thread_id);

                    if let Err(why) = to_parent.send(RenderThreadMessage::Finished(thread_info)) {
                        yuki_error!(
                            "Render thread {}: Error notifying parent on finish: {}",
                            thread_id,
                            why
                        );
                    };

                    break 'work;
                }

                tile
            };

            if let Some(mut tile) = tile {
                assert!(payload.is_some(), "Active tile without payload");

                let tile_start = Instant::now();

                let payload = payload.as_ref().unwrap();
                if payload.mark_tiles {
                    yuki_trace!("Render thread {}: Mark tile {:?}", thread_id, tile.bb);
                    yuki_trace!("Render thread {}: Waiting for lock on film", thread_id);
                    let mut film = payload.film.lock().unwrap();
                    yuki_trace!("Render thread {}: Acquired film", thread_id);

                    if film.matches(&tile) {
                        film.mark(&tile, Spectrum::new(1.0, 0.0, 1.0));
                    }

                    yuki_trace!("Render thread {}: Releasing film", thread_id);
                }

                yuki_trace!("Render thread {}: Render tile {:?}", thread_id, tile.bb);
                let mut interrupted = false;
                let integrator = payload.integrator_type.instantiate();
                let ray_count = integrator.render(
                    &payload.scene,
                    &payload.camera,
                    &payload.sampler,
                    &mut tile,
                    &mut || {
                        // Let's have low latency kills for more interactive view
                        if let Ok(msg) = from_parent.try_recv() {
                            yuki_debug!("Render thread {}: Interrupted by parent", thread_id);
                            newest_msg = Some(Ok(msg));
                            interrupted = true;
                        }
                        interrupted
                    },
                );

                if !interrupted {
                    yuki_trace!("Render thread {}: Update tile {:?}", thread_id, tile.bb);
                    {
                        yuki_trace!("Render thread {}: Waiting for lock on film", thread_id);
                        let mut film = payload.film.lock().unwrap();
                        yuki_trace!("Render thread {}: Acquired film", thread_id);

                        if film.matches(&tile) {
                            film.update_tile(&tile);
                        } else {
                            yuki_trace!("Render thread {}: Stale tile", thread_id);
                        }

                        yuki_trace!("Render thread {}: Releasing film", thread_id);
                    }

                    if let Err(why) = to_parent.send(RenderThreadMessage::TileDone {
                        info: thread_info,
                        ray_count,
                        elapsed_s: tile_start.elapsed().as_secs_f32(),
                    }) {
                        yuki_error!(
                            "Render thread {}: Error notifying parent on tile done: {}",
                            thread_id,
                            why
                        );
                    };
                }
            }
        }
    }
}

enum RenderThreadMessage {
    TileDone {
        info: ThreadInfo,
        ray_count: usize,
        elapsed_s: f32,
    },
    Finished(ThreadInfo),
}

#[derive(Clone, Copy)]
struct ThreadInfo {
    render_id: usize,
    thread_id: usize,
}

enum RenderManagerMessage {
    Progress {
        render_id: usize,
        active_threads: usize,
        tiles_done: usize,
        tiles_total: usize,
        approx_remaining_s: f32,
        current_rays_per_s: f32,
    },
    Finished {
        render_id: usize,
        ray_count: usize,
    },
}

struct RenderManager {
    tx: Sender<Option<RenderManagerPayload>>,
    rx: Receiver<RenderManagerMessage>,
    handle: JoinHandle<()>,
}

struct RenderManagerPayload {
    render_id: usize,
    scene: Arc<Scene>,
    camera_params: CameraParameters,
    film: Arc<Mutex<Film>>,
    sampler: SamplerType,
    integrator: IntegratorType,
    film_settings: FilmSettings,
    mark_tiles: bool,
}

struct RenderThreadPayload {
    render_id: usize,
    tiles: Arc<Mutex<VecDeque<FilmTile>>>,
    camera: Camera,
    scene: Arc<Scene>,
    integrator_type: IntegratorType,
    sampler: Arc<dyn Sampler>,
    film: Arc<Mutex<Film>>,
    mark_tiles: bool,
}

impl Deref for RenderThreadPayload {
    type Target = Self;
    fn deref(&self) -> &Self::Target {
        self
    }
}

impl DerefMut for RenderThreadPayload {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self
    }
}
