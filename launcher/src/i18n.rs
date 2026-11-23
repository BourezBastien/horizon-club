//! The launcher's languages: English (the strings in the code) and French, the two the edition
//! ships (docs/school-edition/launcher-target.md, section 11). A table per language
//! (`launcher/lang/<code>.json`) maps each English string to its translation; a string the table
//! lacks shows in English.
//!
//! The window's language is the club's (`club.toml`) until a student picks one in Settings
//! (`Settings::language`). Translations are Horizon's own, written for this launcher.
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::OnceLock;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Language {
    #[default]
    #[serde(rename = "en")]
    English,
    #[serde(rename = "fr")]
    French,
}

impl Language {
    pub const ALL: [Language; 2] = [Self::English, Self::French];

    /// Its code in `settings.json` and on the command line.
    pub fn code(self) -> &'static str {
        match self {
            Self::English => "en",
            Self::French => "fr",
        }
    }

    /// Its name in itself: what a student looks for in a list of languages.
    pub fn name(self) -> &'static str {
        match self {
            Self::English => "English",
            Self::French => "Français",
        }
    }

    /// The table, as shipped: English text to this language's.
    fn source(self) -> &'static str {
        match self {
            Self::English => "{}",
            Self::French => include_str!("../lang/fr.json"),
        }
    }
}

static CURRENT: AtomicU8 = AtomicU8::new(0);

/// Shows the launcher in `language` from now on.
pub fn set(language: Language) {
    let i = Language::ALL
        .iter()
        .position(|l| *l == language)
        .unwrap_or(0);
    CURRENT.store(i as u8, Ordering::Relaxed);
}

pub fn current() -> Language {
    Language::ALL[CURRENT.load(Ordering::Relaxed) as usize % Language::ALL.len()]
}

type Table = HashMap<String, &'static str>;

fn table(language: Language) -> &'static Table {
    static TABLES: OnceLock<Vec<Table>> = OnceLock::new();
    let tables = TABLES.get_or_init(|| {
        Language::ALL
            .iter()
            .map(|l| {
                serde_json::from_str::<HashMap<String, String>>(l.source())
                    .unwrap_or_default()
                    .into_iter()
                    .map(|(en, said)| (en, &*Box::leak(said.into_boxed_str())))
                    .collect()
            })
            .collect()
    });
    let i = Language::ALL
        .iter()
        .position(|l| *l == language)
        .unwrap_or(0);
    &tables[i]
}

/// `en` in `language`, or `en` itself where the table lacks it.
pub fn tr_in(language: Language, en: &str) -> &str {
    table(language).get(en).copied().unwrap_or(en)
}

/// `en` in the launcher's language.
pub fn tr(en: &str) -> &str {
    tr_in(current(), en)
}

/// `en` in the launcher's language with its `{name}` holes filled.
pub fn trf(en: &str, args: &[(&str, &str)]) -> String {
    let mut out = tr(en).to_string();
    for (name, value) in args {
        out = out.replace(&format!("{{{name}}}"), value);
    }
    out
}

#[cfg(test)]
#[path = "i18n/tests.rs"]
mod tests;
