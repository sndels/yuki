use allocators::{LinearAllocator, ScopedScratch};
use serde::{Deserialize, Serialize};
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
                        RenderManagerMessage::Finished {
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
        render_settings: RenderSettings,
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

struct TileInfo {
    elapsed_s: f32,
    rays: usize,
}

struct ManagerState {
    active_tiles_total: usize,
    active_tiles_done: usize,
    active_render_id: usize,
    active_workers: usize,
    ray_count: usize,
    tile_infos: VecDeque<TileInfo>,
}

impl Default for ManagerState {
    fn default() -> Self {
        Self {
            active_tiles_total: 0,
            active_tiles_done: 0,
            active_render_id: 0,
            active_workers: 0,
            ray_count: 0,
            tile_infos: VecDeque::new(),
        }
    }
}

fn launch_manager(
    to_parent: Sender<RenderManagerMessage>,
    from_parent: Receiver<Option<RenderManagerPayload>>,
) -> JoinHandle<()> {
    std::thread::spawn(move || {
        yuki_trace!("Render manager: Launch threads");
        // TODO: Keep track of how physical vs logical behaves with optimizations
        let thread_count = num_cpus::get() - 1;
        let (worker_send, from_workers) = channel();
        let workers = (0..thread_count)
            .map(|thread| {
                let (to_worker, worker_receive) = channel();
                let worker_send = worker_send.clone();
                (
                    thread,
                    (
                        to_worker,
                        std::thread::spawn(move || {
                            launch_worker(thread, &worker_send, &worker_receive);
                        }),
                    ),
                )
            })
            .collect();

        // Wait for workers to finish
        'thread: loop {
            let mut state = ManagerState::default();
            let avg_tile_window = 2 * thread_count;

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

                if let Some(payload) = payload {
                    propagate_payload(payload, &workers, &mut state);
                } else {
                    let active_workers = state.active_workers;

                    handle_worker_messages(&from_workers, &to_parent, avg_tile_window, &mut state);

                    let task_finished = active_workers > 0 && state.active_workers == 0;

                    if task_finished {
                        yuki_trace!("Render manager: Report back");
                        if let Err(why) = to_parent.send(RenderManagerMessage::Finished {
                            render_id: state.active_render_id,
                            ray_count: state.ray_count,
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

        // Kill workers after being killed
        if !workers.is_empty() {
            // Kill everyone first
            for (tx, _) in workers.values() {
                // No need to check for error, worker having disconnected, since that's our goal
                drop(tx.send(None));
            }
        }

        yuki_debug!("Render manager: End");
    })
}

type WorkerMap = HashMap<usize, (Sender<Option<RenderThreadPayload>>, JoinHandle<()>)>;

fn propagate_payload(
    mut payload: RenderManagerPayload,
    workers: &WorkerMap,
    state: &mut ManagerState,
) {
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

    let mut active_workers = 0;
    for (tx, _) in workers.values() {
        let thread_payload = RenderThreadPayload {
            render_id: payload.render_id,
            tiles: Arc::clone(&tiles),
            camera: camera.clone(),
            scene: Arc::clone(&payload.scene),
            integrator_type: payload.integrator,
            sampler: Arc::clone(&sampler),
            film: Arc::clone(&payload.film),
            mark_tiles: payload.render_settings.mark_tiles,
        };

        if let Err(SendError { .. }) = tx.send(Some(thread_payload)) {
            panic!("launch: Worker has been terminated");
        }

        active_workers += 1;

        if payload.render_settings.use_single_render_thread {
            break;
        }
    }

    *state = ManagerState {
        active_tiles_total: tile_count,
        active_render_id: payload.render_id,
        active_workers: active_workers,
        ..Default::default()
    };
}

fn handle_worker_messages(
    from_workers: &Receiver<RenderThreadMessage>,
    to_parent: &Sender<RenderManagerMessage>,
    avg_tile_window: usize,
    state: &mut ManagerState,
) {
    let ManagerState {
        active_tiles_total,
        active_tiles_done,
        active_render_id,
        active_workers,
        ray_count,
        tile_infos,
    } = state;

    while let Ok(msg) = from_workers.try_recv() {
        match msg {
            RenderThreadMessage::Finished(ThreadInfo {
                thread_id,
                render_id,
            }) => {
                if render_id == *active_render_id {
                    yuki_trace!("Render manager: Worker {} finished", thread_id);
                    *active_workers -= 1;
                } else {
                    yuki_trace!("Render manager: Worker {} finished stale work", thread_id);
                }
            }
            RenderThreadMessage::TileDone {
                info,
                ray_count: rays,
                elapsed_s,
            } => {
                if info.render_id == *active_render_id {
                    *ray_count += rays;

                    if tile_infos.len() >= avg_tile_window {
                        tile_infos.pop_front();
                    }
                    tile_infos.push_back(TileInfo { elapsed_s, rays });

                    *active_tiles_done += 1;

                    let avg_s_per_tile = tile_infos
                        .iter()
                        .map(|TileInfo { elapsed_s, .. }| elapsed_s)
                        .sum::<f32>()
                        / (tile_infos.len() as f32);

                    let approx_remaining_s = avg_s_per_tile
                        * ((*active_tiles_total - *active_tiles_done) as f32)
                        / (*active_workers as f32);

                    let current_rays_per_s = tile_infos
                        .iter()
                        // Sum of averages to downplay overtly expensive threads
                        .map(|&TileInfo { elapsed_s, rays }| (rays as f32) / elapsed_s)
                        .sum::<f32>()
                        / (tile_infos.len() as f32)
                        * (*active_workers as f32);

                    if let Err(why) = to_parent.send(RenderManagerMessage::Progress {
                        render_id: *active_render_id,
                        active_threads: *active_workers,
                        tiles_done: *active_tiles_done,
                        tiles_total: *active_tiles_total,
                        approx_remaining_s,
                        current_rays_per_s,
                    }) {
                        yuki_error!("Render manager: Error sending progress to parent: {}", why);
                    }
                }
            }
        }
    }
}

fn launch_worker(
    thread_id: usize,
    to_parent: &Sender<RenderThreadMessage>,
    from_parent: &Receiver<Option<RenderThreadPayload>>,
) {
    yuki_debug!("Render thread {}: Begin", thread_id);

    let mut alloc = LinearAllocator::new(1024 * 256);
    let scratch = ScopedScratch::new(&mut alloc);

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
            // Handle messages before getting next tile to ensure the tile we get
            // matches the held film
            if handle_manager_messages(
                thread_id,
                newest_msg.take(),
                &from_parent,
                &mut thread_info,
                &mut payload,
            ) {
                break 'thread;
            }

            let tile = match payload.as_deref_mut() {
                Some(p) => match pop_tile_or_signal_finish(&thread_info, p, &to_parent) {
                    Some(tile) => Some(tile),
                    None => break 'work,
                },
                None => None,
            };

            if let Some(mut tile) = tile {
                assert!(payload.is_some(), "Active tile without payload");

                let payload = payload.as_deref().unwrap();

                let tile_start = Instant::now();
                match render_tile(thread_id, &scratch, &mut tile, payload, from_parent) {
                    RenderTileResult::Interrupted(p) => newest_msg = Some(Ok(p)),
                    RenderTileResult::Rendered { ray_count } => update_tile(
                        &thread_info,
                        &mut tile,
                        payload,
                        ray_count,
                        tile_start,
                        &to_parent,
                    ),
                }
            }
        }
    }
}

// Returns `true` if manager signaled kill
fn handle_manager_messages(
    thread_id: usize,
    newest_msg: Option<Result<Option<RenderThreadPayload>, RecvError>>,
    from_parent: &Receiver<Option<RenderThreadPayload>>,
    thread_info: &mut ThreadInfo,
    payload: &mut Option<RenderThreadPayload>,
) -> bool {
    match newest_msg
        .map(|r| match r {
            Ok(msg) => Ok(msg),
            Err(e) => Err(TryRecvError::from(e)),
        })
        .unwrap_or_else(|| from_parent.try_recv())
    {
        Ok(Some(new_payload)) => {
            yuki_debug!("Render thread {}: Received new payload", thread_id);
            thread_info.render_id = new_payload.render_id;
            *payload = Some(new_payload);
        }
        Ok(None) => {
            yuki_debug!("Render thread {}: Killed by parent", thread_id);
            return true;
        }
        Err(TryRecvError::Disconnected) => {
            panic!("Render thread {}: Receive channel disconnected", thread_id);
        }
        Err(TryRecvError::Empty) => (),
    }

    false
}

fn pop_tile_or_signal_finish(
    thread_info: &ThreadInfo,
    payload: &mut RenderThreadPayload,
    to_parent: &Sender<RenderThreadMessage>,
) -> Option<FilmTile> {
    let tile = {
        let mut tiles = payload.tiles.lock().unwrap();
        tiles.pop_front()
    };

    match tile {
        Some(tile) => Some(tile),
        None => {
            yuki_trace!("Render thread {}: Signal done", thread_info.thread_id);

            if let Err(why) = to_parent.send(RenderThreadMessage::Finished(*thread_info)) {
                yuki_error!(
                    "Render thread {}: Error notifying parent on finish: {}",
                    thread_info.thread_id,
                    why
                );
            };

            None
        }
    }
}

enum RenderTileResult {
    Interrupted(Option<RenderThreadPayload>),
    Rendered { ray_count: usize },
}

fn render_tile(
    thread_id: usize,
    scratch: &ScopedScratch,
    tile: &mut FilmTile,
    payload: &RenderThreadPayload,
    from_parent: &Receiver<Option<RenderThreadPayload>>,
) -> RenderTileResult {
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
    let mut received_msg = None;
    let tile_scratch = ScopedScratch::new_scope(&scratch);
    let integrator = payload.integrator_type.instantiate();
    let ray_count = integrator.render(
        &tile_scratch,
        &payload.scene,
        &payload.camera,
        &payload.sampler,
        tile,
        &mut || {
            // Let's have low latency kills for more interactive view
            if let Ok(msg) = from_parent.try_recv() {
                yuki_debug!("Render thread {}: Interrupted by parent", thread_id);
                received_msg = Some(msg);
                true
            } else {
                false
            }
        },
    );

    match received_msg {
        Some(msg) => RenderTileResult::Interrupted(msg),
        None => RenderTileResult::Rendered { ray_count },
    }
}

fn update_tile(
    thread_info: &ThreadInfo,
    tile: &mut FilmTile,
    payload: &RenderThreadPayload,
    ray_count: usize,
    tile_start: Instant,
    to_parent: &Sender<RenderThreadMessage>,
) {
    yuki_trace!(
        "Render thread {}: Update tile {:?}",
        thread_info.thread_id,
        tile.bb
    );
    {
        yuki_trace!(
            "Render thread {}: Waiting for lock on film",
            thread_info.thread_id
        );
        let mut film = payload.film.lock().unwrap();
        yuki_trace!("Render thread {}: Acquired film", thread_info.thread_id);

        if film.matches(&tile) {
            film.update_tile(&tile);
        } else {
            yuki_trace!("Render thread {}: Stale tile", thread_info.thread_id);
        }

        yuki_trace!("Render thread {}: Releasing film", thread_info.thread_id);
    }

    if let Err(why) = to_parent.send(RenderThreadMessage::TileDone {
        info: *thread_info,
        ray_count,
        elapsed_s: tile_start.elapsed().as_secs_f32(),
    }) {
        yuki_error!(
            "Render thread {}: Error notifying parent on tile done: {}",
            thread_info.thread_id,
            why
        );
    };
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
    render_settings: RenderSettings,
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
