//! Home in the night look (docs/design-system.md, "Night"): the club's name over the sky with
//! what it plays and whether it is ready, and Play and the actions beside it.
use super::*;
use crate::shell::{action, c, label, Action};
use theme::night;

impl Launcher {
    /// The focal point: what will start and its state, the club's name. `tag` and `game`: the
    /// build and the Minecraft version it plays.
    pub(super) fn hero(&mut self, tag: &str, game: &str) -> Div {
        let t = self.theme;
        let n = t.night;
        let refused = self.refusal.as_ref().map(|refusal| refusal.rail.clone());
        let icon = self.horizon_mark(56.);
        let mut what = vec![
            format!("Minecraft: Java Edition {game}"),
            format!("Horizon {tag}"),
        ];
        what.push(refused.clone().unwrap_or_else(|| tr("Ready").to_string()));
        let state = div()
            .flex()
            .items_center()
            .gap(px(8.))
            .child(div().flex_none().size(px(6.)).bg(if refused.is_some() {
                t.warn
            } else {
                t.success
            }))
            .child(
                shell::text(night::CAPTION, c(n.ink_soft))
                    .font_weight(FontWeight::MEDIUM)
                    .flex()
                    .gap(px(4.))
                    .child(what.join(" · ")),
            );
        // Never squeezed by the rows below.
        div()
            .flex()
            .flex_none()
            .flex_col()
            .gap(px(10.))
            .child(state)
            .child(
                div().flex().items_center().gap(px(16.)).child(icon).child(
                    shell::text(night::DISPLAY, c(n.ink))
                        .min_w_0()
                        .truncate()
                        .child(self.config.club.name.clone()),
                ),
            )
    }

    /// Play, then what else the game offers: its folder.
    pub(super) fn home_actions(&self, cx: &mut Context<Self>) -> Div {
        let t = self.theme;
        let busy = !self.can_start();
        let mut play = action("play", Action::Main, &t)
            .min_w(px(180.))
            .child(
                div()
                    .relative()
                    .child(blocks::icon("play", 20., c(t.night.on_accent))),
            )
            .child(label(match self.play_step.now() {
                // The step a start is on, once it said one.
                Some(step) if self.starting_client.is_some() => step.to_owned(),
                _ => tr(if busy { "Running" } else { "Play" }).to_string(),
            }));
        // A game runs or starts: Play waits. Otherwise Play reads the club's file again, and
        // starts the game unless Home says what keeps it from starting.
        if busy {
            play = disabled(play);
        } else {
            play = play.on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                this.play();
                cx.notify();
            }));
        }
        let home = backend::game_home();
        div()
            .flex()
            .flex_wrap()
            .items_center()
            .gap(px(10.))
            .child(play)
            .child(
                action("home-folder", Action::Quiet, &t)
                    .child(label(tr("Folder").to_string()))
                    .on_click(move |_, _, cx| {
                        let _ = std::fs::create_dir_all(&home);
                        crate::glass_toast::open(cx, &home);
                    }),
            )
    }
}
