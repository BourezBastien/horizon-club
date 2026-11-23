//! The launcher's update in the window (`horizon_launcher::update`). A session that may change
//! the share looks for a newer release when the launcher starts; the window asks before the
//! share changes, and offers to start the launcher again once the new version is in place. A
//! student's session reads the share only: it asks the Internet for nothing, and is asked
//! nothing.
use super::*;
use horizon_launcher::club;
use horizon_launcher::update::{self, Release};
use std::process::Stdio;
use std::sync::mpsc::{channel, Receiver, Sender};

/// Where the update stands: nothing, while there is nothing to say.
#[derive(Clone, Debug, PartialEq)]
pub(super) enum Step {
    /// A newer release exists: the player is asked whether to install it.
    Asked(Release),
    /// Being downloaded and put on the share.
    Underway(Release),
    /// On the share: the launcher uses it from its next start.
    Installed(String),
    /// What went wrong.
    Failed(String),
}

/// The update: what the window shows, and what its thread tells it.
pub(super) struct Update {
    pub step: Option<Step>,
    tx: Sender<Step>,
    rx: Receiver<Step>,
}

impl Update {
    pub(super) fn new() -> Self {
        let (tx, rx) = channel();
        Self { step: None, tx, rx }
    }

    /// Looks for a newer release than the one running, once, on a thread. A session that may not
    /// write in the share's `app\` (a student's) looks for nothing; nor does a launcher started
    /// away from the club's folder. Nothing found, or no network: the window says nothing.
    pub(super) fn begin(&self, root: &std::path::Path) {
        if root.as_os_str().is_empty() || !update::may_update(root) {
            return;
        }
        let tx = self.tx.clone();
        std::thread::spawn(move || {
            if let Ok(Some(release)) = update::check(horizon_launcher::backend::VERSION) {
                let _ = tx.send(Step::Asked(release));
            }
        });
    }

    /// Takes what the thread said since the time before: whether it said anything, so the window
    /// that asks every 80 ms (`Launcher::drain`) draws it at once, and not at the pointer's next
    /// move.
    pub(super) fn poll(&mut self) -> bool {
        let mut heard = false;
        while let Ok(step) = self.rx.try_recv() {
            self.step = Some(step);
            heard = true;
        }
        heard
    }
}

/// Starts the club's bootstrap (`HorizonClub.exe` in the club's folder), which starts the
/// launcher's version in use. It is found by its name, its folder first in the search path, as
/// the bootstrap starts the launcher.
fn start_again(root: &std::path::Path) -> std::io::Result<()> {
    let inherited = std::env::var_os("PATH").unwrap_or_default();
    let search = std::env::join_paths(
        std::iter::once(root.to_path_buf()).chain(std::env::split_paths(&inherited)),
    )
    .unwrap_or(inherited);
    let mut command = horizon_launcher::process::command("HorizonClub.exe");
    command
        .current_dir(root)
        .env("PATH", search)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    command.spawn().map(|_| ())
}

impl Launcher {
    /// The update's card over the page: a release to agree to, one under way, what became of it.
    pub(super) fn update_dialog(&self, cx: &mut Context<Self>) -> Option<Stateful<Div>> {
        let step = self.update.step.clone()?;
        let t = self.theme;
        let later = |id: &'static str, label: &'static str, kind: Kind| {
            button(id, label, kind, &t).on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                this.update.step = None;
                cx.notify();
            }))
        };
        let (title, about, actions): (String, String, Vec<AnyElement>) = match &step {
            Step::Asked(release) => {
                let release = release.clone();
                (
                    trf(
                        "Version {version} of the launcher is available",
                        &[("version", &release.version)],
                    )
                    .to_string(),
                    tr("It is installed once, on the club's share: every PC uses it the next time it starts. The version in use stays there, to go back.")
                        .to_string(),
                    vec![
                        later("update-later", "Later", Kind::Secondary).into_any_element(),
                        button("update-go", "Update", Kind::Primary, &t)
                            .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                                this.update_now(release.clone());
                                cx.notify();
                            }))
                            .into_any_element(),
                    ],
                )
            }
            Step::Underway(release) => (
                trf(
                    "Downloading version {version}…",
                    &[("version", &release.version)],
                )
                .to_string(),
                tr("A few seconds, by the network's speed.").to_string(),
                Vec::new(),
            ),
            Step::Installed(version) => (
                trf("Version {version} is installed.", &[("version", version)]).to_string(),
                tr("The launcher uses it each time it starts from now on. Starting it again now stops the servers this window hosts: start them again after.")
                    .to_string(),
                vec![
                    later("update-stay", "Keep this window", Kind::Secondary).into_any_element(),
                    button("update-restart", "Start it now", Kind::Primary, &t)
                        .on_click(cx.listener(|this, _: &ClickEvent, _, cx| this.start_again(cx)))
                        .into_any_element(),
                ],
            ),
            Step::Failed(message) => (
                tr("The update could not be made.").to_string(),
                message.clone(),
                vec![later("update-close", "Close", Kind::Secondary).into_any_element()],
            ),
        };
        let failed = matches!(step, Step::Failed(_));
        // Each look's own type: the glass look's plain text, the others' pixel face.
        let heading = |line: String| {
            if t.glass {
                glass_ui::text(22., FontWeight::SEMIBOLD, horizon_theme::glass::ink::FULL)
                    .child(line)
                    .into_any_element()
            } else {
                text(theme::HEADING)
                    .text_color(t.ink)
                    .child(line)
                    .into_any_element()
            }
        };
        let note = |line: String| {
            if t.glass {
                glass_ui::text(14., FontWeight::NORMAL, horizon_theme::glass::ink::BODY)
                    .line_height(px(21.))
                    .when(failed, |d| {
                        d.text_color(glass_ui::color(horizon_theme::glass::DANGER))
                    })
                    .child(line)
                    .into_any_element()
            } else {
                text(theme::CAPTION)
                    .text_color(if failed { t.danger } else { t.muted })
                    .child(line)
                    .into_any_element()
            }
        };
        let (title, about) = (heading(title), note(about));
        let mut card = if t.glass {
            glass_ui::dialog_plate(horizon_theme::glass::radius::MODAL)
                .flex()
                .flex_col()
                .gap(px(10.))
                .p(px(28.))
                .w_full()
                .max_w(px(520.))
                .child(title)
                .child(about)
        } else {
            plate(&t)
                .flex()
                .flex_col()
                .gap(px(8.))
                .p(px(28.))
                .w_full()
                .max_w(px(560.))
                .bg(t.page)
                .border_1()
                .border_color(t.rule)
                .child(title)
                .child(about)
        };
        if !actions.is_empty() {
            card = card.child(
                div()
                    .flex()
                    .items_center()
                    .justify_end()
                    .gap(px(12.))
                    .mt(px(12.))
                    .children(actions),
            );
        }
        Some(if t.glass {
            div()
                .id("update-dialog")
                .absolute()
                .top_0()
                .left_0()
                .size_full()
                .flex()
                .items_start()
                .justify_center()
                .p(px(24.))
                .occlude()
                .on_scroll(|_, _, cx| cx.stop_propagation())
                .child(glass_ui::scrim_backdrop())
                .child(
                    div()
                        .absolute()
                        .top_0()
                        .left_0()
                        .size_full()
                        .bg(glass_ui::scrim()),
                )
                .child(card)
        } else {
            div()
                .id("update-dialog")
                .absolute()
                .top_0()
                .left_0()
                .size_full()
                .flex()
                .flex_col()
                .items_center()
                .px(px(32.))
                .py(px(24.))
                .bg(t.page.opacity(0.92))
                .on_scroll(|_, _, cx| cx.stop_propagation())
                .occlude()
                .child(card)
        })
    }

    /// The player agreed: the release goes onto the share, on a thread.
    fn update_now(&mut self, release: Release) {
        self.update.step = Some(Step::Underway(release.clone()));
        let (root, tx) = (club::root(), self.update.tx.clone());
        std::thread::spawn(move || {
            let step = match update::install(&root, &release) {
                Ok(()) => Step::Installed(release.version),
                Err(e) => Step::Failed(e),
            };
            let _ = tx.send(step);
        });
    }

    /// Starts the launcher again on the new version, and ends this one as closing its window
    /// does (the servers this window hosts stop with it).
    fn start_again(&mut self, cx: &mut Context<Self>) {
        match start_again(&club::root()) {
            Ok(()) => shut_down(cx),
            Err(e) => {
                self.update.step = Some(Step::Failed(
                    trf(
                        "The launcher could not start again: {error}",
                        &[("error", &e.to_string())],
                    )
                    .to_string(),
                ));
                cx.notify();
            }
        }
    }
}
