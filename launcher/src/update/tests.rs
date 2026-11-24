use super::*;
use std::io::{BufRead, BufReader, Write};

/// A server on the loopback answering each of `routes` (a path and its body) in turn, in plain
/// HTTP, one connection at a time. Its address, and the thread that serves.
fn serve(routes: Vec<(&'static str, Vec<u8>)>) -> (String, std::thread::JoinHandle<()>) {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let server = std::thread::spawn(move || {
        for (path, body) in routes {
            let (mut stream, _) = listener.accept().unwrap();
            let mut reader = BufReader::new(stream.try_clone().unwrap());
            let mut request = String::new();
            loop {
                let mut line = String::new();
                if reader.read_line(&mut line).unwrap() == 0 || line == "\r\n" {
                    break;
                }
                request.push_str(&line);
            }
            assert_eq!(
                request.split_whitespace().nth(1),
                Some(path),
                "asked: {request}"
            );
            write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            )
            .unwrap();
            stream.write_all(&body).unwrap();
        }
    });
    (base, server)
}

fn hex(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

/// `sha256sum` writes two forms: two spaces in text mode, a star in binary mode (its default on
/// Git for Windows, so on the workflow that builds a release). The release built that way was
/// refused by a launcher that read the first form only.
#[test]
fn a_list_of_hashes_is_read_as_sha256sum_writes_it_on_any_system() {
    let file = b"MZ the launcher".to_vec();
    let hash = hex(&file);
    let other = hex(b"MZ another program");
    for sums in [
        format!("{hash}  {LAUNCHER}\n"),
        format!("{hash} *{LAUNCHER}\r\n"),
        format!("{other}  HorizonClub.exe\r\n{hash}  {LAUNCHER}\r\n"),
        format!("{other} *HorizonClub.exe\n{hash} *{LAUNCHER}\n"),
    ] {
        assert_eq!(verify(&file, sums.as_bytes()), Ok(()), "{sums}");
    }
    // Another file's hash, a list without the launcher, a file that is no program: all refused.
    assert!(verify(&file, format!("{other} *{LAUNCHER}\n").as_bytes()).is_err());
    assert!(verify(&file, format!("{hash}  HorizonClub.exe\n").as_bytes()).is_err());
    assert!(verify(b"no program", format!("{hash}  {LAUNCHER}\n").as_bytes()).is_err());
}

const DESCRIPTION: &str = r#"{
  "tag_name": "v0.3.0",
  "draft": false,
  "prerelease": false,
  "assets": [
    { "name": "HorizonClub.exe", "browser_download_url": "https://github.com/x/y/releases/download/v0.3.0/HorizonClub.exe" },
    { "name": "HorizonClubLauncher.exe", "browser_download_url": "https://github.com/x/y/releases/download/v0.3.0/HorizonClubLauncher.exe" },
    { "name": "SHA256SUMS.txt", "browser_download_url": "https://github.com/x/y/releases/download/v0.3.0/SHA256SUMS.txt" }
  ]
}"#;

#[test]
fn a_version_is_newer_by_its_numbers_and_nothing_else_is() {
    assert!(is_newer("0.3.0", "0.2.0"));
    assert!(is_newer("0.10.0", "0.9.0"));
    assert!(is_newer("1.0", "0.99.99"));
    assert!(!is_newer("0.3.0", "0.3.0"));
    assert!(!is_newer("0.2.0", "0.3.0"));
    assert!(!is_newer("nightly", "0.3.0"));
    assert!(!is_newer("0.4.0", ""));
}

#[test]
fn a_release_is_read_from_githubs_description() {
    let release = release_of(DESCRIPTION).unwrap();
    assert_eq!(release.version, "0.3.0");
    assert!(release.launcher.ends_with("/HorizonClubLauncher.exe"));
    assert!(release.sums.ends_with("/SHA256SUMS.txt"));
    // A release without the launcher's list of hashes is no update.
    let without: serde_json::Value = serde_json::from_str(DESCRIPTION).unwrap();
    let mut without = without;
    without["assets"]
        .as_array_mut()
        .unwrap()
        .retain(|a| a["name"] != "SHA256SUMS.txt");
    assert!(release_of(&without.to_string()).is_err());
    assert!(release_of("not json").is_err());
    assert!(release_of(r#"{"tag_name": "v1.0.0"}"#).is_err());
}

#[test]
fn an_update_lands_beside_the_old_one_and_points_the_share_at_it() {
    let root = tempfile::tempdir().unwrap();
    let app = root.path().join("app");
    std::fs::create_dir_all(app.join("0.2.0")).unwrap();
    std::fs::write(app.join("0.2.0").join(LAUNCHER), b"MZ the launcher before").unwrap();
    std::fs::write(app.join("current.txt"), "0.2.0\n").unwrap();
    let file = b"MZ the new launcher".to_vec();
    let sums = format!("{}  {}\n", hex(&file), LAUNCHER);
    // The list of hashes comes first: the launcher is checked as it arrives.
    let (base, server) = serve(vec![
        ("/SHA256SUMS.txt", sums.into_bytes()),
        ("/launcher.exe", file.clone()),
    ]);
    let release = Release {
        version: "0.3.0".into(),
        launcher: format!("{base}/launcher.exe"),
        sums: format!("{base}/SHA256SUMS.txt"),
        bootstrap: None,
        admin: None,
    };
    install(root.path(), &release).unwrap();
    server.join().unwrap();
    assert_eq!(
        std::fs::read(app.join("0.3.0").join(LAUNCHER)).unwrap(),
        file
    );
    assert_eq!(
        std::fs::read_to_string(app.join("current.txt")).unwrap(),
        "0.3.0\n"
    );
    // The version before stays: `current.txt` is how one goes back.
    assert_eq!(
        std::fs::read(app.join("0.2.0").join(LAUNCHER)).unwrap(),
        b"MZ the launcher before"
    );
    assert!(std::fs::read_dir(&app).unwrap().all(|e| {
        let name = e.unwrap().file_name().to_string_lossy().into_owned();
        name != "current.0.3.0.txt" && name != "HorizonClubLauncher.exe.part"
    }));
}

#[test]
fn a_file_that_is_not_the_releases_is_not_installed() {
    let root = tempfile::tempdir().unwrap();
    let app = root.path().join("app");
    std::fs::create_dir_all(&app).unwrap();
    std::fs::write(app.join("current.txt"), "0.2.0\n").unwrap();
    // The list names another file's hash.
    let (base, server) = serve(vec![
        (
            "/SHA256SUMS.txt",
            format!("{}  {}\n", hex(b"MZ something else"), LAUNCHER).into_bytes(),
        ),
        ("/launcher.exe", b"MZ another program".to_vec()),
    ]);
    let release = Release {
        version: "0.3.0".into(),
        launcher: format!("{base}/launcher.exe"),
        sums: format!("{base}/SHA256SUMS.txt"),
        bootstrap: None,
        admin: None,
    };
    assert!(install(root.path(), &release).is_err());
    server.join().unwrap();
    assert!(!app.join("0.3.0").exists());
    assert_eq!(
        std::fs::read_to_string(app.join("current.txt")).unwrap(),
        "0.2.0\n"
    );
}

#[test]
fn a_version_that_is_not_a_folder_name_is_refused_before_anything_is_downloaded() {
    let root = tempfile::tempdir().unwrap();
    for version in ["", "..", "../x", "a/b", "v 1"] {
        let release = Release {
            version: version.into(),
            launcher: "http://127.0.0.1:9/never".into(),
            sums: "http://127.0.0.1:9/never".into(),
            bootstrap: None,
            admin: None,
        };
        assert!(install(root.path(), &release).is_err(), "{version:?}");
    }
    assert!(!root.path().join("app").exists());
}

#[test]
fn only_a_session_that_can_write_in_the_share_may_update() {
    let root = tempfile::tempdir().unwrap();
    assert!(!may_update(root.path()), "no app\\ folder there");
    std::fs::create_dir_all(root.path().join("app")).unwrap();
    assert!(may_update(root.path()));
    assert!(std::fs::read_dir(root.path().join("app"))
        .unwrap()
        .next()
        .is_none());
}

#[test]
fn a_newer_release_is_found_and_an_older_one_is_not() {
    let (base, server) = serve(vec![("/latest", DESCRIPTION.as_bytes().to_vec())]);
    let url = format!("{base}/latest");
    let found = check_at(&url, "0.2.0").unwrap();
    assert_eq!(found.as_ref().map(|r| r.version.as_str()), Some("0.3.0"));
    server.join().unwrap();
    // The same release is nothing new to the launcher it is running.
    let (base, server) = serve(vec![("/latest", DESCRIPTION.as_bytes().to_vec())]);
    assert_eq!(check_at(&format!("{base}/latest"), "0.3.0").unwrap(), None);
    server.join().unwrap();
}

/// A release since the installer exists has the bootstrap and the supervisor's tool too; one
/// from before is still an update.
#[test]
fn a_release_names_its_bootstrap_and_tool_when_it_has_them() {
    // The description already holds the bootstrap; one from before the installer had no tool.
    let before = release_of(DESCRIPTION).unwrap();
    assert!(before
        .bootstrap
        .as_deref()
        .unwrap()
        .ends_with("/HorizonClub.exe"));
    assert_eq!(before.admin, None);
    assert_eq!(before.file(ADMIN), None);
    let mut newer: serde_json::Value = serde_json::from_str(DESCRIPTION).unwrap();
    newer["assets"].as_array_mut().unwrap().push(serde_json::json!({
        "name": ADMIN,
        "browser_download_url": format!("https://github.com/x/y/releases/download/v0.3.0/{ADMIN}")
    }));
    let release = release_of(&newer.to_string()).unwrap();
    assert_eq!(
        release.file(BOOTSTRAP),
        Some(release.bootstrap.as_deref().unwrap())
    );
    assert!(release
        .admin
        .as_deref()
        .unwrap()
        .ends_with("/club-admin.exe"));
    assert_eq!(release.file("nothing.txt"), None);
}

/// The first install of a share: a file the supervisor has beside the installer is used when it
/// is the release's, one that differs or is missing is downloaded, and each lands where the
/// share holds it.
#[test]
fn a_share_is_installed_from_the_files_beside_the_installer_then_from_the_release() {
    let root = tempfile::tempdir().unwrap();
    let beside = tempfile::tempdir().unwrap();
    let launcher = b"MZ the launcher".to_vec();
    let bootstrap = b"MZ the bootstrap".to_vec();
    let admin = b"MZ the tool".to_vec();
    let (launcher_hash, bootstrap_hash, admin_hash) =
        (hex(&launcher), hex(&bootstrap), hex(&admin));
    let sums = format!(
        "{launcher_hash}  {LAUNCHER}\n{bootstrap_hash} *{BOOTSTRAP}\n{admin_hash}  {ADMIN}\n"
    );
    std::fs::write(beside.path().join(SUMS), &sums).unwrap();
    // The launcher beside, the bootstrap beside but not the release's, no tool.
    std::fs::write(beside.path().join(LAUNCHER), &launcher).unwrap();
    std::fs::write(beside.path().join(BOOTSTRAP), b"MZ an older bootstrap").unwrap();
    let (base, server) = serve(vec![
        ("/HorizonClub.exe", bootstrap.clone()),
        ("/club-admin.exe", admin.clone()),
    ]);
    let release = Release {
        version: "0.4.0".into(),
        launcher: format!("{base}/never"),
        sums: format!("{base}/never"),
        bootstrap: Some(format!("{base}/HorizonClub.exe")),
        admin: Some(format!("{base}/club-admin.exe")),
    };
    let mut heard = Vec::new();
    let installed = setup(root.path(), &release, beside.path(), &mut |step, line| {
        heard.push((step, line))
    })
    .unwrap();
    server.join().unwrap();
    let read = |file: &Path| std::fs::read(file).unwrap();
    assert_eq!(
        installed.files,
        [
            root.path().join("app").join("0.4.0").join(LAUNCHER),
            root.path().join(BOOTSTRAP),
            root.path().join("admin").join(ADMIN),
        ]
    );
    assert_eq!(read(&installed.files[0]), launcher, "the file beside");
    assert_eq!(
        read(&installed.files[1]),
        bootstrap,
        "downloaded: the other wasn't the release's"
    );
    assert_eq!(read(&installed.files[2]), admin);
    assert_eq!(
        std::fs::read_to_string(root.path().join("app").join("current.txt")).unwrap(),
        "0.4.0\n"
    );
    // Per file: where it came from (two lines when downloaded) and where it landed; then
    // current.txt.
    assert_eq!(heard.len(), 9, "{heard:#?}");
    // Nothing half-written is left, and installing again changes nothing.
    let stray = |folder: &Path| {
        std::fs::read_dir(folder)
            .unwrap()
            .flatten()
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .filter(|n| n.ends_with(".part") || n.contains("current.0"))
            .count()
    };
    assert_eq!(stray(&root.path().join("app")), 0);
    // With the release's files all beside the installer, nothing is asked of the network (the
    // addresses here lead nowhere), and installing again changes nothing.
    std::fs::write(beside.path().join(BOOTSTRAP), &bootstrap).unwrap();
    std::fs::write(beside.path().join(ADMIN), &admin).unwrap();
    let nowhere = "http://127.0.0.1:9/never".to_string();
    let offline = Release {
        launcher: nowhere.clone(),
        sums: nowhere.clone(),
        bootstrap: Some(nowhere.clone()),
        admin: Some(nowhere),
        ..release
    };
    let again = setup(root.path(), &offline, beside.path(), &mut |_, _| ()).unwrap();
    assert_eq!(again, installed);
}
