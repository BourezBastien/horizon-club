use super::*;
use std::io::{BufRead, BufReader, Write};

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("horizon-servers-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn folder_names_are_plain() {
    assert_eq!(slug("Ludo's Survival!"), "ludo-s-survival");
    assert_eq!(slug("  "), "server");
    assert_eq!(slug("Été 2026"), "t-2026");
}

#[test]
fn create_asks_init_and_config_set() {
    let spec = NewServer {
        name: "Friends".into(),
        world: World::New {
            seed: " 12345 ".into(),
        },
        players: vec!["Alex".into(), "Steve".into()],
        ..NewServer::default()
    };
    assert_eq!(
        init_args(&spec, "world", 25566).join(" "),
        "init --yes --eula --offline --world world --seed 12345 --port 25566 --whitelist \
         --player Alex --player Steve --op Alex"
    );
    let open = NewServer {
        friends_only: false,
        world: World::Copy("/saves/Old".into()),
        ..NewServer::default()
    };
    assert_eq!(
        init_args(&open, "old", 25565).join(" "),
        "init --yes --eula --offline --world old --port 25565 --no-whitelist"
    );
    let resources = if cfg!(windows) {
        Path::new(r"\\school\clubs\minecraft\core\v1\.local\horizon-client\26.3")
    } else {
        Path::new("/share/minecraft/core/v1/.local/horizon-client/26.3")
    };
    let args = settings_args(&spec, resources).join(" ");
    assert!(
        args.starts_with(
            "config set world.game_mode=survival world.difficulty=normal server.max_players=10 \
             server.motd=Friends server.online_mode=false panel.bind= engine.resources="
        ),
        "{args}"
    );
    // The game files are the build's prepared folder, read in place from the share.
    let tail = if cfg!(windows) {
        r"core\v1\.local\horizon-client\26.3"
    } else {
        "core/v1/.local/horizon-client/26.3"
    };
    // ...and every player is put in the server's mode, whatever an earlier visit saved for them.
    assert!(
        args.ends_with(&format!("{tail} properties.force-gamemode=true")),
        "{args}"
    );
}

/// A hosted server takes a port from the club's `[hosting]` range, never outside it.
#[test]
fn a_port_comes_from_the_clubs_range_and_none_is_free_when_all_are_taken() {
    let taken = [25565u16];
    assert_eq!(free_port(25565, 25575, &taken), Some(25566));
    assert_eq!(free_port(25565, 25575, &[25565, 25566]), Some(25567));
    // The whole range taken: no server is created.
    assert_eq!(free_port(25565, 25565, &[25565]), None);
    // A port another program holds (not one of `taken`) is skipped: 25565 is bound here.
    let held = std::net::TcpListener::bind(("127.0.0.1", 25565)).unwrap();
    assert_eq!(free_port(25565, 25567, &[]), Some(25566));
    drop(held);
    assert_eq!(free_port(25565, 25567, &[]), Some(25565));
}

#[test]
fn problems_say_what_to_do() {
    let dir = scratch("problems");
    fs::write(dir.join("server.toml"), "[server]\nport = 25566\n").unwrap();
    let port = Problem::explain_with(
        "",
        "[Server thread/WARN]: **** FAILED TO BIND TO PORT!\nThe exception was: Address already in use (os error 48)",
        &dir,
    );
    assert!(port.message.contains("25566"), "{}", port.message);
    assert_eq!(port.fix, Some(Fix::PortTaken));
    let files = Problem::explain_with(
        "horizon-server: no prepared game files. Horizon reads them from",
        "",
        &dir,
    );
    assert_eq!(files.fix, None);
    let other = Problem::explain_with("horizon-server: a server is already running in x", "", &dir);
    assert_eq!(other, Problem::plain("A server is already running in x"));
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn lists_and_status_read_as_vanilla_and_the_supervisor_write_them() {
    let dir = scratch("lists");
    fs::write(
        dir.join("whitelist.json"),
        r#"[{"uuid": "a", "name": "Alex"}, {"uuid": "b", "name": "Steve"}]"#,
    )
    .unwrap();
    assert_eq!(names(&dir, "whitelist.json"), ["Alex", "Steve"]);
    assert!(names(&dir, "ops.json").is_empty());
    fs::remove_dir_all(&dir).unwrap();

    let s = Status::parse(&json!({
        "phase": "running", "engine_uptime_seconds": 61, "crashes": 1, "last_crash": "signal: 9",
        "sample": { "rss_bytes": 800_000_000u64, "cpu_percent": 3.5, "game": {
            "tps": 20.0, "mspt": { "average": 4.2 }, "max_players": 10, "paused": false,
            "players": [{ "name": "Alex", "uuid": "u1", "ping": 12 }] } },
        "update": { "installed": "v1", "available": "v2", "busy": null, "last": "Not updated: x", "ok": false }
    }));
    assert_eq!(s.phase, "running");
    assert_eq!(s.players[0].name, "Alex");
    assert_eq!(s.players[0].ping, Some(12));
    assert_eq!(
        (s.tps, s.mspt, s.max_players),
        (Some(20.0), Some(4.2), Some(10))
    );
    assert!(!Status::stopped().running());
}

/// The port a server's `server.toml` sets, read from the file: what the busy-port problem names
/// and the announcement says.
#[test]
fn the_port_comes_from_server_toml() {
    let dir = scratch("port");
    assert_eq!(port_of(&dir), None);
    fs::write(
        dir.join("server.toml"),
        "[server]\nmotd = \"x\"\nport = 25570\n",
    )
    .unwrap();
    assert_eq!(port_of(&dir), Some(25570));
    let _ = fs::remove_dir_all(&dir);
}

/// The control API: the token goes with every call, and errors come back as the server words them.
#[test]
fn the_api_sends_the_token() {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap().to_string();
    let server = std::thread::spawn(move || {
        let mut seen = Vec::new();
        for reply in [
            r#"{"phase":"running","sample":{"game":{"players":[]}}}"#,
            r#"{"error":"the game didn't answer"}"#,
        ] {
            let (stream, _) = listener.accept().unwrap();
            let mut reader = BufReader::new(stream.try_clone().unwrap());
            let mut head = String::new();
            loop {
                let mut line = String::new();
                reader.read_line(&mut line).unwrap();
                if line == "\r\n" || line.is_empty() {
                    break;
                }
                head.push_str(&line);
            }
            // Read the body too: closing with it unread would reset the connection.
            let length = head
                .lines()
                .find_map(|l| {
                    l.to_ascii_lowercase()
                        .strip_prefix("content-length:")
                        .map(|n| n.trim().parse::<usize>().unwrap())
                })
                .unwrap_or(0);
            let mut body = vec![0; length];
            std::io::Read::read_exact(&mut reader, &mut body).unwrap();
            seen.push(head);
            let code = if reply.contains("error") {
                "409 Conflict"
            } else {
                "200 OK"
            };
            let mut stream = stream;
            write!(
                stream,
                "HTTP/1.1 {code}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{reply}",
                reply.len()
            )
            .unwrap();
        }
        seen
    });
    let dir = scratch("api");
    fs::create_dir_all(dir.join(".horizon")).unwrap();
    fs::write(
        dir.join(".horizon/api.json"),
        json!({ "address": address, "token": "t0k", "pid": 1 }).to_string(),
    )
    .unwrap();
    assert_eq!(status(&dir).phase, "running");
    let api = Api::open(&dir).unwrap();
    assert_eq!(
        api.command("list").unwrap_err(),
        "the game didn't answer".to_string()
    );
    let seen = server.join().unwrap();
    assert!(
        seen[0].starts_with("GET /api/v1/status HTTP/1.1"),
        "{}",
        seen[0]
    );
    assert!(seen
        .iter()
        .all(|h| h.to_ascii_lowercase().contains("authorization: bearer t0k")));
    assert!(seen[1].starts_with("POST /api/v1/command"));
    fs::remove_dir_all(&dir).unwrap();
    // Nothing answers: stopped.
    assert_eq!(status(Path::new("/nonexistent")).phase, "stopped");
}

#[test]
fn deleting_a_stopped_server_moves_its_world_and_backups_to_trash() {
    let root = scratch("delete-stopped");
    let dir = root.join("server");
    let trash = root.join("trash");
    fs::create_dir_all(dir.join("world/region")).unwrap();
    fs::create_dir_all(dir.join("backups")).unwrap();
    fs::write(dir.join("server.toml"), "[server]\n").unwrap();
    fs::write(dir.join("world/region/region.mca"), b"world").unwrap();
    fs::write(dir.join("backups/backup.zip"), b"backup").unwrap();
    trash_stopped_server(&dir, |path| crate::desktop_seams::to_folder(path, &trash)).unwrap();
    assert!(!dir.exists());
    assert_eq!(
        fs::read(trash.join("server/world/region/region.mca")).unwrap(),
        b"world"
    );
    assert_eq!(
        fs::read(trash.join("server/backups/backup.zip")).unwrap(),
        b"backup"
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn deleting_refuses_a_live_supervisor_even_when_its_api_is_unavailable() {
    let root = scratch("delete-running");
    fs::create_dir_all(root.join(".horizon")).unwrap();
    fs::write(root.join("server.toml"), "[server]\n").unwrap();
    let lock = fs::File::create(root.join(".horizon/lock")).unwrap();
    lock.try_lock().unwrap();
    let result = trash_stopped_server(&root, |_| panic!("must not move a running server"));
    assert!(result.unwrap_err().contains("Stop the server"));
    assert!(root.join("server.toml").exists());
    drop(lock);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn deleting_refuses_an_unrecognized_folder_and_keeps_files_on_trash_failure() {
    let root = scratch("delete-invalid");
    assert!(trash_stopped_server(&root, |_| panic!("not a server")).is_err());
    fs::write(root.join("server.toml"), "[server]\n").unwrap();
    assert!(
        trash_stopped_server(&root, |_| Err("trash unavailable".into()))
            .unwrap_err()
            .contains("trash unavailable")
    );
    assert!(root.join("server.toml").exists());
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn the_game_mode_comes_from_server_toml() {
    let dir = scratch("game-mode");
    assert_eq!(game_mode(&dir), None);
    fs::write(
        dir.join("server.toml"),
        "[server]\nport = 25565\n\n[world]\nname = \"world\"\ngame_mode = \"creative\"\n",
    )
    .unwrap();
    assert_eq!(game_mode(&dir).as_deref(), Some("creative"));
    fs::write(dir.join("server.toml"), "[server]\nport = 25565\n").unwrap();
    assert_eq!(game_mode(&dir), None);
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn a_new_game_mode_reaches_the_players_on_a_running_server() {
    let changes = [
        ("world.difficulty", "hard".to_string()),
        ("world.game_mode", "creative".to_string()),
    ];
    let applied = Applied {
        applied: vec!["world.difficulty".into(), "world.game_mode".into()],
        running: true,
        ..Default::default()
    };
    assert_eq!(
        mode_command(&changes, &applied).as_deref(),
        Some("gamemode creative @a")
    );
    // Stopped, the setting waits for the next start; another setting moves nobody.
    assert_eq!(
        mode_command(
            &changes,
            &Applied {
                running: false,
                ..applied.clone()
            }
        ),
        None
    );
    assert_eq!(
        mode_command(
            &changes,
            &Applied {
                applied: vec!["world.difficulty".into()],
                ..applied
            }
        ),
        None
    );
}
