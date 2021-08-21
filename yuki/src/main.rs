#![feature(destructuring_assignment)]
#![warn(clippy::pedantic, clippy::clone_on_ref_ptr)]
// Might be a good idea to check allowed warnings once in a while
#![allow(
    clippy::cast_lossless,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::doc_markdown,
    clippy::module_name_repetitions,
    clippy::similar_names,
    clippy::too_many_arguments,
    clippy::too_many_lines
)]

mod app;
mod bvh;
mod camera;
mod film;
mod integrators;
mod interaction;
mod lights;
mod macros;
mod materials;
mod math;
mod renderer;
mod samplers;
mod scene;
mod shapes;
mod visibility;

use app::{FilmicParams, HeatmapParams, ToneMapType};
use math::Vec2;
use std::{path::PathBuf, str::FromStr};

const HELP: &str = "\
Yuki
USAGE:
  yuki [OPTIONS]
FLAGS:
  -h, --help               Prints help information
OPTIONS:
  --out=FILE               Path for EXR output
  --scene=FILE             Path to scene file to load
  --resolution=X,Y         Resolution to render at (default 640,480)
  --integrator=TYPE        Integrator to use
  --tonemap=TYPE,ARGS,...  Tonemap to use along with its settings
                           Filmic,[EXPOSURE]\n
                           Heatmap,[CHANNEL],[MIN],[MAX]
                           Heatmap,[CHANNEL]  This uses min, max of the output
";
// TODO: Headless output with given EXR name, raw/tonemapped output

fn setup_logger() -> Result<(), fern::InitError> {
    fern::Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                "{}[{}][{}:{}] {}",
                chrono::Local::now().format("[%Y-%m-%d][%H:%M:%S:%3f]"),
                record.level(),
                record.target(),
                record.line().unwrap_or(0),
                message
            ));
        })
        // .level(log::LevelFilter::Trace)
        // .level(log::LevelFilter::Debug)
        .level(log::LevelFilter::Info)
        // .level(log::LevelFilter::Warn)
        // .level(log::LevelFilter::Error)
        .filter(|meta| meta.target().starts_with("yuki"))
        .chain(std::io::stdout())
        .chain(std::fs::File::create("yuki.log")?)
        .apply()?;
    Ok(())
}

fn main() {
    if let Err(why) = setup_logger() {
        win_dbg_logger::output_debug_string(&format!("{}", why));
        panic!("{}", why);
    };

    // Let's catch panic messages ourselves and output everywhere
    std::panic::set_hook(Box::new(|info| {
        let location_str = if let Some(location) = info.location() {
            format!("{}:{}", location.file(), location.line())
        } else {
            yuki_error!("No location for panic!");
            "".into()
        };
        let payload = match info.payload().downcast_ref::<&'static str>() {
            Some(s) => s,
            None => match info.payload().downcast_ref::<String>() {
                Some(s) => s,
                None => "Panic payload is not &'static str or String",
            },
        };

        let msg = format!("{}\n{}", location_str, payload);

        yuki_error!("{}", msg);
        if let Err(why) = msgbox::create("Panic!", &msg, msgbox::IconType::Error) {
            yuki_error!("Failed to create popup window: '{}'", why);
        };
    }));

    match parse_settings() {
        Ok((settings, Some(out_path))) => {
            app::headless::render(&out_path, settings);
        }
        Ok((settings, None)) => {
            let window = app::Window::new("yuki", (1920, 1080), settings);
            window.main_loop();
        }
        Err(why) => {
            panic!("Parsing CLI arguments failed: {}", why);
        }
    };
}

fn parse_settings() -> Result<(app::InitialSettings, Option<PathBuf>), pico_args::Error> {
    let mut pargs = pico_args::Arguments::from_env();

    // Help has a higher priority and should be handled separately.
    if pargs.contains(["-h", "--help"]) {
        print!("{}", HELP);
        std::process::exit(0);
    }

    let mut settings = app::InitialSettings::default();
    let mut out_path = None;

    if let Some(scene_path) = pargs.opt_value_from_str::<&'static str, PathBuf>("--scene")? {
        settings.load_settings.path = if scene_path.has_root() {
            scene_path
        } else {
            std::env::current_dir()
                .expect("Invalid working directory")
                .join(scene_path)
        };
    }

    if let Some(path) = pargs.opt_value_from_str::<&'static str, PathBuf>("--out")? {
        let extension = path
            .extension()
            .ok_or(pico_args::Error::ArgumentParsingFailed {
                cause: "Path does not point to a file".into(),
            })?
            .to_string_lossy();
        if extension != "exr" {
            return Err(pico_args::Error::ArgumentParsingFailed {
                cause: format!(
                    "EXR output should have extension '.exr', got '{}'",
                    extension
                ),
            });
        }

        let full_path = if path.has_root() {
            path
        } else {
            std::env::current_dir()
                .expect("Invalid working directory")
                .join(path)
        };

        out_path = Some(full_path);
    }

    if let Some(resolution) = pargs.opt_value_from_fn("--resolution", parse_resolution)? {
        settings.film_settings.res = resolution;
    };

    if let Some(integrator) = pargs.opt_value_from_str::<&'static str, String>("--integrator")? {
        settings.scene_integrator = parse_enum(&integrator, "Unknown integrator type")?;
    }

    if let Some(tone_map) = pargs.opt_value_from_fn("--tonemap", parse_tone_map)? {
        settings.tone_map = tone_map;
    }

    Ok((settings, out_path))
}

fn parse_resolution(s: &str) -> Result<Vec2<u16>, pico_args::Error> {
    let strs = s.split(',').collect::<Vec<&str>>();
    if strs.len() == 2 {
        let x = parse_num(strs[0], "Invalid resolution X component")?;
        let y = parse_num(strs[1], "Invalid resolution Y component")?;
        Ok(Vec2::new(x, y))
    } else {
        Err(pico_args::Error::ArgumentParsingFailed {
            cause: "Expected --resolution X,Y".into(),
        })
    }
}

fn parse_tone_map(s: &str) -> Result<ToneMapType, pico_args::Error> {
    let strs = s.split(',').collect::<Vec<&str>>();

    let mut tonemap = parse_enum(strs[0], "Unknown tonemap type")?;

    match &mut tonemap {
        ToneMapType::Raw => (),
        ToneMapType::Filmic(FilmicParams { ref mut exposure }) => {
            *exposure = parse_num(strs[1], "Invalid filmic exposure")?;
        }
        ToneMapType::Heatmap(HeatmapParams {
            ref mut channel,
            ref mut bounds,
        }) => {
            *channel = parse_enum(strs[1], "Unknown heatmap channel")?;
            if strs.len() == 4 {
                *bounds = Some((
                    parse_num(strs[2], "Invalid heatmap min")?,
                    parse_num(strs[3], "Invalid heatmap max")?,
                ));
            } else if strs.len() > 2 {
                return Err(pico_args::Error::ArgumentParsingFailed {
                    cause: "Expected tone map type and 1 or 3 parameters".into(),
                });
            }
        }
    }

    Ok(tonemap)
}

fn parse_num<T>(s: &str, err: &str) -> Result<T, pico_args::Error>
where
    T: FromStr,
{
    s.parse()
        .map_err(|_| pico_args::Error::ArgumentParsingFailed {
            cause: format!("{} '{}'", err, s),
        })
}

fn parse_enum<T>(s: &str, err: &str) -> Result<T, pico_args::Error>
where
    T: FromStr,
{
    T::from_str(s).map_err(|_| pico_args::Error::ArgumentParsingFailed {
        cause: format!("{} '{}'", err, s),
    })
}
