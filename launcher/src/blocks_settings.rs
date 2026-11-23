//! The blocky look's Settings page, and the rows both looks share (About). The page holds what a
//! student may change (docs/school-edition/launcher-target.md, section 10): the language and what
//! the window does while the game runs, the look, and what the launcher is. The club decides the
//! rest in `club.toml`.
use super::*;

impl Launcher {
    pub(crate) fn settings_page(&mut self, cx: &mut Context<Self>) -> Div {
        let t = self.theme;
        if t.glass {
            return self.glass_settings(cx);
        }
        let group = |label: &'static str, icon: &'static str| {
            section_title(label, icon, &t).mt(px(20.)).mb(px(12.))
        };
        div()
            .flex()
            .flex_col()
            .gap(px(32.))
            .child(page_header("Settings", "The launcher itself.", &t))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .child(group("General", "monitor").mt_0())
                    .child(self.language_row(cx))
                    .child(self.while_playing_row(0, cx))
                    // Glass and Blur shape the glass look's frosted panels alone
                    // (`glass_settings.rs`).
                    .child(group("Appearance", "image"))
                    .child(self.launcher_theme_row(cx))
                    .child(self.accent_row(cx))
                    .child(self.performance_row(cx))
                    .child(self.motion_row(cx))
                    .children(self.effects_rows(cx))
                    .child(self.home_background_row(cx))
                    .child(group("About", "info-box"))
                    .children(self.about_rows()),
            )
    }

    /// Settings' About: the launcher's version, the club, the logs, the licence, and the notice
    /// that Minecraft's maker asks for.
    pub(crate) fn about_rows(&self) -> Vec<Div> {
        let t = self.theme;
        let none = tr("none").to_string();
        let version = trf(
            "Launcher {launcher}, game {game}",
            &[
                ("launcher", backend::VERSION),
                ("game", self.selected().unwrap_or(&none)),
            ],
        );
        let copied = version.clone();
        let logs = backend::logs_dir();
        let said = |text: String| Some(div().child(text).into_any_element());
        vec![
            index_row(
                0,
                tr("Version"),
                said(version),
                Some(
                    button("copy-version", "Copy", Kind::Secondary, &t)
                        .on_click(move |_, _, cx| {
                            copy_to_clipboard(cx, gpui::ClipboardItem::new_string(copied.clone()))
                        })
                        .into_any_element(),
                ),
                &t,
            ),
            index_row(
                0,
                tr("Club"),
                said(self.config.club.name.clone()),
                None,
                &t,
            ),
            index_row(
                0,
                tr("Logs"),
                Some(
                    div()
                        .font_family(theme::MONO)
                        .child(format!(
                            "{}  ·  {}",
                            short(&logs),
                            tr("crash reports beside it")
                        ))
                        .into_any_element(),
                ),
                Some(
                    button("open-logs", "Open", Kind::Secondary, &t)
                        .on_click(move |_, _, cx| {
                            let dir = logs.clone();
                            let _ = std::fs::create_dir_all(&dir);
                            crate::glass_toast::open(cx, &dir);
                        })
                        .into_any_element(),
                ),
                &t,
            ),
            index_row(
                0,
                tr("Licence"),
                said(trf(
                    "Free software under the GNU Lesser General Public License, version 3 or \
                     later ({licence}).",
                    &[("licence", env!("CARGO_PKG_LICENSE"))],
                )),
                None,
                &t,
            ),
            index_row(
                0,
                tr("Minecraft"),
                said(tr(
                    "Not an official Minecraft product. Not approved by or associated with Mojang \
                     or Microsoft.",
                )
                .to_string()),
                None,
                &t,
            ),
        ]
    }
}
