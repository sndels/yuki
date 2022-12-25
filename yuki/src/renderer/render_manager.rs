use std::{
    collections::{HashMap, VecDeque},
    sync::{
        mpsc::{channel, Receiver, RecvError, SendError, Sender, TryRecvError},
        Arc, Mutex,
    },
    thread::JoinHandle,
};

use super::{render_worker, render_worker::WorkerInfo, RenderSettings};

use crate::{
    camera::{Camera, CameraParameters},
    film::{film_tiles, Film, FilmSettings, FilmTile},
    integrators::IntegratorType,
    sampling::{Sampler, SamplerType},
    scene::Scene,
    yuki_debug, yuki_error, yuki_trace,
};

struct TileInfo {
    elapsed_s: f32,
    rays: usize,
}

pub enum Message {
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

pub struct RenderManager {
    pub tx: Sender<Option<Payload>>,
    pub rx: Receiver<Message>,
    pub handle: JoinHandle<()>,
}

pub struct Payload {
    pub render_id: usize,
    pub scene: Arc<Scene>,
    pub camera_params: CameraParameters,
    pub film: Arc<Mutex<Film>>,
    pub sampler: SamplerType,
    pub integrator: IntegratorType,
    pub film_settings: FilmSettings,
    pub render_settings: RenderSettings,
    pub force_single_sample: bool,
}

#[derive(Default)]
struct ManagerState {
    active_tiles_total: usize,
    active_tiles_done: usize,
    active_render_id: usize,
    active_workers: usize,
    ray_count: usize,
    tile_infos: VecDeque<TileInfo>,
}

pub fn launch(
    to_parent: Sender<Message>,
    from_parent: Receiver<Option<Payload>>,
) -> JoinHandle<()> {
    std::thread::Builder::new()
        .name("RenderManager".into())
        .spawn(move || {
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
                            std::thread::Builder::new()
                                .name("RenderWorker".into())
                                .spawn(move || {
                                    render_worker::launch(thread, &worker_send, &worker_receive);
                                })
                                .expect("Failed to spawn RenderWorker"),
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

                    if let Some(mut payload) = payload {
                        let mut tiles = film_tiles(&mut payload.film, payload.film_settings);

                        let sampler = payload.sampler.instantiate(payload.force_single_sample);

                        let mut initial_tiles = tiles.clone();
                        if payload.film_settings.accumulate {
                            for _ in 1..sampler.samples_per_pixel() {
                                for t in tiles.iter_mut() {
                                    t.sample += 1;
                                    initial_tiles.push_back(t.clone());
                                }
                            }
                        }

                        propagate_payload(
                            payload,
                            Arc::new(Mutex::new(initial_tiles)),
                            sampler,
                            &workers,
                            &mut state,
                        );
                    } else {
                        let active_workers = state.active_workers;

                        handle_worker_messages(
                            &from_workers,
                            &to_parent,
                            avg_tile_window,
                            &mut state,
                        );

                        let task_finished = active_workers > 0 && state.active_workers == 0;

                        if task_finished {
                            yuki_trace!("Render manager: Report back");
                            if let Err(why) = to_parent.send(Message::Finished {
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
        .expect("Failed to spawn RenderManager")
}

type WorkerMap = HashMap<usize, (Sender<Option<render_worker::Payload>>, JoinHandle<()>)>;

fn propagate_payload(
    payload: Payload,
    tiles: Arc<Mutex<VecDeque<FilmTile>>>,
    sampler: Arc<dyn Sampler>,
    workers: &WorkerMap,
    state: &mut ManagerState,
) {
    let camera = Camera::new(payload.camera_params, payload.film_settings);

    // TODO: This would be faster as a proper batch queues with work stealing,
    //       though the gains are likely only seen on interactive "frame rates"
    //       since sync only happens on tile pop (and film write).
    //       Visible rendering order could be retained by distributing the
    //       batches as interleaved (tiles i, 2*i, 3*i, ...)
    let tile_count = { tiles.lock().unwrap().len() };

    let mut active_workers = 0;
    for (tx, _) in workers.values() {
        let thread_payload = render_worker::Payload {
            render_id: payload.render_id,
            tiles: Arc::clone(&tiles),
            camera: camera.clone(),
            scene: Arc::clone(&payload.scene),
            integrator_type: payload.integrator,
            sampler: Arc::clone(&sampler),
            film: Arc::clone(&payload.film),
            mark_tiles: payload.render_settings.mark_tiles,
            accumulate: payload.film_settings.accumulate,
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
        active_workers,
        ..ManagerState::default()
    };
}

fn handle_worker_messages(
    from_workers: &Receiver<render_worker::Message>,
    to_parent: &Sender<Message>,
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
        ..
    } = state;

    while let Ok(msg) = from_workers.try_recv() {
        match msg {
            render_worker::Message::Finished(WorkerInfo {
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
            render_worker::Message::TileDone {
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

                    if let Err(why) = to_parent.send(Message::Progress {
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
