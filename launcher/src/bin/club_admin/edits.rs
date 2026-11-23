//! Edits to `club.toml`'s text (`set-version`, `server add`, `server remove`): the file is the
//! supervisor's, with its comments and its order, so a key is changed by editing the text, never
//! by reading it into values and writing them back. Every answer is checked by parsing it
//! ([`Config::parse`]) before the caller writes it.

use horizon_launcher::config::Config;

/// `[versions] active = "<tag>"`: the line edited when it exists, else added under the section,
/// else a new section at the end. Comments and every other key stay as they are; the edited
/// line keeps its own trailing comment.
pub fn set_active(text: &str, tag: &str) -> Result<String, String> {
    let value = toml_value(tag);
    let mut out = String::new();
    let mut in_versions = false;
    let mut edited = false;
    for line in split_keep(text) {
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            in_versions = trimmed == "[versions]";
        } else if in_versions && !edited && key_of(&line) == Some("active") {
            out.push_str(&replace_value(&line, &value));
            edited = true;
            continue;
        }
        out.push_str(&line);
    }
    if edited {
        return finish(out, tag);
    }
    // No line to edit: the section, when it is there, takes one; else the file ends with a new
    // one.
    let mut lines: Vec<String> = text.lines().map(str::to_string).collect();
    let mut out_lines: Vec<String>;
    if let Some(at) = lines.iter().position(|line| line.trim() == "[versions]") {
        out_lines = lines.drain(..=at).collect();
        out_lines.push(format!("active = {value}"));
        out_lines.extend(lines);
    } else {
        out_lines = lines;
        if !out_lines.is_empty() && !out_lines.last().is_some_and(|l| l.trim().is_empty()) {
            out_lines.push(String::new());
        }
        out_lines.push("[versions]".into());
        out_lines.push(format!("active = {value}"));
    }
    let mut out = out_lines.join("\n");
    if !out.is_empty() {
        out.push('\n');
    }
    finish(out, tag)
}

/// A `[[servers]]` block after the last one there is, or a first one at the end of the file.
/// A name the file already has is refused.
pub fn add_server(text: &str, name: &str, address: &str) -> Result<String, String> {
    if let Ok(config) = Config::parse(text) {
        if config.servers.iter().any(|s| s.name == name) {
            return Err(format!("{name:?} is already in club.toml; remove it first"));
        }
    }
    let mut lines: Vec<String> = text.lines().map(str::to_string).collect();
    // The block joins the last [[servers]] there is, after its last key.
    let insert_at = lines
        .iter()
        .rposition(|line| line.trim() == "[[servers]]")
        .map(|header| {
            let mut end = header + 1;
            while end < lines.len() && !lines[end].trim().starts_with('[') {
                end += 1;
            }
            end
        })
        .unwrap_or(lines.len());
    let mut block = vec![
        String::new(),
        "[[servers]]".into(),
        format!("name = {}", toml_value(name)),
        format!("address = {}", toml_value(address)),
    ];
    if lines
        .get(insert_at.wrapping_sub(1))
        .is_some_and(|before| before.trim().is_empty())
    {
        block.remove(0);
    }
    let mut out_lines: Vec<String> = lines.drain(..insert_at).collect();
    out_lines.extend(block);
    out_lines.extend(lines);
    let mut out = out_lines.join("\n");
    if !out.is_empty() {
        out.push('\n');
    }
    check(&out)?;
    Ok(out)
}

/// The `[[servers]]` block whose `name` is `name`, gone with its header and its keys.
pub fn remove_server(text: &str, name: &str) -> Result<String, String> {
    let lines: Vec<&str> = text.lines().collect();
    let mut chosen: Option<(usize, usize)> = None;
    let mut i = 0;
    while i < lines.len() {
        if lines[i].trim() == "[[servers]]" {
            let start = i;
            i += 1;
            while i < lines.len() && !lines[i].trim().starts_with('[') {
                i += 1;
            }
            let is_it = lines[start..i]
                .iter()
                .any(|line| matches!(key_of(line), Some("name")) && value_of(line) == Some(name));
            if is_it {
                chosen = Some((start, i));
            }
        } else {
            i += 1;
        }
    }
    let Some((start, end)) = chosen else {
        return Err(format!("{name:?} is not in club.toml"));
    };
    let mut kept: Vec<&str> = lines[..start].to_vec();
    kept.extend(lines[end..].iter().copied());
    // The blank lines the block leaves behind collapse into one.
    let mut out_lines: Vec<&str> = Vec::new();
    for line in kept {
        if line.trim().is_empty()
            && out_lines
                .last()
                .is_some_and(|previous| previous.trim().is_empty())
        {
            continue;
        }
        out_lines.push(line);
    }
    while out_lines.last().is_some_and(|line| line.trim().is_empty()) {
        out_lines.pop();
    }
    let mut out = out_lines.join("\n");
    if !out.is_empty() {
        out.push('\n');
    }
    check(&out)?;
    Ok(out)
}

/// The file still parses, and the build it names took.
fn finish(text: String, tag: &str) -> Result<String, String> {
    check(&text)?;
    if Config::parse(&text).unwrap().versions.active.as_deref() != Some(tag) {
        return Err("the edit did not take".into());
    }
    Ok(text)
}

fn check(text: &str) -> Result<(), String> {
    Config::parse(text).map(|_| ()).map_err(|e| e.to_string())
}

/// A line's key (`active = "x" # c` → `active`), None for anything else.
fn key_of(line: &str) -> Option<&str> {
    let trimmed = line.trim();
    if trimmed.starts_with('#') {
        return None;
    }
    let (key, _) = trimmed.split_once('=')?;
    let key = key.trim();
    (!key.is_empty()).then_some(key)
}

/// A line's value as a plain word: what follows `=` without spaces, quotes or a comment.
fn value_of(line: &str) -> Option<&str> {
    let (_, value) = line.split_once('=')?;
    let value = value.trim();
    let value = value.split('#').next().unwrap_or(value).trim();
    Some(value.trim_matches('"'))
}

/// `line` with its value replaced by `value`, keeping the indent before the key and the gap and
/// comment after the value.
fn replace_value(line: &str, value: &str) -> String {
    let indent = " ".repeat(line.len() - line.trim_start().len());
    let (before, from_hash) = match line.find('#') {
        Some(at) if line[..at].contains('=') => (&line[..at], &line[at..]),
        _ => (line, ""),
    };
    // `key = "old"     ` → the key, the new value, and the whitespace the comment sat behind.
    let value_end = before.trim_end();
    let gap = &before[value_end.len()..];
    let key = value_end.split('=').next().unwrap_or_default().trim();
    format!("{indent}{key} = {value}{gap}{from_hash}")
}

/// `text` as a TOML basic string.
fn toml_value(plain: &str) -> String {
    let mut out = String::from("\"");
    for c in plain.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            _ => out.push(c),
        }
    }
    out.push('"');
    out
}

/// The file's lines, each with its line ending kept.
fn split_keep(text: &str) -> Vec<String> {
    let mut out = text
        .split_inclusive('\n')
        .map(str::to_string)
        .collect::<Vec<_>>();
    if out.is_empty() {
        out.push(String::new());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const FILE: &str = "[club]\nname = \"Club Minecraft\"          # the club's own name\n\n[[servers]]                      # the servers the club offers, in this order\nname = \"Serveur du club\"\naddress = \"mc-club.example.edu:25565\"\n\n[[servers]]\nname = \"Salle info\"\naddress = \"192.168.10.5\"\n\n[game]                           # written into a student's options\nrender_distance = 8\n\n[versions]\nactive = \"26.3-20261122\"         # the build the club plays\n";

    /// `set-version` edits the line, keeps every comment and every other key, and takes.
    #[test]
    fn set_version_edits_the_line_and_keeps_the_file() {
        let edited = set_active(FILE, "club-10-0a1b2c4").unwrap();
        assert!(
            edited.contains("active = \"club-10-0a1b2c4\"         # the build the club plays"),
            "{edited}"
        );
        assert!(!edited.contains("26.3-20261122"));
        assert_eq!(edited.lines().count(), FILE.lines().count());
        assert!(edited.contains("name = \"Club Minecraft\"          # the club's own name"));
        assert_eq!(
            Config::parse(&edited).unwrap().versions.active.as_deref(),
            Some("club-10-0a1b2c4")
        );
        assert_eq!(Config::parse(&edited).unwrap().servers.len(), 2);
    }

    /// No `active` line: the section takes one. No `[versions]` at all: one is added at the end.
    #[test]
    fn set_version_adds_what_is_missing() {
        let no_line = FILE.replace(
            "active = \"26.3-20261122\"         # the build the club plays\n",
            "",
        );
        let edited = set_active(&no_line, "v2").unwrap();
        assert!(edited.contains("[versions]\nactive = \"v2\"\n"), "{edited}");
        let no_section = "[club]\nname = \"Club Minecraft\"\n";
        let edited = set_active(no_section, "v3").unwrap();
        assert!(
            edited.ends_with("\n[versions]\nactive = \"v3\"\n"),
            "{edited}"
        );
        assert_eq!(Config::parse(&edited).unwrap().club.name, "Club Minecraft");
        // A file with nothing in it at all.
        let edited = set_active("", "v4").unwrap();
        assert_eq!(edited, "[versions]\nactive = \"v4\"\n");
    }

    #[test]
    fn a_server_is_added_after_the_last_block_and_validated() {
        let added = add_server(FILE, "CDI", "10.0.0.6:25566").unwrap();
        assert!(
            added.contains("[[servers]]\nname = \"CDI\"\naddress = \"10.0.0.6:25566\"\n"),
            "{added}"
        );
        assert_eq!(Config::parse(&added).unwrap().servers.len(), 3);
        assert_eq!(
            Config::parse(&added).unwrap().servers[2].address,
            "10.0.0.6:25566"
        );
        // The first block of a file goes at its end, after a blank line.
        let added = add_server("[club]\nname = \"x\"\n", "Premier", "a.example:1").unwrap();
        assert!(added.ends_with("\n[[servers]]\nname = \"Premier\"\naddress = \"a.example:1\"\n"));
        // A name that already exists is refused, and an address a TOML line cannot hold (a
        // newline inside it) is too: both leave the text alone.
        assert!(add_server(FILE, "Salle info", "b:2").is_err());
        assert!(add_server("[club]\nname = \"x\"\n", "Cassé", "a\nb").is_err());
        // A name with quotes in it is written as a TOML string.
        let added = add_server("[club]\nname = \"x\"\n", "L'été", "c:3").unwrap();
        assert!(added.contains("name = \"L'été\""), "{added}");
    }

    #[test]
    fn a_server_is_removed_with_its_block_and_the_rest_stays() {
        let removed = remove_server(FILE, "Salle info").unwrap();
        assert_eq!(Config::parse(&removed).unwrap().servers.len(), 1);
        assert_eq!(
            Config::parse(&removed).unwrap().servers[0].name,
            "Serveur du club"
        );
        assert!(removed.contains("[game]"));
        assert!(removed.contains("render_distance = 8"));
        assert!(!removed.contains("192.168.10.5"));
        assert!(remove_server(FILE, "Inconnu").is_err());
        // The only block gone leaves a file that still parses.
        let one = "[[servers]]\nname = \"A\"\naddress = \"a:1\"\n";
        let removed = remove_server(one, "A").unwrap();
        assert_eq!(removed, "");
        assert!(Config::parse(&removed).is_ok());
    }
}
