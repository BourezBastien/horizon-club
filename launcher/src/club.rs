//! The club's folders (docs/school-edition/launcher-target.md, section 2). The launcher, the
//! game builds and the game's files live on a read-only share a student cannot write to; each
//! student's own files live under `%LOCALAPPDATA%\HorizonClub`. Nothing here talks to the
//! network: the root is found by where the launcher runs from, never asked for online.

use std::path::{Path, PathBuf};

/// The file that marks a folder as the club's root: the club's configuration (`config.rs`).
pub(crate) const CONFIG_FILE: &str = "club.toml";
/// The rule of the game, the supervisor's tool writes it and the game reads it (`--managed`).
const MANAGED_FILE: &str = "managed.txt";
/// Where the launchers that host a server announce it, one JSON file each (`online.rs`).
const ONLINE_DIR: &str = "online";
/// How many folders up from the current directory the root is searched for, at most.
const WALK_UP: usize = 4;

/// The club's root folder: `HORIZON_CLUB_ROOT` when set; else, when the launcher's own
/// executable is `app\<version>\…` inside the root (`root_of_exe`), that root; else the nearest
/// folder up from the current directory, [`WALK_UP`] levels at most, that holds [`CONFIG_FILE`].
/// An empty path when none is found: the window then asks the supervisor for the folder, and
/// Play does nothing.
///
/// The answer is fixed on the first call, before the launcher makes its data folder the current
/// directory: the search up from where it started would otherwise look inside that data folder,
/// and a root given relative to the starting folder would name another one. Whatever path the
/// root has (a share, a mapped drive), it is kept as given: nothing resolves it to another
/// spelling, which the game and the tools it starts would have to read back.
pub fn root() -> PathBuf {
    static ROOT: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();
    ROOT.get_or_init(resolve_root).clone()
}

/// Whether [`root`] found a club root.
pub fn has_root() -> bool {
    !root().as_os_str().is_empty()
}

fn resolve_root() -> PathBuf {
    if let Some(set) = std::env::var_os("HORIZON_CLUB_ROOT").filter(|v| !v.is_empty()) {
        return absolute(PathBuf::from(set));
    }
    marked(root_of_exe(&std::env::current_exe().unwrap_or_default()))
        .or_else(|| root_around(&std::env::current_dir().unwrap_or_default()))
        .unwrap_or_default()
}

/// The root an executable names, only when it holds [`CONFIG_FILE`]: a folder merely *named*
/// `app` elsewhere (`C:\app\v1\…`) names a root that is then refused for want of the marker.
fn marked(candidate: Option<PathBuf>) -> Option<PathBuf> {
    candidate.filter(|root| root.join(CONFIG_FILE).is_file())
}

/// `path` as an absolute path, against the folder the launcher is in: a relative one is made
/// absolute where it was given. Nothing is read from the disk, and Windows' verbatim prefix
/// (`\\?\`) is left out, as some tools and comparisons trip on it.
fn absolute(path: PathBuf) -> PathBuf {
    std::path::absolute(&path).unwrap_or(path)
}

/// The root an executable sits three levels inside of, `root\app\<version>\<name>.exe`: the
/// version folder's parent must be called `app`, so a launcher anywhere else (a development
/// build in `target\debug`) answers None whatever is around it.
pub(crate) fn root_of_exe(exe: &Path) -> Option<PathBuf> {
    let version = exe.parent()?;
    let app = version.parent()?;
    if app.file_name()? != "app" {
        return None;
    }
    Some(app.parent()?.to_path_buf())
}

/// The nearest folder at or above `start`, [`WALK_UP`] levels at most, that holds [`CONFIG_FILE`].
pub(crate) fn root_around(start: &Path) -> Option<PathBuf> {
    let mut folder = start;
    for _ in 0..=WALK_UP {
        if folder.join(CONFIG_FILE).is_file() {
            return Some(folder.to_path_buf());
        }
        folder = folder.parent()?;
    }
    None
}

/// The launcher's data folder: `HORIZON_LAUNCHER_HOME` when set, else
/// `%LOCALAPPDATA%\HorizonClub\launcher`, made on first use.
pub fn data_dir() -> PathBuf {
    own_folder("HORIZON_LAUNCHER_HOME", "launcher")
}

/// Where the game keeps its saves and settings: `HORIZON_HOME` when set, else
/// `%LOCALAPPDATA%\HorizonClub\game`, made on first use.
pub fn game_home() -> PathBuf {
    own_folder("HORIZON_HOME", "game")
}

/// `leaf` under the student's own HorizonClub folder, or the environment's stand-in. Tests get
/// a scratch folder of their own, so a test run writes nothing into a student's folders.
fn own_folder(environment: &str, leaf: &str) -> PathBuf {
    if let Some(set) = std::env::var_os(environment).filter(|v| !v.is_empty()) {
        return PathBuf::from(set);
    }
    let dir = if cfg!(test) {
        std::env::temp_dir().join(format!("horizon-club-test-{}", std::process::id()))
    } else {
        dirs::data_local_dir()
            .unwrap_or_else(std::env::temp_dir)
            .join("HorizonClub")
    }
    .join(leaf);
    let _ = std::fs::create_dir_all(&dir);
    dir
}

/// The club's game builds: every `<root>\core\<tag>\` that holds the game's program. A read-only
/// share: the launcher lists and reads, and never writes inside.
pub fn versions_dir() -> PathBuf {
    root().join("core")
}

/// The game's program inside a build folder, as [`versions_dir`]'s builds hold it.
pub fn client_program_name() -> &'static str {
    if cfg!(windows) {
        "horizon-client.exe"
    } else {
        "horizon-client"
    }
}

/// The rule of the game, `<root>\managed.txt`, handed to the game with `--managed`. The
/// supervisor's tool writes it: the launcher only checks it is there, and never writes inside the
/// club's root.
pub fn managed_file() -> PathBuf {
    root().join(MANAGED_FILE)
}

/// Where the launchers that host a server announce it: `<root>\online\`, one JSON file each
/// ([`crate::online`]).
pub fn online_dir() -> PathBuf {
    root().join(ONLINE_DIR)
}

// ----- The club folders opened on this computer ---------------------------------------------------

/// Where they are kept, in the launcher's data folder: a club folder a line, the last opened first.
const KNOWN: &str = "clubs.txt";
/// How many are kept: enough for every club folder a computer opens.
const KNOWN_MOST: usize = 12;

/// Remembers `folder` as a club folder opened on this computer ([`known`]): an installer opened
/// in another folder then looks in these for the game, without asking. Nothing is written when
/// the folder already comes first, nor for one with no name; a file that cannot be written is
/// not an error, as nothing else depends on it.
pub fn remember(folder: &Path) {
    remember_in(&data_dir().join(KNOWN), folder);
}

/// The club folders opened on this computer that are still there, the last opened first.
pub fn known() -> Vec<PathBuf> {
    known_in(&data_dir().join(KNOWN))
}

/// [`remember`], with the file given.
pub(crate) fn remember_in(file: &Path, folder: &Path) {
    let folder = tidy(folder);
    if folder.as_os_str().is_empty() {
        return;
    }
    let mut folders: Vec<PathBuf> = read_known(file).into_iter().map(|f| tidy(&f)).collect();
    if folders
        .first()
        .is_some_and(|first| same_folder(first, &folder))
    {
        return;
    }
    folders.retain(|known| !same_folder(known, &folder));
    folders.insert(0, folder);
    folders.truncate(KNOWN_MOST);
    let _ = write_known(file, &folders);
}

/// [`known`], with the file given.
pub(crate) fn known_in(file: &Path) -> Vec<PathBuf> {
    read_known(file)
        .into_iter()
        .filter(|folder| folder.is_dir())
        .collect()
}

/// A folder as it is kept: without a separator at the end (`C:\Horizon\` is `C:\Horizon`), a
/// drive's own (`C:\`) keeping its one.
fn tidy(folder: &Path) -> PathBuf {
    let text = folder.to_string_lossy();
    let stem = text.trim_end_matches(['\\', '/']);
    let whole = stem.len() == 2 && stem.ends_with(':');
    PathBuf::from(if whole { &text[..stem.len() + 1] } else { stem })
}

/// Two spellings of one folder: the same path, compared without regard to case and to a
/// separator at the end (`C:\Horizon` and `c:\horizon\`).
fn same_folder(a: &Path, b: &Path) -> bool {
    let text = |p: &Path| {
        p.to_string_lossy()
            .trim_end_matches(['\\', '/'])
            .to_string()
    };
    text(a).eq_ignore_ascii_case(&text(b))
}

/// The folders in the file at `path`, each line one, as written; blank lines and a file that
/// cannot be read hold none.
fn read_known(path: &Path) -> Vec<PathBuf> {
    std::fs::read_to_string(path)
        .unwrap_or_default()
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(PathBuf::from)
        .collect()
}

/// Writes the folders to `path`, a line each: written beside, then renamed over it, so a launcher
/// starting meanwhile reads either list whole or none.
fn write_known(path: &Path, folders: &[PathBuf]) -> std::io::Result<()> {
    if let Some(folder) = path.parent() {
        std::fs::create_dir_all(folder)?;
    }
    let mut text = String::new();
    for folder in folders {
        text.push_str(&folder.display().to_string());
        text.push('\n');
    }
    let partial = path.with_extension("txt.tmp");
    std::fs::write(&partial, text)?;
    std::fs::rename(partial, path)
}

/// Whether build folder `a` is newer than `b`, for "the newest folder name": digit runs compare
/// as numbers and the rest letter by letter, so the CI's `club-10-…` follows `club-9-…` as its
/// number says (plain text would sort `club-9` last), and a date in a name still orders itself.
pub fn newer(a: &str, b: &str) -> std::cmp::Ordering {
    use std::cmp::Ordering;
    fn digit(c: char) -> bool {
        c.is_ascii_digit()
    }
    /// The text before the first digit, then the rest.
    fn letters(s: &str) -> (&str, &str) {
        s.split_at(s.find(digit).unwrap_or(s.len()))
    }
    /// The digits at the start, then the rest.
    fn digits(s: &str) -> (&str, &str) {
        s.split_at(s.find(|c| !digit(c)).unwrap_or(s.len()))
    }
    // A run of digits as a number: its leading zeros don't count.
    fn number(digits: &str) -> &str {
        digits.trim_start_matches('0')
    }
    let (mut a, mut b) = (a, b);
    loop {
        let (a_letters, a_rest) = letters(a);
        let (b_letters, b_rest) = letters(b);
        let ordered = a_letters.cmp(b_letters);
        if ordered != Ordering::Equal {
            return ordered;
        }
        let (a_number, a_tail) = digits(a_rest);
        let (b_number, b_tail) = digits(b_rest);
        // Both at their end (or two names that differ only in leading zeros).
        if a_number.is_empty() && b_number.is_empty() {
            return Ordering::Equal;
        }
        // As numbers: the longer one, without its leading zeros, is the bigger.
        let ordered = number(a_number)
            .len()
            .cmp(&number(b_number).len())
            .then_with(|| number(a_number).cmp(number(b_number)));
        if ordered != Ordering::Equal {
            return ordered;
        }
        (a, b) = (a_tail, b_tail);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn path(parts: &[&str]) -> PathBuf {
        parts.iter().map(PathBuf::from).collect()
    }

    #[test]
    fn an_exe_three_levels_inside_names_the_root_above_app() {
        let share = path(&[r"\\school\clubs\minecraft"]);
        assert_eq!(
            root_of_exe(
                &share
                    .join("app")
                    .join("26.3-20261122")
                    .join("HorizonClubLauncher.exe")
            ),
            Some(share.clone()),
            "root\\app\\<version>\\HorizonClubLauncher.exe: the share's layout"
        );
        // Any executable name, one level deeper than the version folder's own files.
        assert_eq!(
            root_of_exe(&path(&["C:", "club", "app", "v1", "bin", "launcher.exe"])),
            None,
            "the launcher is four levels inside: not the share's layout"
        );
    }

    #[test]
    fn an_exe_elsewhere_names_no_root() {
        assert_eq!(
            root_of_exe(&path(&["C:", "target", "debug", "launcher.exe"])),
            None
        );
        // A folder *named* app elsewhere names a root that the marker then refuses.
        let bare = root_of_exe(&path(&["C:", "app", "v1", "launcher.exe"]));
        assert_eq!(bare, Some(path(&["C:"])));
        assert_eq!(marked(bare), None);
        assert_eq!(root_of_exe(&path(&["C:", "club", "app.exe"])), None);
        assert_eq!(root_of_exe(Path::new("launcher.exe")), None);
        // The share's layout, with its marker, is accepted as it is.
        let base = std::env::temp_dir().join(format!("horizon-club-marked-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        let version = base.join("app").join("v1");
        std::fs::create_dir_all(&version).unwrap();
        std::fs::write(base.join(CONFIG_FILE), "").unwrap();
        assert_eq!(
            marked(root_of_exe(&version.join("launcher.exe"))),
            Some(base.clone()),
            "root\\app\\<version> with club.toml above: the share's layout"
        );
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn the_root_is_the_nearest_folder_with_the_marker_up_to_four_levels_up() {
        let base = std::env::temp_dir().join(format!("horizon-club-root-{}", std::process::id()));
        let clean = |leaf: &str| {
            let dir = base.join(leaf);
            let _ = std::fs::remove_dir_all(&dir);
            dir
        };
        // The marker one level above: the folder itself has none.
        let above = clean("above");
        std::fs::create_dir_all(above.join("room")).unwrap();
        std::fs::write(above.join(CONFIG_FILE), "").unwrap();
        assert_eq!(
            root_around(&above.join("room")),
            Some(above.clone()),
            "the folder itself counts as the nearest folder"
        );
        // Four levels up: found. The fifth: not.
        let deep_root = clean("deep");
        let deep = deep_root.join("a").join("b").join("c").join("d");
        std::fs::create_dir_all(&deep).unwrap();
        std::fs::write(deep_root.join(CONFIG_FILE), "").unwrap();
        assert_eq!(root_around(&deep), Some(deep_root.clone()));
        let deeper = deep_root.join("a").join("b").join("c").join("d").join("e");
        std::fs::create_dir_all(&deeper).unwrap();
        assert_eq!(root_around(&deeper), None, "five levels up is too far");
        // No marker anywhere: nothing.
        let bare = clean("bare");
        std::fs::create_dir_all(bare.join("x")).unwrap();
        assert_eq!(root_around(&bare.join("x")), None);
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn builds_live_in_core_under_the_root() {
        // The shape of the name a build's folder holds, for the platform the tests run on.
        let program = client_program_name();
        assert!(program.starts_with("horizon-client"), "{program}");
    }

    /// A root given relative to where the launcher starts still names the same folder once the
    /// launcher works from its data folder; a share or a mapped drive keeps its own spelling.
    #[test]
    fn a_relative_root_is_made_absolute_and_an_absolute_one_keeps_its_spelling() {
        let here = absolute(PathBuf::from("."));
        assert!(here.is_absolute(), "{}", here.display());
        assert!(
            !here.to_string_lossy().starts_with(r"\\?\"),
            "no verbatim prefix: {}",
            here.display()
        );
        // A share is a Windows spelling of a path.
        if cfg!(windows) {
            let share = path(&[r"\\school\clubs\minecraft"]);
            assert_eq!(absolute(share.clone()), share);
        }
        let drive = PathBuf::from(if cfg!(windows) {
            r"Z:\Horizon"
        } else {
            "/mnt/horizon"
        });
        assert_eq!(absolute(drive.clone()), drive);
    }

    /// "The newest folder name": the CI's `club-<run>-<commit>` orders itself by its number, a
    /// date in a name by the date, and a longer name follows its prefix.
    #[test]
    fn build_names_order_themselves_by_their_numbers_then_their_letters() {
        use std::cmp::Ordering::*;
        let mut builds = [
            "club-9-0a1b2c3",
            "club-10-0a1b2c4",
            "club-2-0a1b2c2",
            "26.3-20261121",
            "26.3-20261122",
            "club-10-0a1b2c4-extra",
        ];
        assert_eq!(newer("club-10-0a1b2c4", "club-9-0a1b2c3"), Greater);
        assert_eq!(newer("club-10-0a1b2c4", "club-10-0a1b2c4"), Equal);
        builds.sort_by(|a, b| newer(b, a));
        assert_eq!(
            builds,
            [
                "club-10-0a1b2c4-extra",
                "club-10-0a1b2c4",
                "club-9-0a1b2c3",
                "club-2-0a1b2c2",
                "26.3-20261122",
                "26.3-20261121",
            ],
            "newest first"
        );
    }

    /// The club folders opened on this computer: the last opened comes first, a folder is listed
    /// once whatever its spelling, and one that is gone is not listed.
    #[test]
    fn the_club_folders_opened_are_kept_the_last_one_first() {
        let root = std::env::temp_dir().join(format!("horizon-club-known-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let (file, first, second) = (
            root.join("launcher").join("clubs.txt"),
            root.join("Premier"),
            root.join("Second"),
        );
        std::fs::create_dir_all(&first).unwrap();
        std::fs::create_dir_all(&second).unwrap();
        // Nothing is kept yet: none, and no file to read.
        assert!(known_in(&file).is_empty());
        // A folder with no name is none to remember.
        remember_in(&file, Path::new(""));
        assert!(!file.exists());
        remember_in(&file, &first);
        remember_in(&file, &second);
        assert_eq!(known_in(&file), [second.clone(), first.clone()]);
        // Opened again, the first comes back first; its other spelling is the same folder.
        let again = PathBuf::from(format!("{}\\", first.display()));
        remember_in(&file, &again);
        remember_in(&file, &first);
        assert_eq!(known_in(&file), [first.clone(), second.clone()]);
        // Remembering the one that already comes first writes nothing: the file keeps its date.
        let written = std::fs::metadata(&file).unwrap().modified().unwrap();
        remember_in(&file, &first);
        assert_eq!(
            std::fs::metadata(&file).unwrap().modified().unwrap(),
            written
        );
        // A folder that is no longer there is not listed, and the file holds a line a folder.
        std::fs::remove_dir_all(&second).unwrap();
        assert_eq!(known_in(&file), std::slice::from_ref(&first));
        assert_eq!(
            std::fs::read_to_string(&file).unwrap(),
            format!("{}\n{}\n", first.display(), second.display())
        );
        // A file that cannot be read holds none.
        assert!(known_in(&root.join("elsewhere")).is_empty());
        let _ = std::fs::remove_dir_all(&root);
    }
}
