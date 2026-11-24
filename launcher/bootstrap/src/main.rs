//! Horizon Club's stable entry point. The shortcut on every desktop points here, and it starts
//! the launcher's current version from `app\<version>` beside it. The supervisor publishes a new
//! launcher by adding its folder and changing one line in `app\current.txt`; this executable
//! never changes.
//!
//! It takes no arguments and gives the launcher none: it only goes to the launcher's folder and
//! starts the launcher by its name. The launcher finds the club's folder from its own place.
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// The launcher's executable inside `app\<version>`.
const LAUNCHER: &str = "HorizonClubLauncher.exe";
/// The product's folder in the user's local data: the logs.
const DATA: &str = "HorizonClub";

fn main() {
    let root = match std::env::current_exe() {
        Ok(exe) => exe.parent().map(PathBuf::from).unwrap_or_default(),
        Err(e) => fail(&data_dir(), &say(Message::Exe, &[&e.to_string()])),
    };
    let data = data_dir();
    let _ = std::fs::create_dir_all(&data);
    let code = match run(&root, &data) {
        Ok(code) => code,
        Err(message) => fail(&data, &message),
    };
    std::process::exit(code);
}

fn run(root: &Path, data: &Path) -> Result<i32, String> {
    let version = read_version(&current_file(root))?;
    let folder = root.join("app").join(version.get());
    if !folder.join(LAUNCHER).is_file() {
        return Err(say(
            Message::NoLauncher,
            &[&version.get(), &folder.display()],
        ));
    }
    // The launcher is found by its name: its folder comes first in the search path the launcher
    // starts with (Rust looks there before the system folders, never in the current folder).
    std::env::set_current_dir(&folder).map_err(|_| say(Message::Go, &[&folder.display()]))?;
    let status = start(&folder).map_err(|e| say(Message::Start, &[&e.to_string()]))?;
    let code = status.code().unwrap_or(1);
    if code != 0 {
        log_line(data, &say(Message::Exited, &[&code.to_string()]));
    }
    Ok(code)
}

/// Where a program started by this one is looked for: the launcher's folder, then this one's
/// own search path, so the launcher finds the tools it starts as this one does.
fn search_path(folder: &Path) -> std::ffi::OsString {
    let inherited = std::env::var_os("PATH").unwrap_or_default();
    std::env::join_paths(
        std::iter::once(folder.to_path_buf()).chain(std::env::split_paths(&inherited)),
    )
    .unwrap_or(inherited)
}

/// The launcher's version, one line, in `app\current.txt` beside the versioned folders.
fn current_file(root: &Path) -> PathBuf {
    root.join("app").join("current.txt")
}

/// A launcher version as `app\current.txt` names it: a folder name and nothing else, so it
/// cannot leave `app\`.
struct Version(String);

impl Version {
    fn get(&self) -> &str {
        &self.0
    }
}

fn read_version(path: &Path) -> Result<Version, String> {
    let text = std::fs::read_to_string(path)
        .map_err(|_| say(Message::NoCurrent, &[&path.display()]))?;
    let name = text.trim();
    let plain = !name.is_empty()
        && name.len() <= 64
        && name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b));
    if plain && name != "." && name != ".." {
        Ok(Version(name.to_owned()))
    } else {
        Err(say(Message::BadCurrent, &[&path.display()]))
    }
}

/// Start the launcher, the one program of the folder it is looked for in (`LAUNCHER`): a fixed
/// name, no argument, no shell, nothing to read.
fn start(folder: &Path) -> std::io::Result<std::process::ExitStatus> {
    let mut command = Command::new("HorizonClubLauncher.exe");
    command
        .env("PATH", search_path(folder))
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    command.spawn()?.wait()
}

fn data_dir() -> PathBuf {
    std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
        .join(DATA)
}

/// End with the message: in the log, and where the player sees it.
fn fail(data: &Path, message: &str) -> ! {
    log_line(data, message);
    alert(message);
    std::process::exit(1)
}

fn log_line(data: &Path, message: &str) {
    use std::io::Write;
    let path = data.join("bootstrap.log");
    if let Ok(mut file) = std::fs::OpenOptions::new().create(true).append(true).open(path) {
        let _ = writeln!(file, "{message}");
    }
}

/// What this executable can say: French, else English.
#[derive(Clone, Copy)]
enum Message {
    Exe,
    NoCurrent,
    BadCurrent,
    NoLauncher,
    Go,
    Start,
    Exited,
}

impl Message {
    fn text(self) -> (&'static str, &'static str) {
        use Message::*;
        match self {
            Exe => (
                "Horizon Club : impossible de trouver cet exécutable ({0}).",
                "Horizon Club: cannot find this executable ({0}).",
            ),
            // What a supervisor who opened this program before installing needs to read: the
            // installer makes the folder; a student shows the line to the supervisor.
            NoCurrent => (
                "Horizon Club n'est pas installé dans ce dossier ({0} est introuvable). Le responsable du club doit ouvrir HorizonClubSetup.exe une seule fois, dans le dossier du club.",
                "Horizon Club is not installed in this folder ({0} was not found). The club's supervisor has to open HorizonClubSetup.exe once, in the club's folder.",
            ),
            BadCurrent => (
                "Horizon Club : {0} ne donne pas une version valide. Préviens le responsable du club.",
                "Horizon Club: {0} does not name a valid version. Tell the club's supervisor.",
            ),
            NoLauncher => (
                "Horizon Club : le launcher de la version {0} est introuvable ({1}). Préviens le responsable du club.",
                "Horizon Club: the launcher of version {0} was not found ({1}). Tell the club's supervisor.",
            ),
            Go => (
                "Horizon Club : le dossier {0} est inaccessible. Préviens le responsable du club.",
                "Horizon Club: the folder {0} cannot be reached. Tell the club's supervisor.",
            ),
            Start => (
                "Horizon Club : le launcher n'a pas pu démarrer ({0}).",
                "Horizon Club: the launcher could not start ({0}).",
            ),
            Exited => (
                "Le launcher s'est arrêté avec le code {0}.",
                "The launcher exited with code {0}.",
            ),
        }
    }
}

fn say(message: Message, args: &[&dyn std::fmt::Display]) -> String {
    let (french, english) = message.text();
    let mut text = if french_ui() { french } else { english }.to_owned();
    for (n, arg) in args.iter().enumerate() {
        text = text.replace(&format!("{{{n}}}"), &arg.to_string());
    }
    text
}

#[cfg(windows)]
mod platform {
    #[link(name = "user32")]
    extern "system" {
        fn MessageBoxW(window: *mut core::ffi::c_void, text: *const u16, caption: *const u16, kind: u32) -> i32;
    }
    #[link(name = "kernel32")]
    extern "system" {
        fn GetUserDefaultUILanguage() -> u16;
    }

    /// The player's Windows is in French (its primary language code is 0x0c).
    pub fn french_ui() -> bool {
        // SAFETY: `GetUserDefaultUILanguage` takes nothing and reads the user's profile.
        unsafe { GetUserDefaultUILanguage() & 0x3ff == 0x0c }
    }

    /// A message box with an error icon. A release build has no console: this is its only output.
    pub fn alert(text: &str) {
        if cfg!(debug_assertions) {
            eprintln!("{text}");
            return;
        }
        let wide = |s: &str| s.encode_utf16().chain(Some(0)).collect::<Vec<u16>>();
        let (text, caption) = (wide(text), wide("Horizon Club"));
        const MB_ICONERROR: u32 = 0x10;
        // SAFETY: both strings end with a null character, and the window may be none.
        unsafe {
            MessageBoxW(std::ptr::null_mut(), text.as_ptr(), caption.as_ptr(), MB_ICONERROR);
        }
    }
}

#[cfg(not(windows))]
mod platform {
    pub fn french_ui() -> bool {
        std::env::var("LANG").is_ok_and(|l| l.starts_with("fr"))
    }

    pub fn alert(text: &str) {
        eprintln!("Horizon Club: {text}");
    }
}

use platform::{alert, french_ui};
