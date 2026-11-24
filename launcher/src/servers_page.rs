//! The Servers page (docs/school-edition/launcher-target.md, section 7): create a server in one
//! dialog, then start, stop and watch it, manage who may join, change its settings and back it
//! up, all with buttons. Every action runs `horizon-server` of the build in use, in place from
//! the club's share, or calls the running server's control API
//! (`horizon_launcher::servers`). While a server this launcher started runs, it is announced in
//! the club's `online\` folder every ten seconds (`online.rs`), so the other students' launchers
//! list it in their Multiplayer tab.
use super::line_edit::{self, Key, LineEdit};
use super::*;
use horizon_launcher::heads;
use horizon_launcher::servers::{
    self, Applied, Backup, Entry, Fix, NewServer, Problem, Status, World,
};
use horizon_launcher::{club, friends, invited, online, server_list};
use horizon_theme::glass::radius;
use serde_json::Value;
use std::collections::{BTreeSet, HashMap};
use std::path::{Path, PathBuf};

#[path = "glass_server_dialog.rs"]
mod glass_server_dialog;
#[path = "glass_servers.rs"]
mod glass_servers;
use glass_servers::glass_setting_row;
#[path = "servers_format.rs"]
mod servers_format;
use servers_format::{backup_label, bytes, uptime};

/// The servers this launcher started in this session, stopped when it closes: a hosted server
/// lives as long as its launcher window (`stop_on_quit`).
static STARTED_HERE: Mutex<Vec<PathBuf>> = Mutex::new(Vec::new());

/// The player of this launcher, for `stop_on_quit`: who it said has the launcher open.
static ME: Mutex<Option<String>> = Mutex::new(None);

/// Whether the `online\` folder's refusal has been said in the launcher's own log: once is
/// enough, the folder is the supervisor's, not the student's, to fix. One word for its
/// announcement, one for who is here.
static ANNOUNCE_QUIET: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
static PEOPLE_QUIET: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// How often the open server's status and log are read, and the list's.
const POLL_OPEN: Duration = Duration::from_millis(1000);
const POLL_LIST: Duration = Duration::from_secs(3);

/// How often the announcement of a running hosted server is written again: three times over
/// before [`online::FRESH`] passes. Who has the launcher open is said as often.
const ANNOUNCE: Duration = Duration::from_secs(10);

/// Lines of the server's log kept on the page.
const CONSOLE_KEEP: usize = 1000;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub(super) enum Field {
    Name,
    Seed,
    Command,
    Motd,
    /// Home's Console's search (`console_page.rs`).
    ConsoleSearch,
    DeleteServer,
    /// Settings' accent as hex (`themes_page.rs`).
    AccentHex,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub(super) enum Tab {
    Overview,
    Players,
    Settings,
    Backups,
}

const TABS: [(Tab, &str); 4] = [
    (Tab::Overview, "Overview"),
    (Tab::Players, "Players"),
    (Tab::Settings, "Settings"),
    (Tab::Backups, "Backups"),
];

const MODES: [(&str, &str); 3] = [
    ("survival", "Survival"),
    ("creative", "Creative"),
    ("adventure", "Adventure"),
];

const DIFFICULTIES: [(&str, &str); 4] = [
    ("peaceful", "Peaceful"),
    ("easy", "Easy"),
    ("normal", "Normal"),
    ("hard", "Hard"),
];

/// What the background work reports.
pub(super) enum Event {
    Polled {
        dir: PathBuf,
        status: Box<Status>,
        logs: Option<(u64, Vec<String>)>,
    },
    /// Each listed server's state, and how many backups its folder holds.
    Statuses(Vec<(PathBuf, Status, usize)>),
    Line(String),
    Done(Result<String, Problem>),
    Created(Result<Entry, Problem>),
    Deleted {
        dir: PathBuf,
        result: Result<(), String>,
    },
    Details {
        dir: PathBuf,
        config: Option<Value>,
        backups: Vec<Backup>,
    },
    Applied(Result<Applied, Problem>),
    Pvp(Option<bool>),
    /// Who else has the launcher open, and the hosted servers announced now.
    Network {
        people: Vec<online::Person>,
        announced: Vec<online::Announcement>,
    },
    Failed(String),
}

/// The Create dialog's answers.
pub(super) struct Create {
    new_world: bool,
    copy: Option<PathBuf>,
    worlds: Vec<(String, PathBuf)>,
    /// The glass dialog's line under each world: its name and "Horizon · mode · size", read
    /// once when the dialog opens.
    details: HashMap<PathBuf, (String, String)>,
    mode: &'static str,
    difficulty: &'static str,
    max_players: u32,
}

/// One card of who may join: a player, whether their launcher is open, and whether they are a
/// friend of this student. The Create dialog and the Players tab draw the same cards.
pub(super) struct Card {
    name: String,
    /// Their face, when the server knows them (its list has their UUID).
    uuid: Option<String>,
    online: bool,
    friend: bool,
}

/// What the sidebar shows of the Servers page: its list once the page has read it, and whether
/// a server answers.
impl Servers {
    /// Who may join, as cards: the student's friends (their launcher open or not), then the
    /// other students whose launcher is open now, then whoever else the server knows (`known`:
    /// its list's names and UUIDs).
    pub(super) fn cards(&self, known: &[(String, String)]) -> Vec<Card> {
        let mut cards: Vec<Card> = Vec::new();
        let mut add = |card: Card| {
            if !cards.iter().any(|c| c.name == card.name) {
                cards.push(card);
            }
        };
        let face = |name: &str| {
            known
                .iter()
                .find(|(n, _)| n == name)
                .map(|(_, u)| u.clone())
        };
        for name in &self.friends {
            let online = self.people.iter().any(|p| &p.name == name);
            add(Card {
                name: name.clone(),
                uuid: face(name),
                online,
                friend: true,
            });
        }
        for person in &self.people {
            add(Card {
                name: person.name.clone(),
                uuid: face(&person.name),
                online: true,
                friend: false,
            });
        }
        for (name, uuid) in known {
            add(Card {
                name: name.clone(),
                uuid: Some(uuid.clone()),
                online: false,
                friend: false,
            });
        }
        cards
    }
    /// The servers this student was invited to, as the sidebar lists them: those announced now
    /// first.
    pub(super) fn invited(&self) -> &[(invited::Invited, bool)] {
        &self.invited
    }

    /// The same, for the game's server list.
    pub(super) fn remembered(&self) -> Vec<invited::Invited> {
        self.invited.iter().map(|(i, _)| i.clone()).collect()
    }

    /// The server's game mode, in the launcher's language: the open server's is the one its
    /// settings show now, once read; the others' (and the open one's until then) come from their
    /// files.
    fn mode_label(&mut self, dir: &std::path::Path) -> Option<String> {
        let open = Some(self.shown("world", "game_mode"))
            .filter(|m| !m.is_empty() && self.open.as_deref() == Some(dir));
        let mode = open.or_else(|| {
            self.modes
                .entry(dir.to_path_buf())
                .or_insert_with(|| servers::game_mode(dir))
                .clone()
        })?;
        Some(
            MODES
                .iter()
                .find(|(id, _)| *id == mode)
                .map_or(mode.clone(), |(_, label)| tr(label).to_string()),
        )
    }

    /// A stopped server's line in the sidebar: "Survival · stopped", as the mockup's.
    pub(super) fn stopped_line(&mut self, dir: &std::path::Path) -> String {
        match self.mode_label(dir) {
            Some(label) => trf("{mode} · stopped", &[("mode", &label)]),
            None => tr("Stopped").to_string(),
        }
    }

    /// What the list says of a stopped server, where its folder used to be named (a student has
    /// no business in it): its mode, how many players it takes, and how many backups it holds.
    pub(super) fn facts_line(&mut self, dir: &std::path::Path) -> String {
        let mut parts: Vec<String> = self.mode_label(dir).into_iter().collect();
        let open = self.open.as_deref() == Some(dir);
        let max = if open {
            self.shown("server", "max_players")
        } else {
            servers::file_setting(dir, "server", "max_players").unwrap_or_default()
        };
        if !max.is_empty() {
            parts.push(trf("up to {max} players", &[("max", &max)]));
        }
        let backups = if open {
            self.backups.len()
        } else {
            self.backups_count.get(dir).copied().unwrap_or(0)
        };
        parts.push(if backups == 1 {
            tr("One backup").to_string()
        } else {
            trf("{count} backups", &[("count", &backups.to_string())])
        });
        parts.join(" · ")
    }

    pub(super) fn sidebar_list(&self) -> Option<&[Entry]> {
        self.loaded.then_some(&self.list[..])
    }

    pub(super) fn is_up(&self, dir: &std::path::Path) -> bool {
        self.statuses.get(dir).is_some_and(|s| s.phase == "running")
    }
}

pub(super) struct Servers {
    list: Vec<Entry>,
    loaded: bool,
    pub(super) open: Option<PathBuf>,
    tab: Tab,
    statuses: HashMap<PathBuf, Status>,
    /// Each listed server's game mode from its `server.toml`, read once while it is listed.
    modes: HashMap<PathBuf, Option<String>>,
    polling: bool,
    polled: Option<Instant>,
    log: VecDeque<String>,
    log_next: u64,
    log_dir: Option<PathBuf>,
    console_scroll: Scrollbar,
    create_scroll: Scrollbar,
    /// The action under way, and what it printed.
    busy: Option<String>,
    job_log: VecDeque<String>,
    problem: Option<Problem>,
    note: Option<String>,
    pub(super) create: Option<Create>,
    creating: bool,
    config: Option<Value>,
    /// Settings waiting for `config set`, and whether one runs.
    queued: Vec<(String, String)>,
    applying: bool,
    /// Settings saved while the server ran that wait for its next start.
    restart_needed: BTreeSet<String>,
    backups: Vec<Backup>,
    confirm_restore: Option<String>,
    /// Whether the game's own log is shown under the server, as a problem's button asks.
    log_shown: bool,
    confirm_delete: bool,
    pvp: Option<bool>,
    texts: HashMap<Field, LineEdit>,
    focus: HashMap<Field, FocusHandle>,
    focused: Option<Field>,
    /// The field to put the keyboard in on the next frame.
    pub(super) focus_soon: Option<Field>,
    /// A suggested value shown selected: the first key typed replaces it.
    selected: Option<Field>,
    /// A field whose value was refused, and why: shown under it until it's edited.
    invalid: Option<(Field, &'static str)>,
    copied: Option<Instant>,
    faces: HashMap<String, Option<Vec<[u8; 4]>>>,
    /// When the running hosted server's announcement was last written, while one runs.
    announced: Option<Instant>,
    /// The student's friends (`friends.json` on their computer).
    friends: Vec<String>,
    /// Who else has the launcher open, read again every [`ANNOUNCE`] seconds.
    people: Vec<online::Person>,
    people_at: Option<Instant>,
    people_polling: bool,
    /// The servers the student was invited to, each with whether it is announced now, and those
    /// announced at the last reading.
    invited: Vec<(invited::Invited, bool)>,
    seen: Vec<online::Announcement>,
    /// How many backups each listed server's folder holds, read with its status.
    backups_count: HashMap<PathBuf, usize>,
    tx: Sender<Event>,
    rx: Receiver<Event>,
}

impl Servers {
    pub(super) fn new() -> Self {
        let (tx, rx) = channel();
        Self {
            friends: friends::load(),
            people: Vec::new(),
            people_at: None,
            people_polling: false,
            invited: invited::states(&invited::load(), &[]),
            seen: Vec::new(),
            backups_count: HashMap::new(),
            list: Vec::new(),
            loaded: false,
            open: None,
            tab: Tab::Overview,
            statuses: HashMap::new(),
            modes: HashMap::new(),
            polling: false,
            polled: None,
            log: VecDeque::new(),
            log_next: 0,
            log_dir: None,
            console_scroll: Scrollbar::default(),
            create_scroll: Scrollbar::default(),
            busy: None,
            job_log: VecDeque::new(),
            problem: None,
            note: None,
            create: None,
            creating: false,
            config: None,
            queued: Vec::new(),
            applying: false,
            restart_needed: BTreeSet::new(),
            backups: Vec::new(),
            confirm_restore: None,
            log_shown: false,
            confirm_delete: false,
            pvp: None,
            texts: HashMap::new(),
            focus: HashMap::new(),
            focused: None,
            focus_soon: None,
            selected: None,
            invalid: None,
            copied: None,
            faces: HashMap::new(),
            announced: None,
            tx,
            rx,
        }
    }

    pub(super) fn text(&mut self, f: Field) -> &mut LineEdit {
        self.texts.entry(f).or_default()
    }

    pub(super) fn value(&self, f: Field) -> &str {
        self.texts.get(&f).map_or("", LineEdit::text)
    }

    /// Whether this field has the keyboard, as of this frame.
    pub(super) fn has_focus(&self, f: Field) -> bool {
        self.focused == Some(f)
    }

    /// Which field has the keyboard, read every frame.
    pub(super) fn note_focus(&mut self, window: &mut Window, cx: &mut App) {
        if let Some(h) = self.focus_soon.take().and_then(|f| self.focus.get(&f)) {
            window.focus(h, cx);
        }
        self.focused = self
            .focus
            .iter()
            .find(|(_, h)| h.is_focused(window))
            .map(|(f, _)| *f);
        if self.focused.is_some() && self.focused != self.selected {
            self.selected = None;
        }
    }

    fn status(&self) -> Status {
        self.open
            .as_ref()
            .and_then(|d| self.statuses.get(d).cloned())
            .unwrap_or_else(Status::stopped)
    }

    /// A setting from server.toml (as `config --json` gave it).
    fn setting(&self, section: &str, key: &str) -> Option<&Value> {
        self.config
            .as_ref()
            .map(|c| &c[section][key])
            .filter(|v| !v.is_null())
    }

    /// A setting as shown: what was just asked for, else what server.toml holds.
    fn shown(&self, section: &str, key: &str) -> String {
        let full = format!("{section}.{key}");
        if let Some((_, v)) = self.queued.iter().rev().find(|(k, _)| *k == full) {
            return v.clone();
        }
        match self.setting(section, key) {
            Some(Value::String(s)) => s.clone(),
            None => String::new(),
            Some(v) => v.to_string(),
        }
    }
}

/// Stops the servers this launcher started and takes their announcement away: in this edition a
/// hosted server lives as long as its launcher window.
pub(super) fn stop_on_quit() {
    for dir in STARTED_HERE.lock().unwrap().drain(..) {
        if let Some(api) = servers::Api::open(&dir) {
            let _ = api.call("POST", "/api/v1/stop", None);
        }
    }
    if let Err(e) = online::remove(&club::online_dir()) {
        eprintln!("Horizon launcher: could not remove the announcement: {e}");
    }
    // The launcher is closed: it no longer says who is at this computer.
    if let Some(me) = ME.lock().unwrap().clone() {
        if let Err(e) = online::remove_person(&club::online_dir(), &me) {
            eprintln!("Horizon launcher: could not say the launcher closed: {e}");
        }
    }
}

impl Launcher {
    // ----- Background work ------------------------------------------------------------------

    /// Runs `job` on a thread with the page busy; its lines and result arrive as events.
    fn server_job(
        &mut self,
        label: impl Into<String>,
        job: impl FnOnce(&dyn Fn(String)) -> Result<String, Problem> + Send + 'static,
    ) {
        if self.servers.busy.is_some() {
            return;
        }
        self.servers.busy = Some(label.into());
        self.servers.job_log.clear();
        self.servers.problem = None;
        self.servers.note = None;
        let tx = self.servers.tx.clone();
        std::thread::spawn(move || {
            let lines = tx.clone();
            let log = move |l: String| {
                let _ = lines.send(Event::Line(l));
            };
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| job(&log)))
                .unwrap_or_else(|panic| {
                    let error = (panic.downcast_ref::<&str>().map(|s| s.to_string()))
                        .or_else(|| panic.downcast_ref::<String>().cloned())
                        .unwrap_or_default();
                    Err(Problem::plain(trf(
                        "Could not run {program}: {error}",
                        &[("program", "horizon-server"), ("error", &error)],
                    )))
                });
            let _ = tx.send(Event::Done(result));
        });
    }

    /// `horizon-server <args>` on the open server, as a job, from the build in use.
    fn server_command(&mut self, label: &str, args: Vec<String>) {
        let Some(dir) = self.servers.open.clone() else {
            return;
        };
        let Some(tag) = self.selected().map(str::to_owned) else {
            return;
        };
        self.server_job(label.to_string(), move |log| {
            let args: Vec<&str> = args.iter().map(String::as_str).collect();
            let result = match args.first().copied() {
                Some("start" | "restart") => {
                    if args[0] == "start" {
                        log(tr("A new world takes a minute the first time").into());
                    }
                    servers::run_managed(&tag, &dir, &args, log)
                }
                _ => servers::run_streamed(&tag, &dir, &args, log),
            };
            result.map(|_| String::new())
        });
    }

    /// Reads the open server's settings and backups again.
    fn load_details(&self) {
        let Some(dir) = self.servers.open.clone() else {
            return;
        };
        let Some(tag) = self.selected().map(str::to_owned) else {
            return;
        };
        let tx = self.servers.tx.clone();
        std::thread::spawn(move || {
            let config = servers::config(&tag, &dir);
            let backups = servers::backups(&tag, &dir).unwrap_or_default();
            let _ = tx.send(Event::Details {
                dir,
                config,
                backups,
            });
        });
    }

    pub(super) fn open_server(&mut self, dir: PathBuf) {
        if self.servers.open.as_ref() == Some(&dir) {
            return;
        }
        let s = &mut self.servers;
        s.open = Some(dir);
        // Its settings may have changed while it was open.
        s.modes.clear();
        s.tab = Tab::Overview;
        s.config = None;
        s.backups.clear();
        s.pvp = None;
        s.problem = None;
        s.note = None;
        s.confirm_restore = None;
        s.confirm_delete = false;
        s.log_shown = false;
        s.texts.remove(&Field::DeleteServer);
        s.restart_needed.clear();
        s.texts.remove(&Field::Motd);
        s.invalid = None;
        s.polled = None;
        self.load_details();
    }

    /// Called with the launcher's other work: what the background sent, the announcement of a
    /// running hosted server, and the next status poll.
    pub(super) fn servers_tick(&mut self) -> bool {
        let mut changed = false;
        while let Ok(event) = self.servers.rx.try_recv() {
            changed |= self.servers_event(event);
        }
        self.announce();
        self.presence();
        if self.page != Page::Servers || self.servers.polling || !self.servers.loaded {
            return changed;
        }
        let every = if self.servers.open.is_some() {
            POLL_OPEN
        } else {
            POLL_LIST
        };
        if self.servers.polled.is_some_and(|at| at.elapsed() < every) {
            return changed;
        }
        self.servers.polled = Some(Instant::now());
        self.servers.polling = true;
        let tx = self.servers.tx.clone();
        match self.servers.open.clone() {
            Some(dir) => {
                let since = if self.servers.log_dir.as_ref() == Some(&dir) {
                    self.servers.log_next
                } else {
                    0
                };
                // The other servers this launcher started keep running while one is open: their
                // status is read in the same poll, so the announcement follows them honestly.
                let others: Vec<PathBuf> = STARTED_HERE
                    .lock()
                    .unwrap()
                    .iter()
                    .filter(|d| **d != dir)
                    .cloned()
                    .collect();
                std::thread::spawn(move || {
                    let api = servers::Api::open(&dir);
                    let status = api
                        .as_ref()
                        .and_then(|a| a.status())
                        .unwrap_or_else(Status::stopped);
                    let logs = api.and_then(|a| a.logs(since));
                    let _ = tx.send(Event::Polled {
                        dir,
                        status: Box::new(status),
                        logs,
                    });
                    for d in others {
                        let status = servers::status(&d);
                        let _ = tx.send(Event::Polled {
                            dir: d,
                            status: Box::new(status),
                            logs: None,
                        });
                    }
                });
            }
            None => {
                let dirs: Vec<PathBuf> = self.servers.list.iter().map(|e| e.dir.clone()).collect();
                std::thread::spawn(move || {
                    let all = dirs
                        .into_iter()
                        .map(|d| {
                            let s = servers::status(&d);
                            (d.clone(), s, servers::backup_count(&d))
                        })
                        .collect();
                    let _ = tx.send(Event::Statuses(all));
                });
            }
        }
        changed
    }

    /// Writes the announcement of the first server this launcher started that runs, again every
    /// [`ANNOUNCE`] seconds, and takes it away once none does. The `online\` folder is the
    /// supervisor's to make: a student who cannot write there is not hosting for the club, said
    /// once in the launcher's own log and never in the window.
    fn announce(&mut self) {
        let running = STARTED_HERE
            .lock()
            .unwrap()
            .iter()
            .find(|d| {
                self.servers
                    .statuses
                    .get(d.as_path())
                    .is_some_and(|s| s.phase == "running")
            })
            .cloned();
        match running {
            Some(dir)
                if self
                    .servers
                    .announced
                    .is_none_or(|at| at.elapsed() >= ANNOUNCE) =>
            {
                self.servers.announced = Some(Instant::now());
                let Some(owner) = self.player.clone() else {
                    return;
                };
                let Some(name) = self
                    .servers
                    .list
                    .iter()
                    .find(|e| e.dir == dir)
                    .map(|e| e.name.clone())
                else {
                    return;
                };
                std::thread::spawn(move || {
                    let (Some(port), Some(address)) =
                        (servers::port_of(&dir), servers::lan_address())
                    else {
                        return;
                    };
                    let announcement = online::Announcement {
                        name,
                        owner,
                        host: address.to_string(),
                        port,
                        at: server_list::now(),
                    };
                    if online::write(&club::online_dir(), &announcement).is_err()
                        && !ANNOUNCE_QUIET.load(std::sync::atomic::Ordering::Relaxed)
                    {
                        ANNOUNCE_QUIET.store(true, std::sync::atomic::Ordering::Relaxed);
                        eprintln!(
                            "{}",
                            tr(
                                "This launcher cannot write in the club's online folder, so other \
                                 students won't see the hosted server in their list.",
                            )
                        );
                    }
                });
            }
            Some(_) => {}
            None if self.servers.announced.is_some() => {
                self.servers.announced = None;
                std::thread::spawn(move || {
                    if let Err(e) = online::remove(&club::online_dir()) {
                        eprintln!("Horizon launcher: could not remove the announcement: {e}");
                    }
                });
            }
            None => {}
        }
    }

    /// Says that this student's launcher is open (`online\people\`), again every [`ANNOUNCE`]
    /// seconds, and reads who else's is and which hosted servers are announced. Read again at
    /// once when the Create dialog opens or the Players tab shows, whose cards they are. A
    /// student who cannot write there is missing from the others' cards, said once in the
    /// launcher's own log and never in the window.
    fn presence(&mut self) {
        if self.servers.people_polling
            || self
                .servers
                .people_at
                .is_some_and(|at| at.elapsed() < ANNOUNCE)
        {
            return;
        }
        self.servers.people_at = Some(Instant::now());
        self.servers.people_polling = true;
        *ME.lock().unwrap() = self.player.clone();
        let mine = self.player.clone();
        let tx = self.servers.tx.clone();
        std::thread::spawn(move || {
            let dir = club::online_dir();
            if let Some(name) = &mine {
                let person = online::Person {
                    name: name.clone(),
                    at: server_list::now(),
                };
                if online::write_person(&dir, &person).is_err()
                    && !PEOPLE_QUIET.swap(true, std::sync::atomic::Ordering::Relaxed)
                {
                    eprintln!(
                        "{}",
                        tr(
                            "This launcher cannot write in the club's online folder, so other \
                             students won't see it open, nor a hosted server in their list.",
                        )
                    );
                }
            }
            let now = server_list::now();
            let network = Event::Network {
                people: online::people(&dir, now, mine.as_deref().unwrap_or_default()),
                announced: online::list(&dir, now),
            };
            let _ = tx.send(network);
        });
    }

    fn servers_event(&mut self, event: Event) -> bool {
        let s = &mut self.servers;
        match event {
            Event::Polled { dir, status, logs } => {
                s.polling = false;
                let mut changed = false;
                let was = s.statuses.get(&dir).map(|st| st.phase.clone());
                if was.as_deref() != Some("running") && status.phase == "running" {
                    // A fresh start took every saved setting.
                    s.restart_needed.clear();
                    s.pvp = None;
                    changed = true;
                }
                if let Some((next, lines)) = logs {
                    if s.log_dir.as_ref() != Some(&dir) {
                        s.log.clear();
                        s.log_dir = Some(dir.clone());
                        changed = true;
                    }
                    if !lines.is_empty() {
                        s.log.extend(lines);
                        while s.log.len() > CONSOLE_KEEP {
                            s.log.pop_front();
                        }
                        s.console_scroll.handle.scroll_to_bottom();
                        changed = true;
                    }
                    s.log_next = next;
                }
                let read_pvp = status.phase == "running" && s.pvp.is_none();
                if s.statuses.get(&dir) != Some(&status) {
                    s.statuses.insert(dir, *status);
                    changed = true;
                }
                if read_pvp {
                    self.read_pvp();
                    changed = true;
                }
                changed
            }
            Event::Statuses(all) => {
                s.polling = false;
                let mut changed = false;
                for (dir, status, backups) in all {
                    if s.statuses.get(&dir) != Some(&status) {
                        s.statuses.insert(dir.clone(), status);
                        changed = true;
                    }
                    if s.backups_count.get(&dir) != Some(&backups) {
                        s.backups_count.insert(dir, backups);
                        changed = true;
                    }
                }
                changed
            }
            Event::Line(line) => {
                s.job_log.push_back(line);
                while s.job_log.len() > 6 {
                    s.job_log.pop_front();
                }
                true
            }
            Event::Done(result) => {
                s.busy = None;
                s.polled = None;
                match result {
                    Ok(note) if !note.is_empty() => s.note = Some(note),
                    Ok(_) => {}
                    Err(p) => s.problem = Some(p),
                }
                self.load_details();
                true
            }
            Event::Created(result) => {
                s.creating = false;
                match result {
                    Ok(entry) => {
                        s.create = None;
                        s.list = servers::list();
                        s.modes.clear();
                        self.open_server(entry.dir.clone());
                        self.servers.note = Some(trf(
                            "{name} is ready. Press Start, and the other students see it in \
                             their Multiplayer tab.",
                            &[("name", &entry.name)],
                        ));
                    }
                    Err(p) => s.problem = Some(p),
                }
                true
            }
            Event::Details {
                dir,
                config,
                backups,
            } => {
                let count_changed = s.backups_count.get(&dir) != Some(&backups.len());
                if count_changed {
                    s.backups_count.insert(dir.clone(), backups.len());
                }
                if s.open.as_ref() == Some(&dir) {
                    s.config = config;
                    s.backups = backups;
                    true
                } else {
                    count_changed
                }
            }
            Event::Deleted { dir, result } => {
                s.busy = None;
                s.confirm_delete = false;
                s.list = servers::list();
                s.modes.clear();
                s.statuses.remove(&dir);
                STARTED_HERE
                    .lock()
                    .unwrap()
                    .retain(|started| started != &dir);
                if !s.list.iter().any(|entry| entry.dir == dir) {
                    s.open = None;
                    self.side_servers = s.list.clone();
                }
                match result {
                    Ok(()) => s.note = Some(tr("Server moved to the trash.").into()),
                    Err(e) => s.problem = Some(Problem::plain(e)),
                }
                true
            }
            Event::Applied(result) => {
                s.applying = false;
                match result {
                    Ok(applied) if applied.running => s.restart_needed.extend(applied.restart),
                    Ok(_) => {}
                    Err(p) => s.problem = Some(p),
                }
                if s.queued.is_empty() {
                    self.load_details();
                } else {
                    self.apply_settings();
                }
                true
            }
            Event::Pvp(on) => {
                s.pvp = on;
                true
            }
            Event::Network { people, announced } => {
                s.people_polling = false;
                let mut changed = s.people != people;
                s.people = people;
                let before = s.invited.clone();
                changed |= self.remember_invited(&announced, None) != before;
                changed
            }
            Event::Failed(e) => {
                s.problem = Some(Problem::plain(e));
                true
            }
        }
    }

    /// The invited servers, once `joined` was joined (None when none was): a friend's server
    /// announced now is an invitation, and so is a server the student joined. The sidebar's group
    /// says of each whether it is announced now, and the Multiplayer tab lists a remembered one
    /// that is off. What the sidebar lists afterwards.
    fn remember_invited(
        &mut self,
        seen: &[online::Announcement],
        joined: Option<&online::Announcement>,
    ) -> Vec<(invited::Invited, bool)> {
        let s = &mut self.servers;
        s.seen = seen.to_vec();
        let list: Vec<invited::Invited> = s.invited.iter().map(|(i, _)| i.clone()).collect();
        let (list, saved) = invited::remember(&list, seen, &s.friends, joined, server_list::now());
        s.invited = invited::states(&list, seen);
        if saved {
            if let Err(e) = invited::save(&list) {
                eprintln!("Horizon launcher: could not save the invited servers: {e}");
            }
            self.multiplayer.stale();
        }
        s.invited.clone()
    }

    /// The hosted server at `address` is joined from the Multiplayer tab: it is remembered among
    /// the invited servers, where the student finds it again after it stopped.
    pub(super) fn remember_joined(&mut self, address: &str) {
        let seen = self.servers.seen.clone();
        let joined = seen.iter().find(|a| a.address() == address).cloned();
        self.remember_invited(&seen, joined.as_ref());
    }

    /// The `pvp` game rule, asked of the running game (it's a rule of the world, not a setting).
    fn read_pvp(&self) {
        let Some(dir) = self.servers.open.clone() else {
            return;
        };
        let tx = self.servers.tx.clone();
        std::thread::spawn(move || {
            let on = servers::Api::open(&dir)
                .and_then(|a| a.command("gamerule pvp").ok())
                .and_then(|out| {
                    let text = out.join(" ");
                    let text = text.trim_end();
                    if text.ends_with("true") {
                        Some(true)
                    } else if text.ends_with("false") {
                        Some(false)
                    } else {
                        None
                    }
                });
            let _ = tx.send(Event::Pvp(on));
        });
    }

    /// Queues a setting for `horizon-server config set`; they go one batch at a time.
    fn set_setting(&mut self, key: &str, value: String) {
        self.servers.queued.retain(|(k, _)| k != key);
        self.servers.queued.push((key.to_string(), value));
        self.servers.note = None;
        if !self.servers.applying {
            self.apply_settings();
        }
    }

    fn apply_settings(&mut self) {
        let Some(dir) = self.servers.open.clone() else {
            return;
        };
        let batch = std::mem::take(&mut self.servers.queued);
        if batch.is_empty() {
            return;
        }
        self.servers.applying = true;
        // Shown as asked until the file is read back.
        if let Some(config) = &mut self.servers.config {
            for (key, value) in &batch {
                if let Some((section, field)) = key.split_once('.') {
                    let new = match &config[section][field] {
                        Value::Bool(_) => Value::Bool(value == "true"),
                        Value::Number(_) => value.parse::<i64>().map_or(Value::Null, Value::from),
                        _ => Value::String(value.clone()),
                    };
                    config[section][field] = new;
                }
            }
        }
        let tx = self.servers.tx.clone();
        let Some(tag) = self.selected().map(str::to_owned) else {
            return;
        };
        std::thread::spawn(move || {
            let pairs: Vec<(&str, String)> =
                batch.iter().map(|(k, v)| (k.as_str(), v.clone())).collect();
            let _ = tx.send(Event::Applied(servers::set_config(&tag, &dir, &pairs)));
        });
    }

    /// A command for the running game, through the control API; its answer shows in the console.
    fn api_command(&mut self, line: String) {
        let Some(dir) = self.servers.open.clone() else {
            return;
        };
        let tx = self.servers.tx.clone();
        self.servers.log.push_back(format!("> {line}"));
        self.servers.console_scroll.handle.scroll_to_bottom();
        std::thread::spawn(move || {
            let result = servers::Api::open(&dir)
                .ok_or_else(|| "The server isn't running.".to_string())
                .and_then(|a| a.command(&line));
            if let Err(e) = result {
                let _ = tx.send(Event::Failed(e));
            }
        });
    }

    fn start_server(&mut self) {
        let Some(dir) = self.servers.open.clone() else {
            return;
        };
        let Some(tag) = self.selected().map(str::to_owned) else {
            return;
        };
        STARTED_HERE.lock().unwrap().push(dir.clone());
        self.servers.restart_needed.clear();
        self.server_job("Starting", move |log| {
            if let Some(api) = servers::Api::open(&dir) {
                if let Ok(status) = api.call("GET", "/api/v1/status", None) {
                    if status["phase"] == "stopped"
                        && api.call("POST", "/api/v1/start", None).is_ok()
                    {
                        return Ok(String::new());
                    }
                }
            }
            log("A new world takes a minute the first time".into());
            servers::run_managed(&tag, &dir, &["start"], log).map(|_| String::new())
        });
    }

    fn stop_server(&mut self) {
        let Some(dir) = self.servers.open.clone() else {
            return;
        };
        let Some(tag) = self.selected().map(str::to_owned) else {
            return;
        };
        self.server_job("Stopping", move |log| {
            // The running server's API first; when it can't be reached or refuses, the CLI,
            // whose error is the job's.
            let via_api = servers::Api::open(&dir)
                .is_some_and(|api| api.call("POST", "/api/v1/stop", None).is_ok());
            if via_api {
                return Ok(String::new());
            }
            servers::run_streamed(&tag, &dir, &["stop"], log).map(|_| String::new())
        });
    }

    fn restart_server(&mut self) {
        let Some(dir) = self.servers.open.clone() else {
            return;
        };
        let Some(tag) = self.selected().map(str::to_owned) else {
            return;
        };
        self.server_job("Restarting", move |log| {
            // The running server's API first; when it can't be reached or refuses, the CLI,
            // whose error is the job's.
            let via_api = servers::Api::open(&dir)
                .is_some_and(|api| api.call("POST", "/api/v1/restart", None).is_ok());
            if via_api {
                return Ok(String::new());
            }
            servers::run_managed(&tag, &dir, &["restart"], log).map(|_| String::new())
        });
    }

    fn fix(&mut self, fix: Fix, _cx: &mut Context<Self>) {
        self.servers.problem = None;
        match fix {
            Fix::PortTaken => {
                // A free port of the club's own range, taken by no listed server.
                let hosting = self.config.hosting;
                match servers::free_port(
                    hosting.first_port,
                    hosting.last_port,
                    &servers::ports_taken(),
                ) {
                    Some(port) => {
                        self.set_setting("server.port", port.to_string());
                        self.servers.note = Some(trf(
                            "The server now uses port {port}. Press Start again.",
                            &[("port", &port.to_string())],
                        ));
                    }
                    None => {
                        self.servers.problem = Some(Problem::plain(trf(
                            "No port is free between {first} and {last}: stop another server, or \
                             ask the club's supervisor for more of them.",
                            &[
                                ("first", &hosting.first_port.to_string()),
                                ("last", &hosting.last_port.to_string()),
                            ],
                        )));
                    }
                }
            }
            Fix::ShowLog => {
                // In the window, where every student can read it: not in a folder to wander in.
                self.servers.log_shown = true;
            }
        }
    }

    // ----- Creating -------------------------------------------------------------------------

    /// Opens the Create dialog. `pub(super)`: the sidebars open it from their "Create a server".
    pub(super) fn open_create(&mut self, cx: &mut Context<Self>) {
        self.focus_handle(Field::Name, cx);
        self.servers.focus_soon = Some(Field::Name);
        let worlds = servers::player_worlds();
        let name = if self.servers.list.is_empty() {
            tr("My server").to_string()
        } else {
            trf(
                "Server {number}",
                &[("number", &(self.servers.list.len() + 1).to_string())],
            )
        };
        // The glass dialog starts with an empty name: its placeholder says the default.
        let glass = self.theme.glass;
        self.servers
            .text(Field::Name)
            .set(if glass { String::new() } else { name });
        self.servers.selected = (!glass).then_some(Field::Name);
        self.servers.text(Field::Seed).set(String::new());
        // The cards are who has the launcher open now: read again as the dialog opens.
        self.servers.people_at = None;
        let details = if glass {
            horizon_launcher::worlds::list(&backend::game_home())
                .into_iter()
                .map(|w| {
                    let line = format!("Horizon · {} · {}", tr(w.mode), bytes(w.size));
                    (w.path, (w.name, line))
                })
                .collect()
        } else {
            Default::default()
        };
        self.servers.problem = None;
        self.servers.create_scroll = Scrollbar::default();
        self.servers.create = Some(Create {
            new_world: true,
            copy: worlds.first().map(|w| w.1.clone()),
            worlds,
            mode: "survival",
            difficulty: "normal",
            max_players: 10,
            details,
        });
    }

    fn create_server(&mut self) {
        if self.servers.creating {
            return;
        }
        let Some(c) = self.servers.create.as_ref() else {
            return;
        };
        let world = match (&c.copy, c.new_world) {
            (Some(dir), false) => World::Copy(dir.clone()),
            _ => World::New {
                seed: self.servers.value(Field::Seed).to_string(),
            },
        };
        // The student, who is the server's operator, then their friends: a friend is let in on
        // every server the student creates.
        let mut players: Vec<String> = Vec::new();
        if let Some(me) = &self.player {
            players.push(me.clone());
        }
        for friend in &self.servers.friends {
            if !players.contains(friend) {
                players.push(friend.clone());
            }
        }
        let name = self.servers.value(Field::Name).trim().to_string();
        let spec = NewServer {
            name: if !name.is_empty() {
                name
            } else if self.servers.list.is_empty() {
                tr("My server").into()
            } else {
                trf(
                    "Server {number}",
                    &[("number", &(self.servers.list.len() + 1).to_string())],
                )
            },
            world,
            game_mode: c.mode.into(),
            difficulty: c.difficulty.into(),
            max_players: c.max_players,
            friends_only: true,
            players,
        };
        let Some(tag) = self.selected().map(str::to_owned) else {
            return;
        };
        let hosting = self.config.hosting;
        let tx = self.servers.tx.clone();
        self.servers.creating = true;
        self.servers.problem = None;
        self.servers.job_log.clear();
        std::thread::spawn(move || {
            let lines = tx.clone();
            let log = move |l: String| {
                let _ = lines.send(Event::Line(l));
            };
            let _ = tx.send(Event::Created(servers::create(&spec, &tag, &hosting, &log)));
        });
    }

    // ----- Building blocks ------------------------------------------------------------------

    fn focus_handle(&mut self, f: Field, cx: &mut Context<Self>) -> FocusHandle {
        self.servers
            .focus
            .entry(f)
            .or_insert_with(|| cx.focus_handle().tab_index(0).tab_stop(true))
            .clone()
    }

    /// A one-line text field: typing, paste, Backspace, Enter to confirm, Escape, Tab to the next.
    pub(super) fn field(
        &mut self,
        f: Field,
        placeholder: &'static str,
        width: f32,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let t = self.theme;
        let handle = self.focus_handle(f, cx);
        let focused = self.servers.focused == Some(f);
        let selected = focused && self.servers.selected == Some(f);
        let invalid = self.servers.invalid.is_some_and(|(g, _)| g == f);
        self.servers.text(f).show(((width - 24.) / 7.5) as usize);
        // Every field follows its page.
        // The dark appearance draws every field in the glass look.
        let glass = t.glass;
        let base = if glass {
            // The log search sits by buttons: the mockup's small `FIELD`, as tall as they are.
            crate::glass_ui::field(focused, invalid)
                .when(
                    matches!(f, Field::ConsoleSearch | Field::Motd | Field::DeleteServer),
                    |d| d.h(px(36.)).px(px(12.)).rounded(px(radius::CONTROL)),
                )
                // The Create dialog's name and seed are the large `FIELD_LG`: 48 px, 15 px text,
                // white at 6 %, 10 % while typed in.
                .when(!invalid && matches!(f, Field::Name | Field::Seed), |d| {
                    d.h(px(48.))
                        .text_size(px(15.))
                        .bg(crate::glass_ui::white(if focused { 0.1 } else { 0.06 }))
                })
                // The accent's hex: `h-9 w-[96px] rounded-[10px] px-3 text-[13px] font-medium`.
                .when(f == Field::AccentHex, |d| {
                    d.h(px(36.))
                        .px(px(12.))
                        .rounded(px(radius::CONTROL))
                        .text_size(px(13.))
                        .font_weight(FontWeight::MEDIUM)
                })
                // The server's command line: the field is the line's middle, on its fill, in mono.
                .when(f == Field::Command, |d| {
                    d.h_full()
                        .px_0()
                        .rounded_none()
                        .bg(gpui::transparent_black())
                        .font_family(theme::MONO)
                        .text_size(px(13.))
                })
                // The mockup's focus ring (`:focus-visible`, 2 px of the accent's soft shade) on a
                // field of its own, drawn inside its edge: GPUI has no outline.
                .when(focused && !invalid && f != Field::Command, |d| {
                    let small = matches!(
                        f,
                        Field::ConsoleSearch | Field::AccentHex | Field::Motd | Field::DeleteServer
                    );
                    d.child(crate::glass_ui::focus_ring(if small {
                        radius::CONTROL
                    } else {
                        radius::ROW
                    }))
                })
        } else {
            div()
                .flex()
                .items_center()
                .h(px(theme::FIELD))
                .px(px(12.))
                .overflow_hidden()
                .rounded(px(theme::RADIUS))
                .border_1()
                .border_color(if invalid {
                    t.danger
                } else if focused {
                    t.accent
                } else {
                    t.rule
                })
                .bg(t.surface)
                .text_size(px(theme::UI.px))
        };
        let field = base
            .id(SharedString::from(format!("field-{f:?}")))
            .track_focus(&handle)
            .w(px(width))
            .cursor_text()
            .child(line_edit::content(
                self.servers.text(f),
                &t,
                focused,
                selected,
                false,
                placeholder,
            ))
            .on_click(cx.listener(move |this, ev: &ClickEvent, window, cx| {
                // A click puts the caret in: the suggestion is no longer selected.
                if matches!(ev, ClickEvent::Mouse(_)) {
                    this.servers.selected = None;
                }
                if let Some(h) = this.servers.focus.get(&f) {
                    window.focus(h, cx);
                }
                cx.notify();
            }))
            .on_key_down(cx.listener(move |this, ev: &KeyDownEvent, window, cx| {
                if ev.keystroke.key != "enter" && this.servers.invalid.is_some_and(|(g, _)| g == f)
                {
                    this.servers.invalid = None;
                }
                // A suggestion is selected: what's typed or pasted replaces it.
                let selected = this.servers.selected.take() == Some(f);
                if selected {
                    this.servers.text(f).select_all();
                }
                match line_edit::handle(this.servers.text(f), ev, cx) {
                    Key::Enter => this.field_enter(f),
                    Key::Escape => {
                        this.servers.confirm_delete = false;
                        if this.servers.create.is_some() && !this.servers.creating {
                            this.servers.create = None;
                        }
                        window.blur(cx);
                    }
                    // Tab and the rest go on to the window: Tab moves the focus.
                    Key::Pass => return,
                    _ => {}
                }
                cx.stop_propagation();
                cx.notify();
            }));
        line_edit::mouse(field, cx, move |this| this.servers.text(f))
    }

    /// Escape closes the Create dialog. Whether it did.
    pub(super) fn servers_escape(&mut self) -> bool {
        if self.page == Page::Servers && self.servers.confirm_delete {
            self.servers.confirm_delete = false;
            return true;
        }
        if self.page != Page::Servers || self.servers.create.is_none() || self.servers.creating {
            return false;
        }
        self.servers.create = None;
        self.servers.problem = None;
        true
    }

    fn field_enter(&mut self, f: Field) {
        match f {
            Field::Name | Field::Seed => self.create_server(),
            Field::Command => {
                let line = self.servers.text(Field::Command).take();
                let line = line.trim().trim_start_matches('/').to_string();
                if !line.is_empty() {
                    self.api_command(line);
                }
            }
            Field::Motd => {
                let motd = self.servers.value(Field::Motd).trim().to_string();
                if motd.is_empty() {
                    self.servers.problem = Some(Problem::plain(tr(
                        "Type the line players see under the name.",
                    )));
                } else {
                    self.set_setting("server.motd", motd);
                }
            }
            Field::ConsoleSearch | Field::DeleteServer | Field::AccentHex => {}
        }
    }

    /// Picks a friend: the card clicked is toggled in the student's list, saved at once. In the
    /// Players tab (`server` is that server's folder) a friend picked there is let in on that
    /// server too, as a friend is on every server the student creates.
    fn toggle_friend(&mut self, name: &str, server: Option<&Path>) {
        let was = friends::is_friend(&self.servers.friends, name);
        if friends::toggle(&mut self.servers.friends, name) == was {
            return;
        }
        if let Err(e) = friends::save(&self.servers.friends) {
            self.servers.problem = Some(Problem::plain(trf(
                "Your friends could not be saved: {error}",
                &[("error", &e)],
            )));
        }
        if !was {
            if let Some(dir) = server {
                let open = self.servers.open.as_deref() == Some(dir);
                let listed = servers::entries(dir, "whitelist.json")
                    .iter()
                    .any(|(n, _)| n == name);
                if open && !listed && self.servers.busy.is_none() {
                    let name = name.to_string();
                    self.server_command(
                        &trf("Letting {name} in", &[("name", &name)]),
                        vec!["whitelist".into(), "add".into(), name],
                    );
                }
            }
        }
    }

    /// A player's face, from the default skin their UUID picks in the build's own files (the
    /// server knows no other).
    pub(super) fn head(&mut self, uuid: &str, size: f32) -> Div {
        let resources = self.selected().map(backend::resources).unwrap_or_default();
        let face = self
            .servers
            .faces
            .entry(uuid.to_string())
            .or_insert_with(|| heads::default_face(&resources, uuid))
            .clone();
        let cell = size / 8.;
        let mut d = div()
            .flex_none()
            .flex()
            .flex_col()
            .size(px(size))
            .rounded(px(if self.theme.glass { size / 4. } else { 3. }))
            .overflow_hidden()
            .bg(self.theme.plate);
        if let Some(pixels) = face {
            for row in pixels.chunks(8) {
                d = d.child(div().flex().children(row.iter().map(|p| {
                    div().size(px(cell)).bg(Hsla::from(gpui::Rgba {
                        r: p[0] as f32 / 255.,
                        g: p[1] as f32 / 255.,
                        b: p[2] as f32 / 255.,
                        a: 1.,
                    }))
                })));
            }
        }
        d
    }

    /// A row's small button.
    fn small(
        &self,
        id: impl Into<SharedString>,
        label: &'static str,
        enabled: bool,
        cx: &mut Context<Self>,
        action: impl Fn(&mut Self) + 'static,
    ) -> Stateful<Div> {
        let b = button(id, label, Kind::Ghost, &self.theme)
            .h(px(28.))
            .px(px(10.))
            .text_size(px(theme::CAPTION.px));
        if enabled {
            b.on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                action(this);
                cx.notify();
            }))
        } else {
            disabled(b)
        }
    }

    // ----- The page -------------------------------------------------------------------------

    pub(super) fn servers_page(&mut self, cx: &mut Context<Self>) -> Div {
        let t = self.theme;
        if !self.servers.loaded {
            self.servers.list = servers::list();
            self.servers.modes.clear();
            self.servers.loaded = true;
            if self.servers.open.is_none() && self.servers.list.len() == 1 {
                let dir = self.servers.list[0].dir.clone();
                self.open_server(dir);
            }
        }
        // HORIZON_LAUNCHER_SERVERS=create opens the Create dialog right away, for screenshots;
        // without it the page shows its list (and one server opens its dashboard, above).
        if self.servers.create.is_none()
            && std::env::var("HORIZON_LAUNCHER_SERVERS").as_deref() == Ok("create")
        {
            self.open_create(cx);
        }
        if t.glass {
            return self.glass_servers_page(cx);
        }
        if let Some(dir) = self.servers.open.clone() {
            if self.servers.list.iter().any(|e| e.dir == dir) {
                return self.server_dashboard(&dir, cx);
            }
            self.servers.open = None;
        }
        let header = page_header(
            "Servers",
            "Host a world for your friends on this computer. Create one, press Start, and the \
             other students see it in their Multiplayer tab.",
            &t,
        );
        let actions = div().flex().items_center().gap(px(8.)).child(
            button("server-create", "Create a server", Kind::Primary, &t).on_click(cx.listener(
                |this, _: &ClickEvent, _, cx| {
                    this.open_create(cx);
                    cx.notify();
                },
            )),
        );
        let mut page = div()
            .flex()
            .flex_col()
            .gap(px(32.))
            .child(header)
            .child(actions);
        if let Some(p) = self
            .servers
            .problem
            .clone()
            .filter(|_| self.servers.create.is_none())
        {
            page = page.child(self.problem_notice(p, cx));
        }
        if self.servers.list.is_empty() {
            return page.child(centered(
                tr("No server yet. Create one: it takes a few seconds, and with the defaults you get a friends-only survival world your friends join from their Multiplayer tab."),
                &t,
            ));
        }
        let mut list = div().flex().flex_col().border_t_1().border_color(t.rule);
        for (i, e) in self.servers.list.clone().into_iter().enumerate() {
            let status = self
                .servers
                .statuses
                .get(&e.dir)
                .cloned()
                .unwrap_or_else(Status::stopped);
            let detail = if status.running() {
                format!(
                    "{} · {}",
                    phase_label(&status),
                    trf(
                        "{online} of {max} players",
                        &[
                            ("online", &status.players.len().to_string()),
                            ("max", &status.max_players.unwrap_or(0).to_string()),
                        ]
                    )
                )
            } else {
                format!(
                    "{} · {}",
                    phase_label(&status),
                    self.servers.facts_line(&e.dir)
                )
            };
            let dir = e.dir.clone();
            list = list.child(index_row(
                i + 1,
                e.name.clone(),
                Some(detail.into_any_element()),
                Some(
                    button(
                        SharedString::from(format!("server-open-{i}")),
                        "Open",
                        Kind::Secondary,
                        &t,
                    )
                    .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                        this.open_server(dir.clone());
                        cx.notify();
                    }))
                    .into_any_element(),
                ),
                &t,
            ));
        }
        page.child(list)
    }

    /// The end of the server's log, where it says why the game didn't start: in the window, as a
    /// problem's button asks, since no student has to open the server's folder.
    fn failure_log(&mut self, dir: &Path, cx: &mut Context<Self>) -> Div {
        let t = self.theme;
        let lines = servers::log_tail(dir, 60);
        let body = if lines.is_empty() {
            text(theme::CAPTION)
                .text_color(t.muted)
                .child(tr("The server wrote no log."))
                .into_any_element()
        } else {
            div()
                .flex()
                .flex_col()
                .font_family(theme::MONO)
                .text_size(px(12.))
                .line_height(px(19.))
                .text_color(t.ink)
                .children(lines.into_iter().map(|l| div().child(l)))
                .into_any_element()
        };
        plate(&t)
            .flex()
            .flex_col()
            .gap(px(12.))
            .p(px(20.))
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(self.tab_heading(tr("The server's log")))
                    .child(
                        button("log-hide", tr("Hide the log"), Kind::Secondary, &t).on_click(
                            cx.listener(|this, _: &ClickEvent, _, cx| {
                                this.servers.log_shown = false;
                                cx.notify();
                            }),
                        ),
                    ),
            )
            .child(body)
    }

    fn problem_notice(&self, p: Problem, cx: &mut Context<Self>) -> Div {
        let t = self.theme;
        let mut n = notice(p.message.clone(), &t).items_center();
        if let Some(fix) = p.fix {
            let label = match &fix {
                Fix::PortTaken => tr("Use a free port").to_string(),
                Fix::ShowLog => tr("Open the logs").to_string(),
            };
            n = n.child(
                button("problem-fix", label, Kind::Secondary, &t)
                    .bg(t.plate)
                    .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                        this.fix(fix.clone(), cx);
                        cx.notify();
                    })),
            );
        }
        n.child(
            button("problem-dismiss", "Dismiss", Kind::Ghost, &t).on_click(cx.listener(
                |this, _: &ClickEvent, _, cx| {
                    this.servers.problem = None;
                    cx.notify();
                },
            )),
        )
    }

    fn server_dashboard(&mut self, dir: &Path, cx: &mut Context<Self>) -> Div {
        let t = self.theme;
        let status = self.servers.status();
        let name = self
            .servers
            .list
            .iter()
            .find(|e| e.dir == dir)
            .map(|e| e.name.clone())
            .unwrap_or_default();
        let running = status.running();
        let busy = self.servers.busy.clone();
        let mut top = div().flex().flex_col().gap(px(10.));
        if self.servers.list.len() > 1 || self.servers.create.is_none() {
            top = top.child(
                div().flex().child(
                    caps(tr("← All servers"))
                        .id("servers-back")
                        .text_color(t.muted)
                        .cursor_pointer()
                        .hover(move |s| s.text_color(t.ink))
                        .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                            this.servers.open = None;
                            this.servers.polled = None;
                            this.servers.problem = None;
                            this.servers.note = None;
                            cx.notify();
                        })),
                ),
            );
        }
        let dot = match status.phase.as_str() {
            "running" if status.crashes > 0 => t.accent,
            "running" => t.success,
            "stopped" => t.muted,
            _ => t.accent,
        };
        top = top
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(10.))
                    .child(div().size(px(8.)).bg(dot))
                    .child(caps(phase_label(&status)).text_color(t.muted))
                    .when_some(status.uptime.filter(|_| running), |d, up| {
                        d.child(
                            caps(trf("up {time}", &[("time", &uptime(up))])).text_color(t.muted),
                        )
                    }),
            )
            .child(text(theme::DISPLAY).text_color(t.ink).child(name));
        if status.crashes > 0 {
            top = top.child(text(theme::CAPTION).text_color(t.muted).child(format!(
                    "The game crashed {} time{} and was restarted by itself{}.",
                    status.crashes,
                    if status.crashes == 1 { "" } else { "s" },
                    status
                        .last_crash
                        .as_ref()
                        .map(|c| format!(" (last: {c})"))
                        .unwrap_or_default()
                )));
        }

        // The big buttons.
        let big = |b: Stateful<Div>| {
            b.min_w(px(132.))
                .h(px(theme::CONTROL_LARGE))
                .px(px(28.))
                .text_size(px(theme::UI.px))
        };
        let idle = busy.is_none();
        let start = big(button(
            "server-start",
            if running { "Stop" } else { "Start" },
            if running {
                Kind::Secondary
            } else {
                Kind::Primary
            },
            &t,
        ));
        let start = if !idle {
            disabled(start)
        } else if running {
            start.on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                this.stop_server();
                cx.notify();
            }))
        } else {
            start.on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                this.start_server();
                cx.notify();
            }))
        };
        let restart = big(button("server-restart", "Restart", Kind::Secondary, &t));
        let restart = if idle && running {
            restart.on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                this.servers.restart_needed.clear();
                this.restart_server();
                cx.notify();
            }))
        } else {
            disabled(restart)
        };
        let buttons = div()
            .flex()
            .flex_wrap()
            .items_center()
            .gap(px(8.))
            .child(start)
            .child(restart);

        let mut page = div()
            .flex()
            .flex_col()
            .gap(px(24.))
            .child(top)
            .child(buttons)
            .children(self.servers.log_shown.then(|| self.failure_log(dir, cx)));
        if let Some(label) = &busy {
            let last = self.servers.job_log.back().cloned().unwrap_or_default();
            page = page.child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(8.))
                    .child(progress_bar(None, &t))
                    .child(
                        text(theme::CAPTION)
                            .text_color(t.muted)
                            .truncate()
                            .child(format!("{label}… {last}")),
                    ),
            );
        }
        if let Some(p) = self.servers.problem.clone() {
            page = page.child(self.problem_notice(p, cx));
        }
        if let Some(n) = self.servers.note.clone() {
            page = page.child(
                plate(&t)
                    .flex()
                    .items_center()
                    .gap(px(12.))
                    .p(px(16.))
                    .child(div().flex_none().size(px(6.)).bg(t.success))
                    .child(
                        div()
                            .flex_1()
                            .text_size(px(theme::UI.px))
                            .text_color(t.ink)
                            .child(n),
                    ),
            );
        }
        if running && !self.servers.restart_needed.is_empty() {
            let names: Vec<String> = self
                .servers
                .restart_needed
                .iter()
                .map(|k| tr(setting_name(k)).to_string())
                .collect();
            page = page.child(
                plate(&t)
                    .flex()
                    .items_center()
                    .gap(px(12.))
                    .p(px(16.))
                    .child(div().flex_none().size(px(6.)).bg(t.accent))
                    .child(
                        div()
                            .flex_1()
                            .text_size(px(theme::UI.px))
                            .text_color(t.ink)
                            .child(trf(
                                "Saved. Restart to apply: {settings}.",
                                &[("settings", &names.join(", "))],
                            )),
                    )
                    .child({
                        let b =
                            button("restart-apply", "Restart now", Kind::Secondary, &t).bg(t.plate);
                        if idle {
                            b.on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                                this.servers.restart_needed.clear();
                                this.restart_server();
                                cx.notify();
                            }))
                        } else {
                            disabled(b)
                        }
                    }),
            );
        }
        page = page.child(self.stats(&status));
        let this = cx.entity().downgrade();
        page = page.child(div().flex().child(segments(
            "server-tabs",
            &TABS,
            self.servers.tab,
            &t,
            move |tab, _, cx| {
                let _ = this.update(cx, |this, cx| {
                    this.servers.tab = tab;
                    this.servers.confirm_restore = None;
                    cx.notify();
                });
            },
        )));
        let body = match self.servers.tab {
            Tab::Overview => self.overview_tab(&status, cx),
            Tab::Players => self.players_tab(dir, &status, cx),
            Tab::Settings => self.settings_tab(&status, cx),
            Tab::Backups => self.backups_tab(&status, cx),
        };
        page.child(body)
    }

    /// The server at a glance: who is on it, its mode, who may join and its backups; and, while
    /// it runs, how fast it goes on this computer.
    fn stats(&self, status: &Status) -> Div {
        let t = self.theme;
        let tile = |value: String, label: &'static str| {
            plate(&t)
                .flex()
                .flex_col()
                .flex_1()
                .min_w(px(140.))
                .gap(px(4.))
                .p(px(16.))
                .child(text(theme::TITLE).text_color(t.ink).child(value))
                .child(caps(tr(label)).text_color(t.muted))
        };
        let dash = || "–".to_string();
        let on = status.running();
        let mode = self.servers.shown("world", "game_mode");
        let mode = if mode.is_empty() {
            dash()
        } else {
            MODES
                .iter()
                .find(|(id, _)| *id == mode)
                .map_or(mode.clone(), |(_, label)| tr(label).to_string())
        };
        let friends_only = self
            .servers
            .setting("server", "whitelist")
            .and_then(Value::as_bool)
            .unwrap_or(true);
        let mut row = div().flex().flex_wrap().gap(px(12.)).child(tile(
            if on {
                format!(
                    "{} / {}",
                    status.players.len(),
                    status.max_players.unwrap_or(0)
                )
            } else {
                dash()
            },
            "Players online",
        ));
        if !mode.is_empty() {
            row = row.child(tile(mode, "Game mode"));
        }
        row = row
            .child(tile(
                if friends_only { tr("On") } else { tr("Off") }.to_string(),
                "Friends only",
            ))
            .child(tile(self.servers.backups.len().to_string(), "Backups"));
        let mut rows = div().flex().flex_col().gap(px(12.)).child(row);
        if on {
            rows = rows.child(
                div()
                    .flex()
                    .gap(px(12.))
                    .child(tile(
                        status.tps.map_or_else(dash, |v| format!("{v:.1}")),
                        "TPS · 20 is full speed",
                    ))
                    .child(tile(
                        status.mspt.map_or_else(dash, |v| format!("{v:.1} ms")),
                        "MSPT · tick time",
                    ))
                    .child(tile(
                        status
                            .ram_bytes
                            .map_or_else(dash, |b| format!("{:.2} GB", b as f64 / 1e9)),
                        "RAM",
                    )),
            );
        }
        rows
    }

    fn overview_tab(&mut self, status: &Status, cx: &mut Context<Self>) -> Div {
        let t = self.theme;
        let port = self
            .servers
            .setting("server", "port")
            .and_then(Value::as_u64)
            .unwrap_or(25565);
        let lan = servers::lan_address();
        let address = match lan {
            Some(ip) if port == 25565 => ip.to_string(),
            Some(ip) => format!("{ip}:{port}"),
            None => format!("localhost:{port}"),
        };
        let copied = self
            .servers
            .copied
            .is_some_and(|at| at.elapsed() < Duration::from_secs(2));
        let copy_text = address.clone();
        let address_plate = plate(&t)
            .flex()
            .flex_col()
            .gap(px(10.))
            .p(px(20.))
            .child(caps(tr("Address on this network")).text_color(t.muted))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(12.))
                    .child(
                        text(theme::TITLE)
                            .font_family(theme::MONO)
                            .text_color(t.ink)
                            .child(address),
                    )
                    .child(
                        button(
                            "address-copy",
                            if copied { "Copied" } else { "Copy" },
                            Kind::Secondary,
                            &t,
                        )
                        .bg(t.plate)
                        .on_click(cx.listener(
                            move |this, _: &ClickEvent, _, cx| {
                                copy_to_clipboard(
                                    cx,
                                    gpui::ClipboardItem::new_string(copy_text.clone()),
                                );
                                this.servers.copied = Some(Instant::now());
                                cx.notify();
                            },
                        )),
                    ),
            )
            .child(text(theme::CAPTION).text_color(t.ink_soft).child(tr(
                "The other students see this server in their Multiplayer tab while it runs.",
            )));
        let mut tab = div().flex().flex_col().gap(px(20.)).child(address_plate);
        if !status.players.is_empty() {
            let mut row = div().flex().flex_wrap().gap(px(16.));
            for p in status.players.clone() {
                row = row.child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(8.))
                        .child(self.head(&p.uuid, 20.))
                        .child(text(theme::UI).text_color(t.ink).child(p.name.clone())),
                );
            }
            tab = tab.child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(10.))
                    .child(caps(tr("Online now")).text_color(t.muted))
                    .child(row),
            );
        }
        tab.child(self.server_console(status.running(), cx))
    }

    fn server_console(&mut self, running: bool, cx: &mut Context<Self>) -> Div {
        let t = self.theme;
        let lines: Vec<Div> = self
            .servers
            .log
            .iter()
            .map(|l| {
                div()
                    .when(l.starts_with("> "), |d| d.text_color(t.accent))
                    .child(l.clone())
            })
            .collect();
        let empty = self.servers.log.is_empty();
        let log = div()
            .id("server-console")
            .h(px(240.))
            .px(px(20.))
            .py(px(12.))
            .overflow_y_scroll()
            .font_family(theme::MONO)
            .text_size(px(theme::CAPTION.px))
            .line_height(px(19.))
            .text_color(t.ink_soft)
            .when(empty, |d| {
                d.child(div().text_color(t.muted).child(if running {
                    tr("Waiting for the server's log…")
                } else {
                    tr("The server's log shows here while it runs.")
                }))
            })
            .children(lines);
        let log = self.servers.console_scroll.wrap(log, &t);
        let input = if running {
            self.field(
                Field::Command,
                "Type a command: say Hello, time set day, gamemode creative Alex…",
                520.,
                cx,
            )
            .flex_1()
            .border_0()
            .bg(t.plate_soft)
            .font_family(theme::MONO)
            .text_size(px(theme::CAPTION.px))
            .into_any_element()
        } else {
            div()
                .px(px(12.))
                .h(px(theme::FIELD))
                .flex()
                .items_center()
                .text_size(px(theme::CAPTION.px))
                .text_color(t.muted)
                .child(tr("Start the server to type commands"))
                .into_any_element()
        };
        plate(&t)
            .flex()
            .flex_col()
            .overflow_hidden()
            .child(
                div()
                    .flex()
                    .items_center()
                    .h(px(40.))
                    .px(px(20.))
                    .border_b_1()
                    .border_color(t.rule)
                    .child(caps(tr("Console")).text_color(t.muted)),
            )
            .child(log)
            .child(
                div()
                    .flex()
                    .items_center()
                    .px(px(8.))
                    .py(px(4.))
                    .border_t_1()
                    .border_color(t.rule)
                    .child(div().text_color(t.accent).ml(px(12.)).child("›"))
                    .child(input),
            )
    }

    fn players_tab(&mut self, dir: &Path, status: &Status, cx: &mut Context<Self>) -> Div {
        let t = self.theme;
        // The glass look draws no rules between rows, as the mockup.
        let rule = if t.glass {
            gpui::transparent_black()
        } else {
            t.rule
        };
        let running = status.running();
        let idle = self.servers.busy.is_none();
        let ops = servers::names(dir, "ops.json");
        let whitelist = servers::entries(dir, "whitelist.json");
        let banned = servers::names(dir, "banned-players.json");
        let is_op = |n: &str| ops.iter().any(|o| o.eq_ignore_ascii_case(n));
        let mut tab = div().flex().flex_col().gap(px(28.));
        // The glass look's rows are the mockup's: 28 px heads, the name in 14 px medium, its
        // note in 13 px.
        let glass = t.glass;
        let head = if glass { 28. } else { 24. };
        let person = move |name: String| {
            text(theme::UI)
                .text_color(t.ink)
                .when(glass, |d| d.font_weight(FontWeight::MEDIUM))
                .child(name)
        };
        let person_note = move |note: String| {
            text(theme::CAPTION)
                .text_color(t.muted)
                .when(glass, |d| d.text_size(px(13.)))
                .child(note)
        };

        // Online now.
        let mut online = div().flex().flex_col().border_t_1().border_color(rule);
        for (i, p) in status.players.clone().into_iter().enumerate() {
            let op = is_op(&p.name);
            let (n1, n2, n3) = (p.name.clone(), p.name.clone(), p.name.clone());
            let actions = div()
                .flex()
                .gap(px(2.))
                .child(self.small(
                    SharedString::from(format!("online-op-{i}")),
                    if op {
                        "Remove operator"
                    } else {
                        "Make operator"
                    },
                    idle,
                    cx,
                    move |this| {
                        let verb = if op { "deop" } else { "op" };
                        this.server_command("Changing operators", vec![verb.into(), n1.clone()]);
                    },
                ))
                .child(self.small(
                    SharedString::from(format!("online-kick-{i}")),
                    "Kick",
                    running,
                    cx,
                    move |this| this.api_command(format!("kick {n2}")),
                ))
                .child(self.small(
                    SharedString::from(format!("online-ban-{i}")),
                    "Ban",
                    running,
                    cx,
                    move |this| this.api_command(format!("ban {n3}")),
                ));
            let ping = p
                .ping
                .map(|ms| trf("{ms} ms ping", &[("ms", &ms.to_string())]));
            let about = match (op, ping) {
                (true, Some(ping)) => Some(trf("Operator · {ping}", &[("ping", &ping)])),
                (true, None) => Some(tr("Operator").to_string()),
                (false, ping) => ping,
            };
            online = online.child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(14.))
                    .min_h(px(theme::ROW))
                    .py(px(8.))
                    .when(glass, |d| d.min_h(px(0.)).py(px(5.)))
                    .border_b_1()
                    .border_color(rule)
                    .child(self.head(&p.uuid, head))
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .flex_1()
                            .child(person(p.name.clone()))
                            .children(about.map(person_note)),
                    )
                    .child(actions),
            );
        }
        tab = tab.child(
            div()
                .flex()
                .flex_col()
                .gap(px(10.))
                .child(self.tab_heading(trf(
                    "Online now ({count})",
                    &[("count", &status.players.len().to_string())],
                )))
                .child(if status.players.is_empty() {
                    text(theme::UI)
                        .text_color(t.muted)
                        .child(if running {
                            tr("Nobody is playing right now.")
                        } else {
                            tr("The server is stopped.")
                        })
                        .into_any_element()
                } else {
                    online.into_any_element()
                }),
        );

        // Who may join: the friends-only switch, then a card per person. The cards are the
        // student's friends and the students whose launcher is open now (not the college's two
        // hundred computers): a card's switch makes a friend, who is let in on every server the
        // student creates and at once on this one. Its buttons say what the friend may do here.
        let friends_only = self
            .servers
            .setting("server", "whitelist")
            .and_then(Value::as_bool)
            .unwrap_or(true);
        let toggle = switch("friends-only", friends_only, &t).on_click(cx.listener(
            move |this, _: &ClickEvent, _, cx| {
                this.set_setting("server.whitelist", (!friends_only).to_string());
                cx.notify();
            },
        ));
        let mut people: Vec<(String, String)> = whitelist.clone();
        for op in &ops {
            if !people.iter().any(|(n, _)| n.eq_ignore_ascii_case(op)) {
                people.push((op.clone(), String::new()));
            }
        }
        let cards = self.servers.cards(&people);
        let me = self.player.clone();
        let mut allowed = div().flex().flex_col().border_t_1().border_color(rule);
        for (i, card) in cards.into_iter().enumerate() {
            let name = card.name.clone();
            let op = is_op(&name);
            let listed = whitelist.iter().any(|(n, _)| n == &name);
            let mine = me.as_deref() == Some(name.as_str());
            let face = match card.uuid {
                Some(uuid) if !uuid.is_empty() => self.head(&uuid, head),
                _ => div().size(px(head)).flex_none(),
            };
            let mut actions = div()
                .flex()
                .flex_wrap()
                .items_center()
                .justify_end()
                .gap(px(2.));
            if !mine {
                let (n1, n2, n3) = (name.clone(), name.clone(), name.clone());
                actions = actions
                    .child(self.small(
                        SharedString::from(format!("card-op-{i}")),
                        if op {
                            "Remove operator"
                        } else {
                            "Make operator"
                        },
                        idle,
                        cx,
                        move |this| {
                            let verb = if op { "deop" } else { "op" };
                            this.server_command(
                                "Changing operators",
                                vec![verb.into(), n1.clone()],
                            );
                        },
                    ))
                    .child(self.small(
                        SharedString::from(format!("card-list-{i}")),
                        if listed { "Remove" } else { "Let in" },
                        idle,
                        cx,
                        move |this| {
                            let verb = if listed { "remove" } else { "add" };
                            this.server_command(
                                if listed {
                                    "Updating the list"
                                } else {
                                    "Letting someone in"
                                },
                                vec!["whitelist".into(), verb.into(), n2.clone()],
                            );
                        },
                    ))
                    .child(self.small(
                        SharedString::from(format!("card-ban-{i}")),
                        "Ban",
                        running,
                        cx,
                        move |this| this.api_command(format!("ban {n3}")),
                    ));
            }
            let notes: Vec<String> = if mine {
                vec![tr("You: this server's operator").to_string()]
            } else {
                vec![
                    op.then(|| {
                        if listed {
                            tr("Operator: can run every command")
                        } else {
                            tr("Operator (not on the list: can join anyway)")
                        }
                        .to_string()
                    }),
                    card.online.then(|| tr("Launcher open").to_string()),
                ]
                .into_iter()
                .flatten()
                .collect()
            };
            // The switch that makes a friend, its name beside it; not on the student's own card.
            let (n4, folder) = (name.clone(), dir.to_path_buf());
            let friend = (!mine).then(|| {
                div()
                    .flex()
                    .flex_none()
                    .items_center()
                    .gap(px(8.))
                    .child(
                        text(theme::CAPTION)
                            .text_color(if card.friend { t.success } else { t.muted })
                            .child(tr("Friend")),
                    )
                    .child(
                        switch(
                            SharedString::from(format!("card-friend-{i}")),
                            card.friend,
                            &t,
                        )
                        .on_click(cx.listener(
                            move |this, _: &ClickEvent, _, cx| {
                                this.toggle_friend(&n4, Some(&folder));
                                cx.notify();
                            },
                        )),
                    )
            });
            allowed = allowed.child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(14.))
                    .min_h(px(theme::ROW))
                    .py(px(8.))
                    .when(glass, |d| d.min_h(px(0.)))
                    .border_b_1()
                    .border_color(rule)
                    .child(face)
                    .child(
                        div()
                            .flex()
                            .flex_1()
                            .min_w_0()
                            .flex_col()
                            .gap(px(2.))
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(8.))
                                    // A green dot: their launcher is open right now.
                                    .when(card.online, |d| {
                                        d.child(div().flex_none().size(px(8.)).bg(t.success))
                                    })
                                    .child(person(name)),
                            )
                            .children(notes.into_iter().map(person_note)),
                    )
                    .children(friend)
                    .child(actions),
            );
        }
        tab = tab.child(
            div()
                .flex()
                .flex_col()
                .gap(px(14.))
                .child(self.tab_heading(tr("Who may join")))
                .child(
                    self.toggle_row(
                        "Friends only",
                        if friends_only {
                            tr("On: you, your friends and the players let in below can join.")
                        } else {
                            tr("Off: everyone on the club's network can join.")
                        }
                        .into(),
                        toggle,
                    ),
                )
                .child(text(theme::CAPTION).text_color(t.muted).child(tr(
                    "One card for each friend and for each student whose launcher is open now. \
                     The switch makes a friend, let in on every server you create.",
                )))
                .child(allowed),
        );

        // Banned.
        if !banned.is_empty() {
            let mut list = div().flex().flex_col().border_t_1().border_color(rule);
            for (i, name) in banned.into_iter().enumerate() {
                let n = name.clone();
                list = list.child(
                    self.list_row(
                        None,
                        name,
                        None,
                        Some(
                            self.small(
                                SharedString::from(format!("unban-{i}")),
                                "Unban",
                                running,
                                cx,
                                move |this| this.api_command(format!("pardon {n}")),
                            )
                            .into_any_element(),
                        ),
                    ),
                );
            }
            tab = tab.child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(10.))
                    .child(self.tab_heading(tr("Banned")))
                    .when(!running, |d| {
                        d.child(
                            text(theme::CAPTION)
                                .text_color(t.muted)
                                .child(tr("Start the server to unban someone.")),
                        )
                    })
                    .child(list),
            );
        }
        tab
    }

    /// A row of the settings form: a name, one line on it, and its control.
    fn setting_row(&self, name: &str, about: String, control: AnyElement) -> Div {
        self.setting_row_checked(name, about, None, control)
    }

    /// A setting row whose text gives way to `error` when the value was refused.
    fn setting_row_checked(
        &self,
        name: &str,
        about: String,
        error: Option<&'static str>,
        control: AnyElement,
    ) -> Div {
        let t = self.theme;
        if t.glass {
            return glass_setting_row(name, about, error, control);
        }
        div()
            .flex()
            .items_center()
            .gap(px(16.))
            .min_h(px(theme::ROW))
            .py(px(12.))
            .border_b_1()
            .border_color(t.rule)
            .child(
                div()
                    .flex()
                    .flex_col()
                    .flex_1()
                    .min_w_0()
                    .gap(px(2.))
                    .child(
                        text(theme::UI)
                            .text_color(t.ink)
                            .child(tr(name).to_string()),
                    )
                    .child(match error {
                        Some(e) => text(theme::CAPTION).text_color(t.danger).child(tr(e)),
                        None => text(theme::CAPTION).text_color(t.muted).child(about),
                    }),
            )
            .child(control)
    }

    /// − value + for a number setting within `range`.
    fn stepper(
        &self,
        key: &'static str,
        range: std::ops::RangeInclusive<i64>,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let t = self.theme;
        let (section, field) = key.split_once('.').unwrap_or_default();
        let value: i64 = self.servers.shown(section, field).parse().unwrap_or(0);
        if t.glass {
            return self.glass_stepper(key, value, range, cx);
        }
        let make = |id: &'static str, label: &'static str, to: i64, cx: &mut Context<Self>| {
            let b = button(
                SharedString::from(format!("{key}-{id}")),
                label,
                Kind::Secondary,
                &t,
            )
            .w(px(36.))
            .px(px(0.));
            if range.contains(&to) && to != value {
                b.on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                    this.set_setting(key, to.to_string());
                    cx.notify();
                }))
            } else {
                disabled(b)
            }
        };
        div()
            .flex()
            .items_center()
            .gap(px(8.))
            .child(make("less", "−", value - 1, cx))
            .child(
                text(theme::UI)
                    .w(px(44.))
                    .flex()
                    .justify_center()
                    .text_color(t.ink)
                    .child(value.to_string()),
            )
            .child(make("more", "+", value + 1, cx))
            .into_any_element()
    }

    fn choice(
        &self,
        id: &'static str,
        key: &'static str,
        options: &[(&'static str, &'static str)],
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let (section, field) = key.split_once('.').unwrap_or_default();
        let current = self.servers.shown(section, field);
        let current: &'static str = options
            .iter()
            .find(|(v, _)| *v == current)
            .map_or("", |(v, _)| v);
        let this = cx.entity().downgrade();
        segments(id, options, current, &self.theme, move |value, _, cx| {
            let _ = this.update(cx, |this, cx| {
                this.set_setting(key, value.to_string());
                cx.notify();
            });
        })
        .into_any_element()
    }

    fn settings_tab(&mut self, status: &Status, cx: &mut Context<Self>) -> Div {
        let t = self.theme;
        // The glass look draws no rules between rows, as the mockup.
        let rule = if t.glass {
            gpui::transparent_black()
        } else {
            t.rule
        };
        let running = status.running();
        if self.servers.config.is_none() {
            return div().child(centered(tr("Reading the settings…"), &t));
        }
        if self.servers.focused != Some(Field::Motd)
            && !self.servers.texts.contains_key(&Field::Motd)
        {
            let motd = self.servers.shown("server", "motd");
            self.servers.text(Field::Motd).set(motd);
        }
        let save = |id: &'static str, f: Field, cx: &mut Context<Self>| {
            button(id, "Save", Kind::Secondary, &t).on_click(cx.listener(
                move |this, _: &ClickEvent, _, cx| {
                    this.field_enter(f);
                    cx.notify();
                },
            ))
        };
        // 240 px: the server's column is narrow beside the side column, and the row's name and
        // note keep a line more of their own (the field scrolls for a longer message).
        let motd = div()
            .flex()
            .items_center()
            .gap(px(8.))
            .child(self.field(Field::Motd, tr("A Minecraft Server"), 240., cx))
            .child(save("motd-save", Field::Motd, cx))
            .into_any_element();
        let pvp = match (running, self.servers.pvp) {
            (true, Some(on)) => switch("pvp", on, &t)
                .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                    this.servers.pvp = Some(!on);
                    this.api_command(format!("gamerule pvp {}", !on));
                    cx.notify();
                }))
                .into_any_element(),
            _ => disabled(switch("pvp", true, &t)).into_any_element(),
        };
        let live = if running {
            tr("Applies at once.")
        } else {
            tr("Used from the next start.")
        };
        let later = if running {
            tr("Saved now, used after a restart.")
        } else {
            tr("Used from the next start.")
        };
        // Vanilla's own switches, for a club: no monster to deal with while building, no Nether
        // to get lost in. Each is one of the server's `server.properties`, read at its next start;
        // vanilla's default (on) is what an unwritten one means.
        let vanilla = |key: &str| {
            self.servers
                .setting("properties", key)
                .and_then(Value::as_bool)
                .unwrap_or(true)
        };
        let monsters = vanilla("spawn-monsters");
        let nether = vanilla("allow-nether");
        let monsters_switch = switch("set-monsters", monsters, &t).on_click(cx.listener(
            move |this, _: &ClickEvent, _, cx| {
                this.set_setting("properties.spawn-monsters", (!monsters).to_string());
                cx.notify();
            },
        ));
        let nether_switch = switch("set-nether", nether, &t).on_click(cx.listener(
            move |this, _: &ClickEvent, _, cx| {
                this.set_setting("properties.allow-nether", (!nether).to_string());
                cx.notify();
            },
        ));
        let form = div()
            .flex()
            .flex_col()
            .border_t_1()
            .border_color(rule)
            .child(self.setting_row(
                "Message in the server list",
                format!(
                    "{} {later}",
                    tr("What players see under the server's name (the MOTD).")
                ),
                motd,
            ))
            .child(self.setting_row(
                "Game mode",
                format!("{} {live}", tr("The server's mode, for every player.")),
                self.choice("set-mode", "world.game_mode", &MODES, cx),
            ))
            .child(self.setting_row(
                "Difficulty",
                live.into(),
                self.choice("set-difficulty", "world.difficulty", &DIFFICULTIES, cx),
            ))
            .child(self.setting_row(
                "Monsters spawn",
                format!(
                    "{} {later}",
                    tr("Off: no zombie, creeper or skeleton comes. Animals stay.")
                ),
                monsters_switch.into_any_element(),
            ))
            .child(self.setting_row(
                "The Nether",
                format!(
                    "{} {later}",
                    tr("Off: portals don't light, so nobody leaves for the Nether.")
                ),
                nether_switch.into_any_element(),
            ))
            .child(self.setting_row(
                "Players can hurt each other (PvP)",
                if running {
                    tr("A rule of the world. Applies at once.").into()
                } else {
                    tr("A rule of the world: start the server to change it.").into()
                },
                pvp,
            ))
            .child(self.setting_row(
                "Most players at once",
                format!("{} {later}", tr("Operators can always join.")),
                self.stepper("server.max_players", 1..=100, cx),
            ))
            .child(self.setting_row(
                "View distance",
                format!(
                    "{} {later}",
                    tr("How far players see, in chunks (2 to 32). Lower is lighter.")
                ),
                self.stepper("server.view_distance", 2..=32, cx),
            ))
            .child(self.setting_row(
                "Simulation distance",
                format!(
                    "{} {later}",
                    tr("How far from players crops grow and mobs move, in chunks.")
                ),
                self.stepper("server.simulation_distance", 2..=32, cx),
            ))
            .child(self.delete_server_section(status, cx));
        div()
            .flex()
            .flex_col()
            .gap(px(12.))
            .child(text(theme::CAPTION).text_color(t.muted).child(tr(
                "Changes save as you make them. Those the game can't take while it runs wait for \
                 a restart, and the page says so.",
            )))
            .child(form)
    }

    fn delete_server_section(&mut self, status: &Status, cx: &mut Context<Self>) -> Div {
        let t = self.theme;
        let name = self
            .servers
            .list
            .iter()
            .find(|entry| Some(&entry.dir) == self.servers.open.as_ref())
            .map(|entry| entry.name.clone())
            .unwrap_or_default();
        let ready = !status.running()
            && self.servers.busy.is_none()
            && !self.servers.applying
            && self.servers.queued.is_empty();
        let hint = if status.running() {
            Some(tr("Stop the server before deleting it."))
        } else if !ready {
            Some(tr("Wait for the current operation to finish."))
        } else {
            None
        };
        let mut controls = div()
            .flex()
            .flex_col()
            .items_end()
            .flex_none()
            .w(px(260.))
            .gap(px(12.));
        if self.servers.confirm_delete {
            let matches = self.servers.value(Field::DeleteServer) == name && !name.is_empty();
            let field = self.field(Field::DeleteServer, tr("Server name"), 225., cx);
            let confirm = button(
                "server-delete-confirm",
                "Move to trash",
                Kind::Secondary,
                &t,
            )
            .text_color(t.danger);
            let confirm = if ready && matches {
                confirm.on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                    if let Some(dir) = this.servers.open.clone() {
                        this.servers.busy = Some("Moving server to trash".into());
                        let tx = this.servers.tx.clone();
                        std::thread::spawn(move || {
                            let result = servers::delete(&dir);
                            let _ = tx.send(Event::Deleted { dir, result });
                        });
                    }
                    cx.notify();
                }))
            } else {
                disabled(confirm)
            };
            let line = tr("Type {name} to confirm.");
            let (before, after) = line.split_once("{name}").unwrap_or((line, ""));
            let confirm_text = format!("{before}{name}{after}");
            let lit = before.len()..before.len() + name.len();
            controls = controls
                .child(field)
                .child(text(theme::CAPTION).text_color(t.muted).child(
                    gpui::StyledText::new(confirm_text).with_highlights([(
                        lit,
                        gpui::HighlightStyle {
                            font_weight: Some(FontWeight::BOLD),
                            ..Default::default()
                        },
                    )]),
                ))
                .child(
                    div()
                        .flex()
                        .gap(px(8.))
                        .child(
                            button("server-delete-cancel", "Cancel", Kind::Ghost, &t).on_click(
                                cx.listener(|this, _: &ClickEvent, _, cx| {
                                    this.servers.confirm_delete = false;
                                    cx.notify();
                                }),
                            ),
                        )
                        .child(confirm),
                );
        } else {
            let delete =
                button("server-delete", "Delete server…", Kind::Secondary, &t).text_color(t.danger);
            let delete = if ready {
                delete.on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                    this.servers.confirm_delete = true;
                    this.servers.text(Field::DeleteServer).set(String::new());
                    this.focus_handle(Field::DeleteServer, cx);
                    this.servers.focus_soon = Some(Field::DeleteServer);
                    cx.notify();
                }))
            } else {
                disabled(delete)
            };
            controls = controls.child(delete);
        }
        if let Some(hint) = hint {
            controls = controls.child(
                text(theme::CAPTION)
                    .text_color(t.muted)
                    .w(px(260.))
                    .text_align(gpui::TextAlign::Right)
                    .child(hint),
            );
        }
        self.setting_row(
            "Delete server",
            tr("Remove this server from Horizon and move its worlds, settings and backups to the trash.").into(),
            controls.into_any_element(),
        )
        .items_start()
    }

    fn backups_tab(&mut self, status: &Status, cx: &mut Context<Self>) -> Div {
        let t = self.theme;
        // The glass look draws no rules between rows, as the mockup.
        let rule = if t.glass {
            gpui::transparent_black()
        } else {
            t.rule
        };
        let idle = self.servers.busy.is_none();
        let every = self.servers.shown("backups", "every");
        let auto = !every.is_empty() && every != "0";
        let keep = self.servers.shown("backups", "keep");
        let now = {
            let b = button("backup-now", "Back up now", Kind::Primary, &t);
            if idle {
                b.on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                    this.server_command("Backing up", vec!["backup".into()]);
                    cx.notify();
                }))
            } else {
                disabled(b)
            }
        };
        let toggle = switch("auto-backup", auto, &t).on_click(cx.listener(
            move |this, _: &ClickEvent, _, cx| {
                this.set_setting(
                    "backups.every",
                    if auto {
                        "0".into()
                    } else {
                        servers::AUTO_BACKUP.into()
                    },
                );
                cx.notify();
            },
        ));
        let mut list = div().flex().flex_col().border_t_1().border_color(rule);
        for (i, b) in self.servers.backups.clone().into_iter().enumerate() {
            let (when, kind) = backup_label(&b);
            let confirming = self.servers.confirm_restore.as_deref() == Some(b.name.as_str());
            let name = b.name.clone();
            let restored = format!(
                "The world is back as it was on {when}. The one it replaced is in the backups \
                 folder."
            );
            let restore = button(
                SharedString::from(format!("restore-{i}")),
                if confirming {
                    "Replace the world"
                } else {
                    "Restore"
                },
                if confirming {
                    Kind::Primary
                } else {
                    Kind::Ghost
                },
                &t,
            );
            let restore = if idle {
                restore.on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                    if this.servers.confirm_restore.as_ref() == Some(&name) {
                        this.servers.confirm_restore = None;
                        if let Some((dir, tag)) = this
                            .servers
                            .open
                            .clone()
                            .zip(this.selected().map(str::to_owned))
                        {
                            let (name, restored) = (name.clone(), restored.clone());
                            this.server_job("Restoring", move |log| {
                                servers::run_streamed(&tag, &dir, &["restore", &name, "--yes"], log)
                                    .map(|_| restored)
                            });
                        }
                    } else {
                        this.servers.confirm_restore = Some(name.clone());
                    }
                    cx.notify();
                }))
            } else {
                disabled(restore)
            };
            let mut right = div().flex().items_center().gap(px(8.));
            if confirming {
                right = right.child(
                    button("restore-cancel", "Cancel", Kind::Ghost, &t).on_click(cx.listener(
                        |this, _: &ClickEvent, _, cx| {
                            this.servers.confirm_restore = None;
                            cx.notify();
                        },
                    )),
                );
            }
            right = right.child(restore);
            let detail = if kind.is_empty() {
                bytes(b.bytes)
            } else {
                format!("{kind} · {}", bytes(b.bytes))
            };
            list = list.child(self.list_row(
                None,
                when,
                Some(detail.into_any_element()),
                Some(right.into_any_element()),
            ));
        }
        let mut tab = div().flex().flex_col().gap(px(20.)).child(
            div().flex().items_center().gap(px(16.)).child(now).child(
                text(theme::CAPTION)
                    .text_color(t.muted)
                    .child(if status.running() {
                        tr("Players stay online: the world is saved, then zipped.")
                    } else {
                        tr("Zips the world and the server's settings into its backups folder.")
                    }),
            ),
        );
        tab = tab.child(self.toggle_row(
            "Back up automatically",
            if auto {
                trf(
                    "On: every {every} while the server runs, keeping the last \
                         {keep}. Backups you make yourself are never deleted.",
                    &[("every", &every), ("keep", &keep)],
                )
            } else {
                tr("Off: only the backups you make.").to_string()
            },
            toggle,
        ));
        if self.servers.confirm_restore.is_some() {
            tab = tab.child(notice(
                tr(
                    "Restoring replaces the world with this backup. The world as it is now isn't \
                 deleted: it's moved into the backups folder. A running server stops and starts \
                 again with the restored world.",
                )
                .into(),
                &t,
            ));
        }
        if self.servers.backups.is_empty() {
            tab.child(
                text(theme::UI)
                    .text_color(t.muted)
                    .child(tr("No backup yet.")),
            )
        } else {
            tab.child(list)
        }
    }

    // ----- The Create dialog ----------------------------------------------------------------

    pub(super) fn create_dialog(&mut self, cx: &mut Context<Self>) -> Option<Stateful<Div>> {
        let t = self.theme;
        if t.glass {
            return self.glass_create_dialog(cx);
        }
        let c = self.servers.create.as_ref()?;
        let (new_world, copy, worlds) = (c.new_world, c.copy.clone(), c.worlds.clone());
        let (mode, difficulty, max_players) = (c.mode, c.difficulty, c.max_players);
        let creating = self.servers.creating;
        // A section: its name in capital letters, in the launcher's language, over its control.
        let row = |label: String, control: AnyElement| {
            div()
                .flex()
                .flex_col()
                .gap(px(8.))
                .child(caps(label).text_color(t.muted))
                .child(control)
        };
        // Two short sections share a line in two halves (one under the other when narrow).
        let half = |d: Div| d.flex_1().min_w(px(280.));
        let pair = |left: Div, right: Div| {
            div()
                .flex()
                .flex_wrap()
                .gap(px(20.))
                .child(half(left))
                .child(half(right))
        };
        let weak = cx.entity().downgrade();
        let pick = move |f: fn(&mut Create, &'static str)| {
            let weak = weak.clone();
            move |value: &'static str, _: &mut Window, cx: &mut App| {
                let _ = weak.update(cx, |this, cx| {
                    if let Some(c) = &mut this.servers.create {
                        f(c, value);
                    }
                    cx.notify();
                });
            }
        };
        let world_kind = segments(
            "create-world",
            &[("new", "New world"), ("copy", "Copy one of my worlds")],
            if new_world { "new" } else { "copy" },
            &t,
            pick(|c, v| c.new_world = v == "new"),
        );
        let world_detail = if new_world {
            self.field(
                Field::Seed,
                "Seed (optional: leave empty for a random world)",
                400.,
                cx,
            )
            .w_full()
            .into_any_element()
        } else if worlds.is_empty() {
            text(theme::CAPTION)
                .text_color(t.muted)
                .child(tr(
                    "You have no single-player world yet: play one first, or choose New world.",
                ))
                .into_any_element()
        } else {
            let mut list = div().flex().flex_col().gap(px(2.));
            for (i, (name, path)) in worlds.into_iter().take(8).enumerate() {
                let chosen = copy.as_ref() == Some(&path);
                list = list.child(
                    div()
                        .id(SharedString::from(format!("create-copy-{i}")))
                        .flex()
                        .items_center()
                        .gap(px(10.))
                        .h(px(32.))
                        .px(px(10.))
                        .rounded(px(theme::RADIUS))
                        .cursor_pointer()
                        .when(chosen, |d| d.bg(t.plate_soft))
                        .hover(move |s| s.bg(t.plate_soft))
                        .child(
                            div()
                                .size(px(12.))
                                .border_1()
                                .border_color(if chosen { t.accent } else { t.muted })
                                .when(chosen, |d| d.bg(t.accent)),
                        )
                        .child(text(theme::UI).text_color(t.ink).child(name))
                        .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                            if let Some(c) = &mut this.servers.create {
                                c.copy = Some(path.clone());
                            }
                            cx.notify();
                        })),
                );
            }
            div()
                .flex()
                .flex_col()
                .gap(px(6.))
                .child(list)
                .child(text(theme::CAPTION).text_color(t.muted).child(tr(
                    "The server gets a copy: your single-player world stays as it is.",
                )))
                .into_any_element()
        };
        let modes = segments("create-mode", &MODES, mode, &t, pick(|c, v| c.mode = v));
        let difficulties = segments(
            "create-difficulty",
            &DIFFICULTIES,
            difficulty,
            &t,
            pick(|c, v| c.difficulty = v),
        );
        let step = |id: &'static str, label: &'static str, delta: i32, cx: &mut Context<Self>| {
            button(id, label, Kind::Secondary, &t)
                .w(px(36.))
                .px(px(0.))
                .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                    if let Some(c) = &mut this.servers.create {
                        c.max_players = (c.max_players as i32 + delta).clamp(1, 100) as u32;
                    }
                    cx.notify();
                }))
        };
        let players = div()
            .flex()
            .items_center()
            .gap(px(8.))
            .child(step("create-fewer", "−", -1, cx))
            .child(
                // As tall as its buttons: the number sits in their middle.
                text(theme::UI)
                    .w(px(44.))
                    .h(px(36.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_color(t.ink)
                    .child(max_players.to_string()),
            )
            .child(step("create-more", "+", 1, cx));
        let body = div()
            .id("create-body")
            .flex()
            .flex_col()
            .min_h_0()
            .gap(px(20.))
            .px(px(28.))
            .pt(px(28.))
            .pb(px(20.))
            .overflow_y_scroll()
            .child(caps(tr("New server")).text_color(t.muted))
            .child(
                text(theme::HEADING)
                    .text_color(t.ink)
                    .child(tr("Create a server")),
            )
            .child(text(theme::CAPTION).text_color(t.muted).child(tr(
                "Everything has a default: Create makes a friends-only survival world. You can \
                 change all of it later, from the server's Settings tab.",
            )))
            .child(row(
                tr("Name").to_string(),
                self.field(Field::Name, "My server", 400., cx)
                    .w_full()
                    .into_any_element(),
            ))
            .child(row(
                tr("World").to_string(),
                div()
                    .flex()
                    .flex_col()
                    .gap(px(10.))
                    .child(div().flex().flex_wrap().child(world_kind))
                    .child(world_detail)
                    .into_any_element(),
            ))
            .child(pair(
                row(
                    tr("Game mode").to_string(),
                    div().flex().flex_wrap().child(modes).into_any_element(),
                ),
                row(
                    tr("Difficulty").to_string(),
                    div()
                        .flex()
                        .flex_wrap()
                        .child(difficulties)
                        .into_any_element(),
                ),
            ))
            .child(row(
                tr("Most players at once").to_string(),
                players.into_any_element(),
            ))
            .child(row(
                tr("Who can join").to_string(),
                div()
                    .flex()
                    .flex_col()
                    .gap(px(10.))
                    .child(text(theme::CAPTION).text_color(t.muted).child(tr(
                        "The server is yours: you are its operator. Your friends are let in from \
                         the start, on every server you create.",
                    )))
                    .child(self.create_cards(cx))
                    .into_any_element(),
            ));
        let mut foot = div()
            .flex()
            .flex_none()
            .items_center()
            .justify_end()
            .gap(px(12.))
            .px(px(28.))
            .pt(px(8.))
            .pb(px(28.));
        if creating {
            let last = self.servers.job_log.back().cloned().unwrap_or_default();
            foot = foot.child(
                text(theme::CAPTION)
                    .flex_1()
                    .min_w_0()
                    .text_color(t.muted)
                    .truncate()
                    .child(format!("{} {last}", tr("Creating…"))),
            );
        } else if let Some(p) = &self.servers.problem {
            foot = foot.child(
                text(theme::CAPTION)
                    .flex_1()
                    .min_w_0()
                    .text_color(t.danger)
                    .child(p.message.clone()),
            );
        }
        let cancel = button("create-cancel", tr("Cancel"), Kind::Secondary, &t);
        let create = button("create-go", tr("Create"), Kind::Primary, &t);
        foot = if creating {
            foot.child(disabled(cancel)).child(disabled(create))
        } else {
            foot.child(cancel.on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                this.servers.create = None;
                this.servers.problem = None;
                cx.notify();
            })))
            .child(create.on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                this.create_server();
                cx.notify();
            })))
        };
        let card = plate(&t)
            .flex()
            .flex_col()
            .w_full()
            .max_w(px(760.))
            .max_h_full()
            .bg(t.page)
            .border_1()
            .border_color(t.rule)
            .child(self.servers.create_scroll.wrap(body, &t))
            .child(foot);
        Some(
            div()
                .id("create-dialog")
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
                .child(card),
        )
    }
    /// The Create dialog's cards in the blocky look: one for each friend and for each student
    /// whose launcher is open now. A click makes a friend, or takes one away: the card is then
    /// the accent's, and a friend is let in on the server from the start.
    fn create_cards(&mut self, cx: &mut Context<Self>) -> Div {
        let t = self.theme;
        let cards = self.servers.cards(&[]);
        let mut grid = div().flex().flex_wrap().gap(px(8.));
        for (i, card) in cards.into_iter().enumerate() {
            let name = card.name.clone();
            let first = name.chars().next().map(|c| c.to_uppercase().to_string());
            grid = grid.child(
                keyboard(
                    div()
                        .id(SharedString::from(format!("create-card-{i}")))
                        .flex()
                        .flex_1()
                        .min_w(px(220.))
                        .flex_basis(px(0.))
                        .items_center()
                        .gap(px(10.))
                        .h(px(56.))
                        .px(px(10.))
                        .rounded(px(theme::RADIUS))
                        .border_1()
                        .border_color(if card.friend { t.accent } else { t.rule })
                        .bg(if card.friend { t.plate_soft } else { t.surface })
                        .cursor_pointer()
                        .hover(|s| s.bg(t.plate_soft))
                        .child(
                            div()
                                .flex()
                                .flex_none()
                                .items_center()
                                .justify_center()
                                .size(px(30.))
                                .rounded(px(theme::RADIUS))
                                .bg(t.plate)
                                .child(text(theme::UI).text_color(t.ink_soft).children(first)),
                        )
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .flex_1()
                                .min_w_0()
                                .child(
                                    text(theme::UI)
                                        .text_color(t.ink)
                                        .truncate()
                                        .child(name.clone()),
                                )
                                .children(card.online.then(|| {
                                    text(theme::CAPTION)
                                        .text_color(t.success)
                                        .child(tr("Launcher open"))
                                })),
                        )
                        // The box of a friend is the accent's; the others' wait empty.
                        .child(
                            div()
                                .flex_none()
                                .size(px(18.))
                                .rounded(px(4.))
                                .border_2()
                                .border_color(if card.friend { t.accent } else { t.muted })
                                .when(card.friend, |d| d.bg(t.accent)),
                        ),
                    &t,
                )
                .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                    this.toggle_friend(&name, None);
                    cx.notify();
                })),
            );
        }
        if self.servers.people.is_empty() && self.servers.friends.is_empty() {
            return grid.child(text(theme::CAPTION).text_color(t.muted).child(tr(
                "Nobody else has Horizon Club open right now. Their cards appear here as \
                         they open it.",
            )));
        }
        grid
    }
}

/// The state, in a few words.
fn phase_label(status: &Status) -> String {
    match status.phase.as_str() {
        "running" if status.crashes > 0 => tr("Running · restarted after a crash").into(),
        "running" if status.paused => tr("Running · paused while nobody plays").into(),
        "running" => tr("Running").into(),
        "starting" => tr("Starting").into(),
        "restarting" => tr("Restarting").into(),
        "stopping" => tr("Stopping").into(),
        _ => tr("Stopped").into(),
    }
}

/// A setting's name for people.
fn setting_name(key: &str) -> &str {
    match key {
        "server.motd" => "the message in the server list",
        "server.port" => "the port",
        "server.max_players" => "most players",
        "server.view_distance" => "view distance",
        "server.simulation_distance" => "simulation distance",
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backups_read_for_people() {
        let b = |name: &str| Backup {
            name: name.into(),
            bytes: 1,
            modified: None,
        };
        assert_eq!(
            backup_label(&b("2026-10-19_11-28-57-manual.zip")),
            ("2026-10-19 11:28:57 UTC".into(), "Made by hand".into())
        );
        assert_eq!(
            backup_label(&b("2026-10-19_11-28-57-before-v2026.10.20.zip")).1,
            "Before updating to v2026.10.20"
        );
        assert_eq!(
            backup_label(&b("2026-10-19_13-35-36-before-v2026-10-19-10.zip")).1,
            "Before updating to v2026.10.19.10"
        );
        assert_eq!(
            backup_label(&b("2026-10-14_17-46-40-before-update.zip")).1,
            "Before an update"
        );
        assert_eq!(
            backup_label(&b("2026-10-19_11-28-57-auto.zip")).1,
            "Automatic"
        );
        assert_eq!(backup_label(&b("odd.zip")), ("odd".into(), String::new()));
        assert_eq!(bytes(5_292_542), "5.3 MB");
        assert_eq!(uptime(3725), "1 h 02 min");
    }
}
