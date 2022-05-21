use allocators::{LinearAllocator, ScopedScratch};
use std::{
    collections::VecDeque,
    ops::{Deref, DerefMut},
    sync::{
        mpsc::{Receiver, RecvError, Sender, TryRecvError},
        Arc, Mutex,
    },
    time::Instant,
};

use crate::{
    camera::Camera,
    film::{Film, FilmTile},
    integrators::IntegratorType,
    math::Spectrum,
    sampling::Sampler,
    scene::Scene,
    yuki_debug, yuki_error, yuki_trace,
};

pub enum Message {
    TileDone {
        info: WorkerInfo,
        ray_count: usize,
        elapsed_s: f32,
    },
    Finished(WorkerInfo),
}

#[derive(Clone, Copy)]
pub struct WorkerInfo {
    pub render_id: usize,
    pub thread_id: usize,
}

pub struct Payload {
    pub render_id: usize,
    pub tiles: Arc<Mutex<VecDeque<FilmTile>>>,
    pub camera: Camera,
    pub scene: Arc<Scene>,
    pub integrator_type: IntegratorType,
    pub sampler: Arc<dyn Sampler>,
    pub film: Arc<Mutex<Film>>,
    pub mark_tiles: bool,
}

impl Deref for Payload {
    type Target = Self;
    fn deref(&self) -> &Self::Target {
        self
    }
}

impl DerefMut for Payload {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self
    }
}

pub fn launch(
    thread_id: usize,
    to_parent: &Sender<Message>,
    from_parent: &Receiver<Option<Payload>>,
) {
    yuki_debug!("Render thread {}: Begin", thread_id);

    let mut alloc = LinearAllocator::new(1024 * 256);
    let scratch = ScopedScratch::new(&mut alloc);
    let mut tile_pixels = [Spectrum::zeros(); 64 * 64];

    'thread: loop {
        let mut worker_info = WorkerInfo {
            render_id: 0,
            thread_id,
        };
        let mut payload: Option<Payload> = None;

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
                from_parent,
                &mut worker_info,
                &mut payload,
            ) {
                break 'thread;
            }

            let tile = match payload.as_deref_mut() {
                Some(p) => match pop_tile_or_signal_finish(&worker_info, p, to_parent) {
                    Some(tile) => Some(tile),
                    None => break 'work,
                },
                None => None,
            };

            if let Some(mut tile) = tile {
                assert!(payload.is_some(), "Active tile without payload");
                assert!(tile.bb.area() as usize <= tile_pixels.len());

                let payload = payload.as_deref().unwrap();

                let tile_start = Instant::now();
                match render_tile(
                    thread_id,
                    &scratch,
                    &mut tile,
                    &mut tile_pixels,
                    payload,
                    from_parent,
                ) {
                    RenderTileResult::Interrupted(p) => newest_msg = Some(Ok(p)),
                    RenderTileResult::Rendered { ray_count } => update_tile(
                        &worker_info,
                        &mut tile,
                        &tile_pixels,
                        payload,
                        ray_count,
                        tile_start,
                        to_parent,
                    ),
                }
            }
        }
    }
}

// Returns `true` if manager signaled kill
fn handle_manager_messages(
    thread_id: usize,
    newest_msg: Option<Result<Option<Payload>, RecvError>>,
    from_parent: &Receiver<Option<Payload>>,
    worker_info: &mut WorkerInfo,
    payload: &mut Option<Payload>,
) -> bool {
    match newest_msg.map_or_else(
        || from_parent.try_recv(),
        |r| match r {
            Ok(msg) => Ok(msg),
            Err(e) => Err(TryRecvError::from(e)),
        },
    ) {
        Ok(Some(new_payload)) => {
            yuki_debug!("Render thread {}: Received new payload", thread_id);
            worker_info.render_id = new_payload.render_id;
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
    worker_info: &WorkerInfo,
    payload: &mut Payload,
    to_parent: &Sender<Message>,
) -> Option<FilmTile> {
    let tile = {
        let mut tiles = payload.tiles.lock().unwrap();
        tiles.pop_front()
    };

    match tile {
        Some(tile) => Some(tile),
        None => {
            yuki_trace!("Render thread {}: Signal done", worker_info.thread_id);

            if let Err(why) = to_parent.send(Message::Finished(*worker_info)) {
                yuki_error!(
                    "Render thread {}: Error notifying parent on finish: {}",
                    worker_info.thread_id,
                    why
                );
            };

            None
        }
    }
}

#[allow(clippy::large_enum_variant)] // This is not in the hot path, seems to make sense
enum RenderTileResult {
    Interrupted(Option<Payload>),
    Rendered { ray_count: usize },
}

fn render_tile(
    thread_id: usize,
    scratch: &ScopedScratch,
    tile: &mut FilmTile,
    tile_pixels: &mut [Spectrum<f32>],
    payload: &Payload,
    from_parent: &Receiver<Option<Payload>>,
) -> RenderTileResult {
    if payload.mark_tiles {
        yuki_trace!("Render thread {}: Mark tile {:?}", thread_id, tile.bb);
        yuki_trace!("Render thread {}: Waiting for lock on film", thread_id);
        let mut film = payload.film.lock().unwrap();
        yuki_trace!("Render thread {}: Acquired film", thread_id);

        if film.matches(tile) {
            film.mark(tile, Spectrum::new(1.0, 0.0, 1.0));
        }

        yuki_trace!("Render thread {}: Releasing film", thread_id);
    }

    yuki_trace!("Render thread {}: Render tile {:?}", thread_id, tile.bb);
    let mut received_msg = None;
    let tile_scratch = ScopedScratch::new_scope(scratch);
    let integrator = payload.integrator_type.instantiate();
    let ray_count = integrator.render(
        &tile_scratch,
        &payload.scene,
        &payload.camera,
        &payload.sampler,
        tile,
        tile_pixels,
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
    worker_info: &WorkerInfo,
    tile: &mut FilmTile,
    tile_pixels: &[Spectrum<f32>],
    payload: &Payload,
    ray_count: usize,
    tile_start: Instant,
    to_parent: &Sender<Message>,
) {
    yuki_trace!(
        "Render thread {}: Update tile {:?}",
        worker_info.thread_id,
        tile.bb
    );
    {
        yuki_trace!(
            "Render thread {}: Waiting for lock on film",
            worker_info.thread_id
        );
        let mut film = payload.film.lock().unwrap();
        yuki_trace!("Render thread {}: Acquired film", worker_info.thread_id);

        if film.matches(tile) {
            film.update_tile(tile, tile_pixels);
        } else {
            yuki_trace!("Render thread {}: Stale tile", worker_info.thread_id);
        }

        yuki_trace!("Render thread {}: Releasing film", worker_info.thread_id);
    }

    if let Err(why) = to_parent.send(Message::TileDone {
        info: *worker_info,
        ray_count,
        elapsed_s: tile_start.elapsed().as_secs_f32(),
    }) {
        yuki_error!(
            "Render thread {}: Error notifying parent on tile done: {}",
            worker_info.thread_id,
            why
        );
    };
}
