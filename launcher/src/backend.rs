//! Everything that is not UI: the club's game builds, resource preparation, and the game's
//! processes. The GUI goes through this module. Functions here block; the GUI runs them on
//! threads and reads their messages.
use crate::{mojang, sources};
use horizon_platform::prepare;
use std::fs;
use std::io::{BufRead, BufReader, Read};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};

/// This launcher's version: the release workflow sets `HORIZON_LAUNCHER_VERSION` from the
/// tag it builds (no version-bump commits); other builds say Cargo.toml's.
pub const VERSION: &str = version_or(option_env!("HORIZON_LAUNCHER_VERSION"));

const fn version_or(injected: Option<&'static str>) -> &'static str {
    match injected {
        Some(v) if !v.is_empty() => v,
        _ => env!("CARGO_PKG_VERSION"),
    }
}

mod look_settings;

/// The game's program inside a build folder, as the club's `core` holds it.
pub(crate) const CLIENT: &str = if cfg!(windows) {
    "horizon-client.exe"
} else {
    "horizon-client"
};

/// The launcher's data folder: the student's own one ([`crate::club::data_dir`]).
pub fn data_dir() -> PathBuf {
    crate::club::data_dir()
}

/// The club's game builds ([`crate::club::versions_dir`]): `<root>\core`.
fn versions_dir() -> PathBuf {
    crate::club::versions_dir()
}

/// One build's folder, `<root>\core\<tag>`. Read-only for students: the launcher never writes
/// inside.
pub fn version_dir(tag: &str) -> PathBuf {
    versions_dir().join(tag)
}

/// Whether version `tag` is still installed: its folder holds the game's program. The folder is
/// the supervisor's, so the check only reads.
pub fn is_installed(tag: &str) -> bool {
    version_dir(tag)
        .join(crate::club::client_program_name())
        .exists()
}

/// What the preparation says when the build it was asked for is no longer installed: `name` is
/// the build as the window names it.
pub fn version_gone(name: &str) -> String {
    crate::i18n::trf(
        "{version} is no longer on this computer.",
        &[("version", name)],
    )
}

/// The builds installed in the club's `core` folder, newest name first. The supervisor drops
/// builds in whole: a folder counts as a build only once its game program is there.
pub fn installed() -> Vec<String> {
    installed_in(&versions_dir())
}

fn installed_in(versions: &Path) -> Vec<String> {
    let mut tags: Vec<String> = fs::read_dir(versions)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|e| e.path().join(crate::club::client_program_name()).exists())
        .filter_map(|e| e.file_name().into_string().ok())
        .collect();
    // Newest first: `club-10-…` after `club-9-…`, a date after the day before (`club::newer`).
    tags.sort_by(|a, b| crate::club::newer(b, a));
    tags
}

mod game_start;
pub use game_start::client_command;

/// The Minecraft version a Horizon build targets, from its research-lock.json.
pub fn game_version(tag: &str) -> String {
    game_version_at(&version_dir(tag))
}

fn read_json(path: &Path) -> Option<serde_json::Value> {
    serde_json::from_str(&fs::read_to_string(path).ok()?).ok()
}

fn game_version_at(dir: &Path) -> String {
    read_json(&dir.join("research-lock.json"))
        .and_then(|v| v["minecraft"]["version"].as_str().map(String::from))
        .unwrap_or_else(|| horizon_platform::version::VANILLA_VERSION.into())
}

pub fn resources(tag: &str) -> PathBuf {
    resources_at(&version_dir(tag))
}

fn resources_at(dir: &Path) -> PathBuf {
    dir.join(".local/horizon-client").join(game_version_at(dir))
}

const RESOURCES_MANIFEST: &str = "horizon-resources.json";

/// How long a drawn page believes an answer of [`is_prepared`] without looking at the disk again:
/// only the supervisor changes a build's files, and this bounds how late that shows.
const PREPARED_TTL: std::time::Duration = std::time::Duration::from_secs(5);

/// The answers of [`is_prepared`]: version folder, whether it was prepared, when that was checked.
#[derive(Default)]
struct PreparedCache(Vec<(PathBuf, bool, std::time::Instant)>);

impl PreparedCache {
    fn get(&self, dir: &Path, now: std::time::Instant) -> Option<bool> {
        let (_, prepared, at) = self.0.iter().find(|(d, ..)| d == dir)?;
        (now.saturating_duration_since(*at) < PREPARED_TTL).then_some(*prepared)
    }

    fn put(&mut self, dir: &Path, prepared: bool, now: std::time::Instant) {
        self.0.retain(|(d, ..)| d != dir);
        self.0.push((dir.to_path_buf(), prepared, now));
    }
}

static PREPARED: Mutex<PreparedCache> = Mutex::new(PreparedCache(Vec::new()));

fn lock_prepared(cache: &Mutex<PreparedCache>) -> std::sync::MutexGuard<'_, PreparedCache> {
    cache.lock().unwrap_or_else(|e| e.into_inner())
}

/// The answer kept for `dir`, else `check()` (run without the lock held), kept.
fn cached_prepared(
    cache: &Mutex<PreparedCache>,
    dir: &Path,
    now: std::time::Instant,
    check: impl FnOnce() -> bool,
) -> bool {
    if let Some(known) = lock_prepared(cache).get(dir, now) {
        return known;
    }
    let prepared = check();
    lock_prepared(cache).put(dir, prepared, now);
    prepared
}

/// Whether `tag` is prepared, as a page being drawn asks it (every frame, for each version it
/// shows): an answer stands for [`PREPARED_TTL`], so a frame does no file reads. Anything that
/// must not act on a stale answer (starting the game or a server) calls [`check_prepared`].
pub fn is_prepared(tag: &str) -> bool {
    cached_prepared(
        &PREPARED,
        &version_dir(tag),
        std::time::Instant::now(),
        || prepared_at(&resources(tag)),
    )
}

/// Whether `tag` is prepared, checked on the disk now: `research-lock.json`, the manifest and
/// every prepared table are read. The answer replaces the one [`is_prepared`] keeps.
pub fn check_prepared(tag: &str) -> bool {
    checked_prepared(
        &PREPARED,
        &version_dir(tag),
        &resources(tag),
        std::time::Instant::now(),
    )
}

fn checked_prepared(
    cache: &Mutex<PreparedCache>,
    dir: &Path,
    resources: &Path,
    now: std::time::Instant,
) -> bool {
    let prepared = prepared_at(resources);
    lock_prepared(cache).put(dir, prepared, now);
    prepared
}

/// Whether the prepared folder `resources` is usable: its manifest is a JSON
/// document and no top-level table a killed preparation or an interrupted
/// copy left empty. Such a folder used to count as prepared, so Play started
/// the game on it and the loader crashed on the empty file.
fn prepared_at(resources: &Path) -> bool {
    if read_json(&resources.join(RESOURCES_MANIFEST)).is_none() {
        return false;
    }
    let Ok(entries) = fs::read_dir(resources) else {
        return false;
    };
    entries.flatten().all(|entry| {
        let path = entry.path();
        path.extension() != Some(std::ffi::OsStr::new("json"))
            || fs::metadata(&path).is_ok_and(|meta| meta.len() > 0)
    })
}

/// Where a version folder keeps the preparer's sources (the release package copies them).
const PREPARER_CRATE: &str = "crates/horizon-prepare";

/// Copy a folder with everything in it.
pub(crate) fn copy_dir(from: &Path, to: &Path) -> std::io::Result<()> {
    fs::create_dir_all(to)?;
    for entry in fs::read_dir(from)? {
        let entry = entry?;
        let target = to.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_dir(&entry.path(), &target)?;
        } else {
            prepare::clone_or_copy(&entry.path(), &target)?;
        }
    }
    Ok(())
}

fn found(program: &str, arg: &str) -> bool {
    crate::process::command(program)
        .arg(arg)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok()
}

fn python() -> Option<&'static str> {
    let candidates: &[&'static str] = if cfg!(windows) {
        &["py", "python"]
    } else {
        &["python3", "python"]
    };
    candidates.iter().copied().find(|p| found(p, "--version"))
}

/// How a version's resources are prepared.
pub enum Preparer {
    /// horizon-prepare, linked into the launcher: the version ships the same preparer sources.
    InProcess(prepare::Options),
    /// The version's own preparer: its horizon-prepare binary when its sources differ from the
    /// launcher's (a newer or older release), or the Python script of versions made before it.
    Process(Command),
}

/// The version's horizon-prepare binary, when it ships one.
fn preparer_binary(dir: &Path) -> Option<PathBuf> {
    let binary = dir.join(if cfg!(windows) {
        "horizon-prepare.exe"
    } else {
        "horizon-prepare"
    });
    binary.is_file().then_some(binary)
}

/// What the preparation says where a download from Mojang would finish the files: the window
/// never downloads, and the supervisor's tool (`club-admin prepare --download`) is the one that
/// can.
fn download_refused(version: &str) -> String {
    crate::i18n::trf(
        "\n  • Complete {version}'s files from Mojang with: club-admin prepare --download",
        &[("version", version)],
    )
}

/// The folder of the Microsoft Store's Minecraft Launcher (the one Windows offers, and the Xbox
/// app's) under `local`, `%LOCALAPPDATA%`. Its game files are in `%APPDATA%\.minecraft` like the
/// other launcher's, but the Java it downloads is in this folder's `runtime`, so a preparation
/// found none and downloaded the same runtime from Mojang again (411 files, 100 MB for 26.3).
fn store_launcher_folder(local: &Path) -> PathBuf {
    local.join(r"Packages\Microsoft.4297127D64EC6_8wekyb3d8bbwe\LocalCache\Local")
}

/// The Java home for this version: a runtime for precompiled exporters, a JDK for older
/// preparers or changed sources. Forward the selection to external preparers too.
/// Else try the official launcher's Java in `games`, then download the runtime named by
/// `manifest` into Horizon's game folder (Mojang's runtimes are whole JDKs).
fn java(
    major: i64,
    compiler: bool,
    games: &[PathBuf],
    manifest: &Path,
    log: &dyn Fn(String),
) -> Result<PathBuf, String> {
    let find = if compiler {
        prepare::java::find
    } else {
        prepare::java::find_runtime
    };
    if let Ok(jdk) = find(major, &[]) {
        return Ok(jdk.home());
    }
    let mut runtimes: Vec<PathBuf> = Vec::new();
    for home in games
        .iter()
        .flat_map(|game| prepare::java::minecraft_runtimes(&game.join("runtime")))
    {
        if !runtimes.contains(&home) {
            runtimes.push(home);
        }
    }
    let missing = match find(major, &runtimes) {
        Ok(jdk) => return Ok(jdk.home()),
        Err(e) => e,
    };
    let component = read_json(manifest)
        .and_then(|json| json["javaVersion"]["component"].as_str().map(String::from))
        .ok_or_else(|| missing.clone())?;
    // Mojang builds its Linux Java for glibc: without glibc's loader (NixOS without nix-ld,
    // musl distributions) it cannot start, so don't download it.
    if cfg!(all(target_os = "linux", target_arch = "x86_64")) && !Path::new(GLIBC_LOADER).exists() {
        return Err(format!(
            "{missing}\n  • Minecraft's own Java from Mojang needs glibc ({GLIBC_LOADER} is missing; on NixOS, enable programs.nix-ld), so install compatible Java instead."
        ));
    }
    let fetch = crate::downloads::install_metadata;
    let runtime = data_dir().join("minecraft").join("runtime");
    let home = mojang::java_runtime_http(&component, &runtime, mojang::JOBS, &fetch, &mojang::CANCEL, log)
        .map_err(|e| {
        format!("{missing}\n  • Horizon also tried to download Minecraft's own Java from Mojang, but: {e}")
    })?;
    // On macOS the download's Java home is inside its jre.bundle (#17894).
    let home = prepare::java::runtime_home(&home).unwrap_or(home);
    find(major, &[home]).map(|jdk| jdk.home())
}

/// The dynamic loader glibc programs on x86-64 Linux start through.
pub(crate) const GLIBC_LOADER: &str = "/lib64/ld-linux-x86-64.so.2";

/// Runs `cmd` with Java at `home`: JAVA_HOME names it and its `bin` leads the PATH, as the
/// versions' own preparers look for Java.
fn with_java(cmd: &mut Command, home: &Path) {
    let mut path = vec![home.join("bin")];
    path.extend(
        std::env::var_os("PATH")
            .iter()
            .flat_map(std::env::split_paths),
    );
    cmd.env("JAVA_HOME", home);
    if let Ok(path) = std::env::join_paths(path) {
        cmd.env("PATH", path);
    }
}

/// The Minecraft folder a preparation whose game files are imported already reads: the sounds,
/// the title panorama, the fonts and the languages come from its asset store. It is the one
/// `resolve` found, linked into the official layout for another launcher's files; without one, the
/// chosen folder, and the preparation goes without them, saying so. Before, such a preparation
/// read the chosen folder as it was, which a Prism Launcher or Modrinth App install does not
/// keep its assets in: the game was silent and its title screen grey.
fn assets_folder(
    resolved: Result<sources::Resolved, String>,
    chosen: &Path,
    version: &str,
    log: &dyn Fn(String),
) -> PathBuf {
    match resolved {
        Ok(sources::Resolved::Folder(dir)) => dir,
        Ok(sources::Resolved::NotFound(..)) => {
            log(format!(
                "Minecraft {version}'s assets were not found in any launcher folder: the game \
                 will be silent and the title screen plain"
            ));
            chosen.to_path_buf()
        }
        Err(e) => {
            log(format!("Minecraft {version}'s assets: {e}"));
            chosen.to_path_buf()
        }
    }
}

/// Why no Minecraft folder could be prepared from, with `download` (a line starting with a
/// bullet) saying why the launcher can't fetch the game itself. Folders that have the game but
/// lack `incomplete` (a library's file name) get one short sentence; otherwise every folder
/// looked in, with what it lacks.
fn incomplete_or_not_found(
    version: &str,
    checked: &[(PathBuf, String)],
    chosen: &Path,
    incomplete: Option<String>,
    download: &str,
) -> String {
    match incomplete {
        Some(file) => {
            let why = download.trim_start_matches("\n  • ");
            format!("{} {why}", sources::incomplete(version, &file))
        }
        None => sources::not_found(version, checked, chosen) + download,
    }
}

/// Checks what the preparation needs and picks how to run it. PLAY.txt describes the same
/// steps: the player's own launcher install, the server jar from that version's own json, and
/// Java (and Python, for versions before horizon-prepare). Also returns the Minecraft folder
/// to prepare from: `minecraft`, or the files another launcher keeps, linked into the official
/// launcher's layout (sources.rs). The window prepares nothing (`club-admin` is the caller);
/// a folder whose files are incomplete is refused with the command that completes them.
pub fn preparer(
    tag: &str,
    minecraft: &Path,
    log: &dyn Fn(String),
) -> Result<(Preparer, PathBuf), String> {
    // Deleted while the window was open: never the folder's OS error from the steps below.
    if !is_installed(tag) {
        return Err(version_gone(tag));
    }
    let version = game_version(tag);
    let dir = version_dir(tag);
    let reused = prepare::artifacts::reuse_imported(&dir, &|line| log(line.to_string()))?;
    let chosen = minecraft.to_path_buf();
    // Resolved even when the game files are reused: the assets still come from the install.
    let minecraft = {
        let pins = read_json(&dir.join("research-lock.json"))
            .and_then(|lock| sources::Pins::read(&lock))
            .ok_or("The version's research-lock.json doesn't pin the Minecraft files")?;
        let own = data_dir().join("minecraft");
        let mut known = vec![("Horizon's own download", own.clone())];
        known.extend(
            sources::Folders::current()
                .map(|f| sources::known_folders(sources::Os::current(), &f))
                .unwrap_or_default(),
        );
        let fetch = crate::downloads::install_metadata;
        let linked = data_dir().join(sources::LINKED);
        // Reused game files need only the assets: a folder missing libraries still has them.
        // Otherwise a library this version already imported needn't be in the folder.
        let imported = prepare::artifacts::Lock::read(&dir).map(|lock| lock.directory);
        let whole = imported.as_ref().ok().filter(|_| !reused);
        let resolved = sources::resolve(
            minecraft,
            &pins,
            &known,
            &linked,
            whole.map(PathBuf::as_path),
            &fetch,
            log,
        );
        if reused {
            assets_folder(resolved, &chosen, &version, log)
        } else {
            match resolved? {
                sources::Resolved::Folder(dir) => dir,
                sources::Resolved::NotFound(checked, incomplete) => {
                    let download = download_refused(&version);
                    return Err(incomplete_or_not_found(
                        &version, &checked, minecraft, incomplete, &download,
                    ));
                }
            }
        }
    };
    let manifest = minecraft
        .join("versions")
        .join(&version)
        .join(format!("{version}.json"));
    let binary = preparer_binary(&dir);
    let python = match binary {
        Some(_) => None,
        None => Some(python().ok_or(
            "This version prepares with Python: Python 3.10 or newer is needed. Install it, or pick a newer version.",
        )?),
    };
    // The Java the version pins, found as horizon-prepare finds it, so a missing one is
    // reported (or downloaded) before the server jar is downloaded. Versions from before
    // java_major ran Java 25.
    let major = read_json(&dir.join("research-lock.json"))
        .and_then(|lock| lock["minecraft"]["java_major"].as_i64())
        .unwrap_or(25);
    let mut games = vec![chosen, minecraft.clone(), data_dir().join("minecraft")];
    games.extend(
        sources::Folders::current()
            .map(|f| sources::known_folders(sources::Os::current(), &f))
            .unwrap_or_default()
            .into_iter()
            .map(|(_, dir)| dir),
    );
    if cfg!(windows) {
        games.extend(dirs::data_local_dir().map(|local| store_launcher_folder(&local)));
    }
    let java_home = java(
        major,
        !prepare::exporters::available(&dir),
        &games,
        &manifest,
        log,
    )?;
    let jar = data_dir()
        .join("server-jars")
        .join(format!("{version}.jar"));
    if !reused {
        let json: serde_json::Value = serde_json::from_str(
            &fs::read_to_string(&manifest).map_err(|e| format!("{}: {e}", manifest.display()))?,
        )
        .map_err(|e| format!("{}: {e}", manifest.display()))?;
        let server = &json["downloads"]["server"];
        let artifact = crate::downloads::Artifact {
            url: server["url"]
                .as_str()
                .ok_or_else(|| format!("{} has no server download", manifest.display()))?
                .into(),
            sha1: server["sha1"]
                .as_str()
                .ok_or("The server download has no SHA-1")?
                .into(),
            size: server["size"].as_u64(),
            path: jar.clone(),
        };
        log(format!(
            "Checking and downloading the {version} server jar from Mojang"
        ));
        let http = crate::downloads::shared()?.session();
        crate::downloads::run(http.download(&artifact, &mojang::CANCEL))?;
    }

    if let Some(binary) = binary {
        if prepare::sources_sha256(&dir.join(PREPARER_CRATE)).as_deref()
            == Some(prepare::SOURCES_SHA256)
        {
            let options = prepare::Options {
                root: dir,
                import: !reused,
                minecraft_dir: Some(minecraft.clone()),
                server_jar: (!reused).then_some(jar),
                java_home: Some(java_home.clone()),
                inputs: Default::default(),
                output: None,
            };
            return Ok((Preparer::InProcess(options), minecraft));
        }
        let mut cmd = Command::new(binary);
        with_java(&mut cmd, &java_home);
        cmd.current_dir(&dir).arg("--root").arg(&dir);
        if !reused {
            cmd.args(["--prepare", "--server-jar"]).arg(&jar);
        }
        cmd.arg("--minecraft-dir").arg(&minecraft);
        return Ok((Preparer::Process(cmd), minecraft));
    }
    let mut cmd = Command::new(python.unwrap());
    with_java(&mut cmd, &java_home);
    cmd.current_dir(&dir)
        .args(["-u", "tools/prepare_native_client.py"]);
    if !reused {
        cmd.args(["--prepare", "--server-jar"]).arg(&jar);
    }
    cmd.arg("--minecraft-dir").arg(&minecraft);
    Ok((Preparer::Process(cmd), minecraft))
}

/// Copies the pinned version's sounds from the player's own launcher install into the version's
/// resources (`horizon-prepare sounds`), each line to `log`. Sound is optional, so a launcher
/// install without them is a line in the log, never a failed preparation. The version's own
/// preparer does it when it is not the launcher's and knows how; versions from before it read
/// the sounds where the launcher's preparer puts them.
pub fn sounds(tag: &str, minecraft: &Path, log: &dyn Fn(String)) {
    let dir = version_dir(tag);
    let own = prepare::sources_sha256(&dir.join(PREPARER_CRATE)).as_deref()
        == Some(prepare::SOURCES_SHA256);
    let binary = preparer_binary(&dir).filter(|binary| {
        !own && crate::process::command(binary)
            .arg("--help")
            .output()
            .is_ok_and(|o| String::from_utf8_lossy(&o.stdout).contains("horizon-prepare sounds"))
    });
    log("Copying the sounds from your Minecraft install".into());
    let result = match binary {
        Some(binary) => crate::process::command(binary)
            .current_dir(&dir)
            .arg("sounds")
            .arg("--root")
            .arg(&dir)
            .arg("--minecraft-dir")
            .arg(minecraft)
            .output()
            .map_err(|e| e.to_string())
            .and_then(|o| {
                let text = |b: &[u8]| String::from_utf8_lossy(b).trim().to_string();
                for line in text(&o.stdout).lines() {
                    log(line.to_string());
                }
                match o.status.success() {
                    true => Ok(()),
                    false => Err(text(&o.stderr).trim_start_matches("error: ").to_string()),
                }
            }),
        None => std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            prepare::sounds(&dir, Some(minecraft), &|line| log(line.to_string()))
        }))
        .unwrap_or_else(|_| Err("The sound copy stopped unexpectedly".into()))
        .map(|_| ()),
    };
    if let Err(e) = result {
        log(format!("No sound (the game will be silent): {e}"));
    }
}

/// Runs `Preparer::InProcess` on this thread, each progress line to `log`. A panic is an
/// error like any other, so the window never waits on a preparation that died.
pub fn prepare_in_process(options: &prepare::Options, log: &dyn Fn(String)) -> Result<(), String> {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        prepare::prepare(options, &|line| log(line.to_string()))
    }))
    .unwrap_or_else(|_| Err("The preparation stopped unexpectedly".into()))
    .map(|_| ())
}

/// Runs a version's own preparer (`Preparer::Process`), each line it prints to `log`: the
/// supervisor's tool's way with a build whose preparer differs from the launcher's own.
pub fn run_preparer(mut cmd: Command, log: &dyn Fn(String)) -> Result<(), String> {
    cmd.stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = cmd
        .spawn()
        .map_err(|e| format!("{}: {e}", cmd.get_program().display()))?;
    let mut said = String::new();
    if let Some(out) = child.stdout.take() {
        for line in BufReader::new(out).lines().map_while(Result::ok) {
            log(line.clone());
            said.push_str(&line);
            said.push('\n');
        }
    }
    if let Some(err) = child.stderr.take() {
        for line in BufReader::new(err).lines().map_while(Result::ok) {
            log(line.clone());
            said.push_str(&line);
            said.push('\n');
        }
    }
    match child.wait() {
        Ok(status) if status.success() => Ok(()),
        Ok(status) => Err(format!("the preparer exited with {status}: {said}")),
        Err(e) => Err(e.to_string()),
    }
}

/// Where the game keeps its saves and settings: the student's own game home
/// ([`crate::club::game_home`]).
pub fn game_home() -> PathBuf {
    crate::club::game_home()
}

// ----- Crash reports -------------------------------------------------------------------------

/// The game's logs (`latest.log` and the day archives), as it writes them in its home.
pub fn logs_dir() -> PathBuf {
    game_home().join("logs")
}

/// The crash reports in `dir`, oldest first (their names sort by time).
fn crash_reports_in(dir: &Path) -> Vec<PathBuf> {
    let mut reports: Vec<PathBuf> = fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.starts_with("crash-") && n.ends_with(".txt"))
        })
        .collect();
    reports.sort();
    reports
}

/// The newest crash report the game playing in `home` wrote at or after `since` (one from the
/// run that started then).
pub fn crash_report_since(home: &Path, since: std::time::SystemTime) -> Option<PathBuf> {
    crash_report_in_since(&home.join("crash-reports"), since)
}

fn crash_report_in_since(dir: &Path, since: std::time::SystemTime) -> Option<PathBuf> {
    crash_reports_in(dir).into_iter().rev().find(|p| {
        fs::metadata(p)
            .and_then(|m| m.modified())
            .is_ok_and(|t| t >= since)
    })
}

/// The error a crash report describes, on one line: the lines between its description and the
/// stack (`panicked at src/game.rs:10:5: boom`).
pub fn crash_summary(report: &str) -> Option<String> {
    let mut lines = report
        .lines()
        .skip_while(|l| !l.starts_with("Description:"));
    lines.next()?;
    let error: Vec<&str> = lines
        .skip_while(|l| l.trim().is_empty())
        .take_while(|l| !l.trim().is_empty() && !l.starts_with('\t'))
        .map(str::trim)
        .collect();
    (!error.is_empty()).then(|| error.join(" "))
}

/// The crash card's line on what to do next: the student tells the club's supervisor. The card's
/// Open report and Open logs folder buttons give them what to show.
pub fn crash_help() -> String {
    crate::i18n::tr("Tell the club's supervisor that the game crashed.").into()
}

/// The line the game prints when it has drawn its first frame.
pub const GAME_FIRST_FRAME: &str = "Horizon: first frame";

/// A graphics API whose driver can crash natively at the renderer's start, before the game
/// can catch it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StartCrash {
    Vulkan,
    OpenGl,
}

impl StartCrash {
    /// The API's name for the player.
    pub fn label(self) -> &'static str {
        match self {
            StartCrash::Vulkan => "Vulkan",
            StartCrash::OpenGl => "OpenGL",
        }
    }
}

/// The API a game's output ends in a crash on while its renderer started: the last adapter line
/// (`Horizon renderer: <GPU> (<backend>, <type>)`) names Vulkan or OpenGL (`Gl`), and no first
/// frame follows. A native crash in a graphics driver leaves no crash report, only those lines
/// (#746, #18291).
pub fn renderer_start_crash<S: AsRef<str>>(
    lines: impl IntoIterator<Item = S>,
) -> Option<StartCrash> {
    let mut crashed = None;
    for line in lines {
        let line = line.as_ref();
        if line.starts_with(GAME_FIRST_FRAME) {
            return None;
        }
        if let Some(backend) = adapter_backend(line) {
            crashed = match backend {
                "Vulkan" => Some(StartCrash::Vulkan),
                "Gl" => Some(StartCrash::OpenGl),
                _ => None,
            };
        }
    }
    crashed
}

/// The backend an adapter line names: `Horizon renderer: <GPU> (<backend>, <type>)`, `<type>`
/// one of wgpu's device types. The start's step lines can end in a parenthesis too, such as
/// `device ready; configuring the surface (1280x720 Bgra8Unorm, Fifo)`.
fn adapter_backend(line: &str) -> Option<&str> {
    const DEVICE_TYPES: [&str; 5] = ["Other", "IntegratedGpu", "DiscreteGpu", "VirtualGpu", "Cpu"];
    let (_, tail) = line.strip_prefix("Horizon renderer: ")?.rsplit_once(" (")?;
    let (backend, kind) = tail.strip_suffix(')')?.split_once(", ")?;
    DEVICE_TYPES.contains(&kind).then_some(backend)
}

/// The game's graphics settings file in its home (the game's `graphics.rs`).
const GRAPHICS_FILE: &str = "horizon-graphics.txt";

/// The graphics API (the game's setting name, and its label) Auto switches to after a native
/// crash at the renderer's start on `crashed`. Windows: DirectX 12 (#746), which then never
/// tries Vulkan or OpenGL again, so an Intel HD 4000 whose OpenGL driver crashes gets WARP
/// (#18291). Linux: OpenGL after Vulkan, where Auto also runs Mesa's hasvk on Ivy Bridge and
/// Haswell, which Mesa itself calls incomplete there (#4293); nothing after OpenGL, which Auto
/// reached only because Vulkan had no device. `None` where the game has no other API (macOS:
/// Metal only).
pub fn start_crash_fallback(crashed: StartCrash, os: &str) -> Option<(&'static str, &'static str)> {
    match (crashed, os) {
        (_, "macos") => None,
        (_, "windows") => Some(("dx12", "DirectX 12")),
        (StartCrash::Vulkan, _) => Some(("opengl", "OpenGL")),
        (StartCrash::OpenGl, _) => None,
    }
}

/// Whether a game's exit can be a native crash in a graphics driver. On Windows, an NTSTATUS error
/// code such as 0xC0000005; Stop's `TerminateProcess` leaves exit code 1. On Unix, a signal other
/// than the SIGKILL Stop sends to the game's group or the SIGTERM and SIGINT of a shutdown.
pub fn native_crash_exit(status: std::process::ExitStatus) -> bool {
    #[cfg(windows)]
    {
        status
            .code()
            .is_some_and(|code| code as u32 & 0xC000_0000 == 0xC000_0000)
    }
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        status
            .signal()
            .is_some_and(|signal| ![libc::SIGKILL, libc::SIGTERM, libc::SIGINT].contains(&signal))
    }
}

/// Whether `home`'s options.txt names a graphics API the player chose in Video Settings
/// (`preferredGraphicsBackend:"opengl"` or `"vulkan"`), which the game runs when `graphicsApi` is
/// unset or `auto` (the game's `settings.rs`).
fn options_choose_api(home: &Path) -> bool {
    fs::read_to_string(home.join("options.txt")).is_ok_and(|text| {
        text.lines().any(|line| {
            line.split_once(':').is_some_and(|(key, value)| {
                key.trim() == "preferredGraphicsBackend"
                    && matches!(value.trim().trim_matches('"'), "opengl" | "vulkan")
            })
        })
    })
}

/// Sets `graphicsApi:<api>` in `home`'s graphics settings when it is unset or `auto` and options.txt
/// names no API either, keeping every other line. True when it changed; a player's own choice stays.
pub fn switch_from_auto(home: &Path, api: &str) -> Result<bool, String> {
    if options_choose_api(home) {
        return Ok(false);
    }
    let path = home.join(GRAPHICS_FILE);
    let text = match fs::read_to_string(&path) {
        Ok(text) => text,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(e) => return Err(format!("{}: {e}", path.display())),
    };
    let mut lines: Vec<String> = Vec::new();
    let mut found = false;
    for line in text.lines() {
        match line.split_once(':') {
            Some((key, value)) if key.trim() == "graphicsApi" => {
                if value.trim() != "auto" {
                    return Ok(false);
                }
                found = true;
                lines.push(format!("graphicsApi:{api}"));
            }
            _ => lines.push(line.to_owned()),
        }
    }
    if !found {
        lines.push(format!("graphicsApi:{api}"));
    }
    fs::create_dir_all(home).map_err(|e| format!("{}: {e}", home.display()))?;
    fs::write(&path, lines.join("\n") + "\n").map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(true)
}

/// Renderer workarounds a player may have exported to start the launcher itself (a transparent
/// window on old Mesa drivers): they belong to the launcher's own webview and must not reach the
/// game, which picks its own backend. Inherited, they left the client on CPU rendering at 2-3 FPS
/// (#7832).
pub const LAUNCHER_RENDER_ENV: [&str; 4] = [
    "LIBGL_ALWAYS_SOFTWARE",
    "GALLIUM_DRIVER",
    "WEBKIT_DISABLE_DMABUF_RENDERER",
    "WEBKIT_DISABLE_COMPOSITING_MODE",
];

fn strip_launcher_render_env(cmd: &mut Command) {
    for key in LAUNCHER_RENDER_ENV {
        cmd.env_remove(key);
    }
}

/// The game's program: on macOS the one inside the version's Horizon.app, so the Dock and Cmd+Tab
/// show Horizon's name and icon (game_app.rs); the bare binary elsewhere, or if that fails.
pub(crate) fn client_program(tag: &str) -> PathBuf {
    let dir = version_dir(tag);
    #[cfg(target_os = "macos")]
    match crate::game_app::ensure(&dir) {
        Ok(program) => return program,
        Err(e) => eprintln!("Starting the game outside {}: {e}", crate::game_app::APP),
    }
    dir.join(CLIENT)
}

// ----- Shared steps for the front ends -----------------------------------------------------------

/// What a command that needs an installed build says when there is none: the supervisor puts
/// the club's builds in place, never the launcher.
pub fn no_version_installed() -> String {
    use crate::i18n::tr;
    tr("No game build is installed yet. Ask the club's supervisor to install one.").to_string()
}

/// What the window says when `tag` is not prepared: the club's files arrive complete, so one
/// that is not is the supervisor's to fix, never the launcher's.
pub fn not_prepared(tag: &str) -> String {
    crate::i18n::trf(
        "{version}'s files are not ready, so the game can't start. Ask the club's supervisor to prepare the game.",
        &[("version", tag)],
    )
}

/// [`not_prepared`]'s sentence when `tag` is not prepared, checked on the disk now: what a
/// start asks before it starts anything.
pub fn need_prepared(tag: &str) -> Result<(), String> {
    if check_prepared(tag) {
        Ok(())
    } else {
        Err(not_prepared(tag))
    }
}

/// Says on stderr which step Play is at, one line each: a terminal run shows where it stops when
/// it never gets to the game (#12179).
pub fn play_step(step: &str) {
    eprintln!("Horizon: Play: {step}…");
}

/// Whether process `pid` is running. One this user may not look at counts as running.
#[cfg(windows)]
pub(crate) fn process_alive(pid: u32) -> bool {
    use windows_sys::Win32::Foundation::{
        CloseHandle, GetLastError, ERROR_ACCESS_DENIED, STILL_ACTIVE,
    };
    use windows_sys::Win32::System::Threading::{
        GetExitCodeProcess, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION,
    };
    // SAFETY: the handle is checked before use and closed once; `code` outlives the call.
    unsafe {
        let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if handle.is_null() {
            return GetLastError() == ERROR_ACCESS_DENIED;
        }
        let mut code = 0u32;
        // An exited process stays openable while something holds a handle to it.
        let running = GetExitCodeProcess(handle, &mut code) != 0 && code as i32 == STILL_ACTIVE;
        CloseHandle(handle);
        running
    }
}

/// Whether process `pid` is running. One this user may not look at counts as running.
#[cfg(unix)]
pub(crate) fn process_alive(pid: u32) -> bool {
    let Ok(pid) = libc::pid_t::try_from(pid) else {
        return false;
    };
    // SAFETY: signal 0 sends nothing, it only asks whether the process is there.
    pid > 0
        && (unsafe { libc::kill(pid, 0) } == 0
            || std::io::Error::last_os_error().raw_os_error() == Some(libc::EPERM))
}

/// A running process: the game's. Its output arrives as `Msg::Line`, its end as `Msg::Exit`.
pub struct Running {
    pub child: Arc<Mutex<Child>>,
    pub stdin: Option<ChildStdin>,
}

impl Running {
    /// Ends the process where Stop asks for it.
    pub fn kill(&self) {
        let mut child = self.child.lock().unwrap();
        #[cfg(unix)]
        {
            // GUI children own their group. Never signal a cached, already-reaped PID.
            if matches!(child.try_wait(), Ok(None)) {
                unsafe {
                    libc::kill(-(child.id() as i32), libc::SIGKILL);
                }
            }
        }
        #[cfg(not(unix))]
        let _ = child.kill();
    }
}

/// A command as a process runs it ([`prepare_command`]), with what is kept while it runs.
pub struct Staged {
    command: Command,
    /// The game's slot: its lock and the build's program, held until the game ends
    /// (`windows_runtime`). None for any other command.
    #[cfg(windows)]
    slot: Option<crate::windows_runtime::Guard>,
}

/// `cmd` as a process runs it. On Windows the game's command ([`client_command`]) runs from its
/// staged copy (`windows_runtime`), which the first Play of a build copies: on a share that takes
/// seconds, so the window calls this off its own thread.
pub fn prepare_command(cmd: Command) -> Result<Staged, String> {
    #[cfg(windows)]
    {
        let program = cmd.get_program().to_string_lossy().into_owned();
        let (command, slot) = crate::windows_runtime::stage(cmd).map_err(|error| {
            crate::i18n::trf(
                "Could not run {program}: {error}",
                &[("program", &program), ("error", &error.to_string())],
            )
        })?;
        Ok(Staged { command, slot })
    }
    #[cfg(not(windows))]
    {
        Ok(Staged { command: cmd })
    }
}

/// Why `cmd` could not start, naming its program and its folder: the OS error alone says neither.
fn not_started(cmd: &Command, error: std::io::Error) -> String {
    let program = Path::new(cmd.get_program()).display();
    let folder = cmd.get_current_dir().unwrap_or(Path::new(".")).display();
    crate::i18n::trf(
        "Could not complete game launch: {error}",
        &[(
            "error",
            &format!("execute {program} (working directory {folder}): {error}"),
        )],
    )
}

/// Starts a command [`prepare_command`] made ready. Its output arrives as `Msg::Line`, its end
/// as `Msg::Exit`; [`Running::kill`] stops it.
pub fn spawn(staged: Staged, tx: Sender<Msg>) -> Result<Running, String> {
    let mut cmd = staged.command;
    // Held until the process ended: on Windows the game's slot is the next start's only then.
    #[cfg(windows)]
    let slot = staged.slot;
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        cmd.process_group(0);
    }
    crate::process::hide_console(&mut cmd);
    cmd.stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = cmd.spawn().map_err(|error| not_started(&cmd, error))?;
    let stdin = child.stdin.take();
    let mut readers = Vec::new();
    let out = child
        .stdout
        .take()
        .map(|o| Box::new(o) as Box<dyn Read + Send>);
    let err = child
        .stderr
        .take()
        .map(|o| Box::new(o) as Box<dyn Read + Send>);
    for stream in [out, err].into_iter().flatten() {
        let tx = tx.clone();
        readers.push(std::thread::spawn(move || {
            for line in BufReader::new(stream).lines().map_while(Result::ok) {
                let _ = tx.send(Msg::Line(line));
            }
        }));
    }
    let child = Arc::new(Mutex::new(child));
    let waited = child.clone();
    std::thread::spawn(move || {
        for r in readers {
            let _ = r.join();
        }
        let status = loop {
            if let Some(s) = waited.lock().unwrap().try_wait().transpose() {
                break s;
            }
            std::thread::sleep(std::time::Duration::from_millis(100));
        };
        let result = match status {
            Ok(s) if s.success() => Ok(()),
            Ok(s) => Err(format!("Exited with {s}")),
            Err(e) => Err(e.to_string()),
        };
        #[cfg(windows)]
        drop(slot);
        let _ = tx.send(Msg::Exit(result));
    });
    Ok(Running { child, stdin })
}

mod settings;
pub use settings::{Appearance, Settings, WhilePlaying};

#[cfg(test)]
#[path = "backend/java_tests.rs"]
mod java_tests;

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::OsStr;
    /// A preparation that reuses the imported game files still reads the assets from the
    /// launcher that has them: a Prism Launcher instance's are in the launcher root, linked into
    /// the official layout (jan.sonki on Discord: silent, grey title screen since 0.2.48).
    #[test]
    fn reused_imports_take_the_assets_from_the_launcher_that_has_them() {
        let dir =
            std::env::temp_dir().join(format!("horizon-reused-assets-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        let client = b"fake client jar";
        let manifest = br#"{"id": "26.3"}"#;
        let write = |path: &str, data: &[u8]| {
            let path = dir.join(path);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, data).unwrap();
        };
        let prism = "PrismLauncher";
        write(
            &format!("{prism}/libraries/com/mojang/minecraft/26.3/minecraft-26.3-client.jar"),
            client,
        );
        write(&format!("{prism}/assets/indexes/34.json"), b"{}");
        let instance = dir.join(prism).join("instances/26.3/minecraft");
        fs::create_dir_all(&instance).unwrap();
        let pins = sources::Pins::read(&serde_json::json!({ "minecraft": {
            "version": "26.3",
            "client_sha1": sources::sha1_hex(client),
            "manifest_sha1": sources::sha1_hex(manifest),
        }}))
        .unwrap();
        let linked = dir.join("linked");
        let fetch = |_: &str| Ok(manifest.to_vec());
        let lines = std::cell::RefCell::new(Vec::new());
        let log = |line: String| lines.borrow_mut().push(line);
        let resolved = sources::resolve(&instance, &pins, &[], &linked, None, &fetch, &log);
        let folder = assets_folder(resolved, &instance, "26.3", &log);
        assert_eq!(folder, linked);
        assert!(folder.join("assets/indexes/34.json").is_file());

        // Nothing found: the chosen folder, and a line saying what the game will lack.
        let empty = dir.join("empty");
        fs::create_dir_all(&empty).unwrap();
        let resolved = sources::resolve(&empty, &pins, &[], &linked, None, &fetch, &log);
        assert_eq!(assets_folder(resolved, &empty, "26.3", &log), empty);
        assert!(lines.borrow().last().unwrap().contains("will be silent"));
        fs::remove_dir_all(&dir).unwrap();
    }
    #[test]
    fn the_game_plays_under_its_own_audio_stream_name() {
        if std::env::var_os("PIPEWIRE_PROPS").is_some() {
            return;
        }
        let cmd = client_command("v2026.10.13", "Jdupont", Path::new("managed.txt"));
        let props = cmd
            .get_envs()
            .find(|(k, _)| *k == "PIPEWIRE_PROPS")
            .and_then(|(_, v)| v);
        assert!(props.is_some_and(|v| {
            v.to_string_lossy()
                .contains("application.name = \"Horizon\"")
        }));
    }
    #[test]
    fn a_crash_after_the_vulkan_adapter_line_and_before_the_first_frame_is_a_start_crash() {
        let start = [
            "Horizon: resources loaded in 11.3s; 35723 block states, 1658 items, 67 biomes",
            "Horizon renderer: Intel(R) HD Graphics 530 (Vulkan, IntegratedGpu)",
        ];
        let vulkan = Some(StartCrash::Vulkan);
        assert_eq!(renderer_start_crash(start), vulkan);
        // Step lines after the adapter, one ending in a parenthesis, keep it a start crash.
        let step = [
            start[1],
            "Horizon renderer: device ready; configuring the surface (1920x1080 Bgra8Unorm, Fifo)",
            "Horizon renderer: creating the pipelines",
        ];
        assert_eq!(renderer_start_crash(step), vulkan);
        // A crash after the first frame is the game's, not the renderer start's.
        let later = [start[1], "Horizon: first frame after 812 ms"];
        assert_eq!(renderer_start_crash(later), None);
        // Already on DirectX 12, or no renderer yet: nothing to switch.
        let dx12 = ["Horizon renderer: Intel(R) HD Graphics 530 (Dx12, IntegratedGpu)"];
        assert_eq!(renderer_start_crash(dx12), None);
        assert_eq!(renderer_start_crash(&start[..1]), None);
        // Vulkan failed to open and DirectX 12 took over: its line is the last adapter.
        assert_eq!(renderer_start_crash([start[1], dx12[0]]), None);
    }
    /// #18291, #18518: Intel's OpenGL 4.0 driver for Ivy Bridge dies building the pipelines; the
    /// game's output, as #18518 reported it, from its first renderer line to its last.
    #[test]
    fn a_crash_after_the_opengl_adapter_line_and_before_the_first_frame_is_an_opengl_start_crash() {
        let log = [
            "Horizon renderer: asking Vulkan for a graphics device",
            "Horizon renderer: trying DirectX 12 (Vulkan surface: Failed to create surface for any enabled backend: {})",
            "Horizon renderer: DirectX 12 has no conformant adapter (DirectX 12: only a software adapter (Microsoft Basic Render Driver)); asking again for the ones wgpu hides as non-conformant",
            "Horizon renderer: trying OpenGL (Vulkan surface: Failed to create surface for any enabled backend: {}; DirectX 12: only a software adapter (Microsoft Basic Render Driver))",
            "Horizon renderer: Intel(R) HD Graphics (Gl, IntegratedGpu)",
            "Horizon renderer: requesting the device (driver 4.0.0 - Build 10.18.10.5161) with features [CLEAR_TEXTURE | TIMESTAMP_QUERY] and the adapter's limits (buffers up to 2147483647 bytes)",
            "Horizon renderer: device ready; configuring the surface (1280x720 Bgra8Unorm, Fifo)",
            "Horizon renderer: HDR output not offered by this display",
            "Horizon renderer: creating the pipelines",
        ];
        assert_eq!(renderer_start_crash(log), Some(StartCrash::OpenGl));
        let framed = [log[4], log[8], "Horizon: first frame after 2310 ms"];
        assert_eq!(renderer_start_crash(framed), None);
        // Vulkan had a device but OpenGL took over later: OpenGL's crash, not Vulkan's.
        let vulkan = "Horizon renderer: Intel(R) HD Graphics 4000 (Vulkan, IntegratedGpu)";
        assert_eq!(
            renderer_start_crash([vulkan, log[4], log[8]]),
            Some(StartCrash::OpenGl)
        );
    }
    /// Where each start crash sends Auto: DirectX 12 on Windows after either (#746, #18291);
    /// on Linux OpenGL after Vulkan (#4293) and nothing after OpenGL; nothing on macOS.
    #[test]
    fn a_start_crash_falls_back_by_api_and_system() {
        let (vulkan, gl) = (StartCrash::Vulkan, StartCrash::OpenGl);
        let api = |crashed, os| start_crash_fallback(crashed, os).map(|(api, _)| api);
        assert_eq!(api(vulkan, "windows"), Some("dx12"));
        assert_eq!(api(gl, "windows"), Some("dx12"));
        assert_eq!(api(vulkan, "linux"), Some("opengl"));
        assert_eq!(api(gl, "linux"), None);
        assert_eq!(api(vulkan, "macos"), None);
        assert_eq!(api(gl, "macos"), None);
    }
    #[test]
    fn a_renderer_start_crash_switches_auto_to_dx12_once() {
        let home = std::env::temp_dir().join(format!("horizon-dx12-switch-{}", std::process::id()));
        let _ = fs::remove_dir_all(&home);
        // No settings file yet: one with the API alone.
        assert_eq!(switch_from_auto(&home, "dx12"), Ok(true));
        assert_eq!(
            fs::read_to_string(home.join(GRAPHICS_FILE)).unwrap(),
            "graphicsApi:dx12\n"
        );
        // Already DirectX 12, or the player's own Vulkan: left alone.
        assert_eq!(switch_from_auto(&home, "dx12"), Ok(false));
        fs::write(home.join(GRAPHICS_FILE), "sky:false\ngraphicsApi:vulkan\n").unwrap();
        assert_eq!(switch_from_auto(&home, "dx12"), Ok(false));
        // Auto becomes DirectX 12 and the other settings stay.
        fs::write(
            home.join(GRAPHICS_FILE),
            "sky:false\ngraphicsApi:auto\nfog:true\n",
        )
        .unwrap();
        assert_eq!(switch_from_auto(&home, "dx12"), Ok(true));
        assert_eq!(
            fs::read_to_string(home.join(GRAPHICS_FILE)).unwrap(),
            "sky:false\ngraphicsApi:dx12\nfog:true\n"
        );
        let _ = fs::remove_dir_all(&home);
    }
    /// Stop ends a starting game too, without a report: only a driver's crash switches the API.
    #[test]
    fn stop_is_not_a_native_crash() {
        #[cfg(windows)]
        {
            use std::os::windows::process::ExitStatusExt;
            let exit = |code: u32| native_crash_exit(std::process::ExitStatus::from_raw(code));
            assert!(exit(0xC000_0005), "access violation");
            assert!(exit(0xC000_0409), "fail fast");
            assert!(!exit(1), "Stop's TerminateProcess");
            assert!(!exit(0));
        }
        #[cfg(unix)]
        {
            use std::os::unix::process::ExitStatusExt;
            // Raw wait statuses: a signal number alone, or an exit code shifted by 8.
            let exit = |raw: i32| native_crash_exit(std::process::ExitStatus::from_raw(raw));
            assert!(exit(libc::SIGSEGV), "the game itself");
            assert!(
                !exit(1 << 8),
                "exit code 1 is an ordinary exit, as Stop's TerminateProcess leaves on Windows"
            );
            assert!(!exit(libc::SIGKILL), "Stop");
            assert!(!exit(libc::SIGTERM));
            assert!(!exit(2 << 8));
        }
    }
    /// Video Settings' Graphics API (options.txt) is the player's own choice under `graphicsApi:auto`.
    #[test]
    fn an_api_chosen_in_options_txt_is_not_switched() {
        let home = std::env::temp_dir().join(format!("horizon-options-api-{}", std::process::id()));
        let _ = fs::remove_dir_all(&home);
        fs::create_dir_all(&home).unwrap();
        fs::write(home.join(GRAPHICS_FILE), "graphicsApi:auto\n").unwrap();
        fs::write(
            home.join("options.txt"),
            "version:4790\npreferredGraphicsBackend:\"opengl\"\n",
        )
        .unwrap();
        assert_eq!(switch_from_auto(&home, "dx12"), Ok(false));
        assert_eq!(
            fs::read_to_string(home.join(GRAPHICS_FILE)).unwrap(),
            "graphicsApi:auto\n"
        );
        fs::write(
            home.join("options.txt"),
            "version:4790\npreferredGraphicsBackend:\"default\"\n",
        )
        .unwrap();
        assert_eq!(switch_from_auto(&home, "dx12"), Ok(true));
        let _ = fs::remove_dir_all(&home);
    }
    /// #4293: an Ivy Bridge on Linux whose Vulkan driver (Mesa's hasvk) crashes at the renderer's
    /// start plays on OpenGL next time.
    #[test]
    fn a_vulkan_start_crash_on_linux_switches_auto_to_opengl() {
        let (api, _) = start_crash_fallback(StartCrash::Vulkan, "linux").unwrap();
        let home = std::env::temp_dir().join(format!("horizon-gl-switch-{}", std::process::id()));
        let _ = fs::remove_dir_all(&home);
        fs::create_dir_all(&home).unwrap();
        fs::write(home.join(GRAPHICS_FILE), "graphicsApi:auto\nvsync:true\n").unwrap();
        assert_eq!(switch_from_auto(&home, api), Ok(true));
        assert_eq!(
            fs::read_to_string(home.join(GRAPHICS_FILE)).unwrap(),
            "graphicsApi:opengl\nvsync:true\n"
        );
        // The next crash (OpenGL had no GPU either and Vulkan ran again) leaves it: no loop.
        assert_eq!(switch_from_auto(&home, api), Ok(false));
        let _ = fs::remove_dir_all(&home);
    }
    #[test]
    fn the_window_stays_while_playing_unless_picked() {
        assert_eq!(Settings::parse("{}").while_playing, WhilePlaying::Stay);
        let hide = Settings::parse(r#"{"while_playing": "hide", "theme": "nord"}"#);
        assert_eq!(hide.while_playing, WhilePlaying::Hide);
        // A value a launcher doesn't know is the default, and the other settings stay.
        let future = Settings::parse(r#"{"while_playing": "fade", "theme": "nord"}"#);
        assert_eq!(
            (future.while_playing, future.theme.as_deref()),
            (WhilePlaying::Stay, Some("nord"))
        );
        assert_eq!(
            WhilePlaying::parse("Minimise"),
            Some(WhilePlaying::Minimize)
        );
        assert_eq!(WhilePlaying::parse("off"), Some(WhilePlaying::Stay));
        assert_eq!(WhilePlaying::parse("close"), None);
        for w in WhilePlaying::ALL {
            let text = serde_json::to_string(&w).unwrap();
            assert_eq!(text, format!("\"{}\"", w.name()));
        }
        let here = WhilePlaying::Hide.here();
        assert_eq!(here == WhilePlaying::Hide, cfg!(target_os = "macos"));
    }
    /// The window's language is the club's (`club.toml`) until a student picks one in Settings.
    #[test]
    fn the_language_is_the_clubs_unless_picked() {
        use crate::config::Language as Club;
        use crate::i18n::Language;
        assert_eq!(Settings::parse("{}").language, None);
        assert_eq!(
            Settings::parse("{}").language(Club::French),
            Language::French
        );
        assert_eq!(
            Settings::parse("{}").language(Club::English),
            Language::English
        );
        // A student's pick wins over the club's.
        let fr = Settings::parse(r#"{"language": "fr"}"#);
        assert_eq!(fr.language(Club::English), Language::French);
        // A language this launcher doesn't have is dropped: the club's, and the rest stays.
        let future = Settings::parse(r#"{"language": "tlh", "theme": "nord"}"#);
        assert_eq!(
            (future.language(Club::French), future.theme.as_deref()),
            (Language::French, Some("nord"))
        );
        let saved = serde_json::to_value(Settings::default()).unwrap();
        assert!(saved.get("language").is_none(), "unset is left out");
    }
    #[test]
    fn glass_and_blur_default_to_the_mockups_and_stay_in_range() {
        let unset = Settings::parse("{}");
        assert_eq!((unset.glass_tint(), unset.glass_blur()), (0.06, 24.));
        let set = Settings::parse(r#"{"glass": 12, "blur": 8.5}"#);
        assert_eq!((set.glass_tint(), set.glass_blur()), (0.12, 8.5));
        let wild = Settings::parse(r#"{"glass": 90, "blur": -3}"#);
        assert_eq!((wild.glass_tint(), wild.glass_blur()), (0.16, 0.));
        // Not a number: the default, and the other settings survive.
        let wrong = Settings::parse(r#"{"glass": "frosty", "blur": null, "theme": "nord"}"#);
        assert_eq!(
            (
                wrong.glass_tint(),
                wrong.glass_blur(),
                wrong.theme.as_deref()
            ),
            (0.06, 24., Some("nord"))
        );
    }
    #[test]
    fn the_accent_is_violet_unless_picked_and_an_unknown_one_keeps_the_rest() {
        use crate::themes::Accent;
        assert_eq!(Settings::parse("{}").accent, Accent::VIOLET);
        let teal =
            Settings::parse(r#"{"accent": "teal", "background": "background-0123456789ab.png"}"#);
        assert_eq!(
            (teal.accent, teal.background.as_deref()),
            (Accent::TEAL, Some("background-0123456789ab.png"))
        );
        let future = Settings::parse(r#"{"accent": "plaid", "theme": "nord"}"#);
        assert_eq!(
            (future.accent, future.theme.as_deref()),
            (Accent::VIOLET, Some("nord"))
        );
    }
    #[test]
    fn an_old_appearance_moves_to_a_theme_and_an_unknown_one_keeps_the_rest() {
        assert_eq!(Settings::parse("{}").appearance, Appearance::Dark);
        assert_eq!(Settings::parse("{}").theme, None);
        // Light, alone or under a theme, opens on Taiga and keeps the rest.
        for text in [
            r#"{"appearance": "light", "while_playing": "minimize"}"#,
            r#"{"appearance": "light", "theme": "deep_ocean", "while_playing": "minimize"}"#,
        ] {
            let light = Settings::parse(text);
            assert_eq!(
                (
                    light.appearance,
                    light.theme.as_deref(),
                    light.while_playing
                ),
                (Appearance::Dark, Some("taiga"), WhilePlaying::Minimize),
                "{text}"
            );
        }
        // Dark and the system's keep the saved theme's look.
        for appearance in ["dark", "system"] {
            let text = format!(r#"{{"appearance": "{appearance}", "theme": "nord"}}"#);
            let kept = Settings::parse(&text);
            assert_eq!(
                (kept.appearance, kept.theme.as_deref()),
                (Appearance::Dark, Some("nord"))
            );
        }
        let future = Settings::parse(
            r#"{"appearance": "sepia", "theme": "nord", "while_playing": "minimize"}"#,
        );
        assert_eq!(
            (
                future.appearance,
                future.theme.as_deref(),
                future.while_playing
            ),
            (Appearance::Dark, Some("nord"), WhilePlaying::Minimize)
        );
        // Saved back as dark, so a migrated light stays on Taiga only until the player picks.
        let text = serde_json::to_string(&Settings::parse(r#"{"appearance": "light"}"#)).unwrap();
        assert!(text.contains(r#""appearance":"dark""#), "{text}");
    }
    #[test]
    fn the_version_is_the_one_the_release_workflow_injects() {
        assert_eq!(version_or(Some("0.4.2")), "0.4.2");
        assert_eq!(version_or(Some("")), env!("CARGO_PKG_VERSION"));
        assert_eq!(version_or(None), env!("CARGO_PKG_VERSION"));
        assert_eq!(
            VERSION,
            option_env!("HORIZON_LAUNCHER_VERSION")
                .filter(|v| !v.is_empty())
                .unwrap_or(env!("CARGO_PKG_VERSION"))
        );
    }
    #[test]
    fn a_process_that_has_exited_is_not_alive() {
        assert!(process_alive(std::process::id()));
        // The test binary itself, which exits at once on an option it doesn't know.
        let mut child = Command::new(std::env::current_exe().unwrap())
            .arg("--version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        let pid = child.id();
        child.wait().unwrap();
        drop(child);
        assert!(!process_alive(pid));
    }
    #[test]
    fn a_drawn_page_asks_the_disk_once_and_the_launch_check_asks_again() {
        use std::time::{Duration, Instant};
        let dir =
            std::env::temp_dir().join(format!("horizon-launcher-cached-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join(RESOURCES_MANIFEST), "{}").unwrap();
        fs::write(dir.join("items.json"), "{}").unwrap();
        let version = dir.join("version-folder");
        let cache = Mutex::new(PreparedCache::default());
        let t0 = Instant::now();
        // The page's question: without the cache each of these would run the check.
        let disk_reads = std::cell::Cell::new(0);
        let draw = |now| {
            cached_prepared(&cache, &version, now, || {
                disk_reads.set(disk_reads.get() + 1);
                prepared_at(&dir)
            })
        };
        assert!(draw(t0), "a complete folder is prepared");

        // A table is emptied behind the cache (a killed export): a frame still answers from
        // memory, without touching the disk, for hundreds of frames.
        fs::write(dir.join("items.json"), "").unwrap();
        for frame in 1..=500 {
            assert!(draw(t0 + Duration::from_millis(frame)), "frame {frame}");
        }
        assert_eq!(disk_reads.get(), 1, "500 frames read the disk once");

        // Before a launch the folder is checked on the disk, and finds the empty table.
        assert!(
            !checked_prepared(&cache, &version, &dir, t0),
            "an empty table is caught"
        );
        // ... and the pages then show it without checking again.
        assert!(!draw(t0 + Duration::from_secs(1)));
        assert_eq!(disk_reads.get(), 1);

        // Repaired, the folder is believed prepared again once the answer is dropped or old.
        fs::write(dir.join("items.json"), "{}").unwrap();
        assert!(!draw(t0 + Duration::from_secs(2)), "still the old answer");
        cache.lock().unwrap().0.clear();
        assert!(draw(t0 + Duration::from_secs(2)));
        assert_eq!(disk_reads.get(), 2);
        fs::write(dir.join("items.json"), "").unwrap();
        assert!(draw(t0 + Duration::from_secs(3)), "answer is a second old");
        assert!(
            !draw(t0 + Duration::from_secs(2) + PREPARED_TTL),
            "past the time limit a change made by another process shows"
        );
        assert_eq!(disk_reads.get(), 3);
        fs::remove_dir_all(dir).unwrap();
    }
    #[test]
    fn each_version_keeps_its_own_prepared_answer() {
        let now = std::time::Instant::now();
        let cache = Mutex::new(PreparedCache::default());
        let (a, b) = (Path::new("versions/a"), Path::new("versions/b"));
        assert!(cached_prepared(&cache, a, now, || true));
        assert!(!cached_prepared(&cache, b, now, || false));
        assert!(cached_prepared(&cache, a, now, || unreachable!(
            "a is known"
        )));
        assert!(!cached_prepared(&cache, b, now, || unreachable!(
            "b is known"
        )));
    }
    #[test]
    fn a_prepared_folder_with_an_empty_table_is_not_prepared() {
        let dir =
            std::env::temp_dir().join(format!("horizon-launcher-empty-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        assert!(!prepared_at(&dir), "no manifest is not prepared");

        fs::write(dir.join(RESOURCES_MANIFEST), "{}").unwrap();
        assert!(prepared_at(&dir));

        // A killed export left the biggest table empty: Play must refuse, not start a game
        // that crashes on it.
        fs::write(dir.join("gameplay-facts.json"), "").unwrap();
        assert!(!prepared_at(&dir));

        fs::write(dir.join("gameplay-facts.json"), "{}").unwrap();
        assert!(prepared_at(&dir));

        fs::write(dir.join(RESOURCES_MANIFEST), "").unwrap();
        assert!(!prepared_at(&dir), "an empty manifest is not prepared");
        fs::remove_dir_all(dir).unwrap();
    }
    #[test]
    fn crash_reports_are_found_and_summarised() {
        let dir =
            std::env::temp_dir().join(format!("horizon-launcher-crash-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        let start = std::time::SystemTime::now() - std::time::Duration::from_secs(2);
        assert!(crash_report_in_since(&dir, start).is_none());
        fs::create_dir_all(&dir).unwrap();
        let report = "---- Horizon Crash Report ----\n// Details below.\n\nTime: 2026-10-14 09:05:07\nDescription: Unexpected error\n\npanicked at src/game.rs:10:5:\nboom\n\t0: horizon_client::game::frame\n";
        fs::write(dir.join("crash-2026-10-14_09.05.07-client.txt"), report).unwrap();
        fs::write(dir.join("notes.txt"), "x").unwrap();
        assert_eq!(
            crash_report_in_since(&dir, start).unwrap(),
            dir.join("crash-2026-10-14_09.05.07-client.txt")
        );
        let later = std::time::SystemTime::now() + std::time::Duration::from_secs(60);
        assert!(crash_report_in_since(&dir, later).is_none());
        assert_eq!(
            crash_summary(report).as_deref(),
            Some("panicked at src/game.rs:10:5: boom")
        );
        assert_eq!(crash_summary("no report"), None);
        let _ = fs::remove_dir_all(&dir);
    }
    #[test]
    fn the_game_does_not_inherit_the_launchers_renderer_workarounds() {
        let mut cmd = Command::new("true");
        cmd.env("GALLIUM_DRIVER", "llvmpipe")
            .env("LIBGL_ALWAYS_SOFTWARE", "1")
            .env("WEBKIT_DISABLE_DMABUF_RENDERER", "1")
            .env("WEBKIT_DISABLE_COMPOSITING_MODE", "1")
            .env("PIPEWIRE_PROPS", "kept");
        strip_launcher_render_env(&mut cmd);
        let envs = cmd.get_envs().collect::<std::collections::HashMap<_, _>>();
        for key in LAUNCHER_RENDER_ENV {
            assert_eq!(envs.get(OsStr::new(key)), Some(&None), "{key} is removed");
        }
        // Only the launcher's own workarounds go: the rest of the environment still reaches the game.
        assert_eq!(
            envs.get(OsStr::new("PIPEWIRE_PROPS")),
            Some(&Some(OsStr::new("kept")))
        );
    }
    /// The builds of `core\` as the window lists them: every folder with the game's program,
    /// newest first by the numbers in its name (`club-10-…` before `club-9-…`); one the
    /// supervisor is still copying in has no program yet and waits.
    #[test]
    fn the_builds_are_listed_newest_first_and_only_once_whole() {
        let scratch = tempfile::tempdir().unwrap();
        let versions = scratch.path().join("core");
        let program = crate::club::client_program_name();
        for tag in ["club-9-old", "club-10-new", "club-2-older"] {
            fs::create_dir_all(versions.join(tag)).unwrap();
            fs::write(versions.join(tag).join(program), b"").unwrap();
        }
        fs::create_dir_all(versions.join("club-11-copying")).unwrap();
        assert_eq!(
            installed_in(&versions),
            ["club-10-new", "club-9-old", "club-2-older"]
        );
        // Dropped in whole while the window was open: the next read of the list sees it.
        fs::write(versions.join("club-11-copying").join(program), b"").unwrap();
        assert_eq!(
            installed_in(&versions),
            [
                "club-11-copying",
                "club-10-new",
                "club-9-old",
                "club-2-older"
            ]
        );
        // The whole `core` folder gone: nothing left to play.
        fs::remove_dir_all(&versions).unwrap();
        assert!(installed_in(&versions).is_empty());
    }
    /// A preparation asked for a build that is not on the disk: the sentence, never the
    /// folder's own error from the steps below.
    #[test]
    fn a_preparation_of_a_build_that_is_gone_says_so() {
        let scratch = tempfile::tempdir().unwrap();
        let error = preparer("deleted-scratch-version", scratch.path(), &|_| {})
            .err()
            .unwrap();
        assert_eq!(
            error,
            "deleted-scratch-version is no longer on this computer."
        );
    }
    /// The only folders with the game lack one of its libraries, and the window cannot fetch
    /// the game itself: one sentence naming the library, and the tool that completes them.
    #[test]
    fn incomplete_minecraft_files_name_the_library_and_the_completing_tool() {
        let download =
            "\n  • Complete 26.3's files from Mojang with: club-admin prepare --download";
        let chosen = Path::new("chosen");
        let checked = vec![(chosen.to_path_buf(), "no Minecraft 26.3 client jar".into())];
        let error = incomplete_or_not_found(
            "26.3",
            &checked,
            chosen,
            Some("azure-json-1.4.0.jar".into()),
            download,
        );
        assert_eq!(
            error,
            "Your Minecraft 26.3 files are incomplete (azure-json-1.4.0.jar is missing). \
             Complete 26.3's files from Mojang with: club-admin prepare --download"
        );
        // Nothing found at all: every folder looked in, as before.
        let error = incomplete_or_not_found("26.3", &checked, chosen, None, download);
        assert!(
            error.starts_with("Minecraft 26.3 was not found."),
            "{error}"
        );
        assert!(error.ends_with(download), "{error}");
    }
}
mod messages;
pub use messages::Msg;
