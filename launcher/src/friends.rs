//! A student's friends: the players every server they create lets in from the start, beside the
//! student. The list is theirs alone (`friends.json` in their launcher's folder on their
//! computer) and names players, as a server's whitelist and operators do. A friend is picked from
//! the cards of who has the launcher open ([`crate::online::people`]): the folder where those are
//! said is every student's to write, so a name is read only when it is one.

use std::path::{Path, PathBuf};

/// The list's file, in the launcher's data folder.
pub const FILE: &str = "friends.json";

/// A player name Minecraft accepts: 3 to 16 letters, digits or underscores.
pub fn valid_name(name: &str) -> bool {
    (3..=16).contains(&name.len()) && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// Where the list is: the launcher's data folder, on the student's own computer.
pub fn file() -> PathBuf {
    crate::backend::data_dir().join(FILE)
}

/// The student's friends, by name.
pub fn load() -> Vec<String> {
    read(&file())
}

/// [`load`] with the file given.
pub fn read(path: &Path) -> Vec<String> {
    std::fs::read_to_string(path)
        .map(|text| parse(&text))
        .unwrap_or_default()
}

/// Saves the list. Err: why the file could not be written.
pub fn save(friends: &[String]) -> Result<(), String> {
    write(&file(), friends)
}

/// [`save`] with the file given.
pub fn write(path: &Path, friends: &[String]) -> Result<(), String> {
    std::fs::create_dir_all(path.parent().unwrap_or(path))
        .map_err(|e| format!("{}: {e}", path.display()))?;
    let text = serde_json::to_string(&tidy(friends)).map_err(|e| e.to_string())?;
    crate::online::write_whole(path, &text).map_err(|e| format!("{}: {e}", path.display()))
}

/// The list in a file's text: its names that Minecraft would take, each once (two spellings of
/// one name are the same player), by name. A file that is not a list of names holds no friends.
pub fn parse(text: &str) -> Vec<String> {
    let names: Vec<serde_json::Value> = serde_json::from_str(text).unwrap_or_default();
    let names: Vec<String> = names
        .into_iter()
        .filter_map(|v| v.as_str().map(str::to_string))
        .collect();
    tidy(&names)
}

/// Whether `name` is in the list.
pub fn is_friend(friends: &[String], name: &str) -> bool {
    friends.iter().any(|f| f == name)
}

/// Adds `name` to the list or takes it away: false when it is a friend afterwards. A name
/// Minecraft would not take is neither.
pub fn toggle(friends: &mut Vec<String>, name: &str) -> bool {
    if !valid_name(name) {
        return false;
    }
    if let Some(at) = friends.iter().position(|f| f == name) {
        friends.remove(at);
        return false;
    }
    friends.push(name.to_string());
    *friends = tidy(friends);
    true
}

/// The names that are names, each once whatever their case, by name whatever their case.
fn tidy(names: &[String]) -> Vec<String> {
    let mut clean: Vec<String> = Vec::new();
    for name in names {
        if valid_name(name) && !clean.iter().any(|k| k.eq_ignore_ascii_case(name)) {
            clean.push(name.clone());
        }
    }
    clean.sort_by(|a, b| a.to_lowercase().cmp(&b.to_lowercase()).then(a.cmp(b)));
    clean
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn friend_names_follow_minecrafts_rules() {
        assert!(valid_name("Alex"));
        assert!(valid_name("a_b_C9"));
        assert!(valid_name("1234567890123456"));
        assert!(!valid_name("al"));
        assert!(!valid_name("12345678901234567"));
        assert!(!valid_name("Alice Bob"));
        assert!(!valid_name("O'Brien"));
        assert!(!valid_name("é"));
        assert!(!valid_name(""));
    }

    /// Picking a card adds a friend, picking it again takes them away; the list keeps its names
    /// once and in order, wherever they came from.
    #[test]
    fn a_friend_is_added_and_removed_by_their_name() {
        let mut friends = Vec::new();
        assert!(toggle(&mut friends, "Zed"));
        assert!(toggle(&mut friends, "Alice"));
        assert!(is_friend(&friends, "Zed"));
        assert!(is_friend(&friends, "Alice"));
        assert_eq!(friends, ["Alice", "Zed"], "by name, each once");
        assert!(!toggle(&mut friends, "Zed"));
        assert_eq!(friends, ["Alice"]);
        // A name Minecraft would not take is neither added nor removed.
        assert!(!toggle(&mut friends, "a b"));
        assert_eq!(friends, ["Alice"]);
    }

    #[test]
    fn the_list_is_kept_in_a_file_and_read_back() {
        let home = std::env::temp_dir().join(format!("horizon-friends-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&home);
        let path = home.join(FILE);
        // No file yet: no friends.
        assert!(read(&path).is_empty());
        write(
            &path,
            &["Bob".into(), "Alice".into(), "Bob".into(), "../x".into()],
        )
        .unwrap();
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            r#"["Alice","Bob"]"#
        );
        assert_eq!(read(&path), ["Alice", "Bob"]);
        // A file that is not a list holds no friends, and what is not a name is not read: the
        // list is written into commands.
        assert!(parse("{ broken").is_empty());
        assert_eq!(parse(r#"["Eve", "a b", "eve", "x y", 4, null]"#), ["Eve"]);
        let _ = std::fs::remove_dir_all(&home);
    }
}
