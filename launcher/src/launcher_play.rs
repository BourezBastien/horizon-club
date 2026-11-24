//! Starting the game (docs/school-edition/launcher-target.md, sections 4 to 6 and 8): the
//! club's file is read again, then everything that keeps the game from starting is said on Home;
//! when nothing does, the game's server list and a first-time student's options are written, and
//! the game starts.
use super::*;
use horizon_launcher::{backend, club, config, first_play, identity, online, server_list};

/// How the rail says, in a few words, that the Windows session can't play.
fn rail(error: identity::IdentityError) -> &'static str {
    match error {
        identity::IdentityError::TooShort => "No player name",
        identity::IdentityError::Shared => "Shared session",
    }
}

impl Launcher {
    /// After a native crash with no report (`backend::native_crash_exit`): when the game's output
    /// ends while its Vulkan or OpenGL renderer started, a crash in the graphics driver
    /// (arismoko/horizon#746, #18291), switches the game to `backend::start_crash_fallback`
    /// (DirectX 12 on Windows, OpenGL after Vulkan on Linux; only where the game's home left the
    /// graphics API on Auto, `backend::switch_from_auto`). The crashed API's label and the new
    /// one's when it did.
    pub(crate) fn switched_after_start_crash(&self) -> Option<(&'static str, &'static str)> {
        let crashed = backend::renderer_start_crash(&self.console_log)?;
        let (api, label) = backend::start_crash_fallback(crashed, std::env::consts::OS)?;
        match backend::switch_from_auto(&backend::game_home(), api) {
            Ok(switched) => switched.then_some((crashed.label(), label)),
            Err(e) => {
                eprintln!("Horizon launcher: could not switch to {label}: {e}");
                None
            }
        }
    }

    /// The build the club plays ([`config::Versions::build`]): the one its file names, else the
    /// only build in `core\`, else the newest folder name, among those last listed.
    pub(crate) fn selected(&self) -> Option<&str> {
        self.config.versions.build(&self.installed)
    }

    /// Reads the club's file and the builds of `core\` again, then what keeps the game from
    /// starting, if anything: when the window opens, before every Play, when a game ends, and
    /// every [`RECHECK`] while something does. So what the supervisor fixed while the window was
    /// open shows without restarting the launcher. Whether what keeps the game from starting
    /// changed, for a page that reads it as it draws.
    pub(crate) fn reload_club(&mut self) -> bool {
        self.checked = Some(Instant::now());
        match config::Config::load() {
            Ok(config) => {
                // The club's language is the window's, until a student picks one in Settings.
                i18n::set(self.settings.language(config.club.language));
                self.player = identity::player(&config.club.shared_accounts).ok();
                self.config = config;
                self.config_error = None;
            }
            Err(error) => {
                // What was read well last stands for the window: the club's name, its servers.
                self.config_error = Some(error.to_string());
            }
        }
        // The supervisor may have dropped a build in, or taken one away, meanwhile.
        self.installed = backend::installed();
        // The Servers page exists only when the club lets a student host: a page left open when
        // the club turned hosting off goes Home.
        if !self.config.hosting.enabled && self.page == Page::Servers {
            self.go(Page::Play);
        }
        let refusal = self.refusal();
        let changed = refusal != self.refusal;
        self.refusal = refusal;
        changed
    }

    /// What keeps the game from starting now, [`Self::play`]'s checks in their order: the club's
    /// file, who plays, the build and its files, the rule of the game. None when Play would start
    /// the game.
    fn refusal(&self) -> Option<Refusal> {
        let said = |said: String, rail: &str| {
            Some(Refusal {
                said,
                rail: tr(rail).to_string(),
            })
        };
        if let Some(error) = &self.config_error {
            return said(error.clone(), "The club's file could not be read");
        }
        if let Err(error) = identity::player(&self.config.club.shared_accounts) {
            return said(error.message(), rail(error));
        }
        let Some(tag) = self.selected() else {
            return said(self.no_build(), "No version installed");
        };
        // A page's answer, which stands for a few seconds: the club's files arrive complete, and
        // one that is not is the supervisor's to fix, never the launcher's. A start asks the disk
        // itself (`Self::play`).
        if !backend::is_prepared(tag) {
            return said(backend::not_prepared(tag), "Needs preparing");
        }
        if !club::managed_file().is_file() {
            return said(
                tr(
                    "The club's file managed.txt is missing, so the game can't start. Ask the \
                     club's supervisor for it.",
                )
                .to_string(),
                "The rule of the game is missing",
            );
        }
        None
    }

    /// What the window says when the club's folder holds no build the launcher can find.
    pub(crate) fn no_build(&self) -> String {
        if club::has_root() {
            backend::no_version_installed()
        } else {
            tr(
                "This launcher is not in the club's folder. Ask the club's supervisor where the \
                 club's folder is.",
            )
            .to_string()
        }
    }

    /// Play: the club's file and builds are read again, and the game starts when nothing keeps it
    /// from starting. Otherwise Home says what does, and nothing starts.
    pub(crate) fn play(&mut self) {
        self.reload_club();
        if self.refusal.is_some() || !self.can_start() {
            self.quick_play = None;
            return;
        }
        let (Some(tag), Some(player)) = (self.selected(), self.player.clone()) else {
            return;
        };
        // The build's files, asked of the disk now rather than of a page's answer that stands
        // for a few seconds: nothing starts on half of them.
        if let Err(why) = backend::need_prepared(tag) {
            self.refusal = Some(Refusal {
                said: why,
                rail: tr("Needs preparing").to_string(),
            });
            self.quick_play = None;
            return;
        }
        self.start_client(tag.to_owned(), &player);
    }

    /// Whether the game can start now: none is running or starting. A shortcut (a server's
    /// Join, a world's Play) starts it as Play does.
    pub(crate) fn can_start(&self) -> bool {
        self.client.is_none() && self.starting_client.is_none()
    }

    /// Starts the game: its server list and, at a student's first Play, their options are
    /// written, then a thread readies its command and `drain` starts the process. `player` is
    /// who plays: the refusal that says otherwise has been asked already.
    pub(crate) fn start_client(&mut self, tag: String, player: &str) {
        self.client_error = None;
        self.crash = None;
        self.console_log.clear();
        // A little earlier than the spawn: file times can be coarse.
        self.client_started =
            Some(std::time::SystemTime::now() - std::time::Duration::from_secs(2));
        self.starting_client = Some(tag.clone());
        self.play_pressed = Some(Instant::now());
        let home = backend::game_home();
        // The game's server list: the club's servers, then the hosted servers announced (section
        // 6). Neither write keeps the game from starting: Home says why it failed.
        let announced = online::list(&club::online_dir(), server_list::now());
        let servers =
            server_list::club_servers(&self.config, &announced, &self.servers.remembered());
        let mut failed = server_list::replace(&home, &servers).err();
        // A student's first Play: the options the club chose for them (section 8).
        if failed.is_none() {
            failed = first_play::write_missing(&home, &self.config.game, self.config.club.language)
                .err();
        }
        self.client_error = failed;
        // What follows the launcher's own arguments: where the next start goes at once.
        let mut cmd = backend::client_command(&tag, player, &club::managed_file());
        if let Some((flag, value)) = self.quick_play.take() {
            cmd.arg(flag).arg(value);
        }
        let tx = self.tx.clone();
        std::thread::spawn(move || {
            // One step at a time, on stderr and in the window.
            let say = |stderr: &str, shown: &str| play_step::say(&tx, stderr, shown.into());
            say("starting the game", tr("Starting the game…"));
            // Off the window's thread: on Windows the first Play of a build copies it.
            let _ = tx.send(Msg::GamePrepared(Box::new(backend::prepare_command(cmd))));
        });
    }
}
