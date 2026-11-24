//! The launcher's update (docs/school-edition/README.md, « Mettre à jour »).
//!
//! The launcher lives once, on the club's share, in `app\<version>\`, and every PC starts the
//! version `app\current.txt` names (the bootstrap reads it). An update is therefore made once,
//! on the share, and reaches every PC at its next start: no PC updates anything of its own, and
//! a student's launcher never asks the Internet for anything. The new versions are the releases
//! of the club's public repository ([`check`]).
//!
//! Only a session that may write in the share's `app\` (the supervisor's) looks for one, and
//! the launcher asks before it changes the share. [`install`] puts the new version beside the
//! one in use and points `current.txt` at it: the version before stays, and `current.txt` is
//! how one goes back.
//!
//! Every file is read through the launcher's downloader under its GitHub guard: GitHub's servers
//! alone, over HTTPS, each redirect checked again. A launcher is installed only when it weighs
//! as a release's list of hashes says, and that list comes from the same release.
use crate::downloads::{self, Config, Downloader, Guard};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::sync::OnceLock;

/// The launcher's file, in a release and in `app\<version>\` (the name the bootstrap looks for).
pub const LAUNCHER: &str = "HorizonClubLauncher.exe";
/// A release's list of its files' SHA-256, one `<hash>  <name>` a line.
pub const SUMS: &str = "SHA256SUMS.txt";
/// The bootstrap's file, at the share's root: what every PC's shortcut starts.
pub const BOOTSTRAP: &str = "HorizonClub.exe";
/// The supervisor's tool, in the share's `admin\`.
pub const ADMIN: &str = "club-admin.exe";
/// The most a program of a release may weigh: the launcher is some 30 MB today.
const MAX_PROGRAM: usize = 256 << 20;
/// The most a release's description or list of hashes may weigh.
const MAX_DOCUMENT: usize = 1 << 20;

/// A release of the club's repository.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Release {
    /// Its tag without the `v`, which is its folder's name in `app\`.
    pub version: String,
    /// Where GitHub hands the launcher's file over from.
    pub launcher: String,
    /// Where the release's list of hashes is.
    pub sums: String,
    /// Where GitHub hands the bootstrap over from, when the release has one.
    pub bootstrap: Option<String>,
    /// Where GitHub hands the supervisor's tool over from, when the release has one.
    pub admin: Option<String>,
}

impl Release {
    /// Where GitHub hands `name` ([`LAUNCHER`], [`BOOTSTRAP`], [`ADMIN`]) over from.
    pub fn file(&self, name: &str) -> Option<&str> {
        match name {
            LAUNCHER => Some(&self.launcher),
            BOOTSTRAP => self.bootstrap.as_deref(),
            ADMIN => self.admin.as_deref(),
            _ => None,
        }
    }
}

/// A file of a release as the share holds it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Place {
    /// `HorizonClub.exe` at the root.
    Bootstrap,
    /// `app\<version>\HorizonClubLauncher.exe`, then `app\current.txt`.
    Launcher,
    /// `admin\club-admin.exe`.
    Admin,
}

impl Place {
    /// Its file's name in a release.
    pub fn name(self) -> &'static str {
        match self {
            Place::Bootstrap => BOOTSTRAP,
            Place::Launcher => LAUNCHER,
            Place::Admin => ADMIN,
        }
    }
}

/// What [`setup`] put on the share: each file, where it is.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Installed {
    pub files: Vec<PathBuf>,
}

/// The downloader the updates go through: the launcher's settings, under the GitHub guard. The
/// engine's own tests serve from the loopback, so there the guard stays off.
fn downloader() -> Result<&'static Downloader, String> {
    static ENGINE: OnceLock<Result<Downloader, String>> = OnceLock::new();
    ENGINE
        .get_or_init(|| {
            let mut config = Config::from_env();
            config.guard = if cfg!(test) {
                Guard::Off
            } else {
                Guard::GitHub
            };
            Downloader::new(config)
        })
        .as_ref()
        .map_err(Clone::clone)
}

/// The club's newest release when it is newer than `current` (the running launcher's version).
pub fn check(current: &str) -> Result<Option<Release>, String> {
    let url = std::env::var("HORIZON_CLUB_RELEASES")
        .unwrap_or_else(|_| horizon_addresses::CLUB_RELEASES.to_owned());
    check_at(&url, current)
}

/// [`check`] at `url`: the address GitHub's API describes a newest release at.
pub fn check_at(url: &str, current: &str) -> Result<Option<Release>, String> {
    let release = newest_at(url)?;
    Ok(is_newer(&release.version, current).then_some(release))
}

/// The club's newest release, whatever version runs: what the installer ([`setup`]) puts on a
/// share.
pub fn newest() -> Result<Release, String> {
    let url = std::env::var("HORIZON_CLUB_RELEASES")
        .unwrap_or_else(|_| horizon_addresses::CLUB_RELEASES.to_owned());
    newest_at(&url)
}

/// [`newest`] at `url`.
pub fn newest_at(url: &str) -> Result<Release, String> {
    let engine = downloader()?;
    let body = downloads::run(engine.fetch(url, &AtomicBool::new(false), MAX_DOCUMENT))?;
    release_of(&String::from_utf8_lossy(&body))
}

/// Whether `candidate` names a version after `current`: their numbers, dot by dot (`0.10` is
/// after `0.9`). A name that is not numbers and dots is never newer.
pub fn is_newer(candidate: &str, current: &str) -> bool {
    match (numbers(candidate), numbers(current)) {
        (Some(candidate), Some(current)) => candidate > current,
        _ => false,
    }
}

fn numbers(version: &str) -> Option<Vec<u64>> {
    let parts: Option<Vec<u64>> = version.split('.').map(|p| p.parse().ok()).collect();
    parts.filter(|parts| !parts.is_empty())
}

/// The release GitHub's API describes in `json` (<https://docs.github.com/rest/releases/releases>).
fn release_of(json: &str) -> Result<Release, String> {
    let value: serde_json::Value = serde_json::from_str(json).map_err(|e| {
        crate::i18n::trf(
            "GitHub's answer is not a release's description: {error}",
            &[("error", &e.to_string())],
        )
    })?;
    let tag = value["tag_name"]
        .as_str()
        .ok_or_else(|| crate::i18n::tr("GitHub's answer names no release.").to_string())?;
    let file = |name: &str| -> Result<String, String> {
        value["assets"]
            .as_array()
            .and_then(|assets| assets.iter().find(|a| a["name"] == name))
            .and_then(|a| a["browser_download_url"].as_str())
            .map(str::to_owned)
            .ok_or_else(|| {
                // A file's name is not translated: it is the same in every language.
                crate::i18n::trf("The release has no {name}.", &[("name", name)])
            })
    };
    // A release from before the installer existed has the launcher and its list alone.
    let there = |name: &str| {
        value["assets"]
            .as_array()
            .is_some_and(|assets| assets.iter().any(|a| a["name"] == name))
    };
    Ok(Release {
        version: tag.trim_start_matches('v').to_owned(),
        launcher: file(LAUNCHER)?,
        sums: file(SUMS)?,
        bootstrap: there(BOOTSTRAP).then(|| file(BOOTSTRAP)).transpose()?,
        admin: there(ADMIN).then(|| file(ADMIN)).transpose()?,
    })
}

/// Installs a release into `root` from the start: the launcher in `app\<version>\` with
/// `app\current.txt` naming it, the bootstrap at the root, and the supervisor's tool in
/// `admin\`, each checked against the release's list of hashes. The files in `beside` (a folder
/// holding the release's own files, as its download gives them) are used when they are there and
/// are the release's; what is missing is downloaded. `said` hears each step. `root` is written
/// as given: it is what the students' PCs see.
pub fn setup(
    root: &Path,
    release: &Release,
    beside: &Path,
    said: &mut dyn FnMut(String),
) -> Result<Installed, String> {
    let engine = downloader()?;
    let cancel = AtomicBool::new(false);
    let sums = match std::fs::read(beside.join(SUMS)) {
        Ok(sums) if !sums.is_empty() => sums,
        _ => downloads::run(engine.fetch(&release.sums, &cancel, MAX_DOCUMENT))?,
    };
    let mut files = Vec::new();
    for place in [Place::Launcher, Place::Bootstrap, Place::Admin] {
        let name = place.name();
        let mine = std::fs::read(beside.join(name))
            .ok()
            .filter(|bytes| verify_named(bytes, &sums, name).is_ok());
        let bytes = match mine {
            Some(bytes) => {
                said(crate::i18n::trf(
                    "{name}: the file beside the installer.",
                    &[("name", name)],
                ));
                bytes
            }
            None => {
                said(crate::i18n::trf("{name}: downloading…", &[("name", name)]));
                let bytes = fetch(release, name, &sums)?;
                said(crate::i18n::trf(
                    "{name}: checked by its hash.",
                    &[("name", name)],
                ));
                bytes
            }
        };
        let target = match place {
            Place::Bootstrap => root.join(BOOTSTRAP),
            Place::Admin => root.join("admin").join(ADMIN),
            Place::Launcher => folder_of(root, &release.version)?.join(LAUNCHER),
        };
        put(&target, &bytes)?;
        said(crate::i18n::trf(
            "Installed {file}.",
            &[("file", &target.display().to_string())],
        ));
        files.push(target);
    }
    point_at(root, &release.version)?;
    said(crate::i18n::trf(
        "app\\current.txt now says {version}.",
        &[("version", &release.version)],
    ));
    Ok(Installed { files })
}

/// Whether this session may change the share's launcher: a file can be written in its `app\`.
/// Students read the share only: their launcher never looks for an update.
pub fn may_update(root: &Path) -> bool {
    let probe = root
        .join("app")
        .join(format!(".club-write-test-{}", std::process::id()));
    let written = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&probe);
    let _ = std::fs::remove_file(&probe);
    written.is_ok()
}

/// Downloads the release's launcher into `app\<version>\`, after checking that it is what the
/// release's list of hashes says, then makes `app\current.txt` name that version.
pub fn install(root: &Path, release: &Release) -> Result<(), String> {
    // The version's name is checked before anything is downloaded.
    let folder = folder_of(root, &release.version)?;
    let engine = downloader()?;
    let cancel = AtomicBool::new(false);
    let sums = downloads::run(engine.fetch(&release.sums, &cancel, MAX_DOCUMENT))?;
    let file = fetch(release, LAUNCHER, &sums)?;
    put(&folder.join(LAUNCHER), &file)?;
    point_at(root, &release.version)
}

/// A file of the release, downloaded and checked against the release's `sums`: `name` is its
/// name in the release ([`LAUNCHER`], [`BOOTSTRAP`], [`ADMIN`]).
pub fn fetch(release: &Release, name: &str, sums: &[u8]) -> Result<Vec<u8>, String> {
    let url = release.file(name).ok_or_else(|| missing(name))?;
    let engine = downloader()?;
    let bytes = downloads::run(engine.fetch(url, &AtomicBool::new(false), MAX_PROGRAM))?;
    verify_named(&bytes, sums, name)?;
    Ok(bytes)
}

/// Writes a program whole: beside its place, then renamed over it, so nothing starts half of a
/// file. One already there and the same is left alone (it may be running: the bootstrap is, from
/// the shortcut of a PC).
fn put(target: &Path, bytes: &[u8]) -> Result<(), String> {
    if std::fs::read(target).is_ok_and(|there| there == bytes) {
        return Ok(());
    }
    if let Some(folder) = target.parent() {
        std::fs::create_dir_all(folder).map_err(|e| {
            crate::i18n::trf(
                "{folder} cannot be created: {error}",
                &[
                    ("folder", &folder.display().to_string()),
                    ("error", &e.to_string()),
                ],
            )
        })?;
    }
    let part = target.with_extension("part");
    std::fs::write(&part, bytes).map_err(|e| cannot_write(&part, &e))?;
    std::fs::rename(&part, target).map_err(|e| {
        let _ = std::fs::remove_file(&part);
        cannot_write(target, &e)
    })
}

fn cannot_write(file: &Path, error: &std::io::Error) -> String {
    crate::i18n::trf(
        "{file} cannot be written: {error}",
        &[
            ("file", &file.display().to_string()),
            ("error", &error.to_string()),
        ],
    )
}

fn missing(name: &str) -> String {
    // A file's name is not translated: it is the same in every language.
    crate::i18n::trf("The release has no {name}.", &[("name", name)])
}

/// The folder of a version under `app\`: a version is a plain folder name, as the bootstrap
/// reads `current.txt`, so it cannot leave `app\`.
fn folder_of(root: &Path, version: &str) -> Result<PathBuf, String> {
    let plain = !version.is_empty()
        && version.len() <= 64
        && version
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b))
        && version != "."
        && version != "..";
    if plain {
        Ok(root.join("app").join(version))
    } else {
        Err(crate::i18n::trf(
            "\"{version}\" is not a version's name.",
            &[("version", version)],
        ))
    }
}

/// Makes `app\current.txt` name `version`: written beside itself, then renamed over it, so the
/// bootstrap never reads a half-written line.
fn point_at(root: &Path, version: &str) -> Result<(), String> {
    let current = root.join("app").join("current.txt");
    let part = root.join("app").join(format!("current.{version}.txt"));
    std::fs::write(&part, format!("{version}\n")).map_err(|e| {
        let _ = std::fs::remove_file(&part);
        crate::i18n::trf(
            "{file} cannot be written: {error}",
            &[
                ("file", &current.display().to_string()),
                ("error", &e.to_string()),
            ],
        )
    })?;
    std::fs::rename(&part, &current).map_err(|e| {
        let _ = std::fs::remove_file(&part);
        crate::i18n::trf(
            "{file} cannot be put in place: {error}",
            &[
                ("file", &current.display().to_string()),
                ("error", &e.to_string()),
            ],
        )
    })
}

/// The SHA-256 a release's list gives for `file`. A line is `<hash>  <name>` as `sha256sum`
/// writes it in text mode, or `<hash> *<name>` in binary mode, its default on Windows; the line
/// ends are Windows' or another's.
fn listed_hash(sums: &[u8], file: &str) -> Option<String> {
    String::from_utf8_lossy(sums)
        .lines()
        .filter_map(|line| line.trim().split_once(char::is_whitespace))
        .find(|(_, name)| name.trim().trim_start_matches('*') == file)
        .map(|(hash, _)| hash.to_ascii_lowercase())
}

/// Whether `file` is the launcher the release's `sums` names ([`verify_named`] for the other
/// files of a release).
#[cfg(test)]
fn verify(file: &[u8], sums: &[u8]) -> Result<(), String> {
    verify_named(file, sums, LAUNCHER)
}

/// Whether `file` is the release's `name`: a Windows program, with the SHA-256 its line of the
/// `sums` gives.
pub fn verify_named(file: &[u8], sums: &[u8], name: &str) -> Result<(), String> {
    if !file.starts_with(b"MZ") {
        return Err(crate::i18n::tr("The downloaded file is not a program.").to_string());
    }
    let expected = listed_hash(sums, name);
    let hash = format!("{:x}", Sha256::digest(file));
    match expected {
        Some(expected) if expected == hash => Ok(()),
        Some(_) => Err(crate::i18n::tr(
            "The downloaded file is not the release's: its hash differs from the release's list.",
        )
        .to_string()),
        None => Err(crate::i18n::trf(
            "The release's list of hashes has no line for {name}.",
            &[("name", name)],
        )),
    }
}

#[cfg(test)]
mod tests;
