//! The Versions page in the glass look: the club's game builds, one glass card each — its name,
//! the Minecraft version it plays, and which one the club uses. The supervisor drops builds into
//! the club's `core` folder whole; the launcher lists them and installs nothing.

use super::*;
use crate::glass_ui::{self as ui, Tone};
use horizon_theme::glass::{ink, radius};

/// A date ("2026-11-07") as the window writes it ("Nov 7"), with its time in UTC when known.
pub(super) fn published(date: &str, time: &str) -> String {
    const MONTHS: [&str; 12] = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];
    let mut parts = date.split('-');
    let month = parts
        .nth(1)
        .and_then(|m| m.parse::<usize>().ok())
        .and_then(|m| MONTHS.get(m.checked_sub(1)?));
    let day = parts.next().and_then(|d| d.trim().parse::<u8>().ok());
    match (month, day) {
        (Some(month), Some(day)) if time.is_empty() => trf(
            "{month} {day}",
            &[("month", month), ("day", &day.to_string())],
        ),
        (Some(month), Some(day)) => trf(
            "{month} {day}, {time} UTC",
            &[("month", month), ("day", &day.to_string()), ("time", time)],
        ),
        _ => date.to_string(),
    }
}

/// A version's state beside its name: a dot and a word, 13 px at 70 % (`In use`).
fn badge(tone: Tone, label: &str) -> Div {
    div()
        .flex()
        .flex_none()
        .items_center()
        .gap(px(6.))
        .child(ui::dot(tone))
        .child(ui::text(13., FontWeight::NORMAL, ink::BODY).child(tr(label).to_string()))
}

impl Launcher {
    /// The glass Versions page.
    pub(super) fn glass_versions(&self, cx: &mut Context<Self>) -> Div {
        let _ = cx;
        let look = ui::look();
        let page_title = ui::page_head(
            tr("Versions"),
            tr("The club's game builds, newest first. Ask the supervisor for a new one."),
        )
        .mb(px(28.));
        let page = div().flex().flex_col().child(page_title);
        let builds = &self.installed;
        if builds.is_empty() {
            return page.child(self.no_builds_card());
        }
        let selected = self.selected();
        let mut list = div().flex().flex_col().gap(px(10.));
        for (i, tag) in builds.iter().enumerate() {
            let in_use = selected == Some(tag.as_str());
            let line = trf(
                "Minecraft {version}",
                &[("version", &backend::game_version(tag))],
            );
            let card = crate::glass::plate(radius::PANEL)
                .flex()
                .flex_col()
                .gap(px(12.))
                .p(px(20.))
                .child(
                    div().relative().flex().items_center().gap(px(16.)).child(
                        div()
                            .flex()
                            .flex_col()
                            .flex_1()
                            .min_w_0()
                            .gap(px(2.))
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(10.))
                                    .child(
                                        ui::text(16., FontWeight::SEMIBOLD, ink::FULL)
                                            .line_height(px(24.))
                                            .truncate()
                                            .child(tag.clone()),
                                    )
                                    .when(in_use, |d| d.child(badge(Tone::Live, "In use")))
                                    .when(i == 0 && !in_use, |d| {
                                        d.child(badge(Tone::Accent(look.accent_soft), "Newest"))
                                    }),
                            )
                            .child(
                                ui::text(13., FontWeight::NORMAL, 0.6)
                                    .line_height(px(19.5))
                                    .truncate()
                                    .child(line),
                            ),
                    ),
                );
            list = list.child(card);
        }
        page.child(list)
    }

    /// What the page says when the club's root holds no build: the supervisor's sentence.
    fn no_builds_card(&self) -> Div {
        crate::glass::plate(radius::PANEL)
            .flex()
            .flex_col()
            .items_start()
            .gap(px(12.))
            .p(px(20.))
            .child(ui::text(14., FontWeight::NORMAL, ink::NOTE).child(self.no_build()))
    }
}
