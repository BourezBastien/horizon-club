//! Servers hosted from the launcher (docs/school-edition/launcher-target.md, section 7). Every
//! server is a `horizon-server` folder of its own in the launcher's data folder, and the launcher
//! drives the supervisor (`horizon-server`) rather than its own copy of the logic: `init`,
//! `start`, `stop`, `config set`, `backup`, `restore`, and the control API the running supervisor
//! serves on 127.0.0.1 for the live status, the log and console commands.
//!
//! Nothing is copied: the supervisor and the engine it starts (`horizon-client --dedicated`) run
//! in place from the build in use, `<root>\core\<tag>\`, read-only on the share, and the server's
//! `server.toml` points `[engine] resources` at that build's prepared folder. A hosted server
//! plays offline (`online_mode = false`: the edition has no accounts) and takes a port from the
//! club's `[hosting]` range. It lives as long as its launcher window, which stops the servers it
//! started when it closes, and announces it in `<root>\online\` while it runs (`online.rs`).

use crate::backend;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Duration;

const PROGRAM: &str = if cfg!(windows) {
    "horizon-server.exe"
} else {
    "horizon-server"
};

// ----- The list of servers -----------------------------------------------------------------------

/// One server the launcher knows: its name and its folder on this computer.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Entry {
    pub name: String,
    pub dir: PathBuf,
}

/// The game mode a server's `server.toml` sets (`[world] game_mode`), read from the file
/// without asking `horizon-server`: the Servers page's list says it for each stopped server.
pub fn game_mode(dir: &Path) -> Option<String> {
    file_setting(dir, "world", "game_mode")
}

/// A setting a server's `server.toml` holds, read from the file without asking `horizon-server`:
/// the page's lists say of a stopped server what it will be, without starting it.
pub fn file_setting(dir: &Path, section: &str, key: &str) -> Option<String> {
    let text = fs::read_to_string(dir.join("server.toml")).ok()?;
    let config: toml::Value = toml::from_str(&text).ok()?;
    let value = config.get(section)?.get(key)?;
    value
        .as_str()
        .map(str::to_string)
        .or_else(|| value.as_integer().map(|n| n.to_string()))
}

/// How many finished backups a server's folder holds (`backups\*.zip`, as `horizon-server
/// backups` counts them), read from the folder alone.
pub fn backup_count(dir: &Path) -> usize {
    fs::read_dir(dir.join("backups"))
        .into_iter()
        .flatten()
        .flatten()
        .filter(|entry| {
            entry.file_type().is_ok_and(|t| t.is_file())
                && entry.path().extension().is_some_and(|e| e == "zip")
        })
        .count()
}

fn list_file() -> PathBuf {
    backend::data_dir().join("servers.json")
}

/// The servers, in the order they were added; folders that are gone are left out.
pub fn list() -> Vec<Entry> {
    let entries: Vec<Entry> = fs::read_to_string(list_file())
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default();
    entries.into_iter().filter(|e| e.dir.is_dir()).collect()
}

fn save_list(entries: &[Entry]) -> Result<(), String> {
    let path = list_file();
    fs::create_dir_all(backend::data_dir()).map_err(|e| e.to_string())?;
    let text = serde_json::to_string_pretty(entries).map_err(|e| e.to_string())?;
    // A torn write would drop every server from the sidebar: write beside
    // the list, flush it, and rename it into place. Without the flush a
    // power cut can leave a file of the right length full of zeros.
    let partial = path.with_extension("json.tmp");
    let mut file = fs::File::create(&partial).map_err(|e| e.to_string())?;
    file.write_all(text.as_bytes())
        .and_then(|()| file.sync_all())
        .map_err(|e| e.to_string())?;
    drop(file);
    fs::rename(&partial, &path).map_err(|e| e.to_string())
}

/// Adds a server folder to the list (once).
pub fn add(entry: Entry) -> Result<(), String> {
    let mut all = list();
    all.retain(|e| e.dir != entry.dir);
    all.push(entry);
    save_list(&all)
}

/// Takes a server off the list; its folder stays.
pub fn forget(dir: &Path) -> Result<(), String> {
    let mut all = list();
    all.retain(|e| e.dir != dir);
    save_list(&all)
}

/// Removes a registered, stopped server, retaining its files in the system trash.
pub fn delete(dir: &Path) -> Result<(), String> {
    if !list().iter().any(|entry| entry.dir == dir) {
        return Err(
            crate::i18n::tr("This server is no longer in the launcher's list.").to_string(),
        );
    }
    trash_stopped_server(dir, crate::desktop_seams::to_trash)?;
    forget(dir).map_err(|e| {
        crate::i18n::trf(
            "The server was moved to the trash, but its list could not be saved: {error}",
            &[("error", &e)],
        )
    })
}

fn trash_stopped_server(
    dir: &Path,
    trash: impl FnOnce(&Path) -> Result<(), String>,
) -> Result<(), String> {
    if dir
        .symlink_metadata()
        .map_err(|e| e.to_string())?
        .file_type()
        .is_symlink()
        || !dir.join("server.toml").is_file()
    {
        return Err(crate::i18n::tr("This is not a server folder.").to_string());
    }
    let state = dir.join(".horizon");
    fs::create_dir_all(&state).map_err(|e| e.to_string())?;
    // The supervisor holds this same lock throughout starting, running and stopping.
    // Hold it through the move so a stale UI/API status cannot delete a live server.
    let lock = fs::File::options()
        .create(true)
        .truncate(false)
        .write(true)
        .open(state.join("lock"))
        .map_err(|e| e.to_string())?;
    lock.try_lock()
        .map_err(|_| crate::i18n::tr("Stop the server before deleting it.").to_string())?;
    // Windows can't move a folder while a file in it is open, the lock included: let it go
    // first. Still safe: a supervisor starting in between opens the lock again, and then the
    // move fails instead of taking a live server.
    #[cfg(windows)]
    drop(lock);
    trash(dir).map_err(|e| {
        crate::i18n::trf(
            "Could not move the server to the trash: {error}",
            &[("error", &e)],
        )
    })
}

/// A folder name for a server name: lower case letters, digits and dashes.
pub fn slug(name: &str) -> String {
    let mut out = String::new();
    for c in name.trim().chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_lowercase());
        } else if !out.ends_with('-') {
            out.push('-');
        }
    }
    let out = out.trim_matches('-').to_string();
    if out.is_empty() {
        "server".into()
    } else {
        out
    }
}

/// A new folder for a server called `name`, under `servers/` in the data folder.
pub fn new_folder(name: &str) -> PathBuf {
    let root = backend::data_dir().join("servers");
    let base = slug(name);
    let mut dir = root.join(&base);
    let mut n = 2;
    while dir.exists() {
        dir = root.join(format!("{base}-{n}"));
        n += 1;
    }
    dir
}

// ----- The programs: the build in use, read in place ----------------------------------------------

/// `horizon-server` of the build in use, `<root>\core\<tag>\horizon-server.exe`, read in place
/// from the share: nothing is copied, and the supervisor finds the engine and the game's files
/// beside itself.
pub fn program(tag: &str) -> Option<PathBuf> {
    Some(backend::version_dir(tag).join(PROGRAM)).filter(|p| p.is_file())
}

/// Whether the build `tag` can host: it ships `horizon-server` and is prepared. The launcher
/// prepares and installs nothing itself: what is missing is the supervisor's to put in place.
pub fn can_host(tag: &str) -> Result<(), String> {
    if !backend::version_dir(tag).join(PROGRAM).is_file() {
        return Err(crate::i18n::trf(
            "{version} has no server program, so it can't host. Ask the club's supervisor for a \
             build that has one.",
            &[("version", tag)],
        ));
    }
    if !backend::check_prepared(tag) {
        return Err(backend::not_prepared(tag));
    }
    Ok(())
}

// ----- Running horizon-server -------------------------------------------------------------------

/// `horizon-server --dir <dir> <args>`, from the build `tag` in use.
pub fn command(tag: &str, dir: &Path, args: &[&str]) -> Result<Command, String> {
    let program = program(tag).ok_or_else(backend::no_version_installed)?;
    let mut cmd = crate::process::command(program);
    cmd.arg("--dir").arg(dir).args(args);
    Ok(cmd)
}

fn supervisor_log(dir: &Path) -> PathBuf {
    dir.join("logs/supervisor.log")
}

/// The last `lines` of the server's log, where it says why the game didn't start: shown in the
/// window, so no student has to open the folder.
pub fn log_tail(dir: &Path, lines: usize) -> Vec<String> {
    let text = fs::read_to_string(supervisor_log(dir)).unwrap_or_default();
    let all: Vec<&str> = text.lines().collect();
    all[all.len().saturating_sub(lines)..]
        .iter()
        .map(|l| l.to_string())
        .collect()
}

fn log_length(dir: &Path) -> u64 {
    fs::metadata(supervisor_log(dir)).map_or(0, |m| m.len())
}

/// Runs one `horizon-server` command to its end: what it printed, or the problem in plain words.
pub fn run(tag: &str, dir: &Path, args: &[&str]) -> Result<String, Problem> {
    let since = log_length(dir);
    let output = command(tag, dir, args)
        .map_err(Problem::plain)?
        .stdin(Stdio::null())
        .output()
        .map_err(|e| {
            Problem::plain(crate::i18n::trf(
                "Couldn't run horizon-server: {error}",
                &[("error", &e.to_string())],
            ))
        })?;
    let out = String::from_utf8_lossy(&output.stdout).into_owned();
    if output.status.success() {
        Ok(out)
    } else {
        let err = String::from_utf8_lossy(&output.stderr);
        Err(Problem::explain(&format!("{out}\n{err}"), dir, since))
    }
}

/// Runs a long `horizon-server` command, passing each line it prints to `line`.
pub fn run_streamed(
    tag: &str,
    dir: &Path,
    args: &[&str],
    line: &dyn Fn(String),
) -> Result<(), Problem> {
    stream(dir, command(tag, dir, args).map_err(Problem::plain)?, line)
}

/// Launcher-managed Start/Restart: the engine's command as the supervisor's own command
/// description gives it, so that a server without its game files is refused before anything
/// starts; then the supervisor.
pub fn run_managed(
    tag: &str,
    dir: &Path,
    args: &[&str],
    line: &dyn Fn(String),
) -> Result<(), Problem> {
    let supervisor = command(tag, dir, args).map_err(Problem::plain)?;
    horizon_server_launch::engine_command(Path::new(supervisor.get_program()), dir)
        .map_err(|e| Problem::plain(e.to_string()))?;
    stream(dir, supervisor, line)
}

fn stream(dir: &Path, mut command: Command, line: &dyn Fn(String)) -> Result<(), Problem> {
    use std::io::{BufRead, BufReader, Read};
    let since = log_length(dir);
    let mut child = command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| {
            Problem::plain(crate::i18n::trf(
                "Couldn't run horizon-server: {error}",
                &[("error", &e.to_string())],
            ))
        })?;
    let (tx_err, rx_err) = std::sync::mpsc::channel();
    let mut err = child.stderr.take();
    std::thread::spawn(move || {
        let mut text = String::new();
        if let Some(e) = &mut err {
            let _ = e.read_to_string(&mut text);
        }
        let _ = tx_err.send(text);
    });
    let (tx_line, rx_line) = std::sync::mpsc::channel();
    if let Some(stdout) = child.stdout.take() {
        std::thread::spawn(move || {
            for l in BufReader::new(stdout).lines().map_while(Result::ok) {
                if tx_line.send(l).is_err() {
                    break;
                }
            }
        });
    }
    let mut out = String::new();
    let status = loop {
        while let Ok(l) = rx_line.try_recv() {
            let shown = l
                .trim_start_matches(['.', ' '])
                .trim_start_matches("Starting");
            let shown = shown.trim_start_matches('.').trim();
            if !shown.is_empty() {
                line(shown.to_string());
            }
            out.push_str(&l);
            out.push('\n');
        }
        if let Some(status) = child
            .try_wait()
            .map_err(|e| Problem::plain(e.to_string()))?
        {
            std::thread::sleep(Duration::from_millis(100));
            while let Ok(l) = rx_line.try_recv() {
                let shown = l
                    .trim_start_matches(['.', ' '])
                    .trim_start_matches("Starting");
                let shown = shown.trim_start_matches('.').trim();
                if !shown.is_empty() {
                    line(shown.to_string());
                }
                out.push_str(&l);
                out.push('\n');
            }
            break status;
        }
        std::thread::sleep(Duration::from_millis(50));
    };
    let err = rx_err
        .recv_timeout(Duration::from_millis(200))
        .unwrap_or_default();
    if status.success() {
        Ok(())
    } else {
        Err(Problem::explain(&format!("{out}\n{err}"), dir, since))
    }
}

/// What can be done about a problem, with one button.
#[derive(Clone, Debug, PartialEq)]
pub enum Fix {
    /// The port is taken: move the server to a free one of the club's `[hosting]` range.
    PortTaken,
    /// The game's own log says more.
    ShowLog,
}

/// A problem, in plain words.
#[derive(Clone, Debug, PartialEq)]
pub struct Problem {
    pub message: String,
    pub fix: Option<Fix>,
}

impl Problem {
    pub fn plain(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            fix: None,
        }
    }

    /// Reads what `horizon-server` (and the game log it points at) said.
    /// `since` is the background server's log length before the command: only what it logged after counts.
    pub fn explain(output: &str, dir: &Path, since: u64) -> Self {
        let log = fs::read(supervisor_log(dir)).unwrap_or_default();
        let new = log.get(since as usize..).unwrap_or_default();
        Self::explain_with(output, &String::from_utf8_lossy(new), dir)
    }

    fn explain_with(output: &str, log: &str, dir: &Path) -> Self {
        let both = format!("{output}\n{log}");
        let has = |s: &str| both.contains(s);
        // Vanilla says FAILED TO BIND TO PORT for any failure while starting; only these are a busy port.
        let busy = [
            "already in use",
            "Address in use",
            "Only one usage of each socket address",
        ];
        if busy.iter().any(|b| both.contains(b)) {
            let port = port_of(dir).unwrap_or(25565);
            return Self {
                message: crate::i18n::trf(
                    "Port {port} is already used by another program (maybe another server).",
                    &[("port", &port.to_string())],
                )
                .to_owned(),
                fix: Some(Fix::PortTaken),
            };
        }
        if has("no prepared game files") || has("not found next to the resources") {
            return Self {
                message: crate::i18n::tr(
                    "The server needs the game's files, which the club's supervisor prepares.",
                )
                .to_owned(),
                fix: None,
            };
        }
        if has("EULA") && !has("eula=true") {
            return Self {
                message: crate::i18n::tr(
                    "The club's supervisor has to accept Minecraft's EULA in club.toml before a \
                     server can run.",
                )
                .to_owned(),
                fix: None,
            };
        }
        if has("isn't up after") || has("the server stopped") {
            return Self {
                message: crate::i18n::tr("The game didn't start. Its log below says why.")
                    .to_owned(),
                fix: Some(Fix::ShowLog),
            };
        }
        let said = both
            .lines()
            .rev()
            .find_map(|l| l.strip_prefix("horizon-server: "))
            .map(str::to_string)
            .or_else(|| {
                output
                    .lines()
                    .rev()
                    .find(|l| !l.trim().is_empty())
                    .map(str::to_string)
            })
            .unwrap_or_else(|| {
                crate::i18n::tr("horizon-server failed without saying why").to_string()
            });
        Self::plain(capitalised(&said))
    }
}

fn capitalised(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(f) => f.to_uppercase().chain(c).collect(),
        None => String::new(),
    }
}

// ----- Creating a server ------------------------------------------------------------------------

/// The world a new server starts with.
#[derive(Clone, Debug, PartialEq)]
pub enum World {
    /// A new world; an empty seed is a random one.
    New { seed: String },
    /// A copy of one of the player's worlds (its folder).
    Copy(PathBuf),
}

/// What the Create dialog asks; [`NewServer::default`] alone makes a working server.
#[derive(Clone, Debug, PartialEq)]
pub struct NewServer {
    pub name: String,
    pub world: World,
    pub game_mode: String,
    pub difficulty: String,
    pub max_players: u32,
    /// Only listed players (and operators) may join.
    pub friends_only: bool,
    /// Who may join first (whitelisted), and the first of them becomes an operator.
    pub players: Vec<String>,
}

impl Default for NewServer {
    fn default() -> Self {
        Self {
            name: crate::i18n::tr("My server").to_string(),
            world: World::New {
                seed: String::new(),
            },
            game_mode: "survival".into(),
            difficulty: "normal".into(),
            max_players: 10,
            friends_only: true,
            players: Vec::new(),
        }
    }
}

/// A port nobody listens on among the club's `[hosting]` range, and not one of `taken`. None when
/// the range holds no free port: no server is created then.
pub fn free_port(first: u16, last: u16, taken: &[u16]) -> Option<u16> {
    let bind = if cfg!(test) || std::env::var_os("HORIZON_TEST_LOOPBACK").is_some_and(|v| v == "1")
    {
        "127.0.0.1"
    } else {
        "0.0.0.0"
    };
    (first..=last).find(|p| !taken.contains(p) && std::net::TcpListener::bind((bind, *p)).is_ok())
}

/// The port a server's `server.toml` sets (`[server] port`), read from the file without asking
/// `horizon-server`: the busy-port problem and the announcement both want it.
pub fn port_of(dir: &Path) -> Option<u16> {
    let text = fs::read_to_string(dir.join("server.toml")).ok()?;
    let config: toml::Value = toml::from_str(&text).ok()?;
    config
        .get("server")?
        .get("port")?
        .as_integer()
        .and_then(|p| u16::try_from(p).ok())
}

/// The ports the launcher's other servers use: their game port.
pub fn ports_taken() -> Vec<u16> {
    list().iter().filter_map(|e| port_of(&e.dir)).collect()
}

/// The player's worlds (the game home's `saves/`), newest first: their name and folder.
pub fn player_worlds() -> Vec<(String, PathBuf)> {
    let saves = backend::game_home().join("saves");
    let mut worlds: Vec<(std::time::SystemTime, String, PathBuf)> = fs::read_dir(saves)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|e| e.path().join("level.dat").is_file())
        .map(|e| {
            let when = e
                .path()
                .join("level.dat")
                .metadata()
                .and_then(|m| m.modified())
                .unwrap_or(std::time::UNIX_EPOCH);
            (when, e.file_name().to_string_lossy().into_owned(), e.path())
        })
        .collect();
    worlds.sort_by_key(|w| std::cmp::Reverse(w.0));
    worlds.into_iter().map(|(_, n, p)| (n, p)).collect()
}

/// The `horizon-server init` arguments for a new server: the EULA comes from the club's file
/// (the supervisor accepted it there) and the server plays offline, as the edition has no
/// accounts.
pub fn init_args(spec: &NewServer, world: &str, port: u16) -> Vec<String> {
    let mut args: Vec<String> = vec![
        "init".into(),
        "--yes".into(),
        "--eula".into(),
        "--offline".into(),
    ];
    args.extend(["--world".into(), world.to_string()]);
    if let World::New { seed } = &spec.world {
        if !seed.trim().is_empty() {
            args.extend(["--seed".into(), seed.trim().to_string()]);
        }
    }
    args.extend(["--port".into(), port.to_string()]);
    args.push(if spec.friends_only {
        "--whitelist".into()
    } else {
        "--no-whitelist".into()
    });
    for p in &spec.players {
        args.extend(["--player".into(), p.clone()]);
    }
    if let Some(first) = spec.players.first() {
        args.extend(["--op".into(), first.clone()]);
    }
    args
}

/// Vanilla's `force-gamemode`, for every hosted server: each player is put in the server's mode
/// when joining and when the mode changes, whatever an earlier visit saved for them. Without it
/// `defaultgamemode` only reaches a player's first join.
const FORCE_MODE: (&str, &str) = ("properties.force-gamemode", "true");

/// `backups.every` of a hosted server, and what its switch sets back on. The supervisor's own
/// period (6 hours) is for a server that runs all day: it counts from the server's start, so a
/// club that meets for half an hour would never see a backup. Ten minutes gives three or four per
/// session, and the last 12 stay (`backups.keep`).
pub const AUTO_BACKUP: &str = "10m";

/// The `horizon-server config set` arguments that finish a new server: its settings, offline
/// mode, no web panel, and the build's prepared game files (`[engine] resources`).
pub fn settings_args(spec: &NewServer, resources: &Path) -> Vec<String> {
    vec![
        "config".into(),
        "set".into(),
        format!("world.game_mode={}", spec.game_mode),
        format!("world.difficulty={}", spec.difficulty),
        format!("server.max_players={}", spec.max_players.clamp(1, 1000)),
        format!("server.motd={}", spec.name.trim()),
        "server.online_mode=false".into(),
        "panel.bind=".into(),
        format!("engine.resources={}", resources.display()),
        format!("backups.every={AUTO_BACKUP}"),
        format!("{}={}", FORCE_MODE.0, FORCE_MODE.1),
    ]
}

/// Creates a server: its folder, the world (a copy when asked), `init` and the settings, then
/// adds it to the list. `tag` is the build in use, whose supervisor and game files it runs in
/// place; `hosting` is the club's `[hosting]`: its EULA and its ports.
pub fn create(
    spec: &NewServer,
    tag: &str,
    hosting: &crate::config::Hosting,
    log: &dyn Fn(String),
) -> Result<Entry, Problem> {
    if !hosting.eula_accepted {
        return Err(Problem::plain(crate::i18n::tr(
            "The club's supervisor has to accept Minecraft's EULA in club.toml before a server \
             can be created.",
        )));
    }
    can_host(tag).map_err(Problem::plain)?;
    let name = if spec.name.trim().is_empty() {
        crate::i18n::tr("My server").to_string()
    } else {
        spec.name.trim().to_string()
    };
    let dir = new_folder(&name);
    fs::create_dir_all(&dir).map_err(|e| Problem::plain(e.to_string()))?;
    let world = match &spec.world {
        World::New { .. } => "world".to_string(),
        World::Copy(from) => {
            let world = from
                .file_name()
                .map(|n| slug(&n.to_string_lossy()))
                .unwrap_or_else(|| "world".into());
            log(crate::i18n::trf(
                "Copying the world {name}…",
                &[("name", &from.display().to_string())],
            ));
            copy_world(from, &dir.join(&world)).map_err(|e| {
                let _ = fs::remove_dir_all(&dir);
                Problem::plain(crate::i18n::trf(
                    "Couldn't copy the world: {error}",
                    &[("error", &e.to_string())],
                ))
            })?;
            world
        }
    };
    let Some(port) = free_port(hosting.first_port, hosting.last_port, &ports_taken()) else {
        let _ = fs::remove_dir_all(&dir);
        return Err(Problem::plain(crate::i18n::trf(
            "No port is free between {first} and {last}: stop another server, or ask the club's \
             supervisor for more of them.",
            &[
                ("first", &hosting.first_port.to_string()),
                ("last", &hosting.last_port.to_string()),
            ],
        )));
    };
    let undo = |p: Problem| {
        let _ = fs::remove_dir_all(&dir);
        p
    };
    let init = init_args(spec, &world, port);
    log(crate::i18n::tr("Writing the server's settings…").to_string());
    run(
        tag,
        &dir,
        &init.iter().map(String::as_str).collect::<Vec<_>>(),
    )
    .map_err(undo)?;
    let settings = settings_args(spec, &backend::resources(tag));
    run(
        tag,
        &dir,
        &settings.iter().map(String::as_str).collect::<Vec<_>>(),
    )
    .map_err(undo)?;
    let entry = Entry { name, dir };
    add(entry.clone()).map_err(Problem::plain)?;
    Ok(entry)
}

/// A copy of a world folder, without the lock a running game holds.
fn copy_world(from: &Path, to: &Path) -> std::io::Result<()> {
    fs::create_dir_all(to)?;
    for entry in fs::read_dir(from)? {
        let entry = entry?;
        let name = entry.file_name();
        if name == "session.lock" {
            continue;
        }
        let target = to.join(&name);
        if entry.file_type()?.is_dir() {
            copy_world(&entry.path(), &target)?;
        } else {
            fs::copy(entry.path(), target)?;
        }
    }
    Ok(())
}

// ----- A server's files --------------------------------------------------------------------------

/// The settings, as `horizon-server config --json` prints them (`None` before `init`).
pub fn config(tag: &str, dir: &Path) -> Option<Value> {
    if !dir.join("server.toml").is_file() {
        return None;
    }
    let out = run(tag, dir, &["config", "--json"]).ok()?;
    serde_json::from_str(out.trim()).ok()
}

/// What `config set` did: the settings applied now, and those waiting for a restart.
#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
pub struct Applied {
    #[serde(default)]
    pub applied: Vec<String>,
    #[serde(default)]
    pub restart: Vec<String>,
    #[serde(default)]
    pub running: bool,
}

/// Changes settings through `horizon-server config set` (checked there; nothing is written if one is
/// wrong).
pub fn set_config(tag: &str, dir: &Path, changes: &[(&str, String)]) -> Result<Applied, Problem> {
    let mut pairs: Vec<String> = changes.iter().map(|(k, v)| format!("{k}={v}")).collect();
    // A server made before the rule had it gets it the first time its mode is changed here (and
    // one edited by hand keeps it). It is the launcher's own doing: the page goes on telling
    // only what the student changed.
    let forced = changes.iter().any(|(k, _)| *k == "world.game_mode");
    if forced {
        pairs.push(format!("{}={}", FORCE_MODE.0, FORCE_MODE.1));
    }
    let mut args = vec!["config", "set"];
    args.extend(pairs.iter().map(String::as_str));
    args.push("--json");
    let out = run(tag, dir, &args)?;
    let mut applied: Applied =
        serde_json::from_str(out.trim()).map_err(|e| Problem::plain(e.to_string()))?;
    if forced {
        let key = FORCE_MODE.0;
        applied.applied.retain(|s| s != key);
        applied.restart.retain(|s| s != key);
    }
    if let Some(line) = mode_command(changes, &applied) {
        if let Some(api) = Api::open(dir) {
            let _ = api.command(&line);
        }
    }
    Ok(applied)
}

/// The console line that carries a new game mode to the players already on a running server:
/// vanilla's `defaultgamemode` only reaches those who join later, and a club's "this server is
/// creative" means everyone on it. `None` when the server is stopped or its mode did not change.
fn mode_command(changes: &[(&str, String)], applied: &Applied) -> Option<String> {
    if !applied.running || !applied.applied.iter().any(|s| s == "world.game_mode") {
        return None;
    }
    changes
        .iter()
        .rev()
        .find(|(k, _)| *k == "world.game_mode")
        .map(|(_, mode)| format!("gamemode {mode} @a"))
}

/// A list file's names (`whitelist.json`, `ops.json`, `banned-players.json`), as vanilla writes them.
pub fn names(dir: &Path, file: &str) -> Vec<String> {
    fs::read_to_string(dir.join(file))
        .ok()
        .and_then(|t| serde_json::from_str::<Vec<Value>>(&t).ok())
        .unwrap_or_default()
        .iter()
        .filter_map(|e| e["name"].as_str().map(str::to_string))
        .collect()
}

/// A list file's names with their UUIDs (empty when the file has none).
pub fn entries(dir: &Path, file: &str) -> Vec<(String, String)> {
    fs::read_to_string(dir.join(file))
        .ok()
        .and_then(|t| serde_json::from_str::<Vec<Value>>(&t).ok())
        .unwrap_or_default()
        .iter()
        .filter_map(|e| {
            let name = e["name"].as_str()?.to_string();
            Some((name, e["uuid"].as_str().unwrap_or_default().to_string()))
        })
        .collect()
}

/// A backup, as `horizon-server backups --json` lists it.
#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct Backup {
    pub name: String,
    pub bytes: u64,
    #[serde(default)]
    pub modified: Option<u64>,
}

pub fn backups(tag: &str, dir: &Path) -> Result<Vec<Backup>, Problem> {
    let out = run(tag, dir, &["backups", "--json"])?;
    serde_json::from_str(out.trim()).map_err(|e| Problem::plain(e.to_string()))
}

/// The address friends on the same network type: this computer's LAN address. No packet is sent: the
/// system only picks the interface it would use.
pub fn lan_address() -> Option<std::net::IpAddr> {
    let socket = std::net::UdpSocket::bind("0.0.0.0:0").ok()?;
    socket.connect("192.0.2.1:9").ok()?;
    let ip = socket.local_addr().ok()?.ip();
    (!ip.is_unspecified() && !ip.is_loopback()).then_some(ip)
}

// ----- The running server: its control API -------------------------------------------------------

/// The running supervisor's control API (`.horizon/api.json`: its address and token).
#[derive(Clone, Debug)]
pub struct Api {
    address: String,
    token: String,
}

impl Api {
    /// The API of the server running in `dir`, if one runs.
    pub fn open(dir: &Path) -> Option<Self> {
        let text = fs::read_to_string(dir.join(".horizon/api.json")).ok()?;
        let v: Value = serde_json::from_str(&text).ok()?;
        Some(Self {
            address: v["address"].as_str()?.to_string(),
            token: v["token"].as_str()?.to_string(),
        })
    }

    pub fn call(&self, method: &str, path: &str, body: Option<Value>) -> Result<Value, String> {
        // No proxy here on purpose: the server is on this computer, and this is asked every
        // second (`proxy` never sends a loopback address through one either).
        let agent = ureq::AgentBuilder::new()
            .timeout(Duration::from_secs(10))
            .build();
        let request = agent
            .request(method, &format!("http://{}{path}", self.address))
            .set("Authorization", &format!("Bearer {}", self.token));
        let response = match body {
            Some(b) => request.send_json(b),
            None => request.call(),
        };
        match response {
            Ok(r) => r.into_json().map_err(|e| e.to_string()),
            Err(ureq::Error::Status(_, r)) => {
                let v: Value = r.into_json().unwrap_or(Value::Null);
                Err(v["error"]
                    .as_str()
                    .unwrap_or("the server refused")
                    .to_string())
            }
            Err(e) => Err(e.to_string()),
        }
    }

    pub fn status(&self) -> Option<Status> {
        self.call("GET", "/api/v1/status", None)
            .ok()
            .map(|v| Status::parse(&v))
    }

    /// The log lines after `since`, and the number to ask from next time.
    pub fn logs(&self, since: u64) -> Option<(u64, Vec<String>)> {
        let v = self
            .call("GET", &format!("/api/v1/logs?since={since}"), None)
            .ok()?;
        let lines = v["lines"]
            .as_array()?
            .iter()
            .filter_map(|l| l.as_str().map(str::to_string))
            .collect();
        Some((v["next"].as_u64().unwrap_or(since), lines))
    }

    /// One console command; what the game answered.
    pub fn command(&self, line: &str) -> Result<Vec<String>, String> {
        let v = self.call("POST", "/api/v1/command", Some(json!({ "line": line })))?;
        Ok(v["output"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|l| l.as_str().map(str::to_string))
            .collect())
    }
}

/// A player online.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Player {
    pub name: String,
    pub uuid: String,
    pub ping: Option<u64>,
}

/// What the supervisor reports.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Status {
    /// starting, running, stopping or restarting (stopped when nothing answers).
    pub phase: String,
    pub uptime: Option<u64>,
    pub crashes: u64,
    pub last_crash: Option<String>,
    pub tps: Option<f64>,
    pub mspt: Option<f64>,
    pub players: Vec<Player>,
    pub max_players: Option<u64>,
    pub ram_bytes: Option<u64>,
    pub cpu_percent: Option<f64>,
    /// Paused because nobody is online (vanilla's pause-when-empty).
    pub paused: bool,
}

impl Status {
    pub fn stopped() -> Self {
        Self {
            phase: "stopped".into(),
            ..Self::default()
        }
    }

    pub fn parse(v: &Value) -> Self {
        let game = &v["sample"]["game"];
        Self {
            phase: v["phase"].as_str().unwrap_or("stopped").to_string(),
            uptime: v["engine_uptime_seconds"].as_u64(),
            crashes: v["crashes"].as_u64().unwrap_or(0),
            last_crash: v["last_crash"].as_str().map(str::to_string),
            tps: game["tps"].as_f64(),
            mspt: game["mspt"]["average"].as_f64(),
            players: game["players"]
                .as_array()
                .into_iter()
                .flatten()
                .map(|p| Player {
                    name: p["name"].as_str().unwrap_or_default().to_string(),
                    uuid: p["uuid"].as_str().unwrap_or_default().to_string(),
                    ping: p["ping"].as_u64(),
                })
                .collect(),
            max_players: game["max_players"].as_u64(),
            ram_bytes: v["sample"]["rss_bytes"].as_u64(),
            cpu_percent: v["sample"]["cpu_percent"].as_f64(),
            paused: game["paused"] == true,
        }
    }

    pub fn running(&self) -> bool {
        self.phase != "stopped"
    }
}

/// The status of the server in `dir`: what its API says, or stopped.
pub fn status(dir: &Path) -> Status {
    Api::open(dir)
        .and_then(|api| api.status())
        .unwrap_or_else(Status::stopped)
}

#[cfg(test)]
#[path = "servers/tests.rs"]
mod tests;
