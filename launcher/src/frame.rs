//! The glass look's frame (the mockup's `launcher-app.tsx`): the sidebar plate with the game's
//! row and the player's hosted servers; the top bar with the pages in a glass tab strip, the
//! player's name and Settings; and the game row's menu. The row is the club's: its name above the
//! game's state.
//! Dark appearance only for now (`docs/design-system.md`, "Glass").
use super::*;
use crate::glass_ui::{self as ui, white, Tone};
use gpui::{anchored, canvas, deferred, point};
use horizon_theme::glass::{fill, ink, radius};
use std::cell::RefCell;
use std::rc::Rc;

/// The rail's distance from the window's edges (`m-3.5`).
pub const MARGIN: f32 = 14.;

/// The rail's width at rest (icons) and open (icons and names), while the pointer is on it.
pub const RAIL: f32 = 80.;
pub const RAIL_OPEN: f32 = 248.;
/// What drifting particles (Taiga's snow, Hypr Neon's streaks, leaves, petals) keep clear of in a
/// window of `w` by `h`: the title and top bars, the rail, and the text. Home's hero text sits in
/// the left column and its tabs below it, so the sky drifts through the top right between them;
/// every other page is text from the top bar down.
pub fn keep_clear(w: f32, h: f32, home: bool) -> rain::Keep {
    const TOP: f32 = 32. + MARGIN + theme::TOPBAR;
    const HERO: f32 = 400.;
    const HERO_WIDTH: f32 = 640.;
    let left = MARGIN + RAIL;
    let body = if home { HERO } else { TOP };
    rain::Keep([
        [0., 0., w, TOP],
        [0., 0., left, h],
        [left, TOP, HERO_WIDTH.min(w - left), (HERO - TOP).max(0.)],
        [left, body, w - left, (h - body).max(0.)],
        // The hero's right column: its version button.
        [(w - 360.).max(left), TOP, 360., (HERO - TOP).max(0.)],
    ])
}

/// How the rail opens and shuts: quick, settling from a little past its width.
const RAIL_MOTION: horizon_theme::Motion = horizon_theme::Motion {
    name: "rail",
    ms: 320,
    curve: [0.22, 0.9, 0.3, 1.08],
};
/// The rail's state: whether the pointer is on it, and where its last opening or shutting began.
#[derive(Default)]
pub struct Rail {
    pub hovered: bool,
    open: bool,
    since: Option<Instant>,
    from: f32,
    at: f32,
    /// The list's scroll, for the cue that more rows sit below.
    list: gpui::ScrollHandle,
}

impl Rail {
    /// How far open the rail is, 0 to 1 (a little past 1 as it settles). `pinned`: something it
    /// opened is still open (a row's menu), so it stays open without the pointer. Frames are asked
    /// for only while it moves; under reduced motion it jumps.
    pub fn progress(&mut self, window: &mut Window, pinned: bool) -> f32 {
        // `HORIZON_LAUNCHER_RAIL_OPEN=1` holds it open, for screenshots.
        static HELD: std::sync::LazyLock<bool> =
            std::sync::LazyLock::new(|| std::env::var_os("HORIZON_LAUNCHER_RAIL_OPEN").is_some());
        let want = self.hovered || pinned || *HELD;
        if want != self.open {
            self.open = want;
            self.from = self.at;
            self.since = Some(Instant::now());
        }
        let to = if want { 1. } else { 0. };
        let t = match self.since {
            Some(since) if !crate::motion::still() => {
                since.elapsed().as_secs_f32() / RAIL_MOTION.secs()
            }
            _ => 1.,
        };
        if t >= 1. {
            self.since = None;
            self.at = to;
            return to;
        }
        self.at = self.from + (to - self.from) * RAIL_MOTION.ease(t);
        window.request_animation_frame();
        self.at
    }
}

/// The ⋮ menu of the sidebar's game row while open: whether it is, and where the row was drawn
/// last, since the menu opens beside its row.
#[derive(Clone, Default)]
pub struct RowMenu {
    pub open: bool,
    row: Rc<RefCell<Option<Bounds<Pixels>>>>,
}

/// Records where the element it sits in is drawn.
fn measure(at: &Rc<RefCell<Option<Bounds<Pixels>>>>) -> impl IntoElement {
    let at = at.clone();
    canvas(
        move |bounds, _, _| {
            *at.borrow_mut() = Some(bounds);
        },
        |_, _, _, _| {},
    )
    .absolute()
    .top_0()
    .left_0()
    .size_full()
}

impl Launcher {
    /// The game row's state line and its tone (the mockup's `statusOf`), from what the launcher
    /// knows of the game: both sidebars' row says it under the game's name.
    pub(crate) fn row_state(&self) -> (Tone, String) {
        if self.client.is_some() {
            return (Tone::Live, tr("Running").into());
        }
        if self.starting_client.is_some() {
            // The step the start is on, once it said one.
            return (
                Tone::Busy,
                self.play_step.now().unwrap_or(tr("Starting")).into(),
            );
        }
        // What keeps the game from starting, in as many words as the rail has (Home says the
        // sentence).
        if let Some(refusal) = &self.refusal {
            return (Tone::Warn, refusal.rail.clone());
        }
        match self.selected() {
            None => (Tone::Off, tr("No version installed").into()),
            Some(tag) => (Tone::Live, format!("{} · {tag}", tr("Ready"))),
        }
    }

    /// The game's row: Horizon's mark, the club's name and the game's state, open to Home. `open`:
    /// the rail shows the name (else the mark alone, the name on hover); `label`: how far the
    /// names have faded in.
    fn game_row(&mut self, open: bool, label: f32, cx: &mut Context<Self>) -> AnyElement {
        let t = self.theme;
        let state = self.row_state();
        let name = self.config.club.name.clone();
        let menu_open = self.row_menu.open;
        // A right click on the row opens the same menu the ⋮ does.
        let more = div()
            .absolute()
            .top_0()
            .bottom_0()
            .right(px(6.))
            .flex()
            .items_center()
            .child(
                ui::icon_button(
                    "side-more",
                    "more-vertical",
                    32.,
                    18.,
                    radius::SMALL,
                    menu_open,
                    &t,
                )
                .on_click(cx.listener(|this, event: &ClickEvent, _, cx| {
                    if this.closed_by_this_press(event) {
                        return;
                    }
                    this.row_menu.open = !this.row_menu.open;
                    cx.notify();
                })),
            );
        crate::motion::enter(
            ElementId::named_usize("side-enter", 0),
            0,
            div()
                .relative()
                .child(measure(&self.row_menu.row))
                .child(
                    // As in the mockup, the game's row stays marked on every page.
                    ui::row("side-game", true, if open { 40. } else { 8. }, &t)
                        .child(div().relative().flex_none().child(self.horizon_mark(38.)))
                        .child(
                            ui::row_text(name.clone(), state)
                                .overflow_hidden()
                                .opacity(label),
                        )
                        // Narrow, the mark alone: the name on hover.
                        .when(!open, |d| d.tooltip(ui::tip(name)))
                        .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                            this.go(Page::Play);
                            cx.notify();
                        }))
                        // The ⋮ menu, from a right click anywhere on the row.
                        .on_mouse_down(
                            gpui::MouseButton::Right,
                            cx.listener(|this, event: &gpui::MouseDownEvent, _, cx| {
                                // The open menu's mouse-down-out already closed it on this
                                // press: reopening here would make the toggle a no-op.
                                if this.menu_closed_at.take() == Some(event.position) {
                                    cx.notify();
                                    return;
                                }
                                this.row_menu.open = !this.row_menu.open;
                                cx.notify();
                            }),
                        ),
                )
                .when(open, |d| d.child(more)),
        )
    }

    /// The rail: a strip of icons that opens into names and states while the pointer is on it
    /// (or its menu is open). It opens over the page, which keeps the collapsed rail's room, so
    /// the page never moves or squeezes under the pointer. Its foot keeps Settings and the version.
    pub(super) fn glass_rail(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let t = self.theme;
        let pinned = self.row_menu.open;
        let p = self.rail.progress(window, pinned);
        let open = p > 0.5;
        // Names and lines fade in over the second half of the opening, as the width arrives.
        let label = ((p - 0.35) / 0.65).clamp(0., 1.);
        let fade = move |d: Div| d.min_w_0().overflow_hidden().opacity(label);

        // A rail button: an icon, its name beside it once the rail opens.
        let button = |id: SharedString, icon: &'static str, name: SharedString, active: bool| {
            keyboard(
                track_hover(
                    div()
                        .id(id.clone())
                        .relative()
                        .flex()
                        .flex_none()
                        .items_center()
                        .gap(px(12.))
                        .w_full()
                        .h(px(42.))
                        .px(px(17.5))
                        .rounded(px(radius::ROW))
                        .border_1()
                        .border_color(if active {
                            white(ui::edge::STRONG)
                        } else {
                            gpui::transparent_black()
                        })
                        .when(active, |d| d.bg(white(fill::ACTIVE)))
                        .cursor_pointer()
                        .child(ui::hover_edge(&id, radius::ROW, fill::ROW_HOVER, true))
                        .child(ui::icon(icon, 19., white(ink::SOFT)).relative())
                        .child(
                            fade(ui::text(14., FontWeight::MEDIUM, ink::STRONG).relative())
                                .whitespace_nowrap()
                                .child(name.clone()),
                        )
                        // Narrow, the rail shows icons alone: the name on hover.
                        .when(!open, |d| d.tooltip(ui::tip(name))),
                    id,
                ),
                &t,
            )
        };

        let mut list = div()
            .id("side-list")
            .flex()
            .flex_col()
            .flex_1()
            .min_h_0()
            .overflow_y_scroll()
            .track_scroll(&self.rail.list)
            .child(self.game_row(open, label, cx));

        // The player's hosted servers, one more section under the game.
        let servers: Vec<_> = self
            .servers
            .sidebar_list()
            .map(<[_]>::to_vec)
            .unwrap_or_else(|| self.side_servers.clone());
        // The Servers page exists only when the club lets a student host: then the rail has its
        // group from the first day, with the way to a first server under an empty list.
        let hosting = self.config.hosting.enabled;
        let servers = if hosting { servers } else { Vec::new() };
        if hosting {
            list = list.child(
                div()
                    .h(px(22.))
                    .flex()
                    .items_end()
                    .px(px(8.))
                    .pb(px(4.))
                    .child(
                        fade(ui::text(12., FontWeight::MEDIUM, ink::DIM))
                            .whitespace_nowrap()
                            .child(tr("Servers")),
                    ),
            );
        }
        let mut server_rows = div().flex().flex_col().gap(px(2.));
        for e in &servers {
            let up = self.servers.is_up(&e.dir);
            let shown = self.page == Page::Servers && self.servers.open.as_ref() == Some(&e.dir);
            let dir = e.dir.clone();
            server_rows = server_rows.child(
                ui::row(format!("side-server-{}", e.dir.display()), shown, 8., &t)
                    .child(
                        div()
                            .relative()
                            .flex()
                            .flex_none()
                            .items_center()
                            .justify_center()
                            .size(px(38.))
                            .rounded(px(radius::CONTROL))
                            .bg(white(fill::BUTTON))
                            .child(ui::icon("computer-terminal-01", 18., white(ink::STRONG))),
                    )
                    .child(fade(ui::row_text(
                        e.name.clone(),
                        if up {
                            (Tone::Live, tr("Running").into())
                        } else {
                            (Tone::Off, tr("Stopped").into())
                        },
                    )))
                    .when(!open, |d| d.tooltip(ui::tip(e.name.clone())))
                    .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                        this.open_server(dir.clone());
                        this.go(Page::Servers);
                        cx.notify();
                    })),
            );
        }
        if hosting {
            server_rows = server_rows.child(
                ui::row("side-server-create", false, 8., &t)
                    .child(
                        div()
                            .relative()
                            .flex()
                            .flex_none()
                            .items_center()
                            .justify_center()
                            .size(px(38.))
                            .rounded(px(radius::CONTROL))
                            .bg(white(fill::BUTTON))
                            .child(ui::icon("plus-sign", 18., white(ink::STRONG))),
                    )
                    .child(fade(ui::row_text(
                        tr("Create a server").to_string(),
                        (Tone::Off, tr("On this computer").into()),
                    )))
                    .when(!open, |d| {
                        d.tooltip(ui::tip(tr("Create a server").to_string()))
                    })
                    .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                        this.open_create(cx);
                        this.go(Page::Servers);
                        cx.notify();
                    })),
            );
        }
        let list = list.child(server_rows);
        // More rows than the rail shows: while rows sit below the list's foot a fade and a
        // "More" pill say so; the pill scrolls to them.
        let rows = servers.len() + usize::from(hosting);
        let overflows = 52. * (1 + rows) as f32 > f32::from(window.viewport_size().height) - 420.;
        let scroll = self.rail.list.clone();
        // Before the list's first layout the handle knows no height: the estimate stands in.
        let below = match scroll.max_offset().y {
            max if max == px(0.) => overflows,
            max => max + scroll.offset().y > px(2.),
        };
        let more = div()
            .id("rail-more")
            .flex()
            .flex_none()
            .items_center()
            .justify_center()
            .gap(px(4.))
            .h(px(24.))
            .px(px(10.))
            .rounded_full()
            .border_1()
            .border_color(white(ui::edge::STRONG))
            .bg(white(fill::ACTIVE))
            .cursor_pointer()
            .child(ui::icon("arrow-down-01", 14., white(ink::STRONG)))
            .when(open, |d| {
                d.child(
                    fade(ui::text(12., FontWeight::MEDIUM, ink::STRONG))
                        .whitespace_nowrap()
                        .child(tr("More")),
                )
            })
            .when(!open, |d| d.tooltip(ui::tip(tr("More rows below"))))
            .on_click(move |_, _, _| scroll.scroll_to_bottom());
        // The fade sits over the list's last rows; the pill gets its own line under them, so it
        // never covers a tile.
        let list = div()
            .flex()
            .flex_col()
            .flex_1()
            .min_h_0()
            .gap(px(6.))
            .child(
                div()
                    .relative()
                    .flex()
                    .flex_col()
                    .flex_1()
                    .min_h_0()
                    .child(list)
                    .when(below, |d| {
                        d.child(
                            div()
                                .absolute()
                                .left_0()
                                .right_0()
                                .bottom_0()
                                .h(px(28.))
                                .bg(gpui::linear_gradient(
                                    180.,
                                    gpui::linear_color_stop(gpui::black().opacity(0.), 0.),
                                    gpui::linear_color_stop(gpui::black().opacity(0.3), 1.),
                                )),
                        )
                    }),
            )
            .when(below, |d| {
                d.child(div().flex().flex_none().justify_center().child(more))
            });

        // A hairline set in from the rail's sides: between the rows and the buttons, and
        // between Settings and the version.
        let rule = || {
            div()
                .flex_none()
                .h(px(1.))
                .mx(px(10.))
                .my(px(4.))
                .bg(white(0.1))
        };
        let mut foot = div()
            .flex()
            .flex_col()
            .flex_none()
            .gap(px(2.))
            .pt(px(4.))
            .child(rule());

        let settings_open = self.page == Page::Settings;
        foot = foot.child(
            button(
                "rail-settings".into(),
                "sliders-horizontal",
                tr("Settings").into(),
                settings_open,
            )
            .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                this.go(Page::Settings);
                cx.notify();
            })),
        );
        // The launcher's version under it: "Launcher 0.2.0" open, the number alone while narrow.
        let version: SharedString = if open {
            trf("Launcher {version}", &[("version", backend::VERSION)]).into()
        } else {
            backend::VERSION.into()
        };
        foot = foot.child(rule()).child(
            div()
                .flex()
                .items_center()
                .h(px(20.))
                .px(px(if open { 12. } else { 0. }))
                .when(!open, |d| d.justify_center())
                .child(
                    ui::text(11.5, FontWeight::NORMAL, ink::DIM)
                        .id("launcher-version")
                        .whitespace_nowrap()
                        .truncate()
                        .child(version)
                        .when(!open, |d| {
                            d.tooltip(ui::tip(trf(
                                "Launcher {version}",
                                &[("version", backend::VERSION)],
                            )))
                        }),
                ),
        );

        let width = RAIL + (RAIL_OPEN - RAIL) * p;
        crate::glass::plate(radius::PANEL)
            .id("rail")
            .flex()
            .flex_col()
            .flex_none()
            .gap(px(10.))
            .w(px(width))
            .px(px(12.))
            .py(px(12.))
            .overflow_hidden()
            // Open, it lies over the page: a shadow lifts it off what it covers.
            .when(p > 0.02, |d| d.shadow_lg())
            .occlude()
            .on_hover(cx.listener(|this, hovered: &bool, _, cx| {
                this.rail.hovered = *hovered;
                cx.notify();
            }))
            .child(list.relative())
            .child(foot.relative())
    }

    /// The glass top bar, across the window above the rail and the page: Horizon's mark over
    /// the rail, the pages, then search (the palette) and the player.
    /// Launcher settings live in the rail.
    pub(super) fn glass_topbar(&mut self, window: &mut Window, cx: &mut Context<Self>) -> Div {
        let t = self.theme;
        let compact = self.window_width < crate::glass_home::COMPACT;
        // Near the window's smallest width (960) the page tabs need all the room: the player is
        // only a face, with a tooltip.
        let tight = self.window_width < 1000.;
        let mark = div()
            .flex()
            .flex_none()
            .items_center()
            .justify_center()
            .w(px(RAIL))
            .child(self.horizon_mark(20.));
        let mut tabs = div()
            .flex()
            .items_center()
            .gap(px(if compact { 2. } else { 4. }))
            .min_w_0();
        for (page, label, _) in Page::in_top_bar(self.config.hosting.enabled) {
            let mut item = ui::nav_tab(format!("nav-{label}"), tr(label), self.page == page, &t);
            if compact {
                item = item.px(px(8.));
            }
            tabs = tabs.child(
                item.on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                    this.go(page);
                    cx.notify();
                })),
            );
        }
        let performance = self.settings.performance();
        let strip = self.nav_strip.render(tabs, &t, performance, window);

        // Who plays: the Windows session's name, a plain label with no menu.
        let account = self.player.clone().map(|name| {
            let chip = div()
                .id("who-plays")
                .relative()
                .when(tight, |d| d.tooltip(ui::tip(name.clone())))
                .flex()
                .items_center()
                .gap(px(10.))
                .h(px(44.))
                .pl(px(8.))
                .pr(px(12.))
                .rounded(px(radius::ROW))
                .bg(white(fill::ROW_HOVER))
                .child(
                    div()
                        .size(px(26.))
                        .rounded(px(7.))
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(ui::icon("user", 16., white(ink::NOTE))),
                )
                .children((!tight).then(|| {
                    div()
                        .flex()
                        .flex_col()
                        .max_w(px(if compact { 100. } else { 150. }))
                        .child(
                            ui::text(14., FontWeight::SEMIBOLD, ink::FULL)
                                .line_height(px(17.))
                                .truncate()
                                .child(name),
                        )
                }));
            keyboard(chip, &t)
        });
        // Search: the palette (⌘K or Ctrl+K anywhere), as a field-shaped button.
        let sid = SharedString::from("topbar-search");
        let key: &str = if cfg!(target_os = "macos") {
            "⌘K"
        } else {
            tr("Ctrl K")
        };
        let search = keyboard(
            track_hover(
                div()
                    .id(sid.clone())
                    .relative()
                    .flex()
                    .flex_none()
                    .items_center()
                    .gap(px(10.))
                    .h(px(38.))
                    .w(px(if compact { 38. } else { 230. }))
                    .when(compact, |d| d.justify_center())
                    .when(!compact, |d| d.px(px(12.)))
                    .rounded(px(radius::CONTROL))
                    .bg(white(fill::FIELD))
                    .border_1()
                    .border_color(white(ui::edge::REST))
                    .cursor_pointer()
                    .child(ui::hover_edge(&sid, radius::CONTROL, fill::HOVER, true))
                    .child(ui::icon("search-01", 16., white(ink::QUIET)).relative())
                    .when(!compact, |d| {
                        d.child(
                            ui::text(13.5, FontWeight::NORMAL, ink::QUIET)
                                .relative()
                                .flex_1()
                                .min_w_0()
                                .truncate()
                                .child(tr("Search…")),
                        )
                        .child(
                            ui::text(11.5, FontWeight::MEDIUM, ink::DIM)
                                .relative()
                                .flex()
                                .flex_none()
                                .items_center()
                                .h(px(20.))
                                .px(px(6.))
                                .rounded(px(5.))
                                .border_1()
                                .border_color(white(ui::edge::REST))
                                .child(key.to_string()),
                        )
                    })
                    .when(compact, |d| d.tooltip(ui::tip(tr("Search…"))))
                    .on_click(cx.listener(|this, _: &ClickEvent, window, cx| {
                        this.palette_toggle(window, cx);
                        cx.notify();
                    })),
                sid,
            ),
            &t,
        );
        div()
            .flex()
            .flex_none()
            .items_center()
            .gap(px(12.))
            .h(px(64.))
            .pl(px(MARGIN))
            .pr(px(MARGIN + 8.))
            .pt(px(4.))
            .child(mark)
            .child(strip)
            .children(self.toast_in_topbar())
            .child(
                div()
                    .ml_auto()
                    .flex()
                    .flex_none()
                    .items_center()
                    .gap(px(8.))
                    .child(search)
                    .children(account),
            )
    }

    /// The ⋮ menu of the sidebar's game row, beside it: where its folder opens.
    pub(super) fn row_menu(
        &mut self,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        if !self.row_menu.open {
            return None;
        }
        let row = *self.row_menu.row.borrow();
        let row = row?;
        let t = self.theme;
        let viewport = window.viewport_size();
        // The mockup's `rect.right + 12`, `rect.top - 8`: its ⋮ button ends 6 px inside the row
        // and sits in the row's middle.
        let x = f32::from(row.origin.x + row.size.width) - 6. + 12.;
        let button_top = f32::from(row.origin.y) + (f32::from(row.size.height) - 32.) / 2.;
        let y = (button_top - 8.).max(8.);
        let label = tr("Open folder");
        let viewport_width = f32::from(viewport.width);
        let width = ui::menu_content_width(window, &[label])
            .max(250.)
            .min((viewport_width - 16.).max(0.));
        let x = x.min(viewport_width - width - 8.).max(8.);
        let home = backend::game_home();
        let menu = ui::popover(width)
            .id("row-menu")
            .occlude()
            .on_mouse_down_out(cx.listener(|this, event: &MouseDownEvent, _, cx| {
                this.row_menu.open = false;
                this.menu_closed_at = Some(event.position);
                cx.notify();
            }))
            .child(
                ui::menu_row("row-menu-folder", "folder-01", label, false, &t).on_click(
                    cx.listener(move |this, _: &ClickEvent, _, cx| {
                        this.row_menu.open = false;
                        let _ = std::fs::create_dir_all(&home);
                        crate::glass_toast::open(cx, &home);
                        cx.notify();
                    }),
                ),
            );
        Some(
            deferred(anchored().position(point(px(x), px(y))).child(menu))
                .with_priority(1)
                .into_any_element(),
        )
    }
}
