use super::*;

// Windows' process-entry stack is only 1 MiB. Keep the entire launcher on an explicitly sized
// stack instead of constructing startup frames there.
// GPUI initializes OLE, creates windows and pumps their messages on this same thread:
// no COM or GUI state is constructed on the entry thread that waits for it.
// 16 MiB is a generous reserve, sixteen times the 1 MiB default that the startup regression
// test constrains the entry thread to; Windows commits a thread stack only as it grows.
#[cfg(windows)]
pub(super) fn run() {
    let launcher = std::thread::Builder::new()
        .name("launcher".into())
        .stack_size(16 * 1024 * 1024)
        .spawn(run_launcher)
        .unwrap_or_else(|error| {
            eprintln!(
                "{}",
                trf(
                    "Could not start the launcher: {error}",
                    &[("error", &error.to_string())],
                )
            );
            std::process::exit(1);
        });
    if let Err(payload) = launcher.join() {
        std::panic::resume_unwind(payload);
    }
}

// In particular, AppKit must stay on macOS's original main thread.
#[cfg(not(windows))]
pub(super) fn run() {
    run_launcher();
}

/// The bundled faces, which every window of the launcher's sets its text in.
pub(super) fn add_fonts(cx: &mut App) {
    cx.text_system()
        .add_fonts(vec![
            Cow::Borrowed(horizon_theme::fonts::GEIST_MONO),
            Cow::Borrowed(horizon_theme::fonts::BRICOLAGE_REGULAR),
            Cow::Borrowed(horizon_theme::fonts::BRICOLAGE_MEDIUM),
            Cow::Borrowed(horizon_theme::fonts::BRICOLAGE_SEMIBOLD),
            Cow::Borrowed(horizon_theme::fonts::BRICOLAGE_BOLD),
            Cow::Borrowed(horizon_theme::fonts::BRICOLAGE_DISPLAY_BOLD),
            Cow::Borrowed(horizon_theme::fonts::BRICOLAGE_DISPLAY_SEMIBOLD),
            Cow::Borrowed(horizon_theme::fonts::BRICOLAGE_HEADING_SEMIBOLD),
            Cow::Borrowed(horizon_theme::fonts::GEIST_REGULAR),
            Cow::Borrowed(horizon_theme::fonts::GEIST_MEDIUM),
            Cow::Borrowed(horizon_theme::fonts::GEIST_SEMIBOLD),
        ])
        .expect("bundled fonts");
}

fn run_launcher() {
    // The club's root is found from where the launcher started, before the data folder becomes
    // the current directory (the share is read-only, and a UNC path is a poor working directory).
    let _ = horizon_launcher::club::root();
    let _ = std::env::set_current_dir(horizon_launcher::club::data_dir());
    perf::process_started();
    // The GPU's texture limit, which caps the pictures made from disk (`picture_cache`).
    horizon_launcher::pictures::probe_gpu_limit();
    // The window's title is the club's name; a file that can't be read leaves the default one,
    // which the window then says (Home).
    let title = horizon_launcher::config::Config::load()
        .map(|config| config.club.name)
        .unwrap_or_else(|_| horizon_launcher::config::Config::default().club.name);
    gpui_platform::application()
        .with_assets(blocks::Assets)
        .run(move |cx: &mut App| {
            add_fonts(cx);
            cx.bind_keys(key_bindings());
            cx.on_action(|_: &Quit, cx| shut_down(cx));
            // The key press is being handled inside the window, so the window can only be
            // removed once that update has finished.
            cx.on_action(|_: &CloseWindow, cx| {
                cx.defer(|cx| {
                    for window in cx.windows() {
                        let _ = window.update(cx, |_, window, _| window.remove_window());
                    }
                })
            });
            cx.set_menus(menus());
            let bounds = Bounds::centered(None, size(px(1180.), px(760.)), cx);
            cx.open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(bounds)),
                    window_min_size: Some(size(px(960.), px(600.))),
                    titlebar: Some(titlebar::options(&title)),
                    #[cfg(target_os = "linux")]
                    window_decorations: Some(gpui::WindowDecorations::Client),
                    app_id: Some("horizon-launcher".into()),
                    // GPUI draws a window without focus at most every 33 ms to save energy. A
                    // benchmark's window often starts without it (KDE keeps focus from windows
                    // a background process opens), and it measures drawing, not that limit.
                    inactive_frame_interval: if perf::Bench::wanted() {
                        None
                    } else {
                        WindowOptions::default().inactive_frame_interval
                    },
                    ..Default::default()
                },
                |window, cx| cx.new(|cx| crate::window_root::Shell::new(window, cx)),
            )
            .expect("window");
            cx.on_window_closed(|cx, _| shut_down(cx)).detach();
            cx.activate(true);
        });
}
