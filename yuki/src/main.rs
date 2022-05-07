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
mod sampling;
mod scene;
mod shapes;
mod textures;
mod visibility;

use itertools::Itertools;
use std::{fs::File, io::BufReader, path::PathBuf};

const HELP: &str = "\
Yuki
USAGE:
  yuki [OPTIONS]
FLAGS:
  -h, --help   Prints this help information
OPTIONS:
  --out=FILE   Path for EXR output";

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

    let args: Vec<String> = std::env::args().collect();
    let (print_help, out_path) = match args.len() {
        1 => (false, None),
        2 => {
            let arg = &args[1];
            if arg == "--help" || arg == "-h" {
                (true, None)
            } else {
                let parts: Vec<&str> = arg.split('=').collect();
                if parts.len() == 2 {
                    let (arg_name, value) = parts.iter().next_tuple().unwrap();
                    if arg_name == &"--out" {
                        (false, Some(PathBuf::from(value)))
                    } else {
                        yuki_error!("Unexpected option '{}'", arg_name);
                        (true, None)
                    }
                } else {
                    yuki_error!("Unexpected option '{}'", arg);
                    (true, None)
                }
            }
        }
        _ => (true, None),
    };

    if print_help {
        println!("{}", HELP);
        return;
    }

    let settings = match load_settings() {
        Ok(settings) => settings,
        Err(why) => {
            panic!("Failed to load previous settings: {}", why);
        }
    };

    if let Some(path) = out_path {
        app::headless::render(&path, settings);
    } else {
        let window = app::Window::new("yuki", (1920, 1080), settings);
        window.main_loop();
    }
}

fn load_settings() -> Result<app::InitialSettings, serde_yaml::Error> {
    match File::open("settings.yaml") {
        Ok(file) => {
            let reader = BufReader::new(file);
            let settings = serde_yaml::from_reader(reader)?;
            yuki_info!("Found settings");
            Ok(settings)
        }
        Err(why) => {
            yuki_info!("Could not load settings: {}", why);
            Ok(app::InitialSettings::default())
        }
    }
}
