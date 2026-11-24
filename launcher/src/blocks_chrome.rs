//! The blocky look's top bar and sidebar.
use super::*;
use crate::glass_ui::Tone;

impl Launcher {
    /// The top bar: the mark and name over the sidebar (in the title bar on Windows and Linux
    /// client-side decorations), the pages with their icons, then the player's name and Settings.
    /// The top bar over the pages: their tabs, then the player's name and Settings. No plate: the
    /// sky shows through, under a 1 px rule.
    pub(crate) fn topbar(&mut self, cx: &mut Context<Self>) -> Div {
        let t = self.theme;
        let n = t.night;
        let mut nav = div()
            .flex()
            .flex_1()
            .min_w_0()
            .flex_wrap()
            .items_center()
            .gap(px(4.));
        for (page, label, _) in Page::in_top_bar(self.config.hosting.enabled) {
            nav = nav.child(
                shell::tab(format!("nav-{label}"), tr(label), self.page == page, &t).on_click(
                    cx.listener(move |this, _: &ClickEvent, _, cx| {
                        this.go(page);
                        cx.notify();
                    }),
                ),
            );
        }
        // Who plays: the Windows session's name, a plain label with no menu.
        let account = self.player.clone().map(|name| {
            let picture = div()
                .flex()
                .items_center()
                .justify_center()
                .size(px(28.))
                .bg(plate::alpha(n.card_hi, n.card_hi_alpha))
                .child(blocks::icon("user", 16., shell::c(n.muted)));
            shell::chip("account", &t)
                .max_w(px(200.))
                .child(div().relative().child(picture))
                .child(
                    div()
                        .relative()
                        .flex()
                        .flex_col()
                        .flex_1()
                        .min_w_0()
                        .overflow_hidden()
                        .child(
                            shell::text(theme::night::UI, shell::c(n.ink))
                                .font_weight(FontWeight::SEMIBOLD)
                                .line_height(px(16.))
                                .truncate()
                                .child(name),
                        ),
                )
        });
        let settings = shell::icon_button(
            "nav-Settings",
            "sliders",
            40.,
            self.page == Page::Settings,
            &t,
        )
        .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
            this.go(Page::Settings);
            cx.notify();
        }));
        div()
            .flex()
            .flex_none()
            .items_center()
            .min_h(px(theme::TOPBAR))
            .py(px(8.))
            .flex_wrap()
            .min_w_0()
            .pl(px(theme::PAGE_GUTTER - 14.))
            .pr(px(12.))
            .border_b_1()
            .border_color(plate::alpha(n.stroke, n.rule_alpha))
            .child(nav)
            .child(
                div()
                    .ml_auto()
                    .flex()
                    .flex_none()
                    .items_center()
                    .gap(px(8.))
                    .children(account)
                    .child(settings),
            )
    }

    /// The sidebar, from the top of the window: the wordmark, the game's row, the player's
    /// servers, then the launcher's version.
    pub(crate) fn sidebar(&mut self, window: &Window, cx: &mut Context<Self>) -> Stateful<Div> {
        let t = self.theme;
        let n = t.night;
        let heading = |label: String| shell::heading(label, &t);
        // The game's row: its mark, the club's name and the game's state. A click opens Home.
        let (tone, state) = self.row_state();
        let game = shell::row("side-game", self.page == Page::Play, &t)
            .child(div().relative().child(self.horizon_mark(36.)))
            .child(shell::row_text(
                self.config.club.name.clone(),
                state,
                Some(match tone {
                    Tone::Live => shell::c(n.success),
                    Tone::Warn => shell::c(n.warn),
                    _ => shell::c(n.muted),
                }),
                &t,
            ))
            .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                this.go(Page::Play);
                cx.notify();
            }));
        let list = div().flex().flex_col().gap(px(2.)).child(game);
        let servers: Vec<_> = self
            .servers
            .sidebar_list()
            .map(<[_]>::to_vec)
            .unwrap_or_else(|| self.side_servers.clone());
        // The Servers page exists only when the club lets a student host: then the sidebar has
        // its group from the first day, the way to a first server under an empty list.
        let hosting = self.config.hosting.enabled;
        let mut list = list;
        if hosting {
            list = list.child(heading(tr("MY SERVERS").into()));
        }
        for e in servers {
            let up = self.servers.is_up(&e.dir);
            let open = self.page == Page::Servers && self.servers.open.as_ref() == Some(&e.dir);
            let dir = e.dir.clone();
            list = list.child(
                shell::row(format!("side-server-{}", e.dir.display()), open, &t)
                    .child(
                        div()
                            .relative()
                            .flex()
                            .flex_none()
                            .items_center()
                            .justify_center()
                            .size(px(36.))
                            .bg(plate::alpha(n.card_hi, n.card_hi_alpha))
                            .child(blocks::icon("server", 18., shell::c(n.ink_soft))),
                    )
                    .child(shell::row_text(
                        e.name.clone(),
                        tr(if up { "Running" } else { "Stopped" }).into(),
                        Some(shell::c(if up { n.success } else { n.muted })),
                        &t,
                    ))
                    .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                        this.open_server(dir.clone());
                        this.go(Page::Servers);
                        cx.notify();
                    })),
            );
        }
        if hosting {
            list = list.child(
                shell::row("side-server-create", false, &t)
                    .child(
                        div()
                            .relative()
                            .flex()
                            .flex_none()
                            .items_center()
                            .justify_center()
                            .size(px(36.))
                            .bg(plate::alpha(n.card_hi, n.card_hi_alpha))
                            .child(blocks::icon("plus", 18., shell::c(n.ink_soft))),
                    )
                    .child(shell::row_text(
                        tr("Create a server").to_string(),
                        tr("On this computer").into(),
                        None,
                        &t,
                    ))
                    .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                        this.open_create(cx);
                        this.go(Page::Servers);
                        cx.notify();
                    })),
            );
        }
        // The servers this student was invited to, a second group under their own: a friend's
        // server, one they joined. They are joined from the Multiplayer tab, where a click leads;
        // a server that is off is dimmed, and nothing is created here.
        let invited: Vec<_> = self.servers.invited().to_vec();
        if !invited.is_empty() {
            list = list.child(heading(tr("INVITED SERVERS").into()));
        }
        for (i, (server, up)) in invited.into_iter().enumerate() {
            list = list.child(
                shell::row(SharedString::from(format!("side-invited-{i}")), false, &t)
                    .opacity(if up { 1. } else { 0.6 })
                    .child(
                        div()
                            .relative()
                            .flex()
                            .flex_none()
                            .items_center()
                            .justify_center()
                            .size(px(36.))
                            .bg(plate::alpha(n.card_hi, n.card_hi_alpha))
                            .child(blocks::icon("server", 18., shell::c(n.ink_soft))),
                    )
                    .child(shell::row_text(
                        server.name.clone(),
                        trf(
                            "{owner} · {state}",
                            &[
                                ("owner", &server.owner),
                                ("state", tr(if up { "Online" } else { "Offline" })),
                            ],
                        ),
                        Some(shell::c(if up { n.success } else { n.muted })),
                        &t,
                    ))
                    .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                        this.home_tab = home_tabs::HomeTab::Multiplayer;
                        this.go(Page::Play);
                        cx.notify();
                    })),
            );
        }
        let mut foot = div().mt_auto().flex().flex_col().gap(px(2.)).pt(px(12.));
        foot = foot.child(
            shell::text(theme::night::CAPTION, shell::c(n.muted))
                .id("launcher-version")
                .px(px(8.))
                .pt(px(6.))
                .child(trf("Launcher {version}", &[("version", backend::VERSION)])),
        );
        // Custom title bars carry the mark and name already.
        let brand = (!titlebar::draws_brand(window)).then(|| {
            div()
                .flex()
                .flex_none()
                .items_center()
                .gap(px(10.))
                .h(px(theme::TOPBAR))
                .px(px(8.))
                .child(mark(24., &t))
                .child(
                    shell::text(theme::night::TITLE, shell::c(n.ink))
                        .font_family(theme::DISPLAY_FONT)
                        .min_w_0()
                        .truncate()
                        .child(self.config.club.name.clone()),
                )
        });
        shell::sidebar_plate(&t)
            .id("sidebar")
            .flex()
            .flex_col()
            .flex_none()
            .w(px(theme::SIDEBAR))
            .h_full()
            // The same room above the first row as under the last one.
            .px(px(10.))
            .pt(px(12.))
            .pb(px(12.))
            .overflow_y_scroll()
            .children(brand)
            .child(list)
            .child(foot)
    }

    /// Whether `event` is the press that just closed a menu from outside it (on the menu's
    /// own button): the button then leaves the menu closed instead of opening it again.
    pub(crate) fn closed_by_this_press(&mut self, event: &ClickEvent) -> bool {
        closed_by_press(self.menu_closed_at.take(), event)
    }
}
