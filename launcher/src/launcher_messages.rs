//! The backend's messages, drained every frame (`Launcher::drain`).
use super::*;

impl Launcher {
    pub(crate) fn drain(&mut self, cx: &mut Context<Self>) {
        let mut changed = false;
        self.perf.tick();
        match self.bench.as_mut().map(|b| b.tick(&self.perf)) {
            Some(perf::Step::Go(page)) => {
                let pages: Vec<Page> = Page::ALL.iter().map(|p| p.0).collect();
                self.go(pages[page % pages.len()]);
                changed = true;
            }
            Some(perf::Step::Scroll) => {
                self.go(Page::Versions);
                changed = true;
            }
            Some(perf::Step::Quit) => cx.quit(),
            Some(perf::Step::Wait) | None => {}
        }
        while let Ok(msg) = self.rx.try_recv() {
            changed = true;
            match msg {
                Msg::PlayStep(step) => {
                    if let Some(line) = self.play_step.said(self.starting_client.is_some(), step) {
                        console_page::keep_line(
                            &mut self.console_log,
                            &mut self.log_revision,
                            line,
                        );
                    }
                }
                Msg::GamePrepared(prepared) => {
                    self.starting_client = None;
                    match *prepared {
                        Ok(staged) => match backend::spawn(staged, self.tx.clone()) {
                            Ok(r) => {
                                self.client = Some(r);
                                self.step_aside(cx);
                            }
                            Err(e) => self.client_error = Some(e),
                        },
                        Err(e) => self.client_error = Some(e),
                    }
                }
                // Old versions log their session; it never reaches the window.
                Msg::Line(line) if line.contains("Session ID is") => {}
                Msg::Line(line) => {
                    if line.starts_with(perf::GAME_FIRST_FRAME) {
                        if let Some(pressed) = self.play_pressed.take() {
                            self.perf
                                .record_since("launcher.play_to_game_frame_ms", pressed);
                        }
                    }
                    console_page::keep_line(&mut self.console_log, &mut self.log_revision, line);
                }
                Msg::Exit(result) => {
                    let error = result.err();
                    // The exit thread reaped the game; its child keeps the status.
                    let client = self.client.take();
                    let native_crash = client
                        .and_then(|client| client.child.lock().ok()?.try_wait().ok()?)
                        .is_some_and(backend::native_crash_exit);
                    self.come_back(cx);
                    // The game saved: last played, sizes and pictures changed, and its list may
                    // have written what it saw of the servers.
                    self.worlds.stale();
                    self.multiplayer.stale();
                    // The supervisor may have changed the club's files while the game ran.
                    self.reload_club();
                    // The run's own crash report: one written since the game started.
                    let since = self.client_started.take();
                    let report = since.and_then(|since| {
                        backend::crash_report_since(&backend::game_home(), since)
                    });
                    let switched = if native_crash && report.is_none() {
                        self.switched_after_start_crash()
                    } else {
                        None
                    };
                    if let Some((crashed, api)) = switched {
                        self.client_error = Some(trf(
                            "Horizon's {crashed} renderer crashed while starting on this \
                             graphics card, so the game now uses {api}. Press Play to \
                             start it again. Video Settings > Graphics API switches back.",
                            &[("crashed", crashed), ("api", api)],
                        ));
                    } else if error.is_some() || report.is_some() {
                        let summary = report
                            .as_ref()
                            .and_then(|p| std::fs::read_to_string(p).ok())
                            .and_then(|text| backend::crash_summary(&text))
                            .or_else(|| {
                                let tail =
                                    self.console_log.iter().rev().find(|l| !l.trim().is_empty());
                                // How it ended, then the last thing it said.
                                error
                                    .as_ref()
                                    .map(|e| tail.map_or(e.clone(), |t| format!("{e}: {t}")))
                            })
                            .unwrap_or_default();
                        self.crash = Some(Crash {
                            report,
                            summary,
                            copied: false,
                        });
                    }
                }
            }
        }
        // A start that ended (the game started, or it failed) is on no step.
        self.play_step.settle(self.starting_client.is_some());
        changed |= self.servers_tick();
        changed |= self.toast_tick();
        // While Home says the game can't start, the club's file and builds are read again now
        // and then: what the supervisor fixed shows without a restart.
        if self.refusal.is_some() && self.checked.is_some_and(|at| at.elapsed() >= RECHECK) {
            changed |= self.reload_club();
        }
        if changed {
            cx.notify();
        }
    }
}
