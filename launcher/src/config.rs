//! The club's configuration: `club.toml` in the club's root (docs/school-edition/launcher-target.md,
//! section 3). The supervisor writes the file and the students only read it. Every key is
//! optional, and a root without the file has an empty configuration: the defaults let a bare
//! root work. A key got wrong (a name misspelt, a value of another kind) is an error the window
//! shows, never a setting read another way: a rule the supervisor wrote (a session several
//! students share, the build to play) must not fall silent because its name was mistyped.

use crate::i18n;
use serde::Deserialize;
use std::path::Path;

/// The file's name in the club's root ([`crate::club::root`]).
pub const FILE: &str = crate::club::CONFIG_FILE;

/// How a student is named (`[club] identity`): by their Windows session, the only mode, as the
/// edition has no accounts.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Identity {
    #[default]
    Windows,
}

/// The launcher's language the first time a student opens it (`[club] language`): one of the two
/// the edition ships, written by its code. A student who picks another language in Settings keeps
/// theirs.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize)]
pub enum Language {
    #[default]
    #[serde(rename = "fr")]
    French,
    #[serde(rename = "en")]
    English,
}

impl Language {
    /// The launcher's own name for it ([`i18n::Language`]).
    pub fn of(self) -> i18n::Language {
        match self {
            Self::French => i18n::Language::French,
            Self::English => i18n::Language::English,
        }
    }
}

/// `[club]`: what the club is called and who plays.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Club {
    /// Shown as the window's title and on Home.
    pub name: String,
    /// The launcher's language the first time a student opens it.
    pub language: Language,
    pub identity: Identity,
    /// The Windows sessions several students share: with one of them the launcher refuses to play
    /// and says why.
    pub shared_accounts: Vec<String>,
}

impl Default for Club {
    fn default() -> Self {
        Self {
            name: "Club Minecraft".into(),
            language: Language::French,
            identity: Identity::Windows,
            shared_accounts: Vec::new(),
        }
    }
}

/// One server the club offers (`[[servers]]`).
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Server {
    pub name: String,
    pub address: String,
}

/// `[game]`: written into a student's options at their first Play (`first_play.rs`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Game {
    /// `renderDistance`.
    pub render_distance: u32,
    /// `simulationDistance`.
    pub simulation_distance: u32,
    /// `maxFps`.
    pub max_fps: u32,
}

impl Default for Game {
    fn default() -> Self {
        Self {
            render_distance: 8,
            simulation_distance: 5,
            max_fps: 60,
        }
    }
}

/// `[lan]`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Lan {
    /// A world may be opened to the LAN (vanilla's Open to LAN).
    pub open: bool,
}

impl Default for Lan {
    fn default() -> Self {
        Self { open: true }
    }
}

/// `[hosting]`: the Servers page.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Hosting {
    /// The page exists only when this is on.
    pub enabled: bool,
    /// Hosted servers take a port from this one up...
    pub first_port: u16,
    /// ...to this one.
    pub last_port: u16,
    /// The supervisor accepts Minecraft's EULA for hosted servers.
    pub eula_accepted: bool,
}

impl Default for Hosting {
    fn default() -> Self {
        Self {
            enabled: true,
            first_port: 25565,
            last_port: 25575,
            eula_accepted: true,
        }
    }
}

/// `[versions]`.
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Versions {
    /// The build the club plays (`core\<tag>`). Unset, or naming no build of `core\`: the only
    /// build there is, else the newest folder name.
    pub active: Option<String>,
}

impl Versions {
    /// The build the club plays among `installed`, the builds of `core\` newest first: the active
    /// one when it is there; else the only build, else the newest folder name, which is the first
    /// of `installed` either way.
    pub fn build<'a>(&'a self, installed: &'a [String]) -> Option<&'a str> {
        self.active
            .as_deref()
            .filter(|tag| installed.iter().any(|installed| installed == tag))
            .or_else(|| installed.first().map(String::as_str))
    }
}

#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    pub club: Club,
    /// The servers the club offers, in this order.
    pub servers: Vec<Server>,
    pub game: Game,
    pub lan: Lan,
    pub hosting: Hosting,
    pub versions: Versions,
}

/// Why `club.toml` could not be read: the sentence Home shows.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Error(String);

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// What the window says of a file it could not read.
fn unreadable(reason: impl std::fmt::Display) -> Error {
    Error(i18n::trf(
        "The club's file club.toml could not be read: {reason}. Ask the club's supervisor to fix \
         it.",
        &[("reason", &reason.to_string())],
    ))
}

impl Config {
    /// The club's configuration, read again from `club.toml` in the club's root: the default one
    /// when the root has no file.
    pub fn load() -> Result<Self, Error> {
        Self::read(&crate::club::root())
    }

    /// [`Config::load`] with the root given.
    pub fn read(root: &Path) -> Result<Self, Error> {
        let path = root.join(FILE);
        let text = match std::fs::read_to_string(&path) {
            Ok(text) => text,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Self::default()),
            Err(e) => return Err(unreadable(format!("{}: {e}", path.display()))),
        };
        Self::parse(&text).map_err(|e| unreadable(said(&e)))
    }

    /// The configuration of a `club.toml`'s text.
    pub fn parse(text: &str) -> Result<Self, toml::de::Error> {
        let mut config: Self = toml::from_str(text)?;
        // A name of nothing but spaces is no name.
        if config.club.name.trim().is_empty() {
            config.club.name = Club::default().name;
        }
        // A server with no name or no address is none to show or to join.
        if config
            .servers
            .iter()
            .any(|s| s.name.trim().is_empty() || s.address.trim().is_empty())
        {
            return Err(<toml::de::Error as serde::de::Error>::custom(
                "a [[servers]] entry needs a name and an address",
            ));
        }
        Ok(config)
    }
}

/// What the parser found wrong, on one line for the supervisor: where it stopped ("TOML parse
/// error at line 5, column 1"), then what it says, without the quoted line and its caret.
fn said(error: &toml::de::Error) -> String {
    let shown = error.to_string();
    let mut lines = shown.lines();
    let at = lines.next().unwrap_or_default();
    let rest: Vec<&str> = lines.filter(|line| !line.contains('|')).collect();
    format!("{at}: {}", rest.join(" "))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The example of the spec (docs/school-edition/launcher-target.md, section 3), comments and
    /// all.
    const EXAMPLE: &str = r#"
[club]
name = "Club Minecraft"          # shown as the window title and on Home
language = "fr"                  # launcher language the first time a student opens it
identity = "windows"             # how a student is named: "windows" is the only mode
shared_accounts = ["eleve"]      # Windows sessions several students share: with one of them the
                                 # launcher refuses to play and says why

[[servers]]                      # the servers the club offers, in this order
name = "Serveur du club"
address = "mc-club.example.edu"  # a port after a colon only when it is not 25565

[game]                           # written into a student's options at their first Play
render_distance = 8
simulation_distance = 5
max_fps = 60

[lan]
open = true                      # a world may be opened to the LAN (vanilla's Open to LAN)

[hosting]                        # the Servers page
enabled = true
first_port = 25565               # hosted servers take a port from this one up
last_port = 25575
eula_accepted = true             # the supervisor accepts Minecraft's EULA for hosted servers

[versions]
active = "26.3-20261122"         # the build the club plays; else the only build in core\, else
                                 # the newest folder name
"#;

    #[test]
    fn an_empty_file_gives_the_defaults() {
        let empty = Config::parse("").unwrap();
        assert_eq!(empty, Config::default(), "no table at all");
        // The defaults, one by one: what a bare root plays with.
        assert_eq!(empty.club.name, "Club Minecraft");
        assert_eq!(empty.club.language, Language::French);
        assert_eq!(empty.club.identity, Identity::Windows);
        assert!(
            empty.club.shared_accounts.is_empty(),
            "no session is shared unless listed"
        );
        assert!(empty.servers.is_empty());
        assert_eq!(
            empty.game,
            Game {
                render_distance: 8,
                simulation_distance: 5,
                max_fps: 60
            }
        );
        assert!(empty.lan.open);
        assert_eq!(
            empty.hosting,
            Hosting {
                enabled: true,
                first_port: 25565,
                last_port: 25575,
                eula_accepted: true
            }
        );
        assert_eq!(empty.versions.active, None);
        // A name of spaces only is no name: the default one.
        let blank = Config::parse("[club]\nname = \"  \"\n").unwrap();
        assert_eq!(blank.club.name, "Club Minecraft");
    }

    #[test]
    fn the_specs_example_gives_its_values() {
        let config = Config::parse(EXAMPLE).unwrap();
        assert_eq!(config.club.name, "Club Minecraft");
        assert_eq!(config.club.language, Language::French);
        assert_eq!(config.club.identity, Identity::Windows);
        assert_eq!(config.club.shared_accounts, ["eleve"]);
        assert_eq!(
            config.servers,
            [Server {
                name: "Serveur du club".into(),
                // As written: a club's server without a port keeps none (the game's is 25565).
                address: "mc-club.example.edu".into()
            }]
        );
        assert_eq!(config.game.render_distance, 8);
        assert_eq!(config.game.simulation_distance, 5);
        assert_eq!(config.game.max_fps, 60);
        assert!(config.lan.open);
        assert!(config.hosting.enabled);
        assert_eq!(
            (config.hosting.first_port, config.hosting.last_port),
            (25565, 25575)
        );
        assert!(config.hosting.eula_accepted);
        assert_eq!(config.versions.active.as_deref(), Some("26.3-20261122"));
    }

    #[test]
    fn a_wrong_file_is_an_error() {
        let wrong = |text: &str| Config::parse(text).is_err();
        // Not TOML.
        assert!(wrong("[club\nname = \"x\"\n"));
        // A name misspelt: never a rule that silently doesn't apply.
        assert!(wrong("[club]\nshared_acounts = [\"eleve\"]\n"));
        assert!(wrong("[version]\nactive = \"x\"\n"));
        assert!(wrong("active = \"x\"\n"));
        assert!(wrong(
            "[[servers]]\nname = \"Club\"\naddress = \"a:1\"\nicon = \"x\"\n"
        ));
        // A value of another kind.
        assert!(wrong("[game]\nrender_distance = \"eight\"\n"));
        assert!(wrong("[game]\nrender_distance = -8\n"));
        assert!(wrong("[hosting]\nfirst_port = 70000\n"));
        assert!(wrong("[club]\nshared_accounts = \"eleve\"\n"));
        // A server with no name or no address is none to show or to join.
        assert!(wrong("[[servers]]\nname = \"Club\"\naddress = \"  \"\n"));
        assert!(wrong("[[servers]]\nname = \"\"\naddress = \"a:1\"\n"));
        // A mode or a language the launcher doesn't have.
        assert!(wrong("[club]\nidentity = \"none\"\n"));
        assert!(wrong("[club]\nlanguage = \"de\"\n"));
        // Each table and key is optional: what a file may leave out.
        Config::parse("[club]\nname = \"x\"\n").unwrap();
        Config::parse("[game]\nmax_fps = 30\n").unwrap();
        Config::parse("[versions]\n").unwrap();
        Config::parse(
            "[[servers]]\nname = \"A\"\naddress = \"a\"\n[[servers]]\nname = \"B\"\naddress = \"b\"\n",
        )
        .unwrap();
    }

    #[test]
    fn a_missing_file_is_an_empty_configuration_and_an_unreadable_one_an_error() {
        let root = std::env::temp_dir().join(format!("horizon-club-config-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        // No club.toml: the defaults.
        assert_eq!(Config::read(&root).unwrap(), Config::default());
        // The spec's example, as the supervisor wrote it.
        std::fs::write(root.join(FILE), EXAMPLE).unwrap();
        assert_eq!(
            Config::read(&root).unwrap(),
            Config::parse(EXAMPLE).unwrap()
        );
        // A file that doesn't parse: the sentence says where, and to ask the supervisor.
        std::fs::write(root.join(FILE), "[club]\nnme = \"x\"\n").unwrap();
        let said = Config::read(&root).unwrap_err().to_string();
        assert!(said.contains("line 2"), "{said}");
        assert!(said.contains("nme"), "{said}");
        assert!(said.contains("supervisor"), "{said}");
        // A folder where the file should be cannot be read.
        std::fs::remove_file(root.join(FILE)).unwrap();
        std::fs::create_dir(root.join(FILE)).unwrap();
        assert!(Config::read(&root).is_err());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn the_build_in_use_is_the_active_one_else_the_only_one_else_the_newest() {
        let tags = |names: &[&str]| names.iter().map(|n| n.to_string()).collect::<Vec<_>>();
        let none = Versions { active: None };
        let active = Versions {
            active: Some("club-9-old".into()),
        };
        let list = tags(&["club-10-new", "club-9-old", "club-8-older"]);
        // The active build, when core holds it.
        assert_eq!(active.build(&list), Some("club-9-old"));
        // Unset: the newest folder name, the list's first.
        assert_eq!(none.build(&list), Some("club-10-new"));
        // The only build there is.
        assert_eq!(none.build(&tags(&["only-one"])), Some("only-one"));
        // Active names no build of core: the rules of an unset one.
        assert_eq!(active.build(&tags(&["club-10-new"])), Some("club-10-new"));
        assert_eq!(active.build(&tags(&["club-9-old"])), Some("club-9-old"));
        // No build at all.
        assert_eq!(active.build(&[]), None);
        assert_eq!(none.build(&[]), None);
    }

    /// The club's language is the launcher's first one; a student's own pick comes first.
    #[test]
    fn the_clubs_language_names_one_the_launcher_has() {
        assert_eq!(Language::default().of(), i18n::Language::French);
        assert_eq!(Language::English.of(), i18n::Language::English);
        assert_eq!(
            Config::parse("[club]\nlanguage = \"en\"\n")
                .unwrap()
                .club
                .language,
            Language::English
        );
    }
}
