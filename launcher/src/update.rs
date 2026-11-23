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
const SUMS: &str = "SHA256SUMS.txt";
/// The most a launcher may weigh: it is some 30 MB today.
const MAX_LAUNCHER: usize = 256 << 20;
/// The most a release's description or list of hashes may weigh.
const MAX_DOCUMENT: usize = 1 << 20;

/// A release of the club's repository that holds a launcher.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Release {
    /// Its tag without the `v`, which is its folder's name in `app\`.
    pub version: String,
    /// Where GitHub hands the launcher's file over from.
    pub launcher: String,
    /// Where the release's list of hashes is.
    pub sums: String,
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
    let engine = downloader()?;
    let body = downloads::run(engine.fetch(url, &AtomicBool::new(false), MAX_DOCUMENT))?;
    let release = release_of(&String::from_utf8_lossy(&body))?;
    Ok(is_newer(&release.version, current).then_some(release))
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
    Ok(Release {
        version: tag.trim_start_matches('v').to_owned(),
        launcher: file(LAUNCHER)?,
        sums: file(SUMS)?,
    })
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
    let folder = folder_of(root, &release.version)?;
    let engine = downloader()?;
    let cancel = AtomicBool::new(false);
    let file = downloads::run(engine.fetch(&release.launcher, &cancel, MAX_LAUNCHER))?;
    let sums = downloads::run(engine.fetch(&release.sums, &cancel, MAX_DOCUMENT))?;
    verify(&file, &sums)?;
    std::fs::create_dir_all(&folder).map_err(|e| {
        crate::i18n::trf(
            "{folder} cannot be created: {error}",
            &[
                ("folder", &folder.display().to_string()),
                ("error", &e.to_string()),
            ],
        )
    })?;
    // Written beside its place and renamed over it: the bootstrap never starts a half of a file.
    let part = folder.join(format!("{LAUNCHER}.part"));
    std::fs::write(&part, &file).map_err(|e| {
        crate::i18n::trf(
            "{file} cannot be written: {error}",
            &[
                ("file", &part.display().to_string()),
                ("error", &e.to_string()),
            ],
        )
    })?;
    std::fs::rename(&part, folder.join(LAUNCHER)).map_err(|e| {
        let _ = std::fs::remove_file(&part);
        crate::i18n::trf(
            "{file} cannot be put in place: {error}",
            &[
                ("file", &folder.join(LAUNCHER).display().to_string()),
                ("error", &e.to_string()),
            ],
        )
    })?;
    point_at(root, &release.version)
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

/// Whether `file` is the launcher the release's `sums` names: a Windows program, with the
/// SHA-256 its line gives.
fn verify(file: &[u8], sums: &[u8]) -> Result<(), String> {
    if !file.starts_with(b"MZ") {
        return Err(crate::i18n::tr("The downloaded file is not a program.").to_string());
    }
    let expected = listed_hash(sums, LAUNCHER);
    let hash = format!("{:x}", Sha256::digest(file));
    match expected {
        Some(expected) if expected == hash => Ok(()),
        Some(_) => Err(crate::i18n::tr(
            "The downloaded file is not the release's: its hash differs from the release's list.",
        )
        .to_string()),
        None => Err(crate::i18n::trf(
            "The release's list of hashes has no line for {name}.",
            &[("name", LAUNCHER)],
        )),
    }
}

#[cfg(test)]
mod tests;
