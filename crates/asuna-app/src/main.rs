// No console window on Windows release builds.
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

mod app;
mod art;
mod theme;
mod views;

use gpui_kit::component::{Theme, ThemeMode, TitleBar};
use gpui_kit::*;

fn main() {
    env_logger::init();
    let start_hidden = std::env::args().any(|a| a == asuna_core::autostart::HIDDEN_ARG);

    gpui_kit::application().with_assets(gpui_kit::assets::Assets).run(move |cx| {
        gpui_kit::init(cx);
        Theme::change(ThemeMode::Dark, None, cx);
        apply_brand(cx);

        let bounds = Bounds::centered(None, size(px(1280.), px(800.)), cx);
        let options = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            window_min_size: Some(size(px(900.), px(600.))),
            app_id: Some("asuna-launcher".into()),
            ..TitleBar::window_options()
        };
        gpui_kit::open_window(options, cx, |window, cx| {
            window.set_window_title("AsuNa Launcher");
            let view = cx.new(|cx| app::AsunaApp::new(window, cx));
            let minimize = start_hidden || view.read(cx).data.settings.start_minimized;
            if minimize {
                window.minimize_window();
            }
            view
        })
        .expect("failed to open the AsuNa window");
        cx.activate(true);
    });
}

/// AsuNa's pink accent on top of the stock dark theme.
fn apply_brand(cx: &mut App) {
    Theme::update(cx, |theme| {
        let accent: Hsla = theme::c(theme::ACCENT).into();
        theme.primary = accent;
        theme.primary_hover = theme::c(0xff859f).into();
        theme.primary_active = theme::c(0xe85a7a).into();
        theme.primary_foreground = gpui_kit::white();
        theme.button_primary = accent;
        theme.button_primary_hover = theme::c(0xff859f).into();
        theme.button_primary_active = theme::c(0xe85a7a).into();
        theme.button_primary_foreground = gpui_kit::white();
        theme.ring = accent;
        theme.background = theme::c(theme::BG).into();
        theme.title_bar = theme::c(theme::PANEL).into();
    });
}
