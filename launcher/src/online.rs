//! The hosted servers announced in `<root>\online\` (docs/school-edition/launcher-target.md,
//! section 6). The launcher that hosts a server writes `online\<COMPUTERNAME>.json` again every
//! ten seconds while it runs and deletes it when it stops; every launcher lists the
//! announcements under [`FRESH`] old, so a host that died takes its server off the list by
//! itself.

use serde::{Deserialize, Serialize};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::Duration;

/// How long an announcement counts for: its host writes it again three times over before then.
pub const FRESH: Duration = Duration::from_secs(45);

/// One hosted server, as its host announces it.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct Announcement {
    /// The server's name, as the Servers page gave it.
    pub name: String,
    /// Who hosts it: their player name.
    pub owner: String,
    /// The host's address on the network the club's servers are on.
    pub host: String,
    pub port: u16,
    /// When the host wrote the announcement: seconds since the Unix epoch, by its own clock.
    pub at: u64,
}

impl Announcement {
    /// The announcement as its file holds it: one JSON line, the spec's fields in its order and
    /// its spacing, the strings escaped as JSON escapes them.
    pub fn text(&self) -> String {
        let string = |value: &str| serde_json::Value::String(value.to_string()).to_string();
        format!(
            "{{\"name\": {}, \"owner\": {}, \"host\": {}, \"port\": {}, \"at\": {}}}",
            string(&self.name),
            string(&self.owner),
            string(&self.host),
            self.port,
            self.at
        )
    }

    /// Where the game finds the server: `host:port` (`[host]:port` for an IPv6 address).
    pub fn address(&self) -> String {
        if self.host.contains(':') {
            format!("[{}]:{}", self.host, self.port)
        } else {
            format!("{}:{}", self.host, self.port)
        }
    }

    /// Whether the announcement still counts at `now` (seconds since the epoch, by this
    /// computer's clock). A host whose clock runs a little ahead writes a time still to come
    /// here: that announcement counts until its own 45 seconds have passed.
    pub fn is_fresh(&self, now: u64) -> bool {
        now.saturating_sub(self.at) < FRESH.as_secs()
    }
}

/// The file a computer's announcement has in `online\`: the computer's name, made of characters
/// a file name takes, with `.json` after it. One file per computer, so one hosted server is
/// announced from each.
pub fn file_name(computer: &str) -> String {
    let clean: String = computer
        .trim()
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.') {
                c
            } else {
                '-'
            }
        })
        .collect();
    let clean = if clean.is_empty() {
        "pc".to_string()
    } else {
        clean
    };
    format!("{clean}.json")
}

/// This computer's name, as Windows sessions know it (`COMPUTERNAME`).
pub fn computer() -> String {
    std::env::var("COMPUTERNAME")
        .or_else(|_| std::env::var("HOSTNAME"))
        .unwrap_or_default()
}

/// Where a computer's announcement goes: `online\<COMPUTERNAME>.json`.
pub fn file(dir: &Path) -> PathBuf {
    dir.join(file_name(&computer()))
}

/// Writes the announcement whole: a temporary file beside, flushed, then renamed into place, so
/// another launcher never reads half of one. The folder is the supervisor's to make: a student
/// who cannot write there is not hosting for the club, which the caller says once.
pub fn write(dir: &Path, announcement: &Announcement) -> Result<(), String> {
    let path = file(dir);
    let partial = path.with_extension("json.tmp");
    let mut file =
        std::fs::File::create(&partial).map_err(|e| format!("{}: {e}", partial.display()))?;
    let text = announcement.text();
    file.write_all(text.as_bytes())
        .and_then(|()| file.sync_all())
        .map_err(|e| format!("{}: {e}", partial.display()))?;
    drop(file);
    std::fs::rename(&partial, &path).map_err(|e| format!("{}: {e}", path.display()))
}

/// Takes this computer's announcement away: nothing left to join once its server has stopped.
/// A file that is already gone is gone.
pub fn remove(dir: &Path) -> Result<(), String> {
    match std::fs::remove_file(file(dir)) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(format!("{}", e)),
    }
}

/// The announcement in a file's text. None for a broken one: not JSON, a field missing or of
/// another kind, a server with no name or no port, or a host that is not [`local`].
pub fn parse(text: &str) -> Option<Announcement> {
    let announcement: Announcement = serde_json::from_str(text).ok()?;
    (announcement.port != 0 && !announcement.name.trim().is_empty() && local(&announcement.host))
        .then_some(announcement)
}

/// Whether `host` is an address of the local network, written as one: a private address (RFC 1918,
/// RFC 4193), a loopback or a link-local one, nowhere else. The folder is every student's to
/// write, and every launcher joins what it finds there and asks it who is on: an announcement
/// naming any other host would send the whole club there.
pub fn local(host: &str) -> bool {
    match host.trim().parse::<std::net::IpAddr>() {
        Ok(std::net::IpAddr::V4(v4)) => v4.is_private() || v4.is_loopback() || v4.is_link_local(),
        Ok(std::net::IpAddr::V6(v6)) => match v6.to_ipv4_mapped() {
            Some(v4) => local(&v4.to_string()),
            None => v6.is_unique_local() || v6.is_loopback() || v6.is_unicast_link_local(),
        },
        Err(_) => false,
    }
}

/// The servers announced in `dir`, each in a `*.json` file: those under [`FRESH`] old at `now`,
/// by name then address. A file that is broken or cannot be read is skipped, as one that is old.
pub fn list(dir: &Path, now: u64) -> Vec<Announcement> {
    let mut found: Vec<Announcement> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|entry| entry.path().extension().is_some_and(|e| e == "json"))
        .filter_map(|entry| parse(&std::fs::read_to_string(entry.path()).ok()?))
        .filter(|announcement| announcement.is_fresh(now))
        .collect();
    found.sort_by(|a, b| (&a.name, &a.host, a.port).cmp(&(&b.name, &b.host, b.port)));
    found.dedup();
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The announcement of the spec (docs/school-edition/launcher-target.md, section 6).
    const SPEC: &str = r#"{"name": "Monde de Jdupont", "owner": "Jdupont", "host": "192.168.10.42", "port": 25565, "at": 1763800000}"#;
    const AT: u64 = 1763800000;

    #[test]
    fn an_announcement_reads_as_the_spec_writes_it() {
        let announcement = parse(SPEC).unwrap();
        assert_eq!(
            announcement,
            Announcement {
                name: "Monde de Jdupont".into(),
                owner: "Jdupont".into(),
                host: "192.168.10.42".into(),
                port: 25565,
                at: AT,
            }
        );
        assert_eq!(announcement.address(), "192.168.10.42:25565");
        // An IPv6 host is bracketed, as an address has to be.
        let six = Announcement {
            host: "fe80::1".into(),
            ..announcement.clone()
        };
        assert_eq!(six.address(), "[fe80::1]:25565");
    }

    /// The writer's half of the spec: what a host writes is what every launcher reads, one JSON
    /// line with the same fields in the same order.
    #[test]
    fn an_announcement_writes_as_the_spec_writes_it() {
        let announcement = Announcement {
            name: "Monde de Jdupont".into(),
            owner: "Jdupont".into(),
            host: "192.168.10.42".into(),
            port: 25565,
            at: AT,
        };
        assert_eq!(announcement.text(), SPEC);
        // The file round-trips: what `write` puts in `parse` reads back.
        assert_eq!(parse(&announcement.text()), Some(announcement));
    }

    /// One file per computer, named for it: the share takes only file-name characters.
    #[test]
    fn the_announcements_file_is_named_for_the_computer() {
        assert_eq!(file_name("PC-INFO-04"), "PC-INFO-04.json");
        assert_eq!(file_name(" pc-arts "), "pc-arts.json");
        // A name the share could not hold is made into one it can.
        assert_eq!(file_name("PC Info/3"), "PC-Info-3.json");
        assert_eq!(file_name(""), "pc.json");
        assert_eq!(file_name("  "), "pc.json");
    }

    #[test]
    fn an_announcement_under_forty_five_seconds_old_counts_and_an_older_one_does_not() {
        let announcement = parse(SPEC).unwrap();
        assert!(announcement.is_fresh(AT), "just written");
        assert!(announcement.is_fresh(AT + 44), "44 seconds old");
        assert!(
            !announcement.is_fresh(AT + 45),
            "45 seconds old: its host stopped saying so"
        );
        assert!(!announcement.is_fresh(AT + 600));
        // A host whose clock is 20 seconds ahead of this computer's.
        assert!(announcement.is_fresh(AT - 20));
    }

    #[test]
    fn a_broken_announcement_is_skipped() {
        assert_eq!(parse("not json"), None);
        assert_eq!(parse(""), None);
        // A field missing.
        assert_eq!(
            parse(r#"{"name": "Monde", "owner": "Jdupont", "host": "192.168.10.42", "at": 1}"#),
            None
        );
        // A field of another kind.
        assert_eq!(
            parse(
                r#"{"name": "Monde", "owner": "Jdupont", "host": "192.168.10.42", "port": "25565", "at": 1}"#
            ),
            None
        );
        assert_eq!(
            parse(r#"{"name": "Monde", "owner": "Jdupont", "host": 42, "port": 25565, "at": 1}"#),
            None
        );
        assert_eq!(
            parse(
                r#"{"name": "Monde", "owner": "Jdupont", "host": "192.168.10.42", "port": 70000, "at": 1}"#
            ),
            None
        );
        // A server with no name or no port is nowhere to join.
        assert_eq!(
            parse(r#"{"name": "  ", "owner": "J", "host": "10.0.0.1", "port": 25565, "at": 1}"#),
            None
        );
        assert_eq!(
            parse(r#"{"name": "Monde", "owner": "J", "host": "10.0.0.1", "port": 0, "at": 1}"#),
            None
        );
        // A host that is not an address of the local network, written as one.
        assert_eq!(
            parse(r#"{"name": "Monde", "owner": "J", "host": "", "port": 25565, "at": 1}"#),
            None
        );
        assert_eq!(
            parse(r#"{"name": "Monde", "owner": "J", "host": "8.8.8.8", "port": 25565, "at": 1}"#),
            None
        );
        assert_eq!(
            parse(
                r#"{"name": "Monde", "owner": "J", "host": "mc.example.org", "port": 25565, "at": 1}"#
            ),
            None
        );
        // A field a newer launcher writes is not a broken file.
        assert!(parse(r#"{"name": "Monde", "owner": "J", "host": "10.0.0.1", "port": 25565, "at": 1, "world": "seed"}"#).is_some());
    }

    /// The ranges the managed game itself connects to, and the ones outside them.
    #[test]
    fn only_an_address_of_the_local_network_is_a_hosted_server() {
        for host in [
            "10.255.0.1",
            "172.16.0.1",
            "172.31.255.254",
            "192.168.1.2",
            "127.0.0.1",
            "169.254.1.1",
            "::1",
            "fc00::1",
            "fd12:3456:789a::1",
            "fe80::1",
            "::ffff:10.1.2.3",
            "::ffff:192.168.0.1",
        ] {
            assert!(local(host), "{host}");
        }
        for (host, why) in [
            ("8.8.8.8", "the internet"),
            ("172.32.0.1", "just past 172.16/12"),
            ("172.15.255.255", "just before it"),
            ("192.169.0.1", "just past 192.168/16"),
            ("100.64.0.1", "shared address space"),
            ("2001:4860:4860::8888", "the internet, v6"),
            ("::ffff:8.8.8.8", "a v4 address through v6"),
            ("mc.example.org", "a name, not an address"),
            ("10.0.0.256", "not an address"),
            ("", "nothing"),
        ] {
            assert!(!local(host), "{host}: {why}");
        }
    }

    #[test]
    fn the_folder_lists_what_is_fresh_and_skips_what_is_old_or_broken() {
        let dir = std::env::temp_dir().join(format!("horizon-club-online-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let with = |name: &str, host: &str, at: u64| {
            format!(
                r#"{{"name": "{name}", "owner": "Jdupont", "host": "{host}", "port": 25565, "at": {at}}}"#
            )
        };
        let files = [
            ("PC-ARTS.json", with("Monde de Zed", "10.0.0.3", AT - 2)),
            ("PC-MATHS.json", with("Monde de Alice", "10.0.0.1", AT - 40)),
            ("PC-INFO.json", with("Monde de Bob", "10.0.0.2", AT - 200)),
            ("PC-LATIN.json", "{ not json".to_string()),
            ("PC-SVT.txt", with("Monde de Eve", "10.0.0.5", AT)),
        ];
        for (file, text) in &files {
            std::fs::write(dir.join(file), text).unwrap();
        }
        // At AT: two announced within 45 seconds, by name; the old, the broken and the text
        // file are left out.
        let listed: Vec<(String, String)> = list(&dir, AT)
            .into_iter()
            .map(|a| {
                let address = a.address();
                (a.name, address)
            })
            .collect();
        assert_eq!(
            listed,
            [
                ("Monde de Alice".into(), "10.0.0.1:25565".into()),
                ("Monde de Zed".into(), "10.0.0.3:25565".into()),
            ]
        );
        // Ten seconds later the first is 50 seconds old: gone, as its host stopped saying so.
        let names: Vec<String> = list(&dir, AT + 10).into_iter().map(|a| a.name).collect();
        assert_eq!(names, ["Monde de Zed"]);
        // No folder at all: nothing, and no error.
        assert!(list(&dir.join("none"), AT).is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// `write` puts one file in the folder, whole, and `remove` takes it away; a folder that is
    /// not there to write to is an error the caller says once, not a panic.
    #[test]
    fn the_announcement_is_written_whole_and_removed() {
        let dir =
            std::env::temp_dir().join(format!("horizon-club-online-write-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let announcement = Announcement {
            name: "Monde de Jdupont".into(),
            owner: "Jdupont".into(),
            host: "192.168.10.42".into(),
            port: 25565,
            at: AT,
        };
        write(&dir, &announcement).unwrap();
        let written: Vec<_> = list(&dir, AT);
        assert_eq!(written.as_slice(), std::slice::from_ref(&announcement));
        // No temporary file is left beside it.
        let names: Vec<String> = std::fs::read_dir(&dir)
            .unwrap()
            .flatten()
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .collect();
        assert_eq!(names.len(), 1, "{names:?}");
        remove(&dir).unwrap();
        assert!(list(&dir, AT).is_empty());
        // Taking it away again is fine: already gone is gone.
        remove(&dir).unwrap();
        // A student who cannot write there (or a folder that is not there) is an error.
        assert!(write(&dir.join("none"), &announcement).is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
