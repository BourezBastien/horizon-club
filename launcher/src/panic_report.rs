//! A panic of the launcher's own is written to a file, in its data folder's `crash-reports\`.
//! The window closes on it and nothing else says why: the launcher has no log of its own, and
//! Windows' own log holds nothing of a program that ends itself. The report says the version,
//! what was said and where, and the call stack, as the game's reports do for the game.

use std::path::{Path, PathBuf};

/// The folder of the reports, in the launcher's data folder.
pub const FOLDER: &str = "crash-reports";
/// The newest report is always the same file, to be found without looking.
pub const LATEST: &str = "launcher-latest.txt";

/// What a report says: the version, where the launcher failed, what it said, and its call stack.
pub fn report(info: &std::panic::PanicHookInfo<'_>, version: &str, backtrace: &str) -> String {
    let place = info.location().map_or_else(
        || "an unknown place".to_string(),
        |at| format!("{}:{}:{}", at.file(), at.line(), at.column()),
    );
    let thread = std::thread::current();
    text(
        version,
        thread.name().unwrap_or("unnamed"),
        &place,
        &info.to_string(),
        backtrace,
    )
}

/// [`report`], in pieces.
fn text(version: &str, thread: &str, place: &str, said: &str, backtrace: &str) -> String {
    format!(
        "Horizon Club launcher {version} stopped on an internal error.\n\nThread: {thread}\n\
         Where: {place}\nWhat: {said}\n\n{backtrace}\n"
    )
}

/// Writes `text` as the report of `now`: a file named for that time, and [`LATEST`]. Err: why
/// nothing could be written.
pub fn write(
    home: &Path,
    text: &str,
    now: chrono::DateTime<chrono::Local>,
) -> Result<PathBuf, String> {
    let folder = home.join(FOLDER);
    std::fs::create_dir_all(&folder).map_err(|e| format!("{}: {e}", folder.display()))?;
    let _ = std::fs::write(folder.join(LATEST), text);
    let path = folder.join(format!("launcher-{}.txt", now.format("%Y-%m-%d_%H-%M-%S")));
    std::fs::write(&path, text).map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(path)
}

/// From now on, every panic of the launcher's threads is reported ([`report`]) in the
/// launcher's data folder, and said on its error output.
pub fn install() {
    std::panic::set_hook(Box::new(|info| {
        let backtrace = std::backtrace::Backtrace::force_capture();
        let text = report(info, crate::backend::VERSION, &backtrace.to_string());
        eprintln!("{text}");
        let _ = write(&crate::club::data_dir(), &text, chrono::Local::now());
    }));
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    /// A report says the version, the thread, the file and line, and what was said; it is kept
    /// under its time and as the latest one.
    #[test]
    fn a_panic_is_reported_with_where_and_what_and_kept_as_the_latest() {
        let text = text(
            "0.4.0",
            "main",
            "src/servers_page.rs:120:16",
            "panicked at src/servers_page.rs:120:16:\nthe back button, 2 times",
            "stack: the call",
        );
        assert!(text.starts_with("Horizon Club launcher 0.4.0 stopped on an internal error."));
        for expected in [
            "Thread: main",
            "Where: src/servers_page.rs:120:16",
            "the back button, 2 times",
        ] {
            assert!(text.contains(expected), "{expected:?} missing of\n{text}");
        }
        assert!(text.ends_with("stack: the call\n"));

        // The data folder may hold nothing yet: the report's folder is made.
        let home = tempfile::tempdir().unwrap();
        let now = chrono::Local
            .with_ymd_and_hms(2026, 11, 24, 21, 0, 34)
            .unwrap();
        let path = write(home.path(), &text, now).unwrap();
        assert!(
            path.ends_with("crash-reports/launcher-2026-11-24_21-00-34.txt")
                || path.ends_with("crash-reports\\launcher-2026-11-24_21-00-34.txt"),
            "{}",
            path.display()
        );
        let latest = home.path().join(FOLDER).join(LATEST);
        assert_eq!(std::fs::read_to_string(&path).unwrap(), text);
        assert_eq!(std::fs::read_to_string(latest).unwrap(), text);
        // Two panics in the same second share a file: the last one is what it holds.
        let again = write(home.path(), "second", now).unwrap();
        assert_eq!(again, path);
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "second");
    }
}
