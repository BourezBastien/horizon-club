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
pub(super) fn run(given: Option<&Path>, yes: bool) -> i32 {
    match install(given, yes) {
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

fn install(given: Option<&Path>, yes: bool) -> Result<i32, String> {
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
    install_into(&target, &beside, yes)
}

/// The share's folder, as the supervisor says it: Windows' folder picker, then a typed path. The
/// picker opens for a person at the keyboard only (a keyboard is a terminal on the program's
/// input): from a script or a pipe nobody could close it.
#[cfg(windows)]
fn ask_folder() -> Option<PathBuf> {
    use std::io::IsTerminal;
    if std::io::stdin().is_terminal() {
        let question = tr("Where is the club's share? Choose its folder.");
        if let Some(picked) = pick_folder(question) {
            return Some(picked);
        }
    }
    ask_typed()
}

/// The share's folder, typed: no folder picker on this system.
#[cfg(not(windows))]
fn ask_folder() -> Option<PathBuf> {
    ask_typed()
}

fn ask_typed() -> Option<PathBuf> {
    println!(
        "{}",
        tr("Type the share's path (for example \\\\SERVEUR\\Horizon), or press Enter to give up:",)
    );
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

/// Installs Horizon Club into `target`: the release's programs ([`update::setup`]), the folders
/// a share holds, `club.toml` when there is none, and `managed.txt`. `yes` answers the one
/// question a folder on this computer asks.
fn install_into(target: &Path, beside: &Path, yes: bool) -> Result<i32, String> {
    let kind = Kind::of(target);
    if kind == Kind::Local {
        let said = trf(
            "{folder} is {kind}: students' shortcuts would not find Horizon Club there. Install \
             into it anyway, to try it?",
            &[
                ("folder", &target.display().to_string()),
                ("kind", kind.said()),
            ],
        );
        if !yes && !confirmed(&said)? {
            return given_up();
        }
    }
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
    println!("{}", tr("What is left to do:"));
    for step in [
        tr("1. Put the game into core\\: club-admin install-core --from <the built game folder>."),
        tr("2. Prepare the game's files, once, from a computer where Minecraft 26.3 is installed: club-admin prepare."),
        tr("3. Correct club.toml (the club's name, its server's address), then: club-admin sync"),
        tr("4. On the file server, tools/school/setup-share.ps1: students read everything and write in online\\ alone."),
    ] {
        println!("  {step}");
    }
    let bootstrap = installed
        .files
        .iter()
        .find(|file| file.ends_with(update::BOOTSTRAP))
        .cloned()
        .unwrap_or_else(|| target.join(update::BOOTSTRAP));
    println!(
        "  {}",
        trf(
            "5. On each PC, a shortcut to {file}.",
            &[("file", &bootstrap.display().to_string())],
        )
    );
    if !yes {
        wait_for_enter();
    }
    Ok(0)
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
