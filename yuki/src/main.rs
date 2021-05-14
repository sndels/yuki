#![feature(destructuring_assignment)]

mod bvh;
mod camera;
mod film;
mod integrators;
mod interaction;
mod lights;
mod macros;
mod math;
mod samplers;
mod scene;
mod shapes;
mod ui;

use ui::Window;

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
            ))
        })
        // .level(log::LevelFilter::Trace)
        // .level(log::LevelFilter::Debug)
        .level(log::LevelFilter::Info)
        // .level(log::LevelFilter::Warn)
        // .level(log::LevelFilter::Error)
        .level_for("gfx_device_gl", log::LevelFilter::Warn)
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
        yuki_error!("{}\n", info);
    }));

    let window = Window::new("yuki", (1920, 1080));
    window.main_loop();
}
