//! Home's Worlds and Multiplayer tabs in the glass look (the mockup's `Worlds` and
//! `Multiplayer`, `launcher-views.tsx`): one `ROW` per world or server. A world's row plays it,
//! its size turning into "Play →" under the pointer; its ⋯ menu holds what the launcher does
//! besides (back up, copy, folder, delete). A server's row says who is on it and has Join. Dark
//! appearance only.
use super::*;
use crate::glass_ui::{self as ui, white};
use gpui::deferred;
use horizon_launcher::worlds::World;
use horizon_theme::glass::{fill, ink, radius};
use std::path::Path;
use worlds_page::Action;

/// A listener of the launcher's, boxed so a helper can return it.
pub(super) type Listener<E> = Box<dyn Fn(&E, &mut Window, &mut App)>;

/// The 36 px picture at a row's start (`PixelIcon` at 36), round at `round(36 · 0.26)`.
const PICTURE: f32 = 36.;
const PICTURE_RADIUS: f32 = 9.;

impl Launcher {
    /// Records whether the pointer is over the row `key`; the row then shows its actions.
    pub(super) fn list_hover_listener(
        &self,
        key: SharedString,
        cx: &mut Context<Self>,
    ) -> Listener<bool> {
        Box::new(cx.listener(move |this, hovered: &bool, _, cx| {
            if *hovered {
                this.list_hover = Some(key.clone());
            } else if this.list_hover.as_ref() == Some(&key) {
                this.list_hover = None;
            }
            cx.notify();
        }))
    }

    /// Home's Worlds tab: the game's worlds, the last played first.
    pub(super) fn glass_worlds(&mut self, cx: &mut Context<Self>) -> Div {
        let t = self.theme;
        self.load_worlds(cx);
        let list = self.worlds.list.clone();
        let mut body = div().flex().flex_col();
        // What the last back up, copy or delete did, where the mockup's toast would say it.
        match self.worlds.done.clone() {
            Some(Ok((message, zip))) => {
                let open = zip.map(|zip| {
                    let dir = zip.parent().map(Path::to_path_buf).unwrap_or(zip);
                    ui::quiet("world-backups", tr("Open backups"), 36., &t)
                        .on_click(move |_, _, cx| crate::glass_toast::open(cx, &dir))
                });
                body = body.child(ui::list_head(message).children(open));
            }
            Some(Err(e)) => body = body.child(danger_line(e)),
            None => {}
        }
        if list.is_empty() {
            let saves = backend::game_home().join("saves");
            return body.child(
                ui::empty()
                    .flex()
                    .gap(px(4.))
                    .child(tr(
                        "No worlds yet. Make one in the game and it shows up here.",
                    ))
                    .child(ui::link("worlds-folder", tr("Open folder")).on_click(
                        move |_, _, cx| {
                            let _ = std::fs::create_dir_all(&saves);
                            crate::glass_toast::open(cx, &saves);
                        },
                    )),
            );
        }
        // Pictures line up: a world without one gets a plain tile when another has one.
        let pictures = list.iter().any(|w| w.icon.is_some());
        // A long saves folder: only the rows near the view are built (`cull`). The world being
        // worked on and the one whose ⋯ menu is open stay built, wherever the page is scrolled.
        let kept: Vec<bool> = list
            .iter()
            .map(|w| {
                let key = format!("world-{}", w.folder);
                self.worlds.busy.as_ref() == Some(&w.folder)
                    || self.worlds.confirm_delete.as_ref() == Some(&w.folder)
                    || self.list_menu.as_ref().is_some_and(|(k, _)| **k == *key)
            })
            .collect();
        let (cull, scroll) = (self.worlds.cull.clone(), self.content_scroll.handle.clone());
        let rows = cull.column(
            &scroll,
            list.len(),
            0.,
            |i| kept[i],
            |i| self.glass_world_row(&list[i], pictures, cx),
        );
        body.child(rows)
    }

    fn glass_world_row(&mut self, w: &World, pictures: bool, cx: &mut Context<Self>) -> Div {
        let t = self.theme;
        let key = SharedString::from(format!("world-{}", w.folder));
        let hovered = self.list_hover.as_ref() == Some(&key);
        let menu_open = self.list_menu.as_ref().is_some_and(|(k, _)| *k == key);
        let busy = self.worlds.busy.is_some();
        let this_busy = self.worlds.busy.as_ref() == Some(&w.folder);
        let can_play = self.can_start() && !busy;
        let mut facts = vec![tr(w.mode).to_string()];
        facts.extend(w.version.clone());
        facts.push(glass_home::last_played(w.last_played));
        let hover = self.list_hover_listener(key.clone(), cx);
        let mut row = ui::list_row(key.clone(), hovered || menu_open, hover, &t);
        if can_play {
            let folder = w.folder.clone();
            row = row
                .cursor_pointer()
                .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                    this.quick_play("--world", folder.clone());
                    cx.notify();
                }));
        }
        if pictures {
            row = row.child(picture(
                w.icon.clone().map(gpui::ImageSource::from),
                "earth",
            ));
        }
        let size = if this_busy {
            ui::text(13., FontWeight::NORMAL, ink::QUIET).child(tr("Working…"))
        } else if hovered && can_play {
            div()
                .flex()
                .items_center()
                .gap(px(4.))
                .child(ui::text(13., FontWeight::MEDIUM, ink::FULL).child(tr("Play")))
                .child(ui::icon("arrow-right-01", 15., white(ink::FULL)))
        } else {
            ui::text(13., FontWeight::NORMAL, ink::QUIET)
                .child(horizon_launcher::worlds::format_bytes(w.size))
        };
        let more = ui::icon_button(
            SharedString::from(format!("{key}-more")),
            "more-horizontal",
            36.,
            17.,
            radius::CONTROL,
            menu_open,
            &t,
        )
        .tooltip(ui::tip(tr("More for this world")))
        .on_click(cx.listener({
            let key = key.clone();
            move |this, event: &ClickEvent, _, cx| {
                if !this.closed_by_this_press(event) {
                    this.list_menu = match this.list_menu.take() {
                        Some((k, _)) if k == key => None,
                        _ => Some((key.clone(), false)),
                    };
                }
                cx.stop_propagation();
                cx.notify();
            }
        }));
        // Under the pointer only, as the mockup's row actions: the size keeps the row's end.
        let mut more = div().relative().child(more);
        if menu_open {
            more = more.child(world_menu_overlay(self.world_menu(w, cx)));
        }
        div().child(
            row.child(ui::list_text(w.name.clone(), facts.join(" · ")))
                .child(size.relative())
                .when(hovered || menu_open, |r| r.child(more)),
        )
    }

    /// A world's ⋯ menu: Back up, Copy, Open folder, then Delete… (asked twice).
    fn world_menu(&mut self, w: &World, cx: &mut Context<Self>) -> Stateful<Div> {
        let t = self.theme;
        let confirm = self.list_menu.as_ref().is_some_and(|(_, c)| *c);
        let act = |this: &mut Self, action: Action, w: &World, cx: &mut Context<Self>| {
            this.list_menu = None;
            this.world_action(action, w.clone(), cx);
            cx.notify();
        };
        let (backup, copy, gone) = (w.clone(), w.clone(), w.clone());
        let folder = w.path.clone();
        let delete = ui::menu_row(
            "world-menu-delete",
            "delete-02",
            if confirm {
                tr("Click again to delete")
            } else {
                tr("Delete…")
            },
            true,
            &t,
        )
        .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
            match this.list_menu.as_mut() {
                Some((_, confirm)) if !*confirm => *confirm = true,
                _ => act(this, Action::Delete, &gone, cx),
            }
            cx.notify();
        }));
        ui::popover(230.)
            .id("world-menu")
            .occlude()
            .on_mouse_down_out(cx.listener(|this, event: &MouseDownEvent, _, cx| {
                this.list_menu = None;
                this.menu_closed_at = Some(event.position);
                cx.notify();
            }))
            .child(
                ui::menu_row("world-menu-backup", "download-04", tr("Back up"), false, &t)
                    .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                        act(this, Action::Backup, &backup, cx)
                    })),
            )
            .child(
                ui::menu_row("world-menu-copy", "copy-01", tr("Copy"), false, &t).on_click(
                    cx.listener(move |this, _: &ClickEvent, _, cx| {
                        act(this, Action::Copy, &copy, cx)
                    }),
                ),
            )
            .child(
                ui::menu_row(
                    "world-menu-folder",
                    "folder-01",
                    tr("Open folder"),
                    false,
                    &t,
                )
                .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                    this.list_menu = None;
                    crate::glass_toast::open(cx, &folder);
                    cx.notify();
                })),
            )
            .child(div().mt(px(4.)).child(delete))
    }

    /// Home's Multiplayer tab: the servers the club offers, each with who is on and Join.
    pub(super) fn glass_multiplayer(&mut self, cx: &mut Context<Self>) -> Div {
        self.load_servers(cx);
        let list = self.multiplayer.list.clone();
        let mut body = div().flex().flex_col();
        if list.is_empty() {
            return body.child(ui::empty().flex().gap(px(4.)).child(tr(
                "No servers to join: the club's file club.toml lists none, and no server is \
                     hosted right now.",
            )));
        }
        let count = list.len().to_string();
        let note = match list.len() {
            1 => tr("1 server").to_string(),
            _ => trf("{count} servers", &[("count", &count)]),
        };
        body = body.child(ui::list_head(note));
        if let Some(e) = self.multiplayer.error.clone() {
            body = body.child(danger_line(e));
        }
        let pictures = list.iter().any(|(_, icon)| icon.is_some());
        for (i, (server, icon)) in list.into_iter().enumerate() {
            body = body.child(self.glass_server_row(i, server, icon, pictures, cx));
        }
        body
    }

    fn glass_server_row(
        &mut self,
        i: usize,
        server: horizon_launcher::server_list::Server,
        icon: Option<std::sync::Arc<gpui::Image>>,
        pictures: bool,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let t = self.theme;
        let key = SharedString::from(format!("server-{i}"));
        let ip = server.ip.clone();
        // The mockup's dot and "address · 3 of 10", or "address · Offline" with Join off.
        let reach = self
            .multiplayer
            .reach
            .get(&server.ip)
            .copied()
            .unwrap_or(multiplayer_page::Reach::Asking);
        let tone = match reach {
            multiplayer_page::Reach::Online { .. } => ui::Tone::Live,
            _ => ui::Tone::Off,
        };
        let line = multiplayer_page::status(&server, reach);
        let offline = reach == multiplayer_page::Reach::Offline;
        let join = ui::quiet(
            SharedString::from(format!("{key}-join")),
            tr("Join"),
            36.,
            &t,
        );
        // A dimmed Join says why it does nothing.
        let join = if offline {
            join.opacity(0.4)
                .cursor_default()
                .tooltip(ui::tip(tr("This server doesn't answer right now")))
        } else if self.can_start() {
            join.on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                this.quick_play("--server", ip.clone());
                cx.notify();
            }))
        } else {
            join.opacity(0.4)
                .cursor_default()
                .tooltip(ui::tip(tr("Wait for the game to finish starting")))
        };
        ui::list_row(key, false, |_, _, _| {}, &t)
            .child(div().relative().flex_none().child(ui::dot(tone)))
            .when(pictures, |r| {
                r.child(picture(icon.map(gpui::ImageSource::from), "globe-02"))
            })
            .child(ui::list_text(server.name, line))
            .child(join.relative())
    }
}

fn world_menu_overlay(menu: impl IntoElement) -> impl IntoElement {
    // Offset only the downward placement so the flipped menu ends above the button.
    deferred(
        div().absolute().top_0().right_0().child(
            gpui::anchored()
                .anchor(gpui::Anchor::TopRight)
                .offset(gpui::point(px(0.), px(44.)))
                .child(menu),
        ),
    )
    .with_priority(1)
}

/// A row's 36 px picture, or a plain tile with `fallback`'s icon.
pub(super) fn picture(image: Option<gpui::ImageSource>, fallback: &'static str) -> AnyElement {
    match image {
        Some(image) => gpui::img(image)
            .relative()
            .flex_none()
            .size(px(PICTURE))
            .rounded(px(PICTURE_RADIUS))
            .into_any_element(),
        None => div()
            .relative()
            .flex()
            .flex_none()
            .items_center()
            .justify_center()
            .size(px(PICTURE))
            .rounded(px(PICTURE_RADIUS))
            .bg(white(fill::BUTTON))
            .child(ui::icon(fallback, 17., white(ink::QUIET)))
            .into_any_element(),
    }
}

/// Why an action failed, in the menu's Delete red, over the list.
pub(super) fn danger_line(message: String) -> Div {
    div().px(px(12.)).pb(px(8.)).child(
        ui::text(13., FontWeight::NORMAL, ink::FULL)
            .text_color(ui::danger())
            .child(message),
    )
}

#[cfg(all(test, debug_assertions))]
mod tests {
    use super::*;

    struct WorldRow {
        bottom: bool,
    }

    impl Render for WorldRow {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            let menu = div()
                .debug_selector(|| "menu".into())
                .w(px(230.))
                .h(px(200.));
            div().size_full().relative().child(
                div()
                    .absolute()
                    .right(px(16.))
                    .map(|row| {
                        if self.bottom {
                            row.bottom_0()
                        } else {
                            row.top_0()
                        }
                    })
                    .size(px(36.))
                    .debug_selector(|| "button".into())
                    .child(world_menu_overlay(menu)),
            )
        }
    }

    #[gpui::test]
    async fn world_menu_fits_at_both_ends_of_the_window(cx: &mut gpui::TestAppContext) {
        for bottom in [false, true] {
            let (_, window) = cx.add_window_view(|_, _| WorldRow { bottom });
            let menu = window.debug_bounds("menu").unwrap();
            let button = window.debug_bounds("button").unwrap();
            assert_eq!(menu.right(), button.right());
            if bottom {
                assert!(menu.top() >= px(0.) && menu.bottom() <= button.top());
            } else {
                assert!(menu.top() >= button.bottom());
            }
        }
    }
}
