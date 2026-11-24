//! The servers a student was invited to, as their launcher remembers them (`invited.json` in
//! the launcher's folder on their computer): a hosted server they joined, and one a friend
//! hosts (a friend's server lets its host's friends in from the start). The sidebar's second
//! group lists them while they run and after, dimmed, so a student finds a friend's server again
//! at the next session. One that wasn't announced for [`KEPT`] days is forgotten by itself.

use crate::online::Announcement;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// The list's file, in the launcher's data folder.
pub const FILE: &str = "invited.json";

/// How long a server nobody announced stays remembered: a school term.
pub const KEPT: u64 = 120 * 24 * 60 * 60;

/// A hosted server, as it was last announced or joined.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct Invited {
    /// The server's name, as its host announced it.
    pub name: String,
    /// Who hosts it: their player name.
    pub owner: String,
    /// The host's address on the network the club's servers are on.
    pub host: String,
    pub port: u16,
    /// When it was last announced or joined: seconds since the Unix epoch.
    pub at: u64,
}

impl Invited {
    pub fn of(announcement: &Announcement) -> Self {
        Self {
            name: announcement.name.clone(),
            owner: announcement.owner.clone(),
            host: announcement.host.clone(),
            port: announcement.port,
            at: announcement.at,
        }
    }

    /// Two lines are the same server when they name the same one at the same place.
    pub fn same(&self, other: &Self) -> bool {
        self.name == other.name && self.host == other.host && self.port == other.port
    }

    /// Where the game finds the server, as an announcement's `address`.
    pub fn address(&self) -> String {
        Announcement {
            name: String::new(),
            owner: String::new(),
            host: self.host.clone(),
            port: self.port,
            at: self.at,
        }
        .address()
    }
}

/// Where the list is: the launcher's data folder, on the student's own computer.
pub fn file() -> PathBuf {
    crate::backend::data_dir().join(FILE)
}

/// The remembered servers, newest first.
pub fn load() -> Vec<Invited> {
    read(&file())
}

/// [`load`] with the file given.
pub fn read(path: &Path) -> Vec<Invited> {
    let mut list: Vec<Invited> = std::fs::read_to_string(path)
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default();
    // Only what a game could join: a host of the local network, and the same server once.
    list.retain(|i| crate::online::local(&i.host) && i.port != 0 && !i.name.trim().is_empty());
    list.sort_by(|a, b| b.at.cmp(&a.at).then(a.name.cmp(&b.name)));
    list.dedup_by(|a, b| a.same(b));
    list
}

/// Saves the list. Err: why the file could not be written.
pub fn save(list: &[Invited]) -> Result<(), String> {
    write(&file(), list)
}

/// [`save`] with the file given.
pub fn write(path: &Path, list: &[Invited]) -> Result<(), String> {
    std::fs::create_dir_all(path.parent().unwrap_or(path))
        .map_err(|e| format!("{}: {e}", path.display()))?;
    let text = serde_json::to_string(list).map_err(|e| e.to_string())?;
    crate::online::write_whole(path, &text).map_err(|e| format!("{}: {e}", path.display()))
}

/// The list once `seen` were announced now (`now`: seconds since the epoch) and `joined` was
/// joined, and the friends' servers among `seen` are invited too: a server once remembered
/// stays while it was announced or joined in the last [`KEPT`] seconds. Whether the list changed.
pub fn remember(
    list: &[Invited],
    seen: &[Announcement],
    friends: &[String],
    joined: Option<&Announcement>,
    now: u64,
) -> (Vec<Invited>, bool) {
    let mut out: Vec<Invited> = Vec::new();
    let mut add = |line: Invited| match out.iter_mut().find(|i| i.same(&line)) {
        Some(known) => known.at = known.at.max(line.at),
        None => out.push(line),
    };
    let now = now.max(seen.iter().map(|a| a.at).max().unwrap_or(0));
    for a in seen.iter().filter(|a| friends.contains(&a.owner)) {
        add(Invited::of(a));
    }
    if let Some(a) = joined {
        let joined = Invited::of(a);
        add(Invited {
            at: joined.at.max(now),
            ..joined
        });
    }
    for i in list {
        if now.saturating_sub(i.at) < KEPT {
            add(i.clone());
        }
    }
    out.sort_by(|a, b| b.at.cmp(&a.at).then(a.name.cmp(&b.name)));
    let changed = out != *list;
    (out, changed)
}

/// The remembered servers with whether each is announced now, the announced ones first.
pub fn states(list: &[Invited], seen: &[Announcement]) -> Vec<(Invited, bool)> {
    let mut out: Vec<(Invited, bool)> = list
        .iter()
        .map(|i| {
            let up = seen.iter().any(|a| Invited::of(a).same(i));
            (i.clone(), up)
        })
        .collect();
    out.sort_by_key(|(_, up)| !up);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(name: &str, owner: &str, host: &str, time: u64) -> Announcement {
        Announcement {
            name: name.into(),
            owner: owner.into(),
            host: host.into(),
            port: 25565,
            at: time,
        }
    }

    /// A friend's server and a server the student joined are remembered; a stranger's server
    /// only announced is not; what is remembered outlives its announcement, and fades after a
    /// term.
    #[test]
    fn a_friends_server_and_a_joined_one_are_remembered_and_the_others_are_not() {
        let now = 1_000_000;
        let seen = [
            at("Monde de Alice", "Alice", "10.0.0.1", now - 3),
            at("Monde de Zed", "Zed", "10.0.0.2", now - 4),
        ];
        let friends = ["Alice".to_string()];
        let (list, changed) = remember(&[], &seen, &friends, None, now);
        assert!(changed);
        assert_eq!(list, [Invited::of(&seen[0])], "only the friend's");
        // The student joins Zed's: it is remembered too.
        let (list, changed) = remember(&list, &seen, &friends, Some(&seen[1]), now);
        assert!(changed);
        assert_eq!(list.len(), 2);
        // The one just joined is the most recent, first.
        assert_eq!(list[0].address(), "10.0.0.2:25565");
        assert_eq!(list[0].at, now);
        // Read again with nothing announced, the two stay; nothing changed.
        let (same, changed) = remember(&list, &[], &friends, None, now + 60);
        assert!(!changed);
        assert_eq!(same, list);
        let states_list = states(&same, &[]);
        assert_eq!(
            states_list.iter().map(|(_, up)| *up).collect::<Vec<_>>(),
            [false, false]
        );
        // Alice's comes back on: it is first, and marked as announced.
        let (again, _) = remember(
            &same,
            &[at("Monde de Alice", "Alice", "10.0.0.1", now + 3_600)],
            &friends,
            None,
            now + 3_600,
        );
        assert_eq!(again.len(), 2);
        let shown = states(
            &again,
            &[at("Monde de Alice", "Alice", "10.0.0.1", now + 3_600)],
        );
        assert_eq!(
            shown
                .iter()
                .map(|(i, up)| (i.owner.as_str(), *up))
                .collect::<Vec<_>>(),
            [("Alice", true), ("Zed", false)]
        );
        // A term after she was last announced, neither announced: both forgotten.
        let (gone, changed) = remember(&again, &[], &friends, None, now + KEPT + 10 + 3_600);
        assert!(changed);
        assert!(gone.is_empty());
    }

    #[test]
    fn the_list_is_kept_in_a_file_and_read_back() {
        let home = std::env::temp_dir().join(format!("horizon-invited-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&home);
        let path = home.join(FILE);
        assert!(read(&path).is_empty());
        let alice = Invited::of(&at("Monde de Alice", "Alice", "10.0.0.1", 10));
        write(&path, &[alice.clone(), alice.clone()]).unwrap();
        assert_eq!(read(&path), [alice], "the same server once");
        // A line that could send the game anywhere else is not a server to list.
        std::fs::write(
            &path,
            serde_json::to_string(&[
                Invited::of(&at("A", "A", "8.8.8.8", 10)),
                Invited::of(&at("B", "B", "10.0.0.2", 10)),
                Invited::of(&at(" ", "C", "10.0.0.3", 11)),
            ])
            .unwrap(),
        )
        .unwrap();
        assert_eq!(read(&path), [Invited::of(&at("B", "B", "10.0.0.2", 10))]);
        assert!(read(&home.join("none")).is_empty());
        let _ = std::fs::remove_dir_all(&home);
    }
}
