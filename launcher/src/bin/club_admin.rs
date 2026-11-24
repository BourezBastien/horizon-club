//! `club-admin`: the club supervisor's command-line tool for the share
//! (docs/school-edition/README.md, « Déployer »). It lives in the share's `admin\` folder, which
//! students cannot open, and it is the only thing that writes in the club root: the builds of
//! `core\`, `managed.txt`, `app\current.txt`, the bootstrap and `club.toml`'s edits. Opened as
//! `HorizonClubSetup.exe` with nothing on its command line, it is the installer, which makes the
//! share from nothing (`club_admin/installer.rs`).
//!
//! The club root is `--root`'s, else `HORIZON_CLUB_ROOT`'s, else the folder above the tool's own
//! `admin\` folder, else the search the launcher makes (`club::root`). Whatever it is, it is
//! given to the library as `HORIZON_CLUB_ROOT` before anything else reads a folder, so every
//! path the tool writes is the one the students' launchers see. Its messages go through the
//! launcher's own `tr`/`trf`, so they are in the club's language.
#[path = "club_admin/edits.rs"]
mod edits;
#[path = "club_admin/installer.rs"]
mod installer;
#[path = "club_admin/managed.rs"]
mod managed;
#[path = "club_admin/report.rs"]
mod report;

use clap::{Parser, Subcommand};
use horizon_launcher::backend;
use horizon_launcher::i18n::{self, tr, trf};
use horizon_launcher::{club, config, downloads, mojang, sources, update};
use report::Finding;
use std::fs;
use std::net::ToSocketAddrs;
use std::path::{Path, PathBuf};

#[derive(Parser)]
#[command(
    name = "club-admin",
    version = backend::VERSION,
    about = "The club supervisor's tool for the share: install it, check it, write the game's rule, publish the launcher, install and prepare the game builds.",
    disable_help_subcommand = true
)]
struct Cli {
    /// The club's root folder. Default: HORIZON_CLUB_ROOT, else the folder above this program's own admin\ folder, else the search the launcher makes.
    #[arg(long, global = true)]
    root: Option<PathBuf>,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Report of the share: club.toml and its errors, each build of core\, the build in use,
    /// managed.txt, the launcher and the bootstrap. The exit code is 1 when something keeps a
    /// student from playing.
    Check,
    /// Write managed.txt (the rule of the game) from club.toml.
    Sync,
    /// Say which build of core\ the club plays, in club.toml.
    SetVersion { tag: String },
    /// Add or take a server off club.toml's list.
    Server {
        #[command(subcommand)]
        command: ServerCommand,
    },
    /// Publish a launcher: app\<X>\HorizonClubLauncher.exe, point app\current.txt at it, and
    /// with --bootstrap put the bootstrap at the club's root.
    Publish {
        #[arg(long)]
        launcher: PathBuf,
        #[arg(long)]
        version: String,
        #[arg(long)]
        bootstrap: Option<PathBuf>,
    },
    /// Install Horizon Club from the start, asking nothing: the launcher, the bootstrap and this
    /// tool, the folders a club folder holds, club.toml and managed.txt, then the game. It all goes
    /// into the folder this program is in (--root names another). The programs come from beside
    /// it or from GitHub; the game from --game, else from the folder's core\, else from another
    /// club folder of this computer. --yes: for a script, the window doesn't wait for Enter.
    Install {
        /// A folder holding a game build already prepared, to put into core\.
        #[arg(long)]
        game: Option<PathBuf>,
        #[arg(long)]
        yes: bool,
    },
    /// Bring the share's launcher to the club's newest release on GitHub: downloaded, checked by
    /// its hash, put in app\<version>\, with app\current.txt pointing at it. Every PC uses it the
    /// next time it starts. --check only says whether a newer one is there.
    Update {
        #[arg(long)]
        check: bool,
    },
    /// Put a built game folder into core\<tag>\, then write its data file.
    InstallCore {
        #[arg(long)]
        from: PathBuf,
        #[arg(long)]
        tag: Option<String>,
    },
    /// Write horizon-data.bin in a build's prepared folder, so a start over the network opens
    /// one file instead of about 22,000.
    DataFile {
        #[arg(long)]
        tag: Option<String>,
    },
    /// Prepare the game's files into core\<tag>\.local\horizon-client\<Minecraft version>,
    /// from a Minecraft folder or by completing it from Mojang, then write the data file.
    /// Needs Java 25: found, or named by --java-home.
    Prepare {
        #[arg(long)]
        tag: Option<String>,
        #[arg(long)]
        minecraft_dir: Option<PathBuf>,
        #[arg(long)]
        download: bool,
        #[arg(long)]
        server_jar: Option<PathBuf>,
        #[arg(long)]
        java_home: Option<PathBuf>,
    },
}

#[derive(Subcommand)]
enum ServerCommand {
    /// Add a server to the list the club offers.
    Add { name: String, address: String },
    /// Take a server off the list.
    Remove { name: String },
}

fn main() {
    // `HorizonClubSetup.exe` is this program under the installer's name: opened with nothing on
    // its command line, it installs.
    let setup = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.file_name().map(|n| n.to_string_lossy().into_owned()))
        .is_some_and(|name| name.eq_ignore_ascii_case("HorizonClubSetup.exe"));
    let mut args: Vec<std::ffi::OsString> = std::env::args_os().collect();
    if setup && args.len() == 1 {
        args.push("install".into());
    }
    let cli = Cli::parse_from(args);
    // The installer finds or asks for the club's folder itself: none is resolved for it.
    if let Command::Install { game, yes } = &cli.command {
        i18n::set(i18n::Language::French);
        std::process::exit(installer::run(cli.root.as_deref(), game.clone(), *yes));
    }
    let root = match resolve_root(cli.root.as_deref()) {
        Ok(root) => root,
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(2);
        }
    };
    // Everything the library does with the club's folders now goes through this root, spelled
    // as given: the paths it writes are the ones the students' launchers read.
    std::env::set_var("HORIZON_CLUB_ROOT", &root);
    // A club folder this computer opens is remembered: an installer looks there for the game.
    club::remember(&root);
    let config = config::Config::load();
    let language = config
        .as_ref()
        .map(|config| config.club.language.of())
        .unwrap_or(i18n::Language::French);
    i18n::set(language);
    let code = match run(&cli.command, &root, config.as_ref()) {
        Ok(code) => code,
        Err(error) => {
            eprintln!("{error}");
            1
        }
    };
    std::process::exit(code);
}

/// The club's root, as the header of this file says it is found.
fn resolve_root(given: Option<&Path>) -> Result<PathBuf, String> {
    if let Some(root) = given {
        return Ok(std::path::absolute(root).unwrap_or_else(|_| root.to_path_buf()));
    }
    if let Some(set) = std::env::var_os("HORIZON_CLUB_ROOT").filter(|v| !v.is_empty()) {
        return Ok(PathBuf::from(set));
    }
    // The tool's own place in the share: <root>\admin\club-admin.exe.
    if let Ok(exe) = std::env::current_exe() {
        let in_admin = exe
            .parent()
            .and_then(|folder| folder.file_name())
            .is_some_and(|name| name.eq_ignore_ascii_case("admin"));
        if in_admin {
            if let Some(root) = exe.parent().and_then(|admin| admin.parent()) {
                return Ok(root.to_path_buf());
            }
        }
    }
    let found = club::root();
    if found.as_os_str().is_empty() {
        return Err(tr(
            "The club's folder could not be found: say where it is with --root, or run this \
             tool from the share's admin\\ folder.",
        )
        .to_string());
    }
    Ok(found)
}

fn run(
    command: &Command,
    root: &Path,
    config: Result<&config::Config, &config::Error>,
) -> Result<i32, String> {
    match command {
        Command::Check => check(root, config),
        Command::Sync => {
            let config = read_config(config)?;
            let text = managed::text(config, &resolve_names);
            let path = club::managed_file();
            write_atomic(&path, &text)?;
            println!(
                "{}",
                trf(
                    "Wrote {path} ({lines} lines).",
                    &[
                        ("path", &path.display().to_string()),
                        ("lines", &text.lines().count().to_string()),
                    ]
                )
            );
            Ok(0)
        }
        Command::SetVersion { tag } => {
            let installed = backend::installed();
            if !installed.iter().any(|installed| installed == tag) {
                return Err(trf(
                    "{tag} is not a build of core\\, so the club cannot play it. The builds \
                     there: {builds}.",
                    &[("tag", tag), ("builds", &installed.join(", "))],
                )
                .to_string());
            }
            let path = root.join(config::FILE);
            let text = read_club_toml(&path)?;
            let edited = edits::set_active(&text, tag)?;
            write_atomic(&path, &edited)?;
            println!(
                "{}",
                trf(
                    "The club now plays {tag} ([versions] active in club.toml).",
                    &[("tag", tag)]
                )
            );
            Ok(0)
        }
        Command::Server { command } => {
            let path = root.join(config::FILE);
            let text = read_club_toml(&path)?;
            let edited = match command {
                ServerCommand::Add { name, address } => {
                    let edited = edits::add_server(&text, name, address)?;
                    println!(
                        "{}",
                        trf(
                            "Added {name} ({address}) to club.toml's servers.",
                            &[("name", name), ("address", address),]
                        )
                    );
                    edited
                }
                ServerCommand::Remove { name } => {
                    let edited = edits::remove_server(&text, name)?;
                    println!(
                        "{}",
                        trf("Took {name} off club.toml's servers.", &[("name", name)])
                    );
                    edited
                }
            };
            write_atomic(&path, &edited)?;
            println!(
                "{}",
                tr("Run sync so managed.txt says what the new list says.")
            );
            Ok(0)
        }
        Command::Publish {
            launcher,
            version,
            bootstrap,
        } => {
            let target = root.join("app").join(version);
            let program = target.join("HorizonClubLauncher.exe");
            if target.exists() {
                return Err(trf(
                    "app\\{version} is already published: a launcher that is running cannot be \
                     replaced on a share. Choose another version number.",
                    &[("version", version)],
                )
                .to_string());
            }
            fs::create_dir_all(&target).map_err(|e| format!("{}: {e}", target.display()))?;
            fs::copy(launcher, &program).map_err(|e| {
                trf(
                    "Could not copy {from} to {to}: {error}",
                    &[
                        ("from", &launcher.display().to_string()),
                        ("to", &program.display().to_string()),
                        ("error", &e.to_string()),
                    ],
                )
            })?;
            println!(
                "{}",
                trf(
                    "Published the launcher as app\\{version}\\HorizonClubLauncher.exe.",
                    &[("version", version)]
                )
            );
            write_atomic(
                &root.join("app").join("current.txt"),
                &format!("{version}\n"),
            )?;
            println!(
                "{}",
                trf(
                    "app\\current.txt now says {version}.",
                    &[("version", version)]
                )
            );
            if let Some(from) = bootstrap {
                let to = root.join("HorizonClub.exe");
                let _ = fs::remove_file(&to);
                fs::copy(from, &to).map_err(|e| {
                    trf(
                        "Could not copy the bootstrap {from} to {to}: {error}",
                        &[
                            ("from", &from.display().to_string()),
                            ("to", &to.display().to_string()),
                            ("error", &e.to_string()),
                        ],
                    )
                })?;
                println!("{}", tr("Copied the bootstrap to the club's root."));
            }
            Ok(0)
        }
        Command::Update { check } => update_the_share(root, *check),
        Command::Install { .. } => unreachable!("handled before the club's folder is resolved"),
        Command::InstallCore { from, tag } => {
            install_core(from, tag.clone())?;
            Ok(0)
        }
        Command::DataFile { tag } => {
            let tag = the_build(config, tag.clone())?;
            data_file(&tag)?;
            Ok(0)
        }
        Command::Prepare {
            tag,
            minecraft_dir,
            download,
            server_jar,
            java_home,
        } => {
            let tag = the_build(config, tag.clone())?;
            prepare(
                &tag,
                minecraft_dir.as_deref(),
                *download,
                server_jar.as_deref(),
                java_home.as_deref(),
            )?;
            Ok(0)
        }
    }
}

// ----- the game's build -----------------------------------------------------------------------------

/// The file the game's files are grouped in, in a prepared folder: it belongs to the folder it
/// was written in (it names that folder), so it is never carried into another one.
const DATA_FILE: &str = horizon_platform::prepared_files::FILE_NAME;

/// Puts a built game folder into `core\<tag>\`: `tag`, else the folder's own name. Then the
/// build's data file is written ([`data_file`]), when the folder's game files are prepared. The
/// tag the build was installed under.
pub(crate) fn install_core(from: &Path, tag: Option<String>) -> Result<String, String> {
    let tag = install_build(&club::versions_dir(), from, tag)?;
    data_file(&tag)?;
    Ok(tag)
}

/// [`install_core`]'s copy: the folder into `core\<tag>\`, without the data file it may hold. The
/// tag the build was installed under.
pub(crate) fn install_build(
    core: &Path,
    from: &Path,
    tag: Option<String>,
) -> Result<String, String> {
    let tag = tag
        .or_else(|| from.file_name().map(|n| n.to_string_lossy().into_owned()))
        .ok_or_else(|| tr("The folder to install from has no name.").to_string())?;
    // The tag is a folder's name, as the launcher reads the builds of core\: it cannot leave it.
    let plain = !tag.is_empty()
        && tag.len() <= 64
        && tag
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b))
        && tag != "."
        && tag != "..";
    if !plain {
        return Err(trf(
            "\"{tag}\" is not a build's name: letters, digits, dots, dashes and underscores.",
            &[("tag", &tag)],
        ));
    }
    let target = core.join(&tag);
    if target.exists() {
        return Err(trf(
            "core\\{tag} is already there. Take it away first, or install under another tag.",
            &[("tag", &tag)],
        ));
    }
    fs::create_dir_all(&target).map_err(|e| format!("{}: {e}", target.display()))?;
    copy_folder_without(from, &target, DATA_FILE).map_err(|e| {
        trf(
            "Could not copy {from} into core\\{tag}: {error}",
            &[
                ("from", &from.display().to_string()),
                ("tag", &tag),
                ("error", &e),
            ],
        )
    })?;
    println!(
        "{}",
        trf(
            "Installed {from} as core\\{tag}.",
            &[("from", &from.display().to_string()), ("tag", &tag)]
        )
    );
    Ok(tag)
}

/// Whether a prepared folder's data file is there and is this folder's. One written for another
/// folder (the build was copied from elsewhere) is left alone by the game, which then reads its
/// 22,000 files one by one: minutes instead of seconds, at every start.
fn data_file_usable(resources: &Path) -> bool {
    resources.join(DATA_FILE).is_file()
        && horizon_platform::prepared_files::open(resources).is_some()
}

/// `club-admin update`: the share's launcher brought to the club's newest release
/// (`horizon_launcher::update`). The version in use is the one `app\current.txt` names.
fn update_the_share(root: &Path, check_only: bool) -> Result<i32, String> {
    let current = fs::read_to_string(root.join("app").join("current.txt"))
        .map(|text| text.trim().to_string())
        .unwrap_or_default();
    let Some(release) = update::check(&current)? else {
        println!(
            "{}",
            tr("The share already has the club's newest launcher.")
        );
        return Ok(0);
    };
    let version = [("version", release.version.as_str())];
    if check_only {
        println!(
            "{}",
            trf(
                "Version {version} of the launcher is available: run club-admin update to install it on the share.",
                &version
            )
        );
        return Ok(0);
    }
    println!("{}", trf("Downloading version {version}…", &version));
    update::install(root, &release)?;
    println!(
        "{}",
        trf(
            "The share now has version {version}: every PC uses it the next time it starts.",
            &version
        )
    );
    Ok(0)
}

// ----- check --------------------------------------------------------------------------------------

/// The report of the share: facts read from the disk, then their verdicts. The exit code is 1
/// when something keeps a student from playing.
fn check(root: &Path, config: Result<&config::Config, &config::Error>) -> Result<i32, String> {
    println!(
        "{}",
        trf(
            "The club's folder: {root}",
            &[("root", &root.display().to_string())]
        )
    );
    let config = match config {
        Ok(config) => config,
        Err(error) => {
            let facts = report::Facts {
                club: Err(error.to_string()),
                ..report::Facts::empty()
            };
            return print_verdicts(&facts);
        }
    };
    let names = backend::installed();
    let builds = names
        .iter()
        .map(|name| report::Build {
            name: name.clone(),
            game_version: backend::game_version(name),
            prepared: backend::check_prepared(name),
            dxcompiler: backend::version_dir(name).join("dxcompiler.dll").is_file(),
            data_file: data_file_usable(&backend::resources(name)),
            server_program: backend::version_dir(name)
                .join(if cfg!(windows) {
                    "horizon-server.exe"
                } else {
                    "horizon-server"
                })
                .is_file(),
        })
        .collect();
    let active = config.versions.build(&names).map(str::to_string);
    let named_but_absent = config
        .versions
        .active
        .as_ref()
        .filter(|named| !names.iter().any(|name| name == *named))
        .cloned();
    let current = fs::read_to_string(root.join("app").join("current.txt"))
        .ok()
        .map(|text| text.trim().to_string());
    let launcher_there = current.as_deref().is_some_and(|version| {
        !version.is_empty()
            && root
                .join("app")
                .join(version)
                .join("HorizonClubLauncher.exe")
                .is_file()
    });
    let facts = report::Facts {
        club: Ok(config.clone()),
        builds,
        active,
        named_but_absent,
        managed: fs::read_to_string(club::managed_file()).ok(),
        managed_expected: managed::text(config, &resolve_names),
        current,
        launcher_there,
        bootstrap: root.join("HorizonClub.exe").is_file(),
    };
    print_verdicts(&facts)
}

impl report::Facts {
    /// Facts with nothing in them but the error, for a share whose file cannot be read.
    fn empty() -> Self {
        Self {
            club: Err(String::new()),
            builds: Vec::new(),
            active: None,
            named_but_absent: None,
            managed: None,
            managed_expected: String::new(),
            current: None,
            launcher_there: false,
            bootstrap: false,
        }
    }
}

fn print_verdicts(facts: &report::Facts) -> Result<i32, String> {
    let finds = report::verdicts(facts);
    for Finding { blocks, said } in &finds {
        println!("{} {said}", if *blocks { "✗" } else { "✓" });
    }
    if report::any_blocks(&finds) {
        println!(
            "{}",
            tr("Something keeps a student from playing: the lines above with ✗ say what to run.")
        );
        Ok(1)
    } else {
        println!("{}", tr("The share is ready: a student can play."));
        Ok(0)
    }
}

// ----- data-file and prepare ----------------------------------------------------------------------

/// The build a command's `--tag` names, else the one the club plays.
fn the_build(
    config: Result<&config::Config, &config::Error>,
    tag: Option<String>,
) -> Result<String, String> {
    let installed = backend::installed();
    if let Some(tag) = tag {
        if !installed.iter().any(|installed| installed == &tag) {
            return Err(trf(
                "{tag} is not a build of core\\. The builds there: {builds}.",
                &[("tag", &tag), ("builds", &installed.join(", "))],
            )
            .to_string());
        }
        return Ok(tag);
    }
    read_config(config)?
        .versions
        .build(&installed)
        .map(str::to_string)
        .ok_or_else(|| tr("No game build is in core\\, and none was named with --tag.").to_string())
}

/// Writes `horizon-data.bin` in the build's prepared folder, through the path the students'
/// launchers use. The file remembers its folder's exact path, so a root given as a mapped drive
/// is warned about: the students may see the share under another name.
fn data_file(tag: &str) -> Result<(), String> {
    warn_mapped_drive();
    let resources = backend::resources(tag);
    let file = resources.join(DATA_FILE);
    if file.is_file() {
        if data_file_usable(&resources) {
            println!(
                "{}",
                trf(
                    "{file} is already there.",
                    &[("file", &file.display().to_string())]
                )
            );
            return Ok(());
        }
        // Written for another folder, as when the build was copied from elsewhere: the game
        // would leave it alone and read its files one by one.
        println!(
            "{}",
            trf(
                "{file} was written for another folder: it is written again for this one.",
                &[("file", &file.display().to_string())]
            )
        );
        fs::remove_file(&file).map_err(|e| format!("{}: {e}", file.display()))?;
    }
    if !backend::check_prepared(tag) {
        return Err(trf(
            "{tag}'s files are not prepared, so there is no data file to write. Run: club-admin \
             prepare --tag {tag} …",
            &[("tag", tag)],
        )
        .to_string());
    }
    let readers = std::thread::available_parallelism().map_or(1, |n| n.get().min(8));
    horizon_platform::prepared_files::write_with_readers(&resources, readers)
        .map_err(|e| format!("{}: {e}", file.display()))?;
    println!(
        "{}",
        trf(
            "Wrote {file} ({readers} files read at a time).",
            &[
                ("file", &file.display().to_string()),
                ("readers", &readers.to_string()),
            ]
        )
    );
    Ok(())
}

fn prepare(
    tag: &str,
    minecraft_dir: Option<&Path>,
    download: bool,
    server_jar: Option<&Path>,
    java_home: Option<&Path>,
) -> Result<(), String> {
    let log = |line: String| println!("{line}");
    // Java 25: found by the preparer, or named here.
    if let Some(home) = java_home {
        std::env::set_var("JAVA_HOME", home);
        if let Some(path) = std::env::var_os("PATH") {
            let mut paths = vec![home.join("bin")];
            paths.extend(std::env::split_paths(&path));
            if let Ok(joined) = std::env::join_paths(paths) {
                std::env::set_var("PATH", joined);
            }
        }
        println!(
            "{}",
            trf(
                "Using the Java of {home}.",
                &[("home", &home.display().to_string())]
            )
        );
    }
    // The server jar of the pinned version, if the supervisor has one: the preparer checks and
    // downloads it itself otherwise.
    if let Some(jar) = server_jar {
        let version = backend::game_version(tag);
        let to = backend::data_dir()
            .join("server-jars")
            .join(format!("{version}.jar"));
        fs::create_dir_all(to.parent().unwrap()).map_err(|e| e.to_string())?;
        fs::copy(jar, &to).map_err(|e| {
            trf(
                "Could not copy the server jar {from} to {to}: {error}",
                &[
                    ("from", &jar.display().to_string()),
                    ("to", &to.display().to_string()),
                    ("error", &e.to_string()),
                ],
            )
        })?;
        println!(
            "{}",
            trf(
                "Using the server jar {to}.",
                &[("to", &to.display().to_string())]
            )
        );
    }
    // Where the game's files come from: a folder the supervisor names, or the launcher's own
    // download, completed from Mojang.
    let minecraft = match (minecraft_dir, download) {
        (Some(dir), _) => dir.to_path_buf(),
        (None, true) => {
            let dir = backend::data_dir().join("minecraft");
            let version = backend::game_version(tag);
            let lock = fs::read_to_string(backend::version_dir(tag).join("research-lock.json"))
                .map_err(|e| format!("research-lock.json: {e}"))
                .and_then(|text| {
                    serde_json::from_str::<serde_json::Value>(&text).map_err(|e| e.to_string())
                })?;
            let pins = sources::Pins::read(&lock)
                .ok_or("The version's research-lock.json doesn't pin the Minecraft files")?;
            let os = match sources::Os::current() {
                sources::Os::Linux => "linux",
                sources::Os::MacOs => "osx",
                sources::Os::Windows => "windows",
            };
            println!(
                "{}",
                trf(
                    "Downloading Minecraft {version} from Mojang into {dir}…",
                    &[("version", &version), ("dir", &dir.display().to_string()),]
                )
            );
            let fetch = downloads::install_metadata;
            mojang::download_http(&pins, &dir, os, mojang::JOBS, &fetch, &mojang::CANCEL, &log)?;
            dir
        }
        (None, false) => {
            return Err(tr("Say where the game's files come from: --minecraft-dir <folder>, or --download to complete them from Mojang.").to_string());
        }
    };
    let (preparer, minecraft) = backend::preparer(tag, &minecraft, &log)?;
    match preparer {
        backend::Preparer::InProcess(options) => {
            backend::prepare_in_process(&options, &log)?;
        }
        backend::Preparer::Process(command) => {
            backend::run_preparer(command, &log)?;
        }
    }
    backend::sounds(tag, &minecraft, &log);
    data_file(tag)
}

// ----- shared helpers ---------------------------------------------------------------------------

fn read_config<'a>(
    config: Result<&'a config::Config, &config::Error>,
) -> Result<&'a config::Config, String> {
    config.map_err(|error| error.to_string())
}

/// `club.toml`'s text: the supervisor's file, which edits are made in.
fn read_club_toml(path: &Path) -> Result<String, String> {
    fs::read_to_string(path).map_err(|e| {
        trf(
            "Could not read {path}: {error}",
            &[
                ("path", &path.display().to_string()),
                ("error", &e.to_string()),
            ],
        )
        .to_string()
    })
}

/// A name looked up as the game looks it up: every address it resolves to, nothing when it
/// resolves to none.
fn resolve_names(host: &str) -> Vec<std::net::IpAddr> {
    (host, 80)
        .to_socket_addrs()
        .map(|resolved| resolved.map(|a| a.ip()).collect())
        .unwrap_or_default()
}

/// A file written whole: a temporary file beside it, then a rename over it.
fn write_atomic(path: &Path, text: &str) -> Result<(), String> {
    let partial = path.with_extension("tmp");
    fs::write(&partial, text).map_err(|e| format!("{}: {e}", partial.display()))?;
    fs::rename(&partial, path).map_err(|e| format!("{}: {e}", path.display()))
}

/// A folder copied whole, files and folders both, but for any file named `without`: a file
/// that belongs to the folder it was written in is not carried into another one.
fn copy_folder_without(from: &Path, to: &Path, without: &str) -> Result<(), String> {
    fs::create_dir_all(to).map_err(|e| format!("{}: {e}", to.display()))?;
    for entry in fs::read_dir(from).map_err(|e| format!("{}: {e}", from.display()))? {
        let entry = entry.map_err(|e| e.to_string())?;
        if entry.file_name() == without {
            continue;
        }
        let target = to.join(entry.file_name());
        if entry.file_type().map_err(|e| e.to_string())?.is_dir() {
            copy_folder_without(&entry.path(), &target, without)?;
        } else {
            fs::copy(entry.path(), &target)
                .map_err(|e| format!("{}: {e}", entry.path().display()))?;
        }
    }
    Ok(())
}

/// A warning when the club root was given as a mapped drive (`Z:\Horizon`): the data file
/// remembers its folder's exact path, and the students may see the share under its real name
/// (`\\server\Horizon`), which a launcher then cannot find.
fn warn_mapped_drive() {
    #[cfg(windows)]
    if mapped_drive_letter(&club::root()) {
        println!(
            "{}",
            tr(
                "Warning: the club's folder was given as a mapped drive. horizon-data.bin \
               remembers the exact path of its folder, so write it again through the share's \
               own name (\\\\server\\Horizon) if that is what the students see."
            )
        );
    }
    #[cfg(not(windows))]
    let _ = ();
}

/// Whether `root` is a drive letter Windows mapped to a share.
#[cfg(windows)]
fn mapped_drive_letter(root: &Path) -> bool {
    use windows_sys::Win32::NetworkManagement::WNet::WNetGetConnectionW;
    let Some(text) = root.to_str() else {
        return false;
    };
    let bytes = text.as_bytes();
    if bytes.len() < 2 || !bytes[0].is_ascii_alphabetic() || bytes[1] != b':' {
        return false;
    }
    let drive: Vec<u16> = text[..2].encode_utf16().chain(std::iter::once(0)).collect();
    let mut target = [0u16; 512];
    let mut length = target.len() as u32;
    // NO_ERROR (0): the letter is mapped now. ERROR_CONNECTION_UNAVAIL (1): a remembered
    // mapping no session has reconnected yet — still a mapped drive.
    let code = unsafe { WNetGetConnectionW(drive.as_ptr(), target.as_mut_ptr(), &mut length) };
    code == 0 || code == 1
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> PathBuf {
        let folder =
            std::env::temp_dir().join(format!("horizon-club-admin-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&folder);
        folder
    }

    /// A build copied into `core\` leaves its data file behind: that file names the folder it
    /// was written in, and the game reads its files one by one rather than use it elsewhere
    /// (measured: 104 seconds instead of 4).
    #[test]
    fn a_build_is_installed_without_the_data_file_of_its_old_folder() {
        let root = scratch("install-core");
        let from = root.join("26.3-20261122");
        let files = from.join(".local").join("horizon-client").join("26.3");
        fs::create_dir_all(from.join("LICENSES")).unwrap();
        fs::create_dir_all(&files).unwrap();
        fs::write(from.join("horizon-client.exe"), "MZ the game").unwrap();
        fs::write(from.join("LICENSES").join("a.txt"), "licence").unwrap();
        fs::write(files.join("asset-index.json"), "{}").unwrap();
        fs::write(files.join(DATA_FILE), "written for another folder").unwrap();

        let core = root.join("core");
        assert_eq!(
            install_build(&core, &from, None).unwrap(),
            "26.3-20261122",
            "the folder's name is the tag"
        );
        let installed = core.join("26.3-20261122");
        assert_eq!(
            fs::read_to_string(installed.join("horizon-client.exe")).unwrap(),
            "MZ the game"
        );
        assert!(installed.join("LICENSES").join("a.txt").is_file());
        let copied = installed.join(".local").join("horizon-client").join("26.3");
        assert!(copied.join("asset-index.json").is_file());
        assert!(
            !copied.join(DATA_FILE).exists(),
            "the data file of the old folder is not carried"
        );
        // The folder it was copied from keeps its own.
        assert!(files.join(DATA_FILE).is_file());

        // The same tag twice is refused, and so is a tag that would leave core\.
        assert!(install_build(&core, &from, None).is_err());
        assert!(install_build(&core, &from, Some("../elsewhere".into())).is_err());
        assert!(install_build(&core, &from, Some("a/b".into())).is_err());
        assert!(!root.join("elsewhere").exists());
        let _ = fs::remove_dir_all(&root);
    }

    /// A file that is not a data file of its folder counts for none, as one written for another
    /// folder does: the check then says to write it again.
    #[test]
    fn a_data_file_that_is_not_this_folder_s_counts_for_none() {
        let resources = scratch("data-file");
        fs::create_dir_all(&resources).unwrap();
        assert!(!data_file_usable(&resources), "nothing there");
        fs::write(resources.join(DATA_FILE), "not a data file").unwrap();
        assert!(!data_file_usable(&resources));
        let _ = fs::remove_dir_all(&resources);
    }
}
