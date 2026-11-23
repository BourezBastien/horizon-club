//! The game's log as the console shows it: how its search matches a line.

/// Whether a console line matches the search: every word, in any case.
pub fn matches(line: &str, search: &str) -> bool {
    let line = line.to_lowercase();
    search
        .split_whitespace()
        .all(|w| line.contains(&w.to_lowercase()))
}

#[cfg(test)]
#[path = "game_log/tests.rs"]
mod tests;
