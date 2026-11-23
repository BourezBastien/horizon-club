//! Windows runs the game from a staged copy in the launcher's data folder, so that the firewall
//! sees one program path whatever build plays. The build itself stays where the club keeps it,
//! often a network share, and is only read.
//!
//! A slot (`<data>\run\<key>\`) belongs to one game home; its lock is held until the game exits.
//! It holds the build's program and everything beside it but its prepared files (`.local`, which
//! `--resources` names where they are). On a share a copy is seconds of network, so the build is
//! copied only when the slot doesn't hold it: the slot keeps a record of what it was given, and
//! the build is read again at each start to see whether it changed (a folder beside the program
//! is known again by its own time alone, not by what is inside it).
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::ffi::OsStr;
use std::fs;
use std::io;
use std::os::windows::fs::{MetadataExt, OpenOptionsExt};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

/// The mark of the game's command ([`mark`]): the folder of the build it runs, as the club root
/// spells it. The launcher's own: [`stage_in`] takes it off before anything starts.
const SOURCE: &str = "HORIZON_RUNTIME_SOURCE";
/// The build's prepared files: the one thing beside the program that stays on the share.
const NOT_STAGED: &str = ".local";
/// The slot's lock, and what it holds.
const LOCK: &str = "run.lock";
const RECORD: &str = "staged.json";
/// The folder of a slot the build is copied into.
const GAME: &str = "game";

/// Marks the game's command with the folder of the build it runs (`backend::client_command`):
/// [`stage`] stages that command and no other.
pub(crate) fn mark(cmd: &mut Command, source: &Path) {
    cmd.env(SOURCE, source);
}

/// What a start keeps of the staging: the slot's lock and the build's program, each held until
/// the game has exited.
pub(crate) struct Guard {
    _slot: fs::File,
    _program: fs::File,
}

/// `cmd` as it runs: the marked game's from its slot, copied again when its build changed; any
/// other command as it is, without reaching the data folder.
pub(crate) fn stage(cmd: Command) -> io::Result<(Command, Option<Guard>)> {
    if marked(&cmd).is_none() {
        return Ok((cmd, None));
    }
    stage_in(cmd, &crate::backend::data_dir().join("run"))
}

fn stage_in(cmd: Command, root: &Path) -> io::Result<(Command, Option<Guard>)> {
    let Some(source) = marked(&cmd) else {
        return Ok((cmd, None));
    };
    let home = cmd
        .get_envs()
        .find(|(name, _)| *name == "HORIZON_HOME")
        .and_then(|(_, value)| value)
        .map(PathBuf::from)
        .or_else(|| cmd.get_current_dir().map(Path::to_path_buf))
        .expect("the game command has a home");
    let slot = root.join(key(&home));
    let guard = fill(&source, &slot)?;
    let mut out = Command::new(slot.join(GAME).join(crate::backend::CLIENT));
    out.args(cmd.get_args());
    if let Some(dir) = cmd.get_current_dir() {
        out.current_dir(dir);
    }
    for (name, value) in cmd.get_envs() {
        match value {
            Some(value) => {
                out.env(name, value);
            }
            None => {
                out.env_remove(name);
            }
        };
    }
    out.env_remove(SOURCE);
    Ok((out, Some(guard)))
}

/// The build folder a command was [`mark`]ed with.
fn marked(cmd: &Command) -> Option<PathBuf> {
    cmd.get_envs()
        .find(|(name, _)| *name == SOURCE)
        .and_then(|(_, value)| value)
        .map(PathBuf::from)
}

/// A game home's slot: named for the home's path, so that two homes never share one.
fn key(home: &Path) -> String {
    format!("{:x}", Sha256::digest(home.as_os_str().as_encoded_bytes()))
}

/// Makes `slot` hold the build in `source`, and locks it for the game about to run.
fn fill(source: &Path, slot: &Path) -> io::Result<Guard> {
    plain_directory(slot)?;
    let lock = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(slot.join(LOCK))?;
    // Taken already: a game of this home is starting or running from another launcher.
    lock.try_lock().map_err(io::Error::other)?;
    // Opened so, the build's program can be neither replaced nor deleted while the game runs.
    let program = fs::OpenOptions::new()
        .read(true)
        .share_mode(1)
        .open(source.join(crate::backend::CLIENT))?;
    let game = slot.join(GAME);
    plain_directory(&game)?;
    let build = listing(source)?;
    // A record no longer vouches for what the slot holds once a file of it is gone (an antivirus
    // that quarantined one, a folder cleaned by hand).
    let held = record(slot)?.filter(|held| intact(&game, held));
    if held.as_ref() != Some(&build) {
        eprintln!("Horizon: copying the game from {}", source.display());
        // Gone first: a copy that stops halfway must not leave a record vouching for what the
        // slot held before it.
        match fs::remove_file(slot.join(RECORD)) {
            Err(error) if error.kind() != io::ErrorKind::NotFound => return Err(error),
            _ => {}
        }
        copy(source, &game)?;
        fs::write(slot.join(RECORD), serde_json::to_vec(&build)?)?;
    }
    Ok(Guard {
        _slot: lock,
        _program: program,
    })
}

/// A folder that is one (made when missing), not a link: a replaced slot or game folder must
/// never lead a write elsewhere.
fn plain_directory(path: &Path) -> io::Result<()> {
    fs::create_dir_all(path)?;
    if fs::symlink_metadata(path)?.file_attributes() & 0x400 != 0 {
        return Err(io::ErrorKind::InvalidInput.into());
    }
    Ok(())
}

/// What the slot was last given: None when it never was, or the record is unreadable.
fn record(slot: &Path) -> io::Result<Option<Vec<Entry>>> {
    Ok(fs::read(slot.join(RECORD))
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok()))
}

/// Whether everything the record names is still in the slot's game folder.
fn intact(game: &Path, held: &[Entry]) -> bool {
    held.iter()
        .all(|entry| game.join(&entry.name).try_exists().is_ok_and(|there| there))
}

/// The build folder as the record keeps it: its program and everything beside it but its
/// prepared files, by name.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
struct Entry {
    name: String,
    /// A folder: copied whole, and known again by nothing more than its own time.
    folder: bool,
    /// A file's length in bytes; 0 for a folder.
    length: u64,
    /// When it was last written.
    modified: Stamped,
}

/// A time as the record keeps it: the seconds since 1970, then the nanoseconds after them.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
struct Stamped(i64, u32);

impl Stamped {
    fn of(time: SystemTime) -> Self {
        match time.duration_since(UNIX_EPOCH) {
            Ok(after) => Self(after.as_secs() as i64, after.subsec_nanos()),
            // Before 1970 the seconds count down from -1 and the nanoseconds up from it: half a
            // second earlier is (-1, half a second).
            Err(before) => {
                let before = before.duration();
                Self(
                    -(before.as_secs() as i64) - i64::from(before.subsec_nanos() != 0),
                    before.subsec_nanos(),
                )
            }
        }
    }
}

/// The build folder's entries, by name, each asked for itself (not read off the folder's listing,
/// which lags behind a file still being written).
fn listing(source: &Path) -> io::Result<Vec<Entry>> {
    let mut entries = Vec::new();
    for entry in fs::read_dir(source)? {
        let path = entry?.path();
        if path.file_name() == Some(OsStr::new(NOT_STAGED)) {
            continue;
        }
        let meta = fs::metadata(&path)?;
        entries.push(Entry {
            name: path
                .file_name()
                .and_then(|name| name.to_str())
                .map(str::to_owned)
                .ok_or_else(|| {
                    io::Error::new(
                        io::ErrorKind::InvalidData,
                        "a build file's name is not text",
                    )
                })?,
            folder: meta.is_dir(),
            length: if meta.is_dir() { 0 } else { meta.len() },
            modified: Stamped::of(meta.modified()?),
        });
    }
    entries.sort();
    Ok(entries)
}

/// Copies the build into the slot's game folder, replacing what it held.
fn copy(source: &Path, game: &Path) -> io::Result<()> {
    // The program first: Windows refuses to replace it while an earlier game still runs it, and
    // then nothing of what the slot held has been removed.
    fs::copy(
        source.join(crate::backend::CLIENT),
        game.join(crate::backend::CLIENT),
    )?;
    for entry in fs::read_dir(game)? {
        let entry = entry?;
        if entry.file_name() == crate::backend::CLIENT {
            continue;
        }
        if entry.file_type()?.is_dir() {
            fs::remove_dir_all(entry.path())?;
        } else {
            fs::remove_file(entry.path())?;
        }
    }
    // Everything beside the program goes along, `dxcompiler.dll` first of all: DirectX 12 loads
    // it beside the game on integrated graphics.
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        if entry.file_name() == crate::backend::CLIENT || entry.file_name() == NOT_STAGED {
            continue;
        }
        let to = game.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            crate::backend::copy_dir(&entry.path(), &to)?;
        } else {
            fs::copy(entry.path(), to)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    /// A build on the club's share: the game's program, a library it loads beside itself, a
    /// folder, and its prepared files.
    fn fake_build(at: PathBuf) -> PathBuf {
        fs::create_dir_all(at.join(".local/horizon-client/26.3")).unwrap();
        fs::create_dir_all(at.join("LICENSES")).unwrap();
        fs::write(at.join(crate::backend::CLIENT), b"game").unwrap();
        fs::write(at.join("dxcompiler.dll"), b"dxc").unwrap();
        fs::write(at.join("LICENSES/dxc.txt"), b"notice").unwrap();
        fs::write(at.join(".local/horizon-client/26.3/res.json"), b"{}").unwrap();
        at
    }

    /// A start of the game: Play's command, staged under `slots` for the game home `home`.
    fn stage_again(build: &Path, home: &Path, slots: &Path) -> (Command, Option<Guard>) {
        let mut cmd = Command::new(build.join(crate::backend::CLIENT));
        cmd.current_dir(home)
            .env("HORIZON_HOME", home)
            .arg("--resources")
            .arg(build.join(".local/horizon-client/26.3"));
        mark(&mut cmd, build);
        stage_in(cmd, slots).unwrap()
    }

    /// The build's library as of `time`, in place: the same length, another time.
    fn relibrary(build: &Path, text: &[u8], time: Duration) {
        let library = build.join("dxcompiler.dll");
        fs::write(&library, text).unwrap();
        fs::OpenOptions::new()
            .write(true)
            .open(library)
            .unwrap()
            .set_modified(UNIX_EPOCH + time)
            .unwrap();
    }

    #[test]
    fn a_time_is_kept_as_seconds_then_nanoseconds_on_either_side_of_1970() {
        // The nanoseconds are a multiple of one hundred: Windows keeps a time in units of
        // 100 ns, and anything finer would not survive `SystemTime` on the machines this runs on.
        let at = UNIX_EPOCH + Duration::new(1_800_000_000, 123_456_700);
        assert_eq!(Stamped::of(at), Stamped(1_800_000_000, 123_456_700));
        assert_eq!(Stamped::of(UNIX_EPOCH), Stamped(0, 0));
        assert_eq!(
            Stamped::of(UNIX_EPOCH - Duration::new(0, 500_000_000)),
            Stamped(-1, 500_000_000)
        );
        assert_eq!(
            Stamped::of(UNIX_EPOCH - Duration::new(1, 500_000_000)),
            Stamped(-2, 500_000_000)
        );
        assert_eq!(
            Stamped::of(UNIX_EPOCH - Duration::from_secs(2)),
            Stamped(-2, 0)
        );
    }

    /// Play copies the build into its slot once: its program, what sits beside it and its
    /// folders, without the prepared files. While the build stays as it is and the slot keeps
    /// every file, nothing is copied again; a file of the slot gone, or one the supervisor
    /// replaced, is copied, and what the build no longer has goes.
    #[test]
    fn a_build_is_copied_once_and_again_only_when_it_changes() {
        let root = tempfile::tempdir().unwrap();
        let (home, slots) = (root.path().join("home"), root.path().join("run"));
        fs::create_dir_all(&home).unwrap();
        let build = fake_build(root.path().join("core/26.3-20261122"));
        let staged = |slots: &Path| {
            let mut held = fs::read_dir(slots).unwrap();
            let slot = held.next().unwrap().unwrap().path();
            assert!(held.next().is_none(), "one game home, one slot");
            slot
        };

        let (run, lock) = stage_again(&build, &home, &slots);
        let game = staged(&slots).join(GAME);
        assert_eq!(
            Path::new(run.get_program()),
            game.join(crate::backend::CLIENT)
        );
        assert_eq!(run.get_current_dir(), Some(home.as_path()));
        assert_eq!(
            run.get_args().collect::<Vec<_>>(),
            [
                std::ffi::OsStr::new("--resources"),
                build.join(".local/horizon-client/26.3").as_os_str()
            ]
        );
        // Set to be removed, as `get_envs` reports a variable a command takes off.
        let mark = run
            .get_envs()
            .find(|(name, _)| name.to_str() == Some(SOURCE))
            .map(|(_, value)| value);
        assert_eq!(
            mark,
            Some(None),
            "the launcher's mark tells no process anything"
        );
        assert_eq!(
            fs::read(game.join(crate::backend::CLIENT)).unwrap(),
            b"game"
        );
        assert_eq!(fs::read(game.join("dxcompiler.dll")).unwrap(), b"dxc");
        assert_eq!(fs::read(game.join("LICENSES/dxc.txt")).unwrap(), b"notice");
        assert!(
            !game.join(".local").exists(),
            "prepared files stay on the share"
        );
        drop(lock);

        // The same build: the slot is left as it is, down to a file nobody else would keep.
        fs::write(game.join("dxcompiler.dll"), b"kept").unwrap();
        let (_, lock) = stage_again(&build, &home, &slots);
        assert_eq!(
            fs::read(game.join("dxcompiler.dll")).unwrap(),
            b"kept",
            "an unchanged build is not copied again"
        );
        drop(lock);

        // A file of the slot gone: the build is copied again, whole.
        fs::remove_file(game.join("dxcompiler.dll")).unwrap();
        let (_, lock) = stage_again(&build, &home, &slots);
        assert_eq!(fs::read(game.join("dxcompiler.dll")).unwrap(), b"dxc");
        drop(lock);

        // The supervisor replaces the build's library: same length, another content and time.
        relibrary(&build, b"DXC", Duration::from_secs(1_800_000_000));
        fs::write(game.join("gone.dll"), b"").unwrap();
        let (_, lock) = stage_again(&build, &home, &slots);
        assert_eq!(fs::read(game.join("dxcompiler.dll")).unwrap(), b"DXC");
        assert!(
            !game.join("gone.dll").exists(),
            "what the build lacks, the slot lacks"
        );
        assert_eq!(
            fs::read(game.join(crate::backend::CLIENT)).unwrap(),
            b"game"
        );
        drop(lock);

        // A record that is not there or unreadable vouches for nothing: the build is copied again.
        fs::write(game.join("dxcompiler.dll"), b"kept").unwrap();
        fs::remove_file(staged(&slots).join(RECORD)).unwrap();
        let (_, lock) = stage_again(&build, &home, &slots);
        assert_eq!(fs::read(game.join("dxcompiler.dll")).unwrap(), b"DXC");
        drop(lock);

        // Another build: its program goes in the same slot, so the firewall still sees one path.
        let next = fake_build(root.path().join("core/26.3-20261201"));
        fs::write(next.join(crate::backend::CLIENT), b"next game").unwrap();
        let (run, lock) = stage_again(&next, &home, &slots);
        assert_eq!(
            Path::new(run.get_program()),
            game.join(crate::backend::CLIENT)
        );
        assert_eq!(
            fs::read(game.join(crate::backend::CLIENT)).unwrap(),
            b"next game"
        );
        drop(lock);
    }

    /// A command that is not the game's, and a game of another home, each runs as it is or from
    /// a slot of its own.
    #[test]
    fn only_the_marked_game_is_staged_and_each_home_has_its_slot() {
        let root = tempfile::tempdir().unwrap();
        let (home, other, slots) = (
            root.path().join("home"),
            root.path().join("other"),
            root.path().join("run"),
        );
        fs::create_dir_all(&home).unwrap();
        fs::create_dir_all(&other).unwrap();
        let build = fake_build(root.path().join("core/26.3-20261122"));

        let plain = Command::new(build.join(crate::backend::CLIENT));
        let (run, lock) = stage_in(plain, &slots).unwrap();
        assert_eq!(
            Path::new(run.get_program()),
            build.join(crate::backend::CLIENT)
        );
        assert!(lock.is_none());

        let (_, first) = stage_again(&build, &home, &slots);
        let (_, second) = stage_again(&build, &other, &slots);
        assert_eq!(fs::read_dir(&slots).unwrap().count(), 2);
        drop((first, second));
    }

    /// While a game of a home runs from its slot, another start of that home is refused.
    #[test]
    fn a_second_game_of_the_same_home_is_refused_while_the_first_runs() {
        let root = tempfile::tempdir().unwrap();
        let (home, slots) = (root.path().join("home"), root.path().join("run"));
        fs::create_dir_all(&home).unwrap();
        let build = fake_build(root.path().join("core/26.3-20261122"));
        let (_, lock) = stage_again(&build, &home, &slots);
        let mut cmd = Command::new(build.join(crate::backend::CLIENT));
        cmd.current_dir(&home).env("HORIZON_HOME", &home);
        mark(&mut cmd, &build);
        assert!(stage_in(cmd, &slots).is_err());
        drop(lock);
        let _ = stage_again(&build, &home, &slots);
    }
}
