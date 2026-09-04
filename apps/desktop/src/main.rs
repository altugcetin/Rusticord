use gpui::{App, AppContext, Application, Bounds, WindowBounds, WindowOptions, px, size};
use rusticord_ui::RusticordRoot;

fn main() {
    Application::new().run(|cx: &mut App| {
        let bounds = Bounds::centered(None, size(px(1280.0), px(800.0)), cx);

        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                ..Default::default()
            },
            |_window, cx| cx.new(|_| RusticordRoot::new()),
        )
        .expect("failed to create the Rusticord window");

        cx.activate(true);
    });
}
