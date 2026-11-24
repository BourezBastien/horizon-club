//! Home's Multiplayer tab (`multiplayer_page.rs`): the servers the club offers, as the game's own
//! list holds them (`horizon_launcher::server_list`): those of `club.toml`, then the hosted servers
//! announced on the network. Each with who is on it, as the game's Multiplayer screen asks when it
//! opens, and Join: the game starts straight into it. A student adds none and removes none: the
//! list is written again from the club's file whenever the tab refreshes.
use super::*;
use gpui::{Image, ImageFormat};
use horizon_launcher::server_list::{self, Server};
use horizon_launcher::server_status::{self, TIMEOUT};
use horizon_launcher::{club, online};
use std::collections::HashMap;
use std::sync::Arc;

/// How long the tab's list stands before it is written again while the tab shows: the announced
/// servers come and go.
const REFRESH: Duration = Duration::from_secs(5);

/// A server as its status ping found it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum Reach {
    Asking,
    Online { players: u64, max: u64 },
    Offline,
}

#[derive(Default)]
pub(super) struct Multiplayer {
    /// The list as last written, with each picture the game last saw.
    pub(super) list: Vec<(Server, Option<Arc<Image>>)>,
    /// Written again on the next frame: the game ended, or the list could not be saved.
    fresh: bool,
    /// When the list was last written.
    written: Option<Instant>,
    /// What each address answered to the status ping, asked once per list.
    pub(super) reach: HashMap<String, Reach>,
    /// Why the list could not be saved, when it couldn't.
    pub(super) error: Option<String>,
}

impl Multiplayer {
    /// Write the list again on the next frame: the game may have written what it saw of the
    /// servers.
    pub(super) fn stale(&mut self) {
        self.fresh = false;
    }
}

/// A row's line under the server's name: its address, and who is on once it answered.
pub(super) fn status(server: &Server, reach: Reach) -> String {
    match reach {
        Reach::Online { players, max } => format!(
            "{} · {}",
            server.ip,
            trf(
                "{online} of {max}",
                &[("online", &players.to_string()), ("max", &max.to_string())]
            )
        ),
        Reach::Offline => format!("{} · {}", server.ip, tr("Offline")),
        Reach::Asking => server.ip.clone(),
    }
}

impl Launcher {
    /// Writes the game's server list again when it is stale or [`REFRESH`] old, and reads it back:
    /// the tab's rows and its count. While a game runs the list is the game's own: nothing is
    /// written. Whether what keeps the game from starting changed meanwhile (the club's file is
    /// read again with the list), for a page that reads it as it draws.
    pub(super) fn refresh_servers(&mut self) -> bool {
        if self.client.is_some() {
            return false;
        }
        let due = self
            .multiplayer
            .written
            .is_none_or(|at| at.elapsed() >= REFRESH);
        if self.multiplayer.fresh && !due {
            return false;
        }
        // The club's file is read again with the list, so a server the supervisor added shows
        // here without restarting the launcher.
        let changed = self.reload_club();
        let announced = online::list(&club::online_dir(), server_list::now());
        match server_list::write_club_list(
            &backend::game_home(),
            &self.config,
            &announced,
            &self.servers.remembered(),
        ) {
            Ok(servers) => {
                self.multiplayer.list = servers
                    .into_iter()
                    .map(|mut server| {
                        let icon = server
                            .icon
                            .take()
                            .map(|png| Arc::new(Image::from_bytes(ImageFormat::Png, png)));
                        (server, icon)
                    })
                    .collect();
                self.multiplayer.error = None;
            }
            // The list as last written stays on screen, and the line says why it is old.
            Err(error) => self.multiplayer.error = Some(error),
        }
        self.multiplayer.fresh = true;
        self.multiplayer.written = Some(Instant::now());
        // Each server is asked again for the new list.
        self.multiplayer.reach.clear();
        changed
    }

    /// The tab's list, and who is on each of its servers.
    pub(super) fn load_servers(&mut self, cx: &mut Context<Self>) {
        let changed = self.refresh_servers();
        self.ping_servers(cx);
        if changed {
            cx.notify();
        }
    }

    /// Asks each listed server for its players once per list, as the game's Multiplayer list does
    /// when it opens; the rows show the answer (a dot and "3 of 10", or "Offline").
    pub(super) fn ping_servers(&mut self, cx: &mut Context<Self>) {
        let asks: Vec<String> = self
            .multiplayer
            .list
            .iter()
            .map(|(server, _)| server.ip.clone())
            .filter(|ip| !self.multiplayer.reach.contains_key(ip))
            .collect();
        for ip in asks {
            self.multiplayer.reach.insert(ip.clone(), Reach::Asking);
            let address = ip.clone();
            let task = cx
                .background_executor()
                .spawn(async move { server_status::players(&address, TIMEOUT) });
            cx.spawn(async move |this, cx| {
                let answer = task.await;
                let _ = this.update(cx, |this, cx| {
                    let reach = match answer {
                        Ok(players) => Reach::Online {
                            players: players.online,
                            max: players.max,
                        },
                        Err(_) => Reach::Offline,
                    };
                    this.multiplayer.reach.insert(ip, reach);
                    cx.notify();
                });
            })
            .detach();
        }
    }

    /// Home's Multiplayer tab: its list, and nothing beside the tabs (a student adds no server).
    pub(super) fn multiplayer_section(&mut self, cx: &mut Context<Self>) -> (Div, Div) {
        let t = self.theme;
        self.load_servers(cx);
        let mut section = div().flex().flex_col().gap(px(8.));
        if let Some(e) = self.multiplayer.error.clone() {
            section = section.child(notice(e, &t));
        }
        let list = self.multiplayer.list.clone();
        if list.is_empty() {
            return (
                div(),
                section.child(text(theme::CAPTION).text_color(t.muted).child(tr(
                    "No servers to join: the club's file club.toml lists none, and no server is \
                     hosted right now.",
                ))),
            );
        }
        for (i, (server, icon)) in list.into_iter().enumerate() {
            section = section.child(self.server_row(i, server, icon, cx));
        }
        (div(), section)
    }

    fn server_row(
        &mut self,
        i: usize,
        server: Server,
        icon: Option<Arc<Image>>,
        cx: &mut Context<Self>,
    ) -> Div {
        let t = self.theme;
        let reach = self
            .multiplayer
            .reach
            .get(&server.ip)
            .copied()
            .unwrap_or(Reach::Asking);
        let offline = reach == Reach::Offline;
        let picture = div()
            .flex()
            .flex_none()
            .items_center()
            .justify_center()
            .size(px(48.))
            .border_2()
            .border_color(t.outline)
            .bg(t.well);
        let picture = match icon {
            Some(image) => picture.child(gpui::img(image).size(px(44.))),
            None => picture.child(blocks::icon("server", 22., t.muted)),
        };
        // Offline, Join does nothing: the game would only wait for a server that isn't there.
        let join = button(
            SharedString::from(format!("server-join-{i}")),
            "Join",
            Kind::Secondary,
            &t,
        );
        let join = if offline || !self.can_start() {
            disabled(join)
        } else {
            let ip = server.ip.clone();
            join.on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                // Remembered among the invited servers: the student finds it again once it stops.
                this.remember_joined(&ip);
                this.quick_play("--server", ip.clone());
                cx.notify();
            }))
        };
        plate(&t)
            .flex()
            .items_center()
            .gap(px(14.))
            .px(px(14.))
            .py(px(10.))
            .child(picture)
            .child(
                div()
                    .flex()
                    .flex_col()
                    .flex_1()
                    .min_w_0()
                    .child(
                        blocks::pixel(15., FontWeight::MEDIUM)
                            .text_color(t.ink)
                            .truncate()
                            .child(server.name.clone()),
                    )
                    .child(
                        text(theme::CAPTION)
                            .text_color(if offline { t.muted } else { t.success })
                            .truncate()
                            .child(status(&server, reach)),
                    ),
            )
            .child(div().flex().items_center().child(join))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A row says the address alone until the server answers, then who is on, or that nobody
    /// does.
    #[test]
    fn a_row_shows_who_is_on_the_server_once_it_answered() {
        let server = Server::new("Serveur du club", "mc-club:25565");
        assert_eq!(status(&server, Reach::Asking), "mc-club:25565");
        assert_eq!(
            status(
                &server,
                Reach::Online {
                    players: 3,
                    max: 10
                }
            ),
            "mc-club:25565 · 3 of 10"
        );
        assert_eq!(status(&server, Reach::Offline), "mc-club:25565 · Offline");
    }
}
