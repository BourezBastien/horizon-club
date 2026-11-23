//! Quick play, and the blocky look's Versions page.
use super::*;

impl Launcher {
    /// Plays straight into a world (`--world`) or a server (`--server`): flags every Horizon
    /// build takes.
    pub(crate) fn quick_play(&mut self, flag: &'static str, value: String) {
        self.quick_play = Some((flag, value));
        self.play();
        if self.starting_client.is_none() {
            self.quick_play = None;
        }
    }

    pub(crate) fn versions_page(&mut self, cx: &mut Context<Self>) -> Div {
        let _ = cx;
        let t = self.theme;
        let header = page_header(
            "Versions",
            "The club's game builds. Ask the supervisor for a new one.",
            &t,
        );
        let page = div().flex().flex_col().gap(px(32.)).child(header);
        if t.glass {
            return self.glass_versions(cx);
        }
        let builds = &self.installed;
        if builds.is_empty() {
            return page.child(centered(&self.no_build(), &t));
        }
        let selected = self.selected();
        // A page of builds: only the rows near the screen are drawn.
        let keep = |_i: usize| false;
        let list = self.long_list.clone().column(
            &self.content_scroll.handle,
            builds.len(),
            8.,
            keep,
            |i| {
                let tag = builds[i].clone();
                let in_use = selected == Some(tag.as_str());
                let status = if in_use {
                    Some(caps(tr("In use")).text_color(t.accent))
                } else if i == 0 {
                    Some(caps(tr("Newest")).text_color(t.ink_soft))
                } else {
                    None
                };
                let id = SharedString::from(format!("build-{tag}"));
                let row = div()
                    .id(id.clone())
                    .relative()
                    .flex()
                    .flex_col()
                    .gap(px(10.))
                    .py(px(14.))
                    .px(px(16.))
                    .bg(t.panel)
                    .border_2()
                    .border_color(t.outline)
                    .children(blocks::edges(t.bevel, t.lip, theme::BEVEL))
                    .child(hover_layer(&id, t.raised, 0., 0.))
                    .child(
                        div()
                            .relative()
                            .flex()
                            .items_center()
                            .gap(px(16.))
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .flex_1()
                                    .min_w_0()
                                    .gap(px(4.))
                                    .child(version_tag(&tag).text_color(t.ink))
                                    .child(text(theme::UI).text_color(t.muted).truncate().child(
                                        trf(
                                            "Minecraft {version}",
                                            &[("version", &backend::game_version(&tag))],
                                        ),
                                    )),
                            )
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .items_end()
                                    .gap(px(2.))
                                    .flex_none()
                                    .children(status),
                            ),
                    );
                track_hover(row, id)
            },
        );
        page.child(list)
    }
}
