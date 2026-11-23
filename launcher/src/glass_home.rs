//! Home's head in the glass look (the mockup's `HomeView`, `launcher-views.tsx`): the club's
//! name beside Horizon's mark, how long the game has been running, then Play, the build and the
//! game folder. Dark appearance only.
use super::*;
use crate::glass_ui::{self as ui, white};
use gpui::deferred;
use horizon_theme::glass::{fill, ink, radius};
use std::time::Instant;

/// Play, the version and the big icons are this tall (`h-14`).
const TALL: f32 = 56.;
/// The window width from which Home's head is laid out as the mockup's `xl` (1280 px); under it
/// Play and the version are narrower. The theme editor also puts its two columns side by side
/// from it.
pub(super) const WIDE: f32 = 1280.;
/// The window width from which pages put their head and cards side by side (Overview,
/// Versions, Settings' About); under it they stack, and Home's tabs drop their icons.
pub(super) const SIDE_BY_SIDE: f32 = 1200.;
/// Under this window width the top bar's tabs tighten and Settings' sections go above the page.
pub(super) const COMPACT: f32 = 1180.;

/// Which of Home's menus is open.
#[derive(Clone, Copy, PartialEq, Default)]
pub(super) enum HomeMenu {
    #[default]
    None,
    Folder,
}

impl Launcher {
    /// The head: what Home shows above its tabs. `tag`: the build it plays.
    pub(super) fn glass_home_head(&mut self, tag: &str, cx: &mut Context<Self>) -> Div {
        // How long this game has run, for "Playing for 5 min now".
        match (&self.client, self.client_since) {
            (Some(_), None) => self.client_since = Some(Instant::now()),
            (None, Some(_)) => self.client_since = None,
            _ => {}
        }
        // One line, unlike the mockup's `flex-wrap`: at 1280 px the folder's ⋯ would wrap the
        // row away from under the pointer, which hides it again.
        let wide = self.window_width >= WIDE;
        let actions = if wide {
            div()
                .mt(px(32.))
                .flex()
                .items_center()
                .gap(px(10.))
                .child(self.play_group(cx))
                .child(self.folder_group(cx))
                .child(div().flex_1().min_w(px(22.)))
        } else {
            div().mt(px(32.)).flex().flex_col().gap(px(12.)).child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(10.))
                    .child(self.play_group(cx))
                    .child(self.folder_group(cx)),
            )
        };
        div()
            .flex()
            .flex_col()
            .min_w_0()
            .pt(px(64. - crate::glass::CONTENT_INSET))
            .child(motion::enter(
                ElementId::named_usize("home-title", self.switches),
                1,
                self.home_title(tag),
            ))
            .children(self.running_text().map(|line| {
                motion::enter(
                    ElementId::named_usize("home-status", self.switches),
                    2,
                    line,
                )
            }))
            .child(motion::enter(
                ElementId::named_usize("home-actions", self.switches),
                3,
                actions,
            ))
    }

    /// The title: Horizon's mark and the club's name in Bricolage, and what it plays at the right.
    fn home_title(&mut self, tag: &str) -> Div {
        let version = hero_label("package-01", tag.to_owned().into());
        div()
            .mt(px(16.))
            .flex()
            .items_center()
            .gap(px(16.))
            .min_w_0()
            .child(self.horizon_mark(56.))
            .child(
                div()
                    .id("home-title")
                    .relative()
                    .min_w_0()
                    .truncate()
                    .font_family(horizon_theme::glass::DISPLAY_FONT)
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_size(px(44.))
                    .line_height(px(48.))
                    .text_color(white(ink::FULL))
                    .child(self.config.club.name.clone()),
            )
            .child(div().flex_1())
            .child(
                div()
                    .flex()
                    .flex_none()
                    .items_center()
                    .gap(px(8.))
                    .child(version),
            )
    }

    /// "Playing for 5 min now. Have fun!", while the game runs.
    fn running_text(&self) -> Option<Div> {
        let line = trf(
            "Playing for {time} now. Have fun!",
            &[("time", &play_time(self.client_since?.elapsed()))],
        );
        // A soft dark scrim keeps the line readable over the brightest sky (a low sun, snow).
        Some(
            ui::text(15., FontWeight::NORMAL, ink::STRONG)
                .mt(px(10.))
                .ml(px(-10.))
                .px(px(10.))
                .py(px(6.))
                .rounded(px(radius::CONTROL))
                .bg(gpui::black().opacity(0.28))
                .self_start()
                .flex()
                .flex_wrap()
                .gap(px(4.))
                .child(line),
        )
    }

    /// Play (`PlayGroup`): Stop while the game runs, the step while a start is on, Play otherwise.
    fn play_group(&mut self, cx: &mut Context<Self>) -> Div {
        let t = self.theme;
        let accent = ui::color(self.look.accent);
        let running = self.client.is_some();
        let busy: Option<SharedString> = self.starting_client.as_ref().map(|_| {
            self.play_step
                .now()
                .unwrap_or(tr("Starting"))
                .to_owned()
                .into()
        });
        // What is drawn on the accent takes its own ink (white on the default violet), in the
        // light look too, where `white` is the page's dark ink; Stop sits on a quiet fill.
        let on_accent = ui::color(self.look.on_accent);
        let glyph = if running && busy.is_none() {
            white(ink::FULL)
        } else {
            on_accent
        };
        let base = div()
            .relative()
            .flex()
            .items_center()
            .flex_none()
            // At least the mockup's width, wider when the label needs it (a long translation).
            .min_w(px(if self.window_width >= WIDE {
                200.
            } else {
                160.
            }))
            .h(px(TALL))
            .overflow_hidden()
            .border_1()
            .rounded(px(radius::LARGE));
        let body = |label: SharedString, icon: &'static str| {
            div()
                .flex()
                .flex_1()
                .h_full()
                .items_center()
                .justify_center()
                .gap(px(10.))
                .px(px(20.))
                .whitespace_nowrap()
                .font_family(horizon_theme::glass::FONT)
                .text_size(px(18.))
                .font_weight(FontWeight::SEMIBOLD)
                // The glyph leans into the press while the pointer is on Play.
                .child(
                    ui::icon(icon, 20., glyph)
                        .group_hover("play-glyph", |s| s.ml(px(3.)).mr(px(-3.))),
                )
                .child(label)
        };
        let busy_none = busy.is_none();
        let group = match busy {
            Some(label) => base
                .bg(accent.opacity(0.35))
                .border_color(accent.opacity(0.5))
                .child(
                    div()
                        .relative()
                        .flex()
                        .flex_1()
                        .justify_center()
                        .px(px(16.))
                        .whitespace_nowrap()
                        .text_color(on_accent)
                        .font_family(horizon_theme::glass::FONT)
                        .text_size(px(16.))
                        .font_weight(FontWeight::SEMIBOLD)
                        .child(format!("{label}…")),
                ),
            None if running => base
                .bg(white(0.14))
                .border_color(white(ui::edge::HOVER))
                .child(
                    body(tr("Stop").into(), "stop")
                        .id("play")
                        .cursor_pointer()
                        .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                            if let Some(client) = &this.client {
                                client.kill();
                            }
                            cx.notify();
                        })),
                ),
            None => {
                let main = crate::track_hover(
                    body(tr("Play").into(), "play").id("play").cursor_pointer(),
                    SharedString::from("play"),
                );
                // Play reads the club's file again and starts the game, unless Home says what
                // keeps it from starting.
                let main = main.on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                    this.play();
                    cx.notify();
                }));
                // The gradient's far end leans away from the label's ink (deeper under white
                // ink, lighter under dark ink), so the label stays legible across the button.
                let (accent_hex, ink_hex) = (self.look.accent, self.look.on_accent);
                let far = if horizon_theme::contrast(ink_hex, 0x000000)
                    > horizon_theme::contrast(ink_hex, 0xffffff)
                {
                    horizon_theme::glass::mix_oklab(accent_hex, 0x000000, 0.75)
                } else {
                    ui::accent2()
                };
                base.bg(ui::grad(135., accent, ui::color(far)))
                    .border_color(white(0.16))
                    .shadow(ui::glows(vec![
                        // The site's primary: a light line along the top edge, a glow under it.
                        gpui::BoxShadow {
                            color: white(0.22),
                            offset: gpui::point(px(0.), px(1.)),
                            blur_radius: px(0.),
                            spread_radius: px(0.),
                            inset: true,
                        },
                        gpui::BoxShadow {
                            color: accent.opacity(0.7),
                            offset: gpui::point(px(0.), px(6.)),
                            blur_radius: px(22.),
                            spread_radius: px(-8.),
                            inset: false,
                        },
                    ]))
                    .text_color(on_accent)
                    .child(main)
                    .child(ui::sheen(&SharedString::from("play")))
            }
        };
        // Ready to play: the accent's glow comes up under the pointer, a press dims it.
        let primary = !running && busy_none;
        let soft = ui::color(self.look.accent_soft);
        let group = group
            .id("play-group")
            .group("play-glyph")
            .when(primary, |d| {
                d.hover(move |s| {
                    s.border_color(white(0.34)).shadow(ui::glows(vec![
                        gpui::BoxShadow {
                            color: white(0.28),
                            offset: gpui::point(px(0.), px(1.)),
                            blur_radius: px(0.),
                            spread_radius: px(0.),
                            inset: true,
                        },
                        gpui::BoxShadow {
                            color: accent.opacity(0.95),
                            offset: gpui::point(px(0.), px(12.)),
                            blur_radius: px(34.),
                            spread_radius: px(-8.),
                            inset: false,
                        },
                        gpui::BoxShadow {
                            color: soft.opacity(0.5),
                            offset: gpui::point(px(0.), px(0.)),
                            blur_radius: px(0.),
                            spread_radius: px(1.),
                            inset: false,
                        },
                    ]))
                })
                .active(|s| s.opacity(0.92).shadow(Vec::new()))
            });
        div().relative().child(keyboard(group, &t))
    }

    /// The folder button and its ⋯ (`FolderGroup`), which shows under the pointer or while its
    /// menu is open: the game folder and its screenshots.
    fn folder_group(&self, cx: &mut Context<Self>) -> Div {
        let t = self.theme;
        let home = backend::game_home();
        let open = self.home_menu == HomeMenu::Folder;
        let opener = home.clone();
        let more = div()
            .id("folder-more")
            .relative()
            .flex()
            .flex_none()
            .items_center()
            .justify_center()
            .h(px(40.))
            .w(px(36.))
            .mr(px(8.))
            .rounded(px(radius::CONTROL))
            .cursor_pointer()
            .when(open, |d| {
                d.w(px(36.))
                    .mr(px(8.))
                    .opacity(1.)
                    .bg(white(fill::SELECTED))
            })
            .group_hover("folder-group", |s| s.w(px(36.)).mr(px(8.)).opacity(1.))
            .when(!open, |d| {
                d.child(ui::back_tint(radius::CONTROL, white(fill::SELECTED)))
            })
            .child(ui::icon("more-horizontal", 20., white(ink::STRONG)).relative())
            .on_click(cx.listener(|this, event: &ClickEvent, _, cx| {
                if !this.closed_by_this_press(event) {
                    this.home_menu = match this.home_menu {
                        HomeMenu::Folder => HomeMenu::None,
                        _ => HomeMenu::Folder,
                    };
                }
                cx.notify();
            }));
        let group = div()
            .id("folder-group")
            .relative()
            .flex()
            .items_center()
            .h(px(TALL))
            .rounded(px(radius::LARGE))
            .bg(white(fill::BUTTON))
            .border_1()
            .border_color(white(if open {
                ui::edge::STRONG
            } else {
                ui::edge::REST
            }))
            .child(ui::back_edge(radius::LARGE, 0.05, true))
            .child(
                div()
                    .id("home-folder")
                    .relative()
                    .flex()
                    .items_center()
                    .justify_center()
                    .size(px(TALL - 2.))
                    .cursor_pointer()
                    .active(|s| s.bg(white(0.1)))
                    .child(ui::icon("folder-01", 21., white(ink::STRONG)))
                    .tooltip(ui::tip(tr("Open the game folder")))
                    .on_click(move |_, _, cx| {
                        let _ = std::fs::create_dir_all(&opener);
                        crate::glass_toast::open(cx, &opener);
                    }),
            )
            .when(open || self.folder_hovered, |d| d.child(more))
            .on_hover(cx.listener(|this, hovered: &bool, _, cx| {
                this.folder_hovered = *hovered;
                cx.notify();
            }));
        let mut wrap = div().relative().child(keyboard(group, &t));
        if open {
            let shots = home.join("screenshots");
            let count = std::fs::read_dir(&shots).map_or(0, |d| d.count());
            let items = vec![
                (
                    "folder-game",
                    tr("Open the game folder"),
                    short(&home),
                    home.clone(),
                ),
                (
                    "folder-shots",
                    tr("Open the screenshots folder"),
                    if count == 1 {
                        tr("1 screenshot").into()
                    } else {
                        trf("{count} screenshots", &[("count", &count.to_string())])
                    },
                    shots,
                ),
            ];
            let mut menu = ui::popover(290.)
                .id("folder-menu")
                .occlude()
                .on_mouse_down_out(cx.listener(|this, event: &MouseDownEvent, _, cx| {
                    this.home_menu = HomeMenu::None;
                    this.menu_closed_at = Some(event.position);
                    cx.notify();
                }));
            for (key, label, note, path) in items {
                menu = menu.child(ui::menu_item(key, label.into(), note, false, &t).on_click(
                    cx.listener(move |this, _: &ClickEvent, _, cx| {
                        this.home_menu = HomeMenu::None;
                        let _ = std::fs::create_dir_all(&path);
                        crate::glass_toast::open(cx, &path);
                        cx.notify();
                    }),
                ));
            }
            wrap = wrap.child(
                deferred(div().absolute().top(px(TALL + 8.)).left_0().child(menu)).with_priority(1),
            );
        }
        wrap
    }
}

/// A label at the hero title's right: 40 px glass with an 18 px icon and its label.
fn hero_label(icon: &'static str, label: SharedString) -> Div {
    div()
        .relative()
        .flex()
        .flex_none()
        .items_center()
        .gap(px(8.))
        .h(px(40.))
        .pl(px(14.))
        .pr(px(16.))
        .rounded(px(radius::CONTROL))
        .bg(white(fill::BUTTON))
        .border_1()
        .border_color(white(ui::edge::REST))
        .child(ui::back_edge(radius::CONTROL, 0.05, true))
        .font_family(horizon_theme::glass::FONT)
        .text_size(px(14.))
        .line_height(px(20.))
        .font_weight(FontWeight::MEDIUM)
        .text_color(white(ink::BODY))
        .child(ui::icon(icon, 18., white(ink::STRONG)).relative())
        .child(div().relative().child(label))
}

/// "12 min", "3 h 05 min", "140 h": how long the game has run, as players read it.
fn play_time(played: Duration) -> String {
    let minutes = played.as_secs() / 60;
    match (minutes / 60, minutes % 60) {
        (0, 0) => tr("under a minute").into(),
        (0, m) => format!("{m} min"),
        (h, m) if h < 100 => format!("{h} h {m:02} min"),
        (h, _) => format!("{h} h"),
    }
}

/// When something was last played, as the mockup writes it: "just now", "25 min ago", "2 hours
/// ago" within a day, then "yesterday", then the day ("Oct 31").
pub(super) fn last_played(when: std::time::SystemTime) -> String {
    let local: chrono::DateTime<chrono::Local> = when.into();
    since(
        std::time::SystemTime::now()
            .duration_since(when)
            .unwrap_or_default(),
        (chrono::Local::now().date_naive() - local.date_naive()).num_days(),
        &local.format("%Y-%m-%d").to_string(),
    )
}

/// [`last_played`] from how long ago it was, how many calendar days back, and its date.
fn since(ago: Duration, days: i64, date: &str) -> String {
    match ago.as_secs() / 60 {
        0 => tr("just now").into(),
        m @ 1..=59 => trf("{count} min ago", &[("count", &m.to_string())]),
        60..=119 => tr("1 hour ago").into(),
        m if m < 24 * 60 => trf("{count} hours ago", &[("count", &(m / 60).to_string())]),
        _ if days <= 1 => tr("yesterday").into(),
        _ => glass_versions::published(date, ""),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_running_time_reads_as_minutes_then_hours() {
        let t = |s| play_time(Duration::from_secs(s));
        assert_eq!(t(59), "under a minute");
        assert_eq!(t(12 * 60 + 59), "12 min");
        assert_eq!(t(3 * 3600 + 5 * 60), "3 h 05 min");
        assert_eq!(t(140 * 3600 + 7 * 60), "140 h");
    }

    #[test]
    fn last_played_counts_hours_within_a_day_then_days() {
        let m = |min: u64| Duration::from_secs(min * 60);
        assert_eq!(since(m(0), 0, "2026-11-03"), "just now");
        assert_eq!(since(m(25), 0, "2026-11-03"), "25 min ago");
        assert_eq!(since(m(130), 0, "2026-11-03"), "2 hours ago");
        // Under 24 hours, even across midnight.
        assert_eq!(since(m(20 * 60), 1, "2026-11-02"), "20 hours ago");
        assert_eq!(since(m(30 * 60), 1, "2026-11-02"), "yesterday");
        assert_eq!(since(m(3 * 24 * 60), 3, "2026-10-31"), "Oct 31");
    }
}
