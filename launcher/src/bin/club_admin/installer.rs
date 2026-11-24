//! `club-admin install`: the installer (docs/school-edition/README.md, « Déployer »).
//!
//! Horizon Club lives once, on the club's share, and every PC starts it from there. So the
//! installer never settles for a folder of the computer it runs on. It installs where it is told
//! (`--root`); else where it itself sits, when that is a share (a `\\server\share` path, or a
//! drive letter mapped to one); else it asks the supervisor where the share is, with Windows'
//! own folder picker. A folder that is on this computer is installed into only when the
//! supervisor says so, as a try.
//!
//! The release's files are taken beside the program when they are there and are the release's
//! (the supervisor downloaded them all), and downloaded from GitHub otherwise, each checked by
//! its hash.
//!
//! The release holds no file of Minecraft's: the game comes from a folder the supervisor has
//! (`--game`, else the program asks), a build already prepared, and it is put into `core\` with a
//! data file of its own. Without one, the game is put there later with `club-admin install-core`.
use super::*;

/// What holds a folder.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    /// Another computer's share: where the students' shortcuts lead.
    Share,
    /// A drive of this computer.
    Local,
}

impl Kind {
    fn of(folder: &Path) -> Self {
        if on_a_share(folder) {
            Kind::Share
        } else {
            Kind::Local
        }
    }

    fn said(self) -> &'static str {
        match self {
            Kind::Share => tr("a network share"),
            Kind::Local => tr("a folder of this computer"),
        }
    }
}

/// Whether `folder` is on another computer: written as `\\server\share\…`, or on a drive letter
/// Windows mapped to one.
fn on_a_share(folder: &Path) -> bool {
    let Some(text) = folder.to_str() else {
        return false;
    };
    if text.starts_with(r"\\") || text.starts_with("//") {
        return true;
    }
    #[cfg(windows)]
    if mapped_drive_letter(folder) {
        return true;
    }
    false
}

/// `club-admin install`: the share found or asked for, then [`install_into`]. The exit code. A
/// failure is said, and the window stays until Enter when nobody asked for silence: it opens by
/// a double-click, and closes by itself otherwise before its last line is read.
pub(super) fn run(given: Option<&Path>, game: Option<PathBuf>, yes: bool) -> i32 {
    match install(given, game, yes) {
        Ok(code) => code,
        Err(error) => {
            eprintln!("{error}");
            if !yes {
                wait_for_enter();
            }
            1
        }
    }
}

fn wait_for_enter() {
    println!("{}", tr("Press Enter to close."));
    let mut line = String::new();
    let _ = std::io::stdin().read_line(&mut line);
}

fn install(given: Option<&Path>, game: Option<PathBuf>, yes: bool) -> Result<i32, String> {
    println!(
        "{}",
        tr(
            "Horizon Club is installed once, on the club's share: every PC starts it from \
             there, and it is updated there too.",
        )
    );
    let beside = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(Path::to_path_buf))
        .unwrap_or_default();
    let target = match given {
        Some(given) => std::path::absolute(given).unwrap_or_else(|_| given.to_path_buf()),
        None if Kind::of(&beside) == Kind::Share => {
            println!(
                "{}",
                trf(
                    "This program is in {folder}, {kind}.",
                    &[
                        ("folder", &beside.display().to_string()),
                        ("kind", Kind::Share.said()),
                    ],
                )
            );
            if !yes && !confirmed(tr("Install Horizon Club there?"))? {
                return given_up();
            }
            beside.clone()
        }
        None if yes => {
            return Err(
                tr("This program is not on a share: say where to install with --root.").to_string(),
            );
        }
        None => {
            let here = beside.display().to_string();
            println!(
                "{}",
                trf(
                    "This program is in {folder}, {kind}: Horizon Club is not installed on a \
                     student's computer. Where is the club's share?",
                    &[("folder", &here), ("kind", Kind::Local.said())],
                )
            );
            let Some(target) = ask_folder() else {
                return given_up();
            };
            target
        }
    };
    if Kind::of(&target) == Kind::Local {
        let said = trf(
            "{folder} is {kind}: students' shortcuts would not find Horizon Club there. Install \
             into it anyway, to try it?",
            &[
                ("folder", &target.display().to_string()),
                ("kind", Kind::Local.said()),
            ],
        );
        if !yes && !confirmed(&said)? {
            return given_up();
        }
    }
    // The game comes last: cancelling its question only leaves the game for later.
    let game = match game {
        Some(game) => Some(game),
        None if yes => None,
        None => ask_game(),
    };
    install_into(&target, &beside, game, yes)
}

/// The folder of a game already prepared, when the supervisor has one: Windows' folder picker,
/// then a typed path. None to put the game in place later.
fn ask_game() -> Option<PathBuf> {
    println!(
        "{}",
        tr(
            "The release holds no file of Minecraft's: Horizon Club needs the game, from a folder \
             that already has it (for example another install's core\\26.3-…).",
        )
    );
    ask_a_folder(
        tr("Where is the game? Choose the folder of a build already prepared."),
        tr(
            "Type the game folder's path (a build already prepared), or press Enter to put the \
             game there later:",
        ),
    )
}

/// A folder as the supervisor says it: Windows' folder picker (titled `title`), then a typed
/// path answering `typed`. The picker opens for a person at the keyboard only (a keyboard is a
/// terminal on the program's input): from a script or a pipe nobody could close it.
#[cfg(windows)]
fn ask_a_folder(title: &str, typed: &str) -> Option<PathBuf> {
    use std::io::IsTerminal;
    if std::io::stdin().is_terminal() {
        if let Some(picked) = pick_folder(title) {
            return Some(picked);
        }
    }
    ask_typed(typed)
}

/// A folder as the supervisor types it: no folder picker on this system.
#[cfg(not(windows))]
fn ask_a_folder(title: &str, typed: &str) -> Option<PathBuf> {
    let _ = title;
    ask_typed(typed)
}

fn ask_folder() -> Option<PathBuf> {
    ask_a_folder(
        tr("Where is the club's share? Choose its folder."),
        tr("Type the share's path (for example \\\\SERVEUR\\Horizon), or press Enter to give up:"),
    )
}

fn ask_typed(prompt: &str) -> Option<PathBuf> {
    println!("{prompt}");
    let mut line = String::new();
    let _ = std::io::stdin().read_line(&mut line);
    let typed = line.trim().trim_matches('"').to_string();
    (!typed.is_empty()).then(|| PathBuf::from(typed))
}

/// Windows' folder picker, with `title` over its tree: the folder chosen, None when the
/// supervisor closes it.
#[cfg(windows)]
fn pick_folder(title: &str) -> Option<PathBuf> {
    use windows_sys::Win32::System::Com::{
        CoInitializeEx, CoTaskMemFree, CoUninitialize, COINIT_APARTMENTTHREADED,
    };
    use windows_sys::Win32::System::Console::GetConsoleWindow;
    use windows_sys::Win32::UI::Shell::{
        SHBrowseForFolderW, SHGetPathFromIDListW, BIF_NEWDIALOGSTYLE, BIF_RETURNONLYFSDIRS,
        BROWSEINFOW,
    };
    let mut wide: Vec<u16> = title.encode_utf16().chain(std::iter::once(0)).collect();
    let mut path = [0u16; 1024];
    unsafe {
        // The new dialog style needs COM on its thread. It is ended only if it started here.
        let started = CoInitializeEx(std::ptr::null(), COINIT_APARTMENTTHREADED as u32) >= 0;
        let mut info: BROWSEINFOW = std::mem::zeroed();
        // The dialog is the console window's own: it stays in front of it, not behind everything.
        info.hwndOwner = GetConsoleWindow();
        info.lpszTitle = wide.as_mut_ptr();
        info.ulFlags = BIF_RETURNONLYFSDIRS | BIF_NEWDIALOGSTYLE;
        let picked = SHBrowseForFolderW(&info);
        let found = !picked.is_null() && SHGetPathFromIDListW(picked, path.as_mut_ptr()) != 0;
        if !picked.is_null() {
            CoTaskMemFree(picked.cast());
        }
        if started {
            CoUninitialize();
        }
        let end = path.iter().position(|&c| c == 0).unwrap_or(0);
        found.then(|| PathBuf::from(String::from_utf16_lossy(&path[..end])))
    }
}

/// A question on one line: Enter, o, oui, y or yes say yes. Nobody answering (nothing more to
/// read, as from a script without `--yes`) says no.
fn confirmed(question: &str) -> Result<bool, String> {
    println!("{question} {}", tr("(Enter = yes)"));
    let mut line = String::new();
    if std::io::stdin().read_line(&mut line).unwrap_or(0) == 0 {
        return Ok(false);
    }
    Ok(matches!(
        line.trim().to_lowercase().as_str(),
        "" | "o" | "oui" | "y" | "yes"
    ))
}

/// Nothing was installed: said, and the exit code of a try given up.
fn given_up() -> Result<i32, String> {
    println!("{}", tr("Nothing was installed."));
    Ok(1)
}

/// What became of the game (`put_game`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Game {
    /// In `core\`, with a data file of its own: students can play.
    There,
    /// In `core\`, but its game files are not prepared.
    ToPrepare,
    /// Not put in place: later, with `club-admin install-core`.
    Later,
}

/// Installs Horizon Club into `target`: the release's programs ([`update::setup`]), the folders
/// a share holds, `club.toml` when there is none, `managed.txt`, then the game when the
/// supervisor named a folder holding one.
fn install_into(
    target: &Path,
    beside: &Path,
    game: Option<PathBuf>,
    yes: bool,
) -> Result<i32, String> {
    // Everything the library writes now goes to the folder chosen, spelled as it was given.
    std::env::set_var("HORIZON_CLUB_ROOT", target);
    let release = newest_release(beside)?;
    let mut said = |line: String| println!("  {line}");
    let installed = update::setup(target, &release, beside, &mut said)?;
    for folder in ["core", "online/people"] {
        fs::create_dir_all(target.join(folder))
            .map_err(|e| format!("{}: {e}", target.join(folder).display()))?;
    }
    let file = target.join(config::FILE);
    if !file.exists() {
        write_atomic(&file, MODEL)?;
        println!(
            "{}",
            trf(
                "Wrote {file}: read it, and put the club's name and its server's address there.",
                &[("file", &file.display().to_string())],
            )
        );
    }
    let config = config::Config::read(target).map_err(|e| e.to_string())?;
    let text = managed::text(&config, &resolve_names);
    let rule = club::managed_file();
    write_atomic(&rule, &text)?;
    println!(
        "{}",
        trf("Wrote {file}.", &[("file", &rule.display().to_string())])
    );
    println!(
        "{}",
        trf(
            "Installed Horizon Club {version} in {folder}.",
            &[
                ("version", &release.version),
                ("folder", &target.display().to_string()),
            ],
        )
    );
    let game = match game {
        Some(game) => put_game(&game)?,
        None => Game::Later,
    };
    println!("{}", tr("What is left to do:"));
    let mut steps: Vec<&str> = Vec::new();
    if game == Game::Later {
        steps.push(tr(
            "Put the game into core\\: club-admin install-core --from <the built game folder>.",
        ));
    }
    if game != Game::There {
        steps.push(tr("Prepare the game's files, once, from a computer where Minecraft 26.3 is installed: club-admin prepare."));
    }
    steps.extend([
        tr("Correct club.toml (the club's name, its server's address), then: club-admin sync"),
        tr("On the file server, tools/school/setup-share.ps1: students read everything and write in online\\ alone."),
    ]);
    for step in steps {
        println!("  • {step}");
    }
    let bootstrap = installed
        .files
        .iter()
        .find(|file| file.ends_with(update::BOOTSTRAP))
        .cloned()
        .unwrap_or_else(|| target.join(update::BOOTSTRAP));
    println!(
        "  • {}",
        trf(
            "On each PC, a shortcut to {file}.",
            &[("file", &bootstrap.display().to_string())],
        )
    );
    if !yes {
        wait_for_enter();
    }
    Ok(0)
}

/// Puts the game of the folder `from` into `core\` ([`install_build`]), then writes its data
/// file. A folder that is already in `core\` is kept as it is. A folder that holds no game is
/// not one: the game is left for later, and said.
fn put_game(named: &Path) -> Result<Game, String> {
    let program = if cfg!(windows) {
        "horizon-client.exe"
    } else {
        "horizon-client"
    };
    let mut from = named.to_path_buf();
    // The folder that holds the builds (a core\ folder) may be named for the one build in it.
    if !from.join(program).is_file() {
        let builds: Vec<PathBuf> = fs::read_dir(&from)
            .into_iter()
            .flatten()
            .flatten()
            .map(|entry| entry.path())
            .filter(|path| path.join(program).is_file())
            .collect();
        if let [only] = builds.as_slice() {
            from = only.clone();
        }
    }
    if !from.join(program).is_file() {
        println!(
            "{}",
            trf(
                "{folder} holds no horizon-client.exe, so it is not a game's build: the game \
                 will be put in place later.",
                &[("folder", &from.display().to_string())],
            )
        );
        return Ok(Game::Later);
    }
    let tag = from
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    if club::versions_dir().join(&tag).join(program).is_file() {
        println!(
            "{}",
            trf(
                "core\\{tag} is already there: the game is kept.",
                &[("tag", &tag)]
            )
        );
    } else {
        println!(
            "{}",
            trf(
                "Copying the game from {folder}… It takes a few minutes.",
                &[("folder", &from.display().to_string())]
            )
        );
        let tag = install_build(&club::versions_dir(), &from, None)?;
        println!("{}", trf("The game is in core\\{tag}.", &[("tag", &tag)]));
    }
    // The build's files may not be prepared (a build straight from the workflow): then there is
    // no data file to write, and `prepare` is one of the steps said at the end.
    match data_file(&tag) {
        Ok(()) => Ok(Game::There),
        Err(why) => {
            println!("{why}");
            Ok(Game::ToPrepare)
        }
    }
}

/// The release to install: GitHub's newest; offline, the one whose files sit beside the
/// program, which is the program's own version.
fn newest_release(beside: &Path) -> Result<update::Release, String> {
    match update::newest() {
        Ok(release) => Ok(release),
        Err(error) => {
            let complete = [
                update::LAUNCHER,
                update::BOOTSTRAP,
                update::ADMIN,
                update::SUMS,
            ]
            .iter()
            .all(|name| beside.join(name).is_file());
            if !complete {
                return Err(trf(
                    "The release's files are not beside this program, and GitHub could not be \
                     reached: {error}",
                    &[("error", &error)],
                ));
            }
            Ok(update::Release {
                version: backend::VERSION.into(),
                launcher: String::new(),
                sums: String::new(),
                bootstrap: None,
                admin: None,
            })
        }
    }
}

/// `club.toml` as a share starts with it: the spec's example (launcher-target.md, section 3),
/// for the supervisor to correct.
const MODEL: &str = r#"# Horizon Club : la configuration du club. Chaque ligne a une valeur par défaut :
# sans rien changer, le club joue. À corriger : le nom du club et l'adresse de son serveur.
# Après une modification : admin\club-admin.exe sync

[club]
name = "Club Minecraft"          # le titre de la fenêtre
language = "fr"                  # la langue du launcher : "fr" ou "en"
identity = "windows"             # le nom du joueur vient de la session Windows
shared_accounts = []             # sessions Windows que plusieurs élèves partagent : avec l'une
                                 # d'elles, le launcher refuse de jouer et dit pourquoi

[[servers]]                      # les serveurs que le club propose, dans cet ordre
name = "Serveur du club"
address = "mc-club.example.edu"  # un port après les deux points seulement s'il n'est pas 25565

[game]                           # écrit dans les options d'un élève à son premier lancement
render_distance = 8
simulation_distance = 5
max_fps = 60
gui_scale = 2                    # la taille de l'interface : 0 laisse le jeu choisir, sinon 1 à 4

[lan]
open = true                      # un monde peut être ouvert au réseau local

[hosting]                        # la page Serveurs
enabled = true
first_port = 25565               # les serveurs hébergés prennent un port de celui-ci
last_port = 25575
eula_accepted = true             # le responsable accepte l'EULA de Minecraft pour les serveurs hébergés
"#;

#[cfg(test)]
mod tests {
    use super::*;

    /// A share is a path written `\\server\share` (either slash), or a mapped drive; a drive
    /// that holds no mapping is this computer's.
    #[test]
    fn a_share_is_told_from_a_computers_folder() {
        assert_eq!(Kind::of(Path::new(r"\\SERVEUR\Horizon")), Kind::Share);
        assert_eq!(Kind::of(Path::new("//server/share/Horizon")), Kind::Share);
        assert_eq!(Kind::of(Path::new(r"C:\Horizon")), Kind::Local);
        assert_eq!(Kind::of(Path::new("/srv/horizon")), Kind::Local);
        assert_eq!(Kind::of(Path::new(r"C:\és\ Horizon")), Kind::Local);
        assert_ne!(Kind::Share.said(), Kind::Local.said());
    }

    /// The model the installer writes is a `club.toml` the launcher reads whole.
    #[test]
    fn the_model_club_toml_is_read_as_it_is_written() {
        let config = config::Config::parse(MODEL).unwrap();
        assert_eq!(config.club.language, config::Language::French);
        assert_eq!(config.game.gui_scale, 2);
        assert!(config.club.shared_accounts.is_empty());
        assert_eq!(config.servers.len(), 1);
    }
}
