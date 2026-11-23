//! Home's glass tabbed panel holds the game's worlds and servers, and its logs.
//! The glass look has no Overview tab: what Play needs is said above the tabs.
use super::*;
use crate::glass_ui::{self as ui, white};
use home_tabs::HomeTab;
use horizon_theme::glass::ink;

/// The tabbed panel's body is at least this tall (`min-h-[340px]`).
const BODY: f32 = 340.;

impl Launcher {
    /// The tabbed panel: the strip, then the open tab. With no tab of its own, the first one
    /// (Worlds) opens.
    pub(super) fn glass_home_tabs(&mut self, cx: &mut Context<Self>) -> Div {
        let t = self.theme;
        let tabs = self.home_tab_list(cx);
        let open = tabs
            .iter()
            .map(|(tab, _)| *tab)
            .find(|tab| *tab == self.home_tab)
            .unwrap_or(HomeTab::Worlds);
        // Icons beside the names only where every tab still has the room for both.
        let icons = self.window_width >= crate::glass_home::SIDE_BY_SIDE + 160.;
        let active = tabs.iter().position(|(tab, _)| *tab == open).unwrap_or(0);
        let items: Vec<_> = tabs
            .into_iter()
            .map(|(tab, count)| {
                let (name, icon) = match tab {
                    HomeTab::Worlds => (tr("Worlds"), "earth"),
                    HomeTab::Multiplayer => (tr("Multiplayer"), "user-multiple-02"),
                    HomeTab::Console => (tr("Log"), "computer-terminal-01"),
                };
                let on = tab == open;
                let ink_of = white(if on { ink::FULL } else { ink::BODY });
                ui::pill_tab(format!("home-tab-{tab:?}"), on, 38., &t)
                    // Each grows from its own width, so long names keep their room.
                    .flex_auto()
                    .min_w_0()
                    .px(px(8.))
                    .text_size(px(14.))
                    .line_height(px(20.))
                    .children(icons.then(|| ui::icon(icon, 16., ink_of).relative()))
                    .child(
                        div()
                            .relative()
                            .min_w_0()
                            .whitespace_nowrap()
                            .overflow_hidden()
                            .text_ellipsis()
                            .child(name),
                    )
                    .children(count.filter(|n| *n > 0).map(|n| {
                        div()
                            .relative()
                            .flex_none()
                            .text_color(white(ink::DIM))
                            .child(n.to_string())
                    }))
                    .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                        this.home_tab = tab;
                        cx.notify();
                    }))
            })
            .collect();
        let row = div()
            .flex()
            .items_center()
            .w_full()
            .min_w_0()
            .gap(px(2.))
            .px(px(8.))
            .pt(px(8.))
            .pb(px(4.))
            .child(crate::glass::measure(self.home_strip.clone()));
        let strip = ui::pill_strip("home-tabs", active, 38., 8., row, items).w_full();
        let body: Option<AnyElement> = match open {
            HomeTab::Worlds => Some(self.glass_worlds(cx).into_any_element()),
            HomeTab::Multiplayer => Some(self.glass_multiplayer(cx).into_any_element()),
            HomeTab::Console => self.glass_console(cx).map(IntoElement::into_any_element),
        };
        div()
            .relative()
            // `mt-12` under the head, of which Home's own gap gives 28.
            .mt(px(20.))
            .child(crate::glass::tabbed(self.home_strip.clone()))
            .child(
                div()
                    .relative()
                    .flex()
                    .w_full()
                    .min_w_0()
                    .overflow_hidden()
                    .child(strip),
            )
            .child(
                div().relative().p(px(8.)).child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(12.))
                        .min_h(px(BODY))
                        .px(px(4.))
                        .pt(px(8.))
                        .pb(px(4.))
                        .children(body),
                ),
            )
    }
}
