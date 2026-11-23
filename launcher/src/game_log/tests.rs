use super::*;

#[test]
fn search_matches_every_word_in_any_case() {
    assert!(matches(
        "[12:00:01] [Render thread/WARN]: Missing texture",
        "warn texture"
    ));
    assert!(!matches("[12:00:01] [Render thread/INFO]: Loaded", "warn"));
    assert!(matches("anything", "  "));
}
