//! Home's tabs under its head (docs/design-system.md, "Night"; plan R0, screen 5): the game's
//! worlds, its servers and, once the game has said something, its console. One at a time,
//! instead of one long page under the head. What Play needs is said above the tabs.
use super::*;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) enum HomeTab {
    #[default]
    Worlds,
    Multiplayer,
    Console,
}

impl HomeTab {
    /// `HORIZON_LAUNCHER_PAGE=worlds` (or `multiplayer`, `console`) opens Home on that tab, for
    /// screenshots.
    pub(super) fn from_env() -> Self {
        match std::env::var("HORIZON_LAUNCHER_PAGE").as_deref() {
            Ok("multiplayer") => HomeTab::Multiplayer,
            Ok("console") => HomeTab::Console,
            _ => HomeTab::Worlds,
        }
    }
}

impl Launcher {
    /// The game's tabs, with how many things each holds (None: not known yet, or nothing to
    /// count). The glass look shows every tab (an empty one says so); the blocky look shows the
    /// Console only once there is something in it.
    pub(super) fn home_tab_list(
        &mut self,
        cx: &mut Context<Self>,
    ) -> Vec<(HomeTab, Option<usize>)> {
        let mut tabs = vec![(HomeTab::Worlds, self.worlds_count(cx))];
        // The tab's count is the list as last written. Reading the club's file again with it may
        // change what Home says above the tabs: the frame is drawn again then.
        if self.refresh_servers() {
            cx.notify();
        }
        tabs.push((HomeTab::Multiplayer, Some(self.multiplayer.list.len())));
        if self.theme.glass || !self.console_log.is_empty() || self.client.is_some() {
            tabs.push((HomeTab::Console, None));
        }
        tabs
    }

    /// The tab strip and the open tab's content.
    pub(super) fn home_tabs(&mut self, cx: &mut Context<Self>) -> Div {
        let t = self.theme;
        let tabs = self.home_tab_list(cx);
        // A tab that went away (the Console before anything is logged) falls back to Worlds.
        let open = tabs
            .iter()
            .map(|(tab, _)| *tab)
            .find(|tab| *tab == self.home_tab)
            .unwrap_or(HomeTab::Worlds);
        let strip = div()
            .flex()
            .flex_wrap()
            .gap(px(4.))
            .children(tabs.into_iter().map(|(tab, count)| {
                let name = match tab {
                    HomeTab::Worlds => tr("Worlds"),
                    HomeTab::Multiplayer => tr("Multiplayer"),
                    HomeTab::Console => tr("Console"),
                };
                let label = match count {
                    Some(n) if n > 0 => format!("{name} {n}"),
                    _ => name.to_string(),
                };
                shell::tab(format!("home-tab-{tab:?}"), &label, tab == open, &t).on_click(
                    cx.listener(move |this, _: &ClickEvent, _, cx| {
                        this.home_tab = tab;
                        cx.notify();
                    }),
                )
            }));
        // The open tab's own buttons sit at the strip's end, as a list's header would.
        let (actions, body): (Option<Div>, Option<AnyElement>) = match open {
            HomeTab::Worlds => (None, Some(self.worlds_section(cx).into_any_element())),
            HomeTab::Multiplayer => {
                let (actions, section) = self.multiplayer_section(cx);
                (Some(actions), Some(section.into_any_element()))
            }
            HomeTab::Console => match self.console_section(cx) {
                Some((actions, section)) => (Some(actions), Some(section.into_any_element())),
                None => (None, None),
            },
        };
        div()
            .flex()
            .flex_col()
            .gap(px(20.))
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap(px(16.))
                    .child(strip)
                    .children(actions),
            )
            .children(body)
    }
}
