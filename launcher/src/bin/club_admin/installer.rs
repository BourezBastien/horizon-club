//! `club-admin install`: the installer (docs/school-edition/README.md, « Déployer »).
//!
//! Horizon Club lives once, in a folder: the club's share, where every PC's shortcut leads, or a
//! folder of a computer for a try. The installer is one file, opened where that folder is to be
//! (or already is), and it asks nothing:
//!
//! - it installs into its own folder (`--root` names another, for a script);
//! - the release's programs are taken beside it when they are there and are the release's, and
//!   downloaded from GitHub otherwise, each checked by its hash: the file alone is enough;
//! - the release holds no file of Minecraft's, so the game is looked for without asking: in
//!   `core\` of the folder already, else in the club folders opened on this computer
//!   ([`club::known`]). None found, one line says what to do, and opening it again is enough.
//!
//! A failure is said, and the window stays until Enter when nobody asked otherwise: it opens by
//! a double-click, and would close by itself before its last line is read.
use super::*;

/// `club-admin install`: the exit code.
pub(super) fn run(given: Option<&Path>, game: Option<PathBuf>, yes: bool) -> i32 {
    match install(given, game, yes) {
        Ok(code) => code,
        Err(error) => {
            say!(out::Level::Error, "{error}");
            out::blank();
            out::banner(
                out::Banner::Bad,
                &[tr("The install could not finish.").to_string()],
            );
            if !yes {
                wait_for_enter();
            }
            1
        }
    }
}

fn wait_for_enter() {
    say!(out::Level::Detail, "{}", tr("Press Enter to close."));
    let mut line = String::new();
    let _ = std::io::stdin().read_line(&mut line);
}

fn install(given: Option<&Path>, game: Option<PathBuf>, yes: bool) -> Result<i32, String> {
    let beside = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(Path::to_path_buf))
        .unwrap_or_default();
    // The folder the program is in is the folder it installs into.
    let folder = folder_of(given, &beside);
    say!(
        Level::Title,
        "{}",
        trf(
            "Horizon Club is installed into {folder}.",
            &[("folder", &folder.display().to_string())]
        )
    );
    install_into(&folder, &beside, game, yes)
}

/// Where the installer installs: the folder `--root` names, else the one the program is in. From
/// the `admin\` of a club folder (`club-admin.exe install`), into the club folder: a second
/// Horizon Club inside the first is not what was asked.
fn folder_of(given: Option<&Path>, beside: &Path) -> PathBuf {
    if let Some(given) = given {
        return std::path::absolute(given).unwrap_or_else(|_| given.to_path_buf());
    }
    let in_club_admin = beside
        .file_name()
        .is_some_and(|name| name.to_string_lossy().eq_ignore_ascii_case("admin"))
        && beside
            .parent()
            .is_some_and(|parent| parent.join(config::FILE).is_file());
    if in_club_admin {
        beside.parent().unwrap_or(beside).to_path_buf()
    } else {
        beside.to_path_buf()
    }
}

/// The game's program, as a build folder holds it.
fn program() -> &'static str {
    club::client_program_name()
}

/// What became of the game.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Game {
    /// In `core\`, with a data file of its own: students can play.
    There,
    /// In `core\`, but its game files are not prepared.
    ToPrepare,
    /// In `core\` and prepared, but its data file would not be written: the game reads its files
    /// one by one, and starts in minutes.
    Unwritten,
    /// Not put in place.
    Later,
}

impl Game {
    /// The band that ends the install: how it went, and its first line.
    fn band(&self, version: &str) -> (out::Banner, String) {
        let version = [("version", version)];
        match self {
            Game::There => (
                out::Banner::Good,
                trf(
                    "Horizon Club {version} is installed, with no error.",
                    &version,
                )
                .to_string(),
            ),
            Game::Later => (
                out::Banner::Attention,
                trf(
                    "Horizon Club {version} is installed, but the game is missing.",
                    &version,
                )
                .to_string(),
            ),
            Game::ToPrepare => (
                out::Banner::Attention,
                trf(
                    "Horizon Club {version} is installed, but its game files are not prepared.",
                    &version,
                )
                .to_string(),
            ),
            Game::Unwritten => (
                out::Banner::Attention,
                trf(
                    "Horizon Club {version} is installed, but its data file could not be \
                     written: the game will take minutes to start.",
                    &version,
                )
                .to_string(),
            ),
        }
    }
}

/// Where the game to put in place was found ([`find_game`]).
#[derive(Clone, Debug, PartialEq, Eq)]
enum Found {
    /// Already in `core\` of the folder, under that tag.
    There(String),
    /// In another folder of this computer, to copy under that tag.
    From { from: PathBuf, tag: String },
    /// Nowhere.
    None,
}

/// Installs Horizon Club into `folder`: the release's programs ([`update::setup`]), the folders a
/// club folder holds, `club.toml` when there is none, `managed.txt`, then the game that
/// [`find_game`] found.
fn install_into(
    folder: &Path,
    beside: &Path,
    game: Option<PathBuf>,
    yes: bool,
) -> Result<i32, String> {
    // Everything the library writes now goes to that folder, spelled as it was given.
    std::env::set_var("HORIZON_CLUB_ROOT", folder);
    let release = newest_release(beside)?;
    let installed = update::setup(folder, &release, beside, &mut |step, line: String| {
        // On its way: an arrow. Checked, in place: a tick.
        out::line(
            match step {
                update::Step::Taking => Level::Step,
                update::Step::Checked | update::Step::Placed => Level::Done,
            },
            &line,
        )
    })?;
    for part in ["core", "online/people"] {
        fs::create_dir_all(folder.join(part))
            .map_err(|e| format!("{}: {e}", folder.join(part).display()))?;
    }
    let file = folder.join(config::FILE);
    if !file.exists() {
        write_atomic(&file, MODEL)?;
        say!(
            Level::Warn,
            "{}",
            trf(
                "Wrote {file}: read it, and put the club's name and its server's address there.",
                &[("file", &file.display().to_string())],
            )
        );
    }
    let config = config::Config::read(folder).map_err(|e| e.to_string())?;
    let text = managed::text(&config, &resolve_names);
    let rule = club::managed_file();
    write_atomic(&rule, &text)?;
    say!(
        Level::Detail,
        "{}",
        trf("Wrote {file}.", &[("file", &rule.display().to_string())])
    );
    say!(
        Level::Info,
        "{}",
        trf(
            "Installed Horizon Club {version} in {folder}.",
            &[
                ("version", &release.version),
                ("folder", &folder.display().to_string()),
            ],
        )
    );
    let game = put_game(&find_game(folder, game.as_deref(), &club::known()));
    // The next installer on this computer knows the game may be here.
    club::remember(folder);
    let bootstrap = installed
        .files
        .iter()
        .find(|file| file.ends_with(update::BOOTSTRAP))
        .cloned()
        .unwrap_or_else(|| folder.join(update::BOOTSTRAP));
    // The last word: a band across the window, green when all is in place.
    let (how, head) = game.band(&release.version);
    let mut band = vec![head];
    let shown = folder.display().to_string();
    let here = [("folder", shown.as_str())];
    match game {
        Game::There | Game::Unwritten => (),
        Game::Later => band.push(
            trf(
                "No game was found on this computer: copy the `core` folder of a folder that has \
                 the game next to this program (into {folder}), then open this program again.",
                &here
            )
            .to_string(),
        ),
        Game::ToPrepare => band.push(
            tr("Prepare the game's files, once, from a computer where Minecraft 26.3 is installed: club-admin prepare.")
                .to_string(),
        ),
    }
    band.push(
        trf(
            "On each PC, make a shortcut to {file}.",
            &[("file", &bootstrap.display().to_string())],
        )
        .to_string(),
    );
    out::blank();
    out::banner(how, &band);
    say!(out::Level::Title, "{}", tr("What is left to do:"));
    for step in [
        tr("Correct club.toml (the club's name, its server's address), then: club-admin sync"),
        tr("On the file server, tools/school/setup-share.ps1: students read everything and write in online\\ alone."),
    ] {
        say!(out::Level::Info, "  • {}", step);
    }
    if !yes {
        wait_for_enter();
    }
    Ok(0)
}

/// The newest build a `core` folder holds, by its folder's name ([`club::newer`]).
fn build_in(core: &Path) -> Option<String> {
    fs::read_dir(core)
        .into_iter()
        .flatten()
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.join(program()).is_file())
        .filter_map(|path| path.file_name().map(|n| n.to_string_lossy().into_owned()))
        .max_by(|a, b| club::newer(a, b))
}

/// A build named by `--game`: the folder itself, or the one build it holds (a `core` folder).
fn build_within(named: &Path) -> Option<PathBuf> {
    if named.join(program()).is_file() {
        return Some(named.to_path_buf());
    }
    let builds: Vec<PathBuf> = fs::read_dir(named)
        .into_iter()
        .flatten()
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.join(program()).is_file())
        .collect();
    match builds.as_slice() {
        [only] => Some(only.clone()),
        _ => None,
    }
}

/// The game to put into `folder`, without asking anyone: what `named` (`--game`) says, when it is
/// a build; else a build already in `folder`'s `core\`; else a build of the first of the `known`
/// club folders that has one.
fn find_game(folder: &Path, named: Option<&Path>, known: &[PathBuf]) -> Found {
    if let Some(from) = named.and_then(build_within) {
        let tag = from
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        return Found::From { from, tag };
    }
    if let Some(tag) = build_in(&folder.join("core")) {
        return Found::There(tag);
    }
    known
        .iter()
        .find_map(|known| {
            let tag = build_in(&known.join("core"))?;
            Some(Found::From {
                from: known.join("core").join(&tag),
                tag,
            })
        })
        .unwrap_or(Found::None)
}

/// Puts the game found in place, then writes its data file: a build whose game files are not
/// prepared has none to write, and `prepare` is said at the end.
fn put_game(found: &Found) -> Game {
    let tag = match found {
        Found::There(tag) => {
            say!(
                Level::Done,
                "{}",
                trf(
                    "core\\{tag} is already there: the game is kept.",
                    &[("tag", tag)]
                )
            );
            tag.clone()
        }
        Found::From { from, tag } => {
            say!(
                Level::Step,
                "{}",
                trf(
                    "Copying the game from {folder}… It takes a few minutes.",
                    &[("folder", &from.display().to_string())]
                )
            );
            match install_build(&club::versions_dir(), from, Some(tag.clone())) {
                Ok(tag) => {
                    say!(
                        Level::Info,
                        "{}",
                        trf("The game is in core\\{tag}.", &[("tag", &tag)])
                    );
                    tag
                }
                Err(why) => {
                    say!(out::Level::Error, "{why}");
                    return Game::Later;
                }
            }
        }
        Found::None => return Game::Later,
    };
    match data_file(&tag) {
        Ok(()) => Game::There,
        Err(why) => {
            // Not prepared is said and has its way (`prepare`); prepared, with a file that would
            // not be written, is an error.
            let prepared = backend::check_prepared(&tag);
            say!(if prepared { Level::Error } else { Level::Warn }, "{why}");
            if prepared {
                Game::Unwritten
            } else {
                Game::ToPrepare
            }
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

/// `club.toml` as a club folder starts with it: the spec's example (launcher-target.md, section
/// 3), for the supervisor to correct.
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

    fn scratch(name: &str) -> PathBuf {
        let folder =
            std::env::temp_dir().join(format!("horizon-club-admin-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&folder);
        folder
    }

    /// A build folder holding the game's program.
    fn build(core: &Path, tag: &str) -> PathBuf {
        let path = core.join(tag);
        fs::create_dir_all(&path).unwrap();
        fs::write(path.join(program()), "MZ the game").unwrap();
        path
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

    /// The program installs where it is, `--root` elsewhere; from the `admin\` of a club folder,
    /// into the club folder.
    #[test]
    fn it_installs_where_it_is_unless_named_or_in_a_club_s_admin_folder() {
        let root = scratch("folder-of");
        let club = root.join("club");
        let admin = club.join("admin");
        fs::create_dir_all(&admin).unwrap();
        // An `admin` folder that is not a club's is a folder like any other.
        assert_eq!(folder_of(None, &admin), admin);
        fs::write(club.join(config::FILE), MODEL).unwrap();
        assert_eq!(folder_of(None, &admin), club);
        assert_eq!(folder_of(None, &root), root);
        assert_eq!(
            folder_of(Some(&club), &admin),
            std::path::absolute(&club).unwrap()
        );
    }

    /// The game is found without asking: what was named, else what the folder already holds,
    /// else what another club folder of this computer holds; never the folder itself twice.
    #[test]
    fn the_game_is_found_in_the_folder_then_on_this_computer() {
        let root = scratch("find-game");
        let (folder, other, empty, gone) = (
            root.join("folder"),
            root.join("other"),
            root.join("empty"),
            root.join("gone"),
        );
        fs::create_dir_all(&folder).unwrap();
        build(&root.join("empty").join("core"), "26.3-20261101");
        let _ = fs::remove_dir_all(&empty);
        build(&other.join("core"), "26.3-20261101");
        build(&other.join("core"), "26.3-20261102");
        // A folder with only a file named like a build is no build.
        fs::create_dir_all(other.join("core").join("26.3-20261130")).unwrap();
        fs::write(
            other.join("core").join("26.3-20261130").join("notes.txt"),
            "x",
        )
        .unwrap();

        // Nothing anywhere: none.
        assert_eq!(
            find_game(&folder, None, std::slice::from_ref(&gone)),
            Found::None
        );
        // The folder's own game comes first, and the newest build of it.
        build(&folder.join("core"), "26.3-20261101");
        build(&folder.join("core"), "26.3-20261102");
        assert_eq!(
            find_game(&folder, None, std::slice::from_ref(&other)),
            Found::There("26.3-20261102".into())
        );
        // Without one, the first known club folder that has the game, its newest build.
        let _ = fs::remove_dir_all(folder.join("core"));
        let known = vec![gone, other.join(""), folder.clone()];
        assert_eq!(
            find_game(&folder, None, &known),
            Found::From {
                from: other.join("core").join("26.3-20261102"),
                tag: "26.3-20261102".into()
            }
        );
        // What was named wins over everything, as a build or as the folder holding one. A folder
        // that is no build changes nothing: the search goes on as if nothing was named.
        let named = root.join("named");
        fs::create_dir_all(named.join("26.3-20261109")).unwrap();
        assert_eq!(
            find_game(&folder, Some(&named), &known),
            Found::From {
                from: other.join("core").join("26.3-20261102"),
                tag: "26.3-20261102".into()
            }
        );
        let only = build(&named, "26.3-20261109");
        assert_eq!(
            find_game(&folder, Some(&named), &known),
            Found::From {
                from: only.clone(),
                tag: "26.3-20261109".into()
            }
        );
        assert_eq!(
            find_game(&folder, Some(&only), &[]),
            Found::From {
                from: only,
                tag: "26.3-20261109".into()
            }
        );
        let _ = fs::remove_dir_all(&root);
    }
}
