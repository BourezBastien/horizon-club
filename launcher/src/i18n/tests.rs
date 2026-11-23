use super::*;
use std::collections::BTreeSet;
use std::path::Path;

/// Every Rust file of the launcher's sources.
fn sources(dir: &Path, out: &mut Vec<String>) {
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            sources(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(std::fs::read_to_string(path).unwrap());
        }
    }
}

/// Read Rust tokens rather than quote positions: whitespace, comments, escaped and raw strings
/// must not hide a call or turn a quoted example into one. Recurse into macro bodies too.
fn strings_in(
    tokens: proc_macro2::TokenStream,
    asked: &mut BTreeSet<String>,
    all: &mut BTreeSet<String>,
) {
    use proc_macro2::{Delimiter, TokenTree};
    let mut tokens = tokens.into_iter().peekable();
    while let Some(token) = tokens.next() {
        match token {
            // `tr` and `trf` take their key first. A row's title (`setting_row`…) and a
            // field's placeholder reach `tr` from inside the widget: the first string
            // literal of the call is the key there (a field's name comes before it).
            TokenTree::Ident(name)
                if [
                    "tr",
                    "trf",
                    "setting_row",
                    "toggle_row",
                    "list_row",
                    "field",
                ]
                .iter()
                .any(|f| name == *f) =>
            {
                if let Some(TokenTree::Group(args)) = tokens.peek() {
                    if args.delimiter() == Delimiter::Parenthesis {
                        let stream = args.stream();
                        // The launcher's text fields are named by the `Field` enum; another
                        // `field(…)` reads data, and its string is not shown.
                        let text_field = name != "field"
                            || matches!(
                                stream.clone().into_iter().next(),
                                Some(TokenTree::Ident(first)) if first == "Field"
                            );
                        if text_field {
                            let first = stream
                                .into_iter()
                                .find(|t| matches!(t, TokenTree::Literal(_)));
                            if let Some(TokenTree::Literal(value)) = first {
                                if let Ok(value) = syn::parse_str::<syn::LitStr>(&value.to_string())
                                {
                                    asked.insert(value.value());
                                }
                            }
                        }
                    }
                }
            }
            TokenTree::Group(group) => strings_in(group.stream(), asked, all),
            TokenTree::Literal(value) => {
                if let Ok(value) = syn::parse_str::<syn::LitStr>(&value.to_string()) {
                    all.insert(value.value());
                }
            }
            _ => {}
        }
    }
}

/// The English strings requested directly, plus every literal (some reach `tr` via a widget or
/// constant). The tables' shared key set below covers those indirect translations.
fn strings() -> (BTreeSet<String>, BTreeSet<String>) {
    let mut files = Vec::new();
    sources(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("src"),
        &mut files,
    );
    let (mut asked, mut all) = (BTreeSet::new(), BTreeSet::new());
    for file in &files {
        strings_in(
            file.parse().expect("valid Rust tokens"),
            &mut asked,
            &mut all,
        );
    }
    (asked, all)
}

#[test]
fn the_scan_finds_multiline_calls_without_reading_comments_or_quoted_code() {
    let code = r###"
        tr
        (
            "multiline"
        );
        trf(/* before the string */ "hello {name}", &[("name", "player")]);
        some_macro!(crate::i18n::tr(r#"raw "quotes""#));
        tr("escaped \"quotes\" and \u{4e2d}\nnext");
        tr("continued \
            line");
        // tr("line comment");
        /* tr("block comment"); */
        let example = r#"tr("quoted code")"#;
        other_tr("not a translation");
        trf_other("not a translation either");
        let indirect = "widget label";
        tr(indirect);
        self.setting_row("Row title", about, control);
        self.toggle_row("Switch title", about, toggle);
        self.field(Field::Name, "Placeholder text", 400., cx);
        data.field("GameType");
        crate::glass_ui::field(focused, invalid);
    "###;
    let (mut asked, mut all) = (BTreeSet::new(), BTreeSet::new());
    strings_in(code.parse().unwrap(), &mut asked, &mut all);
    assert_eq!(
        asked,
        [
            "multiline",
            "hello {name}",
            "raw \"quotes\"",
            "escaped \"quotes\" and 中\nnext",
            "continued line",
            "Row title",
            "Switch title",
            "Placeholder text",
        ]
        .into_iter()
        .map(str::to_string)
        .collect()
    );
    assert!(all.contains("widget label"));
    assert!(!asked.contains("GameType"));
    assert!(asked.is_subset(&all));
    assert!(!all.contains("line comment"));
    assert!(!all.contains("block comment"));
}

fn holes(s: &str) -> BTreeSet<&str> {
    s.split('{')
        .skip(1)
        .filter_map(|part| part.split_once('}').map(|(name, _)| name))
        .collect()
}

#[test]
fn translations_are_valid_and_coverage_is_reported() {
    let (mut asked, all) = strings();
    assert!(asked.contains("Language"), "the scan finds tr(\"…\")");
    let tables: Vec<_> = Language::ALL
        .into_iter()
        .filter(|language| *language != Language::English)
        .map(|language| {
            let table: HashMap<String, String> = serde_json::from_str(language.source())
                .unwrap_or_else(|e| panic!("lang/{}.json: {e}", language.code()));
            (language, table)
        })
        .collect();
    // Diverges from upstream: tables are sparse, using the existing English-source fallback.
    // Missing and stale entries are coverage information, not failures that force every new
    // label to edit every language. Keep existing translations; never add dummy English entries.
    // A live label passed via a widget/constant is also known once any table translates it.
    for (_, table) in &tables {
        asked.extend(table.keys().filter(|key| all.contains(*key)).cloned());
    }
    // Show the detailed lists with HORIZON_LAUNCHER_I18N_DETAILS=1 cargo test
    // --manifest-path launcher/Cargo.toml --lib i18n::tests::translations_are_valid_and_coverage_is_reported
    use std::fmt::Write as _;
    use std::io::Write as _;
    let show_details = std::env::var("HORIZON_LAUNCHER_I18N_DETAILS").as_deref() == Ok("1");
    let (mut missing_count, mut stale_count) = (0, 0);
    let mut details = String::new();
    for (language, table) in &tables {
        let missing: Vec<_> = asked.iter().filter(|s| !table.contains_key(*s)).collect();
        let stale: BTreeSet<_> = table.keys().filter(|s| !all.contains(*s)).collect();
        missing_count += missing.len();
        stale_count += stale.len();
        if show_details && (!missing.is_empty() || !stale.is_empty()) {
            writeln!(
                details,
                "lang/{}.json missing: {missing:#?}; stale: {stale:#?}",
                language.code()
            )
            .unwrap();
        }
        for (en, said) in table {
            assert_eq!(holes(en), holes(said), "{}: {en:?}", language.code());
            assert!(
                !said.trim().is_empty(),
                "{}: {en:?} is empty",
                language.code()
            );
        }
    }
    // Write directly so the single summary is visible even when libtest captures passing tests.
    let mut out = std::io::stdout().lock();
    writeln!(out, "Launcher locale coverage: {} languages, {missing_count} missing entries, {stale_count} stale entries (HORIZON_LAUNCHER_I18N_DETAILS=1 for lists).", tables.len()).unwrap();
    if show_details {
        write!(out, "{details}").unwrap();
    }
}

/// A language is saved in `settings.json` under its code, and read back from it: a student's
/// pick outlives the launcher's version that wrote it.
#[test]
fn a_language_is_saved_under_its_code() {
    assert_eq!(serde_json::to_string(&Language::French).unwrap(), "\"fr\"");
    assert_eq!(serde_json::to_string(&Language::English).unwrap(), "\"en\"");
    assert_eq!(
        serde_json::from_str::<Language>("\"fr\"").unwrap(),
        Language::French
    );
    assert_eq!(
        serde_json::from_str::<Language>("\"en\"").unwrap(),
        Language::English
    );
    // A code no launcher has is not a language.
    assert!(serde_json::from_str::<Language>("\"french\"").is_err());
    assert!(serde_json::from_str::<Language>("\"de\"").is_err());
}

#[test]
fn a_missing_string_shows_in_english() {
    assert_eq!(tr_in(Language::French, "Settings"), "Paramètres");
    assert_eq!(tr_in(Language::English, "Settings"), "Settings");
    assert_eq!(tr_in(Language::French, "No such string"), "No such string");
}
