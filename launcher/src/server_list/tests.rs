use super::*;
use crate::{config::Config, online};
use std::path::PathBuf;

fn scratch(name: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "horizon-launcher-servers-{name}-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();
    root
}

fn s(v: &str) -> Tag {
    Tag::String(v.into())
}

const PNG: [u8; 7] = [0x89, b'P', b'N', b'G', 1, 2, 3];

/// A servers.dat as the game writes it: one shown server with a picture and the fields the
/// launcher doesn't use, then one joined through Direct Connection.
fn vanilla_file(home: &Path) -> Vec<u8> {
    let icon = base64::engine::general_purpose::STANDARD.encode(PNG);
    let root = NamedTag {
        name: "".into(),
        tag: Tag::compound([(
            "servers",
            Tag::List(vec![
                Tag::compound([
                    ("name", s("Friends")),
                    ("ip", s("friends.example:25570")),
                    ("icon", s(&icon)),
                    ("acceptTextures", Tag::Byte(1)),
                    ("acceptedCodeOfConduct", Tag::Int(-12345)),
                    ("hidden", Tag::Byte(0)),
                ]),
                Tag::compound([
                    ("name", s("Minecraft Server")),
                    ("ip", s("direct.example")),
                    ("hidden", Tag::Byte(1)),
                ]),
            ]),
        )]),
    };
    let bytes = nbt::write_named(&root, Limits::default()).unwrap();
    fs::write(home.join(FILE), &bytes).unwrap();
    bytes
}

fn entries(home: &Path) -> Vec<Tag> {
    let bytes = fs::read(home.join(FILE)).unwrap();
    match nbt::read_named(&bytes, Limits::default())
        .unwrap()
        .0
        .tag
        .get("servers")
    {
        Some(Tag::List(list)) => list.clone(),
        other => panic!("no servers list: {other:?}"),
    }
}

fn name_and_address(servers: &[Server]) -> Vec<(&str, &str)> {
    servers
        .iter()
        .map(|s| (s.name.as_str(), s.ip.as_str()))
        .collect()
}

#[test]
fn the_list_is_the_clubs_servers_then_the_announced_ones() {
    let config = Config::parse(
        r#"
[[servers]]
name = "Serveur du club"
address = "mc-club.example.edu:25565"

[[servers]]
name = "Second"
address = "10.0.0.2"
"#,
    )
    .unwrap();
    let announced = [
        online::parse(
            r#"{"name": "Monde de Jdupont", "owner": "Jdupont", "host": "192.168.10.42", "port": 25565, "at": 1763800000}"#,
        )
        .unwrap(),
        online::parse(r#"{"name": "Monde de Alice", "owner": "Alice", "host": "192.168.10.7", "port": 25570, "at": 1763800000}"#)
            .unwrap(),
    ];
    assert_eq!(
        name_and_address(&club_servers(&config, &announced)),
        [
            ("Serveur du club", "mc-club.example.edu:25565"),
            ("Second", "10.0.0.2"),
            ("Monde de Jdupont", "192.168.10.42:25565"),
            ("Monde de Alice", "192.168.10.7:25570"),
        ],
        "the club's, in its file's order, then the announced ones as they were given"
    );
    // A bare club offers nothing: an empty list, not an error.
    assert!(club_servers(&Config::default(), &[]).is_empty());
}

#[test]
fn the_list_is_rewritten_with_what_the_club_offers_and_nothing_else() {
    let home = scratch("rewrite");
    let original = vanilla_file(home.as_path());
    let written = replace(
        home.as_path(),
        &[
            Server::new("Serveur du club", "mc-club.example.edu:25565"),
            Server::new("Monde de Jdupont", "192.168.10.42:25565"),
        ],
    )
    .unwrap();
    // The game's own backup of what was there, and no temporary file left.
    assert_eq!(fs::read(home.join("servers.dat_old")).unwrap(), original);
    assert!(!home.join("servers.dat_new").exists());
    assert_eq!(
        name_and_address(&written),
        [
            ("Serveur du club", "mc-club.example.edu:25565"),
            ("Monde de Jdupont", "192.168.10.42:25565"),
        ]
    );
    assert!(written.iter().all(|server| server.icon.is_none()));
    // What the game reads back: the club's servers alone, each taking no resource pack.
    assert_eq!(
        entries(home.as_path()),
        vec![
            Tag::compound([
                ("name", s("Serveur du club")),
                ("ip", s("mc-club.example.edu:25565")),
                ("acceptTextures", Tag::Byte(0)),
                ("hidden", Tag::Byte(0)),
            ]),
            Tag::compound([
                ("name", s("Monde de Jdupont")),
                ("ip", s("192.168.10.42:25565")),
                ("acceptTextures", Tag::Byte(0)),
                ("hidden", Tag::Byte(0)),
            ]),
        ],
        "the old entries, the one joined through Direct Connection too, are gone"
    );
    // A club that offers nothing empties the list.
    replace(home.as_path(), &[]).unwrap();
    assert_eq!(entries(home.as_path()), Vec::new());
}

#[test]
fn a_server_keeps_the_picture_the_game_last_saw_of_it() {
    let home = scratch("pictures");
    vanilla_file(home.as_path());
    let icon = base64::engine::general_purpose::STANDARD.encode(PNG);
    // The same address, under another name: its picture stays, and the club's rule replaces the
    // one the old entry had (it took the server's pack).
    let written = replace(
        home.as_path(),
        &[Server::new("Les amis", "friends.example:25570")],
    )
    .unwrap();
    assert_eq!(written[0].icon.as_deref(), Some(&PNG[..]));
    assert_eq!(
        entries(home.as_path()),
        vec![Tag::compound([
            ("name", s("Les amis")),
            ("ip", s("friends.example:25570")),
            ("icon", s(&icon)),
            ("acceptTextures", Tag::Byte(0)),
            ("hidden", Tag::Byte(0)),
        ])]
    );
    // Another address is another server: no picture of it yet.
    let written = replace(
        home.as_path(),
        &[Server::new("Friends", "other.example:25570")],
    )
    .unwrap();
    assert_eq!(written[0].icon, None);
    // A list that is not NBT is read as no pictures, and replaced all the same.
    fs::write(home.join(FILE), b"not nbt").unwrap();
    let written = replace(home.as_path(), &[Server::new("A", "a:1")]).unwrap();
    assert_eq!(name_and_address(&written), [("A", "a:1")]);
}

#[test]
fn the_first_list_needs_no_backup() {
    let home = scratch("first").join("game");
    replace(&home, &[Server::new("Hub", "hub.example")]).unwrap();
    assert!(
        !home.join("servers.dat_old").exists(),
        "nothing was there to keep"
    );
    let read: Vec<Server> = entries_in(&home)
        .iter()
        .map(|e| Server::new(text(e, "name"), text(e, "ip")))
        .collect();
    assert_eq!(name_and_address(&read), [("Hub", "hub.example")]);
}
