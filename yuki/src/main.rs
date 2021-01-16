mod macros;
mod ui;

use ui::Window;

fn setup_logger() -> Result<(), fern::InitError> {
    fern::Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                "{}[{}][{}:{}] {}",
                chrono::Local::now().format("[%Y-%m-%d][%H:%M:%S]"),
                record.level(),
                record.target(),
                record.line().unwrap_or(0),
                message
            ))
        })
        // .level(log::LevelFilter::Info)
        // .level(log::LevelFilter::Debug)
        .level(log::LevelFilter::Warn)
        // .level(log::LevelFilter::Error)
        .chain(std::io::stdout())
        .chain(std::fs::File::create("yuki.log")?)
        .apply()?;
    Ok(())
}

fn main() {
    if let Err(why) = setup_logger() {
        panic!("{}", why);
    };

    let window = Window::new("yuki", (1920, 1080));
    window.main_loop();
}
