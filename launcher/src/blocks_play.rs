//! Home, both looks: what Play starts and its state, Play, what keeps the game from starting or
//! what the last start said, then the game's tabs. The blocky look draws the head here; the
//! glass one draws its own (`glass_home.rs`) over the same tabs. The crash notice too.
use super::*;

impl Launcher {
    pub(crate) fn play_page(&mut self, cx: &mut Context<Self>) -> Div {
        let t = self.theme;
        let Some(tag) = self.selected().map(str::to_owned) else {
            // No build to play, or the club's file can't be read: what Home says is the
            // supervisor's to put in place, never a download.
            let said = self
                .refusal
                .as_ref()
                .map_or_else(|| self.no_build(), |refusal| refusal.said.clone());
            let mut page = div()
                .flex()
                .flex_col()
                .gap(px(24.))
                .pt(px(28.))
                .child(page_header(
                    "Play",
                    "Start the game, or run a server for others to join.",
                    &t,
                ))
                .child(centered(&said, &t));
            if let Some(error) = self.client_error.clone() {
                page = page.child(notice(error, &t));
            }
            return page;
        };
        let version = backend::game_version(&tag);

        // The glass look's head (the mockup's `HomeView`) replaces the banner.
        let banner = if t.glass {
            div()
                .flex()
                .flex_col()
                .gap(px(24.))
                .child(self.glass_home_head(&tag, cx))
        } else {
            let hero = self.hero(&tag, &version);
            let actions = self.home_actions(cx);
            // The banner: the hero and the Play row over the sky (`sky.rs`), or over the player's
            // own picture under a veil of the page colour. It reaches the content's edges and the top
            // bar.
            let veil = |angle: f32, from: f32, to: f32, at: f32| {
                div()
                    .absolute()
                    .top_0()
                    .left_0()
                    .size_full()
                    .bg(gpui::linear_gradient(
                        angle,
                        gpui::linear_color_stop(t.page.opacity(from), 0.),
                        gpui::linear_color_stop(t.page.opacity(to), at),
                    ))
            };
            // At the window's size (`picture_cache`): a chosen picture may be far bigger.
            let side = picture_cache::Look::Fit(self.window_side());
            let picture = self
                .home_background()
                .and_then(|path| self.picture(&path, side, cx))
                .map(|image| {
                    [
                        gpui::img(image)
                            .absolute()
                            .top_0()
                            .left_0()
                            .size_full()
                            .object_fit(gpui::ObjectFit::Cover)
                            .into_any_element(),
                        veil(90., 0.82, 0., 0.62).into_any_element(),
                        veil(180., 0., 0.75, 1.).into_any_element(),
                    ]
                });
            div()
                .relative()
                .flex()
                .flex_col()
                .justify_end()
                .min_h(px(340.))
                .mx(px(-theme::PAGE_GUTTER))
                .px(px(theme::PAGE_GUTTER))
                .pb(px(28.))
                .gap(px(24.))
                .overflow_hidden()
                .children(picture.into_iter().flatten())
                .child(
                    div().relative().flex().items_end().gap(px(32.)).child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .flex()
                            .flex_col()
                            .gap(px(28.))
                            .child(hero)
                            .child(actions),
                    ),
                )
        };
        let mut page = div().flex().flex_col().gap(px(28.)).child(banner);
        // Above the tabs: what keeps the game from starting, then what crashed and the way back
        // into the game.
        if let Some(refusal) = &self.refusal {
            page = page.child(notice(refusal.said.clone(), &t));
        }
        if let Some(crash) = self.crash.clone() {
            page = page.child(self.crash_notice(crash, cx));
        }
        page = if t.glass {
            page.child(self.glass_home_tabs(cx))
        } else {
            page.child(self.home_tabs(cx))
        };
        // What the last start said.
        if let Some(e) = self.client_error.clone() {
            page = page.child(notice(e, &t));
        }
        page
    }

    /// "Horizon crashed": the report and the logs.
    pub(crate) fn crash_notice(&self, crash: Crash, cx: &mut Context<Self>) -> Div {
        let t = self.theme;
        let mut actions = div().flex().flex_wrap().gap(px(8.)).mt(px(8.));
        if let Some(report) = crash.report.clone() {
            let open = report.clone();
            actions = actions
                .child(
                    button("crash-open", "Open report", Kind::Secondary, &t)
                        .bg(t.plate)
                        .on_click(move |_, _, cx| crate::glass_toast::open(cx, &open)),
                )
                .child(
                    button(
                        "crash-copy",
                        if crash.copied {
                            "Copied"
                        } else {
                            "Copy report"
                        },
                        Kind::Secondary,
                        &t,
                    )
                    .bg(t.plate)
                    .on_click(cx.listener(
                        move |this, _: &ClickEvent, _, cx| {
                            if let Ok(text) = std::fs::read_to_string(&report) {
                                copy_to_clipboard(cx, gpui::ClipboardItem::new_string(text));
                                if let Some(c) = &mut this.crash {
                                    c.copied = true;
                                }
                            }
                            cx.notify();
                        },
                    )),
                );
        }
        actions = actions.child(
            button("crash-logs", "Open logs folder", Kind::Secondary, &t)
                .bg(t.plate)
                .on_click(|_, _, cx| {
                    let dir = backend::logs_dir();
                    let _ = std::fs::create_dir_all(&dir);
                    crate::glass_toast::open(cx, &dir);
                }),
        );
        actions = actions.child(
            button("crash-dismiss", "Dismiss", Kind::Ghost, &t).on_click(cx.listener(
                |this, _: &ClickEvent, _, cx| {
                    this.crash = None;
                    cx.notify();
                },
            )),
        );
        let mut body = div()
            .flex_1()
            .min_w_0()
            .flex()
            .flex_col()
            .gap(px(4.))
            .child(
                text(theme::HEADING)
                    .text_color(t.ink)
                    .child(tr("Horizon crashed")),
            );
        if !crash.summary.is_empty() {
            body = body.child(div().text_color(t.ink_soft).child(crash.summary));
        }
        if let Some(report) = &crash.report {
            body = body.child(
                text(theme::MICRO)
                    .text_color(t.muted)
                    .truncate()
                    .child(short(report)),
            );
        }
        body = body.child(
            text(theme::CAPTION)
                .text_color(t.muted)
                .child(backend::crash_help()),
        );
        let body = body.child(actions);
        plate(&t)
            .flex()
            .gap(px(14.))
            .p(px(24.))
            .text_size(px(theme::UI.px))
            .line_height(px(21.))
            .text_color(t.ink)
            .child(div().flex_none().mt(px(11.)).size(px(6.)).bg(t.danger))
            .child(body)
    }
}
