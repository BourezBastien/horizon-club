//! Who plays: the Windows session's login, folded to a Minecraft name. The school edition has no
//! accounts; the name is the only thing that tells one student's game from another's.

/// Why a Windows session cannot play.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IdentityError {
    /// The login folds to fewer than three characters, too short for a Minecraft name.
    TooShort,
    /// The session is one several students share (`shared_accounts` of the club's `club.toml`):
    /// they would all play under the one name.
    Shared,
}

impl IdentityError {
    /// What the window says of it.
    pub fn message(self) -> String {
        crate::i18n::tr(match self {
            IdentityError::TooShort => {
                "This Windows session's name cannot make a player name, so the game can't start."
            }
            IdentityError::Shared => {
                "This Windows session is shared by several students, so the game can't start. Ask \
                 the club's supervisor for a session of your own."
            }
        })
        .to_string()
    }
}

impl std::fmt::Display for IdentityError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IdentityError::TooShort => write!(f, "the login folds to fewer than three characters"),
            IdentityError::Shared => write!(f, "the login is one several students share"),
        }
    }
}

/// The Windows session's login: `USERNAME` (else `USER`), keeping only the part after the last
/// `\`, as `USERDOMAIN\USERNAME` spells it.
pub fn login() -> Option<String> {
    let who = std::env::var("USERNAME")
        .or_else(|_| std::env::var("USER"))
        .ok()?;
    let who = after_last_backslash(&who).to_owned();
    (!who.is_empty()).then_some(who)
}

/// Whether the Windows session `login` is one of the `shared` ones, whatever the case either is
/// written in (`eleve` is `Élève`).
pub fn is_shared(login: &str, shared: &[String]) -> bool {
    let login = login.trim().to_lowercase();
    shared
        .iter()
        .any(|account| account.trim().to_lowercase() == login)
}

/// The part of `who` after the last `\`, or all of it when there is none.
fn after_last_backslash(who: &str) -> &str {
    who.rsplit('\\').next().unwrap_or_default()
}

/// One lower-cased character folded the way a Minecraft name is written: itself when it is a
/// letter, a digit or `_`; a Latin-1 letter with its accent dropped; anything else dropped.
fn fold_char(c: char) -> Option<char> {
    match c {
        'a'..='z' | '0'..='9' | '_' => Some(c),
        // The letters of Latin-1 that carry only an accent (à-å, ç, è-ë, ì-ï, ñ, ò-ö, ù-ü, ý, ÿ):
        // the letter without it. Other letters (æ, ø, þ, ß) and everything beyond Latin-1 are
        // dropped, as any character that is not a letter, a digit or `_` is.
        '\u{e0}'..='\u{e5}' => Some('a'),
        '\u{e7}' => Some('c'),
        '\u{e8}'..='\u{eb}' => Some('e'),
        '\u{ec}'..='\u{ef}' => Some('i'),
        '\u{f1}' => Some('n'),
        '\u{f2}'..='\u{f6}' => Some('o'),
        '\u{f9}'..='\u{fc}' => Some('u'),
        '\u{fd}' | '\u{ff}' => Some('y'),
        _ => None,
    }
}

/// The login folded to what a Minecraft name is made of: accents dropped, lower-cased, and only
/// letters, digits and `_` kept (`j.dupont` and `Jérôme.Dùpont` both fold to `jeromedupont`).
fn fold(login: &str) -> String {
    login
        .chars()
        .flat_map(char::to_lowercase)
        .filter_map(fold_char)
        .collect()
}

/// FNV-1a, 32 bits, over the folded login: a stable hash, so the same Windows login gives the
/// same name on every PC of the club.
fn fnv1a(text: &str) -> u32 {
    let mut hash = 2166136261u32;
    for byte in text.as_bytes() {
        hash ^= u32::from(*byte);
        hash = hash.wrapping_mul(16777619);
    }
    hash
}

/// The Minecraft name of a Windows login: the folded login with its first letter capitalised
/// (`j.dupont` becomes `Jdupont`). Past 16 characters the name is the first 12, `_`, and three
/// hexadecimal characters of the folded login's hash (its low 12 bits). A login that folds to
/// fewer than three characters is an error.
pub fn player_name(login: &str) -> Result<String, IdentityError> {
    let folded = fold(login);
    if folded.chars().count() < 3 {
        return Err(IdentityError::TooShort);
    }
    if folded.chars().count() > 16 {
        let stem: String = folded.chars().take(12).collect();
        return Ok(format!("{}_{:03x}", stem, fnv1a(&folded) & 0xfff));
    }
    let mut chars = folded.chars();
    // The first character capitalised, a digit or `_` left as it is.
    let first = chars.next().unwrap().to_uppercase().to_string();
    Ok(first + chars.as_str())
}

/// Who plays on this Windows session: their Minecraft name, unless `shared` lists the session's
/// login among those several students share, which play under no name at all.
pub fn player(shared: &[String]) -> Result<String, IdentityError> {
    let login = login().ok_or(IdentityError::TooShort)?;
    if is_shared(&login, shared) {
        return Err(IdentityError::Shared);
    }
    player_name(&login)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_login_folds_to_a_minecraft_name() {
        assert_eq!(player_name("j.dupont").unwrap(), "Jdupont");
        assert_eq!(player_name("Jérôme.Dùpont").unwrap(), "Jeromedupont");
        // Letters, digits and `_` only; exactly 16 characters is still the folded name itself.
        assert_eq!(player_name("Dupont_Jean-Luc2").unwrap(), "Dupont_jeanluc2");
        // The part after the last `\` is the login.
        assert_eq!(after_last_backslash(r"CLUB\j.dupont"), "j.dupont");
        assert_eq!(after_last_backslash("j.dupont"), "j.dupont");
    }

    #[test]
    fn a_login_that_folds_to_two_characters_is_an_error() {
        assert_eq!(player_name("É4"), Err(IdentityError::TooShort));
        assert_eq!(player_name(".."), Err(IdentityError::TooShort));
        assert_eq!(player_name(""), Err(IdentityError::TooShort));
        assert!(player_name("E4_").is_ok());
    }

    #[test]
    fn a_shared_session_is_found_whatever_its_case() {
        let shared = |names: &[&str]| names.iter().map(|n| n.to_string()).collect::<Vec<_>>();
        let listed = shared(&["eleve", " Eleve-Test "]);
        assert!(is_shared("eleve", &listed));
        assert!(is_shared("ELEVE", &listed));
        assert!(is_shared("eleve-test", &listed));
        assert!(
            !is_shared("eleves", &listed),
            "a longer login is another session"
        );
        assert!(!is_shared("dupont", &listed));
        // Nothing listed: no session is shared.
        assert!(!is_shared("eleve", &Vec::new()));
        assert!(is_shared("Élève", &shared(&["éLèVe"])));
    }

    /// Every session that cannot play has its message, said in the launcher's language.
    #[test]
    fn a_session_that_cannot_play_says_why() {
        let said = crate::i18n::tr_in(
            crate::i18n::current(),
            "This Windows session is shared by several students, so the game can't start. Ask the \
             club's supervisor for a session of your own.",
        );
        assert_eq!(IdentityError::Shared.message(), said);
        assert!(IdentityError::TooShort.message().contains("player name"));
    }

    #[test]
    fn a_long_login_is_shortened_by_a_stable_hash_of_its_folding() {
        let twenty = "abcdefghijklmnopqrst";
        let name = player_name(twenty).unwrap();
        // The first 12, `_`, three hexadecimal characters: 16 at most.
        assert_eq!(name.chars().count(), 16);
        assert_eq!(&name[..12], "abcdefghijkl");
        assert_eq!(name.as_bytes()[12], b'_');
        let digits = &name[13..];
        assert_eq!(digits.len(), 3);
        assert!(digits
            .bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase()));
        // The same login in any case gives the same name.
        assert_eq!(player_name(&twenty.to_uppercase()).unwrap(), name);
        assert_eq!(player_name("AbCdEfGhIjKlMnOpQrSt").unwrap(), name);
        // Two long logins that share their first 13 characters stay apart.
        assert_ne!(
            player_name("aaaaaaaaaaaaaXXXX").unwrap(),
            player_name("aaaaaaaaaaaaaYYYY").unwrap()
        );
    }

    /// The published FNV-1a 32-bit test vectors (draft-eastlake-fnv): the hash a long login's
    /// name stands on is the standard one, not a private mixing.
    #[test]
    fn the_hash_is_fnv1a_32() {
        assert_eq!(fnv1a(""), 0x811c9dc5);
        assert_eq!(fnv1a("a"), 0xe40c292c);
        assert_eq!(fnv1a("foobar"), 0xbf9cf968);
    }
}
