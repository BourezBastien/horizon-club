//! What `club-admin` says in its window, in colours: at a glance, what is under way, what is
//! done, what is mere detail, what is to read before going on, and what went wrong. A line's
//! symbol and colour follow its [`Level`]; a command's last word is a [`banner`] across the
//! window. Written to a pipe or a file (a script, a test), the same lines come plain, and so
//! when `NO_COLOR` is set; `CLICOLOR_FORCE` colours them anyway.

use std::cell::Cell;
#[cfg(not(windows))]
use std::io::IsTerminal;
use std::io::Write;

thread_local! {
    /// Whether the lines are coloured ([`start`]).
    static ON: Cell<bool> = const { Cell::new(false) };
    /// The window's width in columns, 0 when it is not known: a band is not padded then.
    static WIDTH: Cell<usize> = const { Cell::new(0) };
}

/// What a line tells, which its symbol and colour say.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Level {
    /// The line a command opens with: where it works.
    Title,
    /// Under way.
    Step,
    /// In place, checked, written.
    Done,
    /// Paths and counts, to read when something went wrong and to pass over otherwise.
    Detail,
    /// To know: what is there, what is left to do.
    Info,
    /// To read before going on.
    Warn,
    /// What stopped the command.
    Error,
}

/// A command's last word: a band across the window in the colour of how it went.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Banner {
    /// All is in place.
    Good,
    /// In place, with something to do before it works.
    Attention,
    /// It could not finish.
    Bad,
}

/// Whether the lines are coloured from now on: when the output is a window that takes colours.
/// Called once, as the tool starts.
pub(crate) fn start() {
    let forced = std::env::var_os("CLICOLOR_FORCE").is_some_and(|set| !set.is_empty());
    #[cfg(windows)]
    let (window, width) = match colours_allowed() {
        Some(width) => (true, width),
        None => (false, 0),
    };
    #[cfg(not(windows))]
    let (window, width) = (std::io::stdout().is_terminal(), 0);
    let asked = std::env::var_os("NO_COLOR").is_none_or(|set| set.is_empty());
    ON.set(forced || (window && asked));
    WIDTH.set(width);
}

/// Lets the window take colour codes, which it would print as characters, on both of the tool's
/// outputs. Whether it could, and the window's width in columns.
#[cfg(windows)]
fn colours_allowed() -> Option<usize> {
    use windows_sys::Win32::System::Console::{
        GetConsoleMode, GetConsoleScreenBufferInfo, GetStdHandle, SetConsoleMode,
        CONSOLE_SCREEN_BUFFER_INFO, ENABLE_VIRTUAL_TERMINAL_PROCESSING, STD_ERROR_HANDLE,
        STD_OUTPUT_HANDLE,
    };
    // SAFETY: the calls read and change the mode of this process's own standard handles; the
    // other pointer arguments are modes and a screen's facts that this code initialised.
    unsafe {
        let out = GetStdHandle(STD_OUTPUT_HANDLE);
        let mut mode = 0;
        if GetConsoleMode(out, &mut mode) == 0
            || SetConsoleMode(out, mode | ENABLE_VIRTUAL_TERMINAL_PROCESSING) == 0
        {
            return None;
        }
        let err = GetStdHandle(STD_ERROR_HANDLE);
        if GetConsoleMode(err, &mut mode) != 0 {
            SetConsoleMode(err, mode | ENABLE_VIRTUAL_TERMINAL_PROCESSING);
        }
        let mut screen: CONSOLE_SCREEN_BUFFER_INFO = std::mem::zeroed();
        (GetConsoleScreenBufferInfo(out, &mut screen) != 0).then(|| screen.dwSize.X.max(0) as usize)
    }
}

/// `println!` with a [`Level`].
macro_rules! say {
    ($level:expr, $($arg:tt)*) => {
        crate::out::line($level, &format!($($arg)*))
    };
}

/// An empty line.
pub(crate) fn blank() {
    let _ = writeln!(std::io::stdout());
}

/// One line, saying what it tells ([`Level`]). An error goes to the error output.
pub(crate) fn line(level: Level, text: &str) {
    let coloured = ON.get();
    for part in text.split('\n') {
        let shown = if coloured {
            paint(level, part)
        } else {
            words(level, part)
        };
        if level == Level::Error {
            let _ = writeln!(std::io::stderr(), "{shown}");
        } else {
            let _ = writeln!(std::io::stdout(), "{shown}");
        }
    }
}

/// [`banner`], written.
pub(crate) fn banner(how: Banner, lines: &[String]) {
    let _ = writeln!(
        std::io::stdout(),
        "{}",
        band(how, lines, WIDTH.get(), ON.get())
    );
}

/// A line with its level's symbol, as it reads without colour.
fn words(level: Level, text: &str) -> String {
    let (indent, symbol) = match level {
        Level::Title | Level::Info => ("", ""),
        Level::Step => ("  ", "→ "),
        Level::Done => ("  ", "✓ "),
        Level::Detail => ("    ", ""),
        Level::Warn => ("  ", "! "),
        Level::Error => ("  ", "✗ "),
    };
    format!("{indent}{symbol}{text}")
}

/// The colour code of what is coloured at this level: the whole line, or its symbol alone.
fn codes(level: Level) -> (&'static str, bool) {
    match level {
        Level::Title => ("1;96", true),
        Level::Step => ("96", false),
        Level::Done => ("92", false),
        Level::Detail => ("90", true),
        Level::Info => ("", false),
        Level::Warn => ("93", true),
        Level::Error => ("91", true),
    }
}

/// [`words`] in its level's colour.
fn paint(level: Level, text: &str) -> String {
    let (code, whole) = codes(level);
    if code.is_empty() {
        return words(level, text);
    }
    if whole {
        return format!("\x1b[{code}m{}\x1b[0m", words(level, text));
    }
    let symbol = level_symbol(level);
    format!("  \x1b[{code}m{symbol}\x1b[0m{text}")
}

/// The symbol of the levels whose line keeps its own colour and has a symbol alone coloured.
fn level_symbol(level: Level) -> &'static str {
    match level {
        Level::Step => "→ ",
        Level::Done => "✓ ",
        _ => "",
    }
}

/// The band: the first line with the banner's symbol, the others under it, each to the window's
/// last column but one (a full one makes some windows skip a line). Plain, the same lines.
fn band(how: Banner, lines: &[String], width: usize, coloured: bool) -> String {
    let (symbol, ground, ink) = match how {
        Banner::Good => ("✓", "42", "30"),
        Banner::Attention => ("!", "43", "30"),
        Banner::Bad => ("✗", "41", "97"),
    };
    let mut band = String::new();
    for (at, line) in lines.iter().enumerate() {
        if at > 0 {
            band.push('\n');
        }
        let line = if at == 0 {
            format!(" {symbol} {line}")
        } else {
            format!("   {line}")
        };
        if !coloured {
            band.push_str(&line);
            continue;
        }
        let room = width.saturating_sub(1).saturating_sub(line.chars().count());
        band.push_str(&format!(
            "\x1b[{ground}m\x1b[{ink}m{line}{}\x1b[0m",
            " ".repeat(room)
        ));
    }
    band
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A line says its level by its symbol, and stays plain when nothing colours it.
    #[test]
    fn a_line_tells_its_level_by_its_symbol_and_is_plain_without_a_window() {
        assert_eq!(words(Level::Title, "where"), "where");
        assert_eq!(words(Level::Info, "what"), "what");
        assert_eq!(words(Level::Step, "under way"), "  → under way");
        assert_eq!(words(Level::Done, "in place"), "  ✓ in place");
        assert_eq!(words(Level::Detail, "a path"), "    a path");
        assert_eq!(words(Level::Warn, "to read"), "  ! to read");
        assert_eq!(words(Level::Error, "stopped"), "  ✗ stopped");
        for level in [
            Level::Title,
            Level::Step,
            Level::Done,
            Level::Detail,
            Level::Info,
            Level::Warn,
            Level::Error,
        ] {
            assert!(!words(level, "x").contains('\x1b'), "{level:?}");
        }
    }

    /// Coloured, a step's arrow is cyan, what is done is green, a warning is yellow and an error
    /// red; each code is closed, so nothing runs into the next line.
    #[test]
    fn a_coloured_line_wears_its_level_s_colour_and_closes_it() {
        assert_eq!(paint(Level::Title, "where"), "\x1b[1;96mwhere\x1b[0m");
        assert_eq!(
            paint(Level::Step, "under way"),
            "  \x1b[96m→ \x1b[0munder way"
        );
        assert_eq!(
            paint(Level::Done, "in place"),
            "  \x1b[92m✓ \x1b[0min place"
        );
        assert_eq!(paint(Level::Detail, "a path"), "\x1b[90m    a path\x1b[0m");
        assert_eq!(paint(Level::Warn, "to read"), "\x1b[93m  ! to read\x1b[0m");
        assert_eq!(paint(Level::Error, "stopped"), "\x1b[91m  ✗ stopped\x1b[0m");
        assert_eq!(paint(Level::Info, "what"), "what");
    }

    /// The band's first line carries the symbol, the others sit under its text, each to the
    /// window's last column but one; a line longer than the window is left as it is.
    #[test]
    fn a_band_spans_the_window_less_its_last_column() {
        let lines = [
            "All is in place.".to_string(),
            "Make a shortcut.".to_string(),
        ];
        // Each line reads 19 columns; the window has 29 to fill.
        let pad = " ".repeat(10);
        assert_eq!(
            band(Banner::Good, &lines, 30, true),
            format!(
                "\x1b[42m\x1b[30m ✓ All is in place.{pad}\x1b[0m\n\
                 \x1b[42m\x1b[30m   Make a shortcut.{pad}\x1b[0m"
            )
        );
        assert_eq!(
            band(Banner::Attention, &lines, 0, true),
            "\x1b[43m\x1b[30m ! All is in place.\x1b[0m\n\x1b[43m\x1b[30m   Make a shortcut.\x1b[0m"
        );
        assert_eq!(
            band(Banner::Bad, &lines, 12, false),
            " ✗ All is in place.\n   Make a shortcut."
        );
    }
}
