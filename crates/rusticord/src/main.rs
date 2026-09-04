use gpui_kit::AppContext;
use gpui_kit::component::Root;
use rusticord_ui::Shell;

fn main() {
    gpui_kit::application()
        .with_assets(gpui_kit::assets::Assets)
        .run(|cx| {
            gpui_kit::init(cx);
            let options = rusticord_ui::application_window_options(cx);
            cx.spawn(async move |cx| {
                if cx
                    .open_window(options, |window, cx| {
                        let shell = cx.new(Shell::new);
                        cx.new(|cx| Root::new(shell, window, cx))
                    })
                    .is_err()
                {
                    std::process::exit(1);
                }
            })
            .detach();
        });
}
