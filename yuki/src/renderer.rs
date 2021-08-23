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
    math::Vec3,
    sampling::{create_sampler, Sampler, SamplerSettings},
    scene::Scene,
    yuki_debug, yuki_error, yuki_trace,
};

#[derive(Copy, Clone)]
pub struct RenderResult {
    pub secs: f32,
    pub ray_count: usize,
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

    /// Waits for the render task to end and returns its result
    pub fn wait_result(&mut self) -> Result<RenderResult, String> {
        if self.render_in_progress {
            loop {
                if let Ok(RenderManagerResult {
                    render_id,
                    secs,
                    ray_count,
                }) = self.manager.as_ref().unwrap().rx.recv()
                {
                    if render_id == self.render_id {
                        yuki_debug!("check_result: Render job has finished");
                        self.render_in_progress = false;

                        return Ok(RenderResult { secs, ray_count });
                    }
                    assert!(
                        render_id < self.render_id,
                        "Render result appears to be from the future"
                    );
                    yuki_debug!("check_result: Stale render job has finished");
                } else {
                    panic!("Render manager disconnected");
                }
            }
        } else {
            Err("No render in progress".into())
        }
    }

    /// Returns the `RenderResult` if the task has finished.
    pub fn check_result(&mut self) -> Option<RenderResult> {
        if self.manager.is_some() && self.render_in_progress {
            match self.manager.as_ref().unwrap().rx.try_recv() {
                Ok(RenderManagerResult {
                    render_id,
                    secs,
                    ray_count,
                }) => {
                    if render_id == self.render_id {
                        yuki_debug!("check_result: Render job has finished");
                        self.render_in_progress = false;
                        Some(RenderResult { secs, ray_count })
                    } else {
                        yuki_debug!("check_result: Stale render job has finished");
                        None
                    }
                }
                Err(why) => match why {
                    TryRecvError::Empty => {
                        yuki_debug!("check_result: Render job still running");
                        None
                    }
                    TryRecvError::Disconnected => {
                        panic!("check_result: Render manager has been terminated");
                    }
                },
            }
        } else {
            None
        }
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
        sampler_settings: SamplerSettings,
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
            sampler_settings,
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
    to_parent: Sender<RenderManagerResult>,
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
            let mut active_render_id = 0;
            let mut active_children = 0;
            let mut ray_count = 0;
            let mut render_start = Instant::now();

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
                    let sampler = Arc::new(create_sampler(payload.sampler_settings));

                    // TODO: This would be faster as a proper batch queues with work stealing,
                    //       though the gains are likely only seen on interactive "frame rates"
                    //       since sync only happens on tile pop (and film write).
                    //       Visible rendering order could be retained by distributing the
                    //       batches as interleaved (tiles i, 2*i, 3*i, ...)
                    let tiles = Arc::new(Mutex::new(film_tiles(
                        &mut payload.film,
                        payload.film_settings,
                    )));

                    active_render_id = payload.render_id;

                    render_start = Instant::now();

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

                    while let Ok(RenderThreadResult {
                        thread_id,
                        render_id,
                        ray_count: rays,
                    }) = from_children.try_recv()
                    {
                        if render_id == active_render_id {
                            yuki_trace!("Render manager: Worker {} finished", thread_id);
                            active_children -= 1;
                            ray_count += rays;
                        } else {
                            yuki_trace!("Render manager: Worker {} finished stale work", thread_id);
                        }
                    }

                    let task_finished = prev_active_children > 0 && active_children == 0;

                    if task_finished {
                        yuki_trace!("Render manager: Report back");
                        let secs = (render_start.elapsed().as_micros() as f32) * 1e-6;
                        if let Err(why) = to_parent.send(RenderManagerResult {
                            render_id: active_render_id,
                            secs,
                            ray_count,
                        }) {
                            yuki_error!("Render manager: Error notifying parent: {}", why);
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
    to_parent: &Sender<RenderThreadResult>,
    from_parent: &Receiver<Option<RenderThreadPayload>>,
) {
    yuki_debug!("Render thread {}: Begin", thread_id);

    'thread: loop {
        let mut ray_count = 0;
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
                        payload = Some(new_payload);
                        ray_count = 0;
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

                    if let Err(why) = to_parent.send(RenderThreadResult {
                        render_id: payload.as_ref().unwrap().render_id,
                        thread_id,
                        ray_count,
                    }) {
                        yuki_error!("Render thread {}: Error: {}", thread_id, why);
                    };

                    break 'work;
                }

                tile
            };

            if let Some(mut tile) = tile {
                assert!(payload.is_some(), "Active tile without payload");

                let payload = payload.as_ref().unwrap();
                if payload.mark_tiles {
                    yuki_trace!("Render thread {}: Mark tile {:?}", thread_id, tile.bb);
                    yuki_trace!("Render thread {}: Waiting for lock on film", thread_id);
                    let mut film = payload.film.lock().unwrap();
                    yuki_trace!("Render thread {}: Acquired film", thread_id);

                    if film.matches(&tile) {
                        film.mark(&tile, Vec3::new(1.0, 0.0, 1.0));
                    }

                    yuki_trace!("Render thread {}: Releasing film", thread_id);
                }

                yuki_trace!("Render thread {}: Render tile {:?}", thread_id, tile.bb);
                let mut interrupted = false;
                let integrator = payload.integrator_type.instantiate();
                ray_count += integrator.render(
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
                }
            }
        }
    }
}

struct RenderThreadResult {
    render_id: usize,
    thread_id: usize,
    ray_count: usize,
}

struct RenderManagerResult {
    render_id: usize,
    secs: f32,
    ray_count: usize,
}

struct RenderManager {
    tx: Sender<Option<RenderManagerPayload>>,
    rx: Receiver<RenderManagerResult>,
    handle: JoinHandle<()>,
}

struct RenderManagerPayload {
    render_id: usize,
    scene: Arc<Scene>,
    camera_params: CameraParameters,
    film: Arc<Mutex<Film>>,
    sampler_settings: SamplerSettings,
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
