//! The game's own command line: what every start of the game (Play, a world's Play, a server's
//! Join) runs.
use super::*;

/// The command the game `tag` plays: `core\<tag>\horizon-client.exe` (from its staged copy on
/// Windows), the game home as its working directory, and
///
/// ```text
/// --resources <core>\<tag>\.local\horizon-client\<Minecraft version>
/// --managed <root>\managed.txt
/// --name <Name>
/// ```
///
/// (`docs/school-edition/launcher-target.md`, section 5). `player` is who plays, named by their
/// Windows session; `managed` is the club's rule of the game, which the supervisor's tool writes
/// and the launcher only hands over. No session of any kind: the name is all a student plays
/// under.
pub fn client_command(tag: &str, player: &str, managed: &Path) -> Command {
    let home = game_home();
    let _ = fs::create_dir_all(&home);
    let mut cmd = Command::new(client_program(tag));
    // Windows runs the game from its staged copy, so that the firewall sees one program path
    // whatever build plays (`windows_runtime`).
    #[cfg(windows)]
    crate::windows_runtime::mark(&mut cmd, &version_dir(tag));
    strip_launcher_render_env(&mut cmd);
    // The school edition sends nothing: the game's anonymous "I'm playing" ping is always off.
    cmd.env("HORIZON_NO_HEARTBEAT", "1");
    cmd.current_dir(&home)
        .env("HORIZON_HOME", &home)
        .arg("--resources")
        .arg(resources(tag))
        .arg("--managed")
        .arg(managed)
        .arg("--name")
        .arg(player);
    // PipeWire remembers volume and mute per application name. The game's own name is
    // "PipeWire ALSA [horizon-client]", shared with development and test runs, where
    // muting it once left every later game silent: played from here it is "Horizon".
    if std::env::var_os("PIPEWIRE_PROPS").is_none() {
        cmd.env(
            "PIPEWIRE_PROPS",
            "{ application.name = \"Horizon\" media.role = \"Game\" }",
        );
    }
    cmd
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(cmd: &Command) -> Vec<String> {
        cmd.get_args()
            .map(|a| a.to_string_lossy().into_owned())
            .collect()
    }

    /// The command line of the spec, in its order, and nothing else: the build's prepared
    /// folder, the club's rule, who plays. No session, no argument a student could add.
    #[test]
    fn the_game_starts_with_its_resources_the_rule_of_the_game_and_who_plays() {
        let managed = Path::new(r"\\school\clubs\minecraft\managed.txt");
        let cmd = client_command("club-10-0a1b2c4", "Jdupont", managed);
        let args = args(&cmd);
        assert_eq!(args.len(), 6, "{args:?}");
        assert_eq!(args[0], "--resources");
        // `<core>\<tag>\.local\horizon-client\<Minecraft version>`: the club's root as it is
        // spelled, which a test's may leave relative.
        let resources = args[1].replace('\\', "/");
        assert!(
            resources.ends_with("core/club-10-0a1b2c4/.local/horizon-client/26.3"),
            "{resources}"
        );
        assert_eq!(
            &args[2..],
            [
                "--managed",
                r"\\school\clubs\minecraft\managed.txt",
                "--name",
                "Jdupont",
            ]
        );
        // The working directory is the game home.
        assert_eq!(cmd.get_current_dir(), Some(game_home().as_path()));
    }
}
