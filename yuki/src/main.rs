mod ui;

use ui::Window;

fn main() {
    let window = Window::new("yuki", (1920, 1080));
    window.main_loop();
}
