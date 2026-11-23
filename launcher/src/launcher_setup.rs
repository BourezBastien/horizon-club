//! The launcher's state set up (`Launcher::new`), its settings saved, and page changes.
use super::*;

impl Launcher {
    pub(crate) fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let (tx, rx) = channel();
        let appearance = cx.observe_window_appearance(window, |this, window, cx| {
            this.theme = this.settings_theme(window);
            cx.notify();
        });
        cx.spawn(async move |this, cx| loop {
            cx.background_executor()
                .timer(Duration::from_millis(80))
                .await;
            if this.update(cx, |this, cx| this.drain(cx)).is_err() {
                break;
            }
        })
        .detach();
        let settings = Settings::load();
        let installed = backend::installed();
        // Pin the window's chrome (macOS title bar) to the choice before the theme reads
        // `window.appearance()`, so both draw the same look.
        titlebar::apply_appearance(window, settings.appearance);
        let scene = cx.new(|_| scene::Scene::new());
        let sky = cx.new(|cx| sky::SkyView::new(window, cx));
        let rain = cx.new(|_| rain::Rain::new());
        let backdrop =
            cx.new(|_| window_root::Backdrop::new(scene.clone(), sky.clone(), rain.clone()));
        // The opening: the wordmark, then the launcher. The window's own focus takes the click
        // or the key that skips ahead.
        let opening = {
            let focus = cx.focus_handle();
            window.focus(&focus, cx);
            Some(Opening {
                started: None,
                reveal: None,
                focus,
            })
        };
        let mut this = Self {
            theme: crate::theme::from_settings(&settings, window.appearance()),
            page: Page::from_env(),
            behind: Page::Play,
            switches: 0,
            content_scroll: Scrollbar::default(),
            nav_strip: nav_strip::NavStrip::default(),
            settings_pane: gpui::ScrollHandle::new(),
            settings_query: line_edit::LineEdit::new(String::new(), 80, 40),
            settings_query_focus: cx.focus_handle(),
            settings_query_focused: false,
            settings_marked: 0,
            long_list: cull::Cull::default(),
            opening,
            installed,
            accent_strip: false,
            look_sliders: Default::default(),
            hue_area: std::rc::Rc::new(std::cell::Cell::new(None)),
            hue_dragging: false,
            language_menu: false,
            row_menu: Default::default(),
            rail: Default::default(),
            look: horizon_theme::glass::Glass::default_look(),
            menu_closed_at: None,
            // The first draw takes the launcher theme from Settings.
            restyle: true,
            background_error: None,
            settings,
            settings_save_error: None,
            config: Default::default(),
            config_error: None,
            refusal: None,
            checked: None,
            console_log: VecDeque::new(),
            log_revision: 0,
            client: None,
            client_error: None,
            client_started: None,
            crash: None,
            servers: servers_page::Servers::new(),
            root_focus: cx.focus_handle(),
            palette: None,
            toast_seen: None,
            // Who plays is read with the club's file, below: a shared session plays under no name.
            player: None,
            starting_client: None,
            window_width: 0.,
            window_long_side: 0.,
            scale_factor: 1.,
            quick_play: None,
            tx,
            rx,
            _appearance: appearance,
            _activation: cx.observe_window_activation(window, |_, _, cx| cx.notify()),
            perf: perf::Perf::new("session"),
            home_tab: home_tabs::HomeTab::from_env(),
            home_menu: Default::default(),
            folder_hovered: false,
            list_hover: None,
            list_menu: None,
            home_strip: Default::default(),
            client_since: None,
            play_pressed: None,
            play_step: Default::default(),
            bench: perf::Bench::from_env(),
            marks: Default::default(),
            worlds: worlds_page::Worlds::default(),
            multiplayer: multiplayer_page::Multiplayer::default(),
            pictures: Default::default(),
            console: Default::default(),
            aside: while_playing::Aside::new(window.window_handle()),
            side_servers: horizon_launcher::servers::list(),
            update: update_page::Update::new(),
            sky,
            scene,
            rain,
            backdrop,
            pushed: None,
            scale: window.scale_factor(),
            title: None,
        };
        if this.bench.is_some() {
            this.perf.keep_local_log(false);
        }
        // The club's file, read before the first frame: the window is in the club's language and
        // says what keeps the game from starting, if anything.
        this.reload_club();
        // A newer launcher on GitHub, for a session that may put it on the share.
        this.update.begin(&horizon_launcher::club::root());
        this
    }

    /// Keeps the current choices in memory if writing fails; every page offers the same retry.
    pub(crate) fn save_settings(&mut self) -> bool {
        self.settings_save_error = self.settings.save().err();
        self.settings_save_error.is_none()
    }

    pub(crate) fn settings_save_notice(&self, cx: &mut Context<Self>) -> Option<Div> {
        let error = self.settings_save_error.as_ref()?;
        let t = self.theme;
        Some(
            div()
                .flex_none()
                .px(px(theme::PAGE_GUTTER))
                .py(px(12.))
                .child(
                    crate::plate::plate(&t.night)
                        .flex()
                        .items_center()
                        .gap(px(16.))
                        .p(px(16.))
                        .child(
                            div()
                                .relative()
                                .flex_1()
                                .min_w_0()
                                .child(
                                    shell::text(theme::night::UI, t.danger)
                                        .child(tr("Settings were not saved.")),
                                )
                                .child(
                                    shell::text(theme::night::CAPTION, t.ink_soft).child(trf(
                                        "Your changes are still in this window. Retry to save them. {error}",
                                        &[("error", error)],
                                    )),
                                ),
                        )
                        .child(
                            button("settings-save-retry", "Retry", Kind::Secondary, &t)
                                .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                                    this.save_settings();
                                    cx.notify();
                                })),
                        ),
                ),
        )
    }

    pub(crate) fn go(&mut self, page: Page) {
        // The Servers page exists only when the club lets a student host: a request for it (a
        // stale page, the bench's round of pages) opens Home instead.
        let page = if page == Page::Servers && !self.config.hosting.enabled {
            Page::Play
        } else {
            page
        };
        // The glass look's Settings is a dialog over the page: opening or closing it keeps the
        // page under it as it was (its scroll, no fade).
        let dialog = self.theme.glass
            && (page == Page::Settings || (self.page == Page::Settings && page == self.behind));
        if page != Page::Settings {
            self.behind = page;
        }
        if page != self.page && dialog {
            self.page = page;
        } else if page != self.page {
            self.page = page;
            self.switches += 1;
            self.content_scroll = Scrollbar::default();
            self.long_list = cull::Cull::default();
        }
        self.language_menu = false;
    }
}
