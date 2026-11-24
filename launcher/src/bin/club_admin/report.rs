//! What `club-admin check` looks at, and what each thing it finds means: the report's verdicts
//! are decided here, from facts gathered on the disk, so the reading and the judging stay apart
//! and the judging can be pinned by tests.

use horizon_launcher::i18n::{tr, trf};

/// One build of `core\`, as the report sees it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Build {
    /// Its folder's name in `core\`.
    pub name: String,
    /// The Minecraft version it plays (`research-lock.json`).
    pub game_version: String,
    /// The prepared game files are there and whole.
    pub prepared: bool,
    /// `dxcompiler.dll` sits beside the game's program (DirectX 12 compiles its shaders with it,
    /// on the integrated GPUs the club's PCs have).
    pub dxcompiler: bool,
    /// A `horizon-data.bin` of this very folder is in the prepared folder, so a start over the
    /// network opens one file, not about 22,000. One written for another folder (the build was
    /// copied from elsewhere) is left alone by the game, so it counts for none.
    pub data_file: bool,
    /// `horizon-server.exe` is beside the game's program, so students can host with it.
    pub server_program: bool,
}

/// Everything the report is read from.
#[derive(Clone, Debug)]
pub struct Facts {
    /// The club's configuration, or the sentence saying why it could not be read.
    pub club: Result<horizon_launcher::config::Config, String>,
    /// The builds of `core\`, newest first.
    pub builds: Vec<Build>,
    /// The build the club plays (`[versions]` rules among `builds`' names).
    pub active: Option<String>,
    /// What `[versions] active` names, when it names no build of `core\`.
    pub named_but_absent: Option<String>,
    /// `managed.txt`'s text, or None when the file is not there.
    pub managed: Option<String>,
    /// What `sync` would write into it.
    pub managed_expected: String,
    /// `app\current.txt`'s content, trimmed, or None when the file is not there.
    pub current: Option<String>,
    /// The launcher `app\current.txt` names is there.
    pub launcher_there: bool,
    /// The bootstrap (`HorizonClub.exe`) is at the club's root.
    pub bootstrap: bool,
}

/// One line of the report: what it says, and whether what it says keeps a student from playing.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Finding {
    pub blocks: bool,
    pub said: String,
}

impl Finding {
    fn ok(said: String) -> Self {
        Self {
            blocks: false,
            said,
        }
    }

    fn blocking(said: String) -> Self {
        Self { blocks: true, said }
    }
}

/// The report's verdicts, one per thing that is something. What keeps a student from playing
/// says what to run; what only costs speed or comfort says so without stopping anyone.
pub fn verdicts(facts: &Facts) -> Vec<Finding> {
    let mut out = Vec::new();
    if let Err(error) = &facts.club {
        out.push(Finding::blocking(trf(
            "club.toml could not be read: {error}. Fix the file: every key of the file is \
             optional, but a key misspelt or of the wrong kind is an error.",
            &[("error", error)],
        )));
        return out;
    }
    out.push(Finding::ok(tr("club.toml reads fine.").into()));
    if let Some(named) = &facts.named_but_absent {
        out.push(Finding::ok(trf(
            "club.toml names the build {named}, which core\\ does not hold; the club plays \
             {active}. Put the build in place, or run: club-admin set-version <tag>.",
            &[
                ("named", named),
                ("active", facts.active.as_deref().unwrap_or("?")),
            ],
        )));
    }
    match facts
        .active
        .as_deref()
        .and_then(|active| facts.builds.iter().find(|build| build.name == active))
    {
        Some(build) => {
            let active = &build.name;
            if build.prepared {
                out.push(Finding::ok(trf(
                    "The build in use is {tag} (Minecraft {version}), prepared.",
                    &[("tag", active), ("version", &build.game_version)],
                )));
            } else {
                out.push(Finding::blocking(trf(
                    "The build in use is {tag}, and its game files are not prepared. Run: \
                     club-admin prepare --tag {tag} --minecraft-dir <folder> (or --download)",
                    &[("tag", active)],
                )));
            }
            if !build.dxcompiler {
                out.push(Finding::ok(trf(
                    "{tag} has no dxcompiler.dll: the game still plays, but DirectX 12 compiles \
                     its shaders slower without it. Copy it beside horizon-client.exe \
                     (tools/ci/fetch_dxc.py).",
                    &[("tag", active)],
                )));
            }
            if !build.data_file {
                out.push(Finding::ok(trf(
                    "{tag} has no horizon-data.bin of its own: the game still plays, but every \
                     start reads about 22,000 files instead of one, minutes instead of seconds. \
                     Run: club-admin data-file --tag {tag}",
                    &[("tag", active)],
                )));
            }
            if !build.server_program {
                out.push(Finding::ok(trf(
                    "{tag} has no horizon-server.exe, so students cannot host. Put a build in \
                     place that has one: club-admin install-core --from <folder>",
                    &[("tag", active)],
                )));
            }
        }
        None => {
            out.push(Finding::blocking(
                tr(
                    "No game build is in core\\. Put one there: club-admin install-core --from \
                 <folder> [--tag <tag>], then club-admin prepare",
                )
                .into(),
            ));
            return out;
        }
    }
    // The other builds: named, with what they would need before the club can play them.
    for build in &facts.builds {
        if Some(&build.name) == facts.active.as_ref() {
            continue;
        }
        let mut said = trf(
            "{tag}: Minecraft {version}",
            &[("tag", &build.name), ("version", &build.game_version)],
        );
        if !build.prepared {
            said += &trf(
                ", not prepared (club-admin prepare --tag {tag})",
                &[("tag", &build.name)],
            );
        }
        if !build.server_program {
            said += tr(", no horizon-server.exe (no hosting)");
        }
        out.push(Finding::ok(said));
    }
    match &facts.managed {
        Some(text) if *text == facts.managed_expected => {
            out.push(Finding::ok(
                tr("managed.txt says what club.toml says.").into(),
            ));
        }
        Some(_) => out.push(Finding::blocking(
            tr("managed.txt is not what club.toml says. Run: club-admin sync").into(),
        )),
        None => out.push(Finding::blocking(
            tr("managed.txt is missing: the game cannot start without it. Run: club-admin sync")
                .into(),
        )),
    }
    match &facts.current {
        Some(version) if facts.launcher_there => {
            out.push(Finding::ok(trf(
                "app\\current.txt says {version}, and its launcher is there.",
                &[("version", version)],
            )));
        }
        Some(version) => out.push(Finding::blocking(trf(
            "app\\current.txt says {version}, but app\\{version}\\HorizonClubLauncher.exe is not \
             there. Run: club-admin publish --launcher <exe> --version {version}",
            &[("version", version)],
        ))),
        None => out.push(Finding::blocking(tr(
            "app\\current.txt is missing, so the bootstrap cannot say which launcher to start. \
             Run: club-admin publish --launcher <exe> --version <X>",
        )
        .into())),
    }
    if facts.bootstrap {
        out.push(Finding::ok(
            tr("The bootstrap (HorizonClub.exe) is at the club's root.").into(),
        ));
    } else {
        out.push(Finding::blocking(
            tr(
                "The bootstrap (HorizonClub.exe) is missing at the club's root. Run: club-admin \
             publish --launcher <exe> --version <X> --bootstrap <exe>",
            )
            .into(),
        ));
    }
    out
}

/// Whether anything the report found keeps a student from playing.
pub fn any_blocks(finds: &[Finding]) -> bool {
    finds.iter().any(|f| f.blocks)
}

#[cfg(test)]
mod tests {
    use super::*;
    use horizon_launcher::config::Config;

    fn facts() -> Facts {
        Facts {
            club: Ok(Config::default()),
            builds: vec![Build {
                name: "club-10-0a1b2c4".into(),
                game_version: "26.3".into(),
                prepared: true,
                dxcompiler: true,
                data_file: true,
                server_program: true,
            }],
            active: Some("club-10-0a1b2c4".into()),
            named_but_absent: None,
            managed: Some("lan:true\n".into()),
            managed_expected: "lan:true\n".into(),
            current: Some("1".into()),
            launcher_there: true,
            bootstrap: true,
        }
    }

    /// A share with everything in place says so and blocks nothing.
    #[test]
    fn a_ready_share_blocks_nobody() {
        let finds = verdicts(&facts());
        assert!(!any_blocks(&finds), "{finds:?}");
        assert!(
            finds.iter().any(|f| f.said.contains("prepared")),
            "{finds:?}"
        );
    }

    /// Each thing that keeps a student from playing says what to run; each of these alone is
    /// enough for the exit code.
    #[test]
    fn everything_that_stops_a_student_says_what_to_run() {
        for broken in [
            // club.toml unreadable.
            Facts {
                club: Err("TOML parse error at line 4, column 12".into()),
                ..facts()
            },
            // No build at all.
            Facts {
                builds: Vec::new(),
                active: None,
                ..facts()
            },
            // The build in use without its files.
            Facts {
                builds: vec![Build {
                    prepared: false,
                    ..facts().builds[0].clone()
                }],
                ..facts()
            },
            // The rule of the game missing, and out of date.
            Facts {
                managed: None,
                ..facts()
            },
            Facts {
                managed: Some("lan:false\n".into()),
                ..facts()
            },
            // The launcher pointer missing, and pointing at nothing.
            Facts {
                current: None,
                ..facts()
            },
            Facts {
                launcher_there: false,
                ..facts()
            },
            // The bootstrap missing.
            Facts {
                bootstrap: false,
                ..facts()
            },
        ] {
            let finds = verdicts(&broken);
            assert!(any_blocks(&finds), "{}", broken.active.unwrap_or_default());
            assert!(
                finds
                    .iter()
                    .filter(|f| f.blocks)
                    .all(|f| f.said.contains("club-admin") || f.said.contains("Fix the file")),
                "{:?}",
                finds.iter().filter(|f| f.blocks).collect::<Vec<_>>()
            );
        }
    }

    /// What only costs speed or comfort says so without stopping anyone.
    #[test]
    fn the_slow_paths_are_said_without_blocking_anyone() {
        let mut slow = facts();
        slow.builds[0].dxcompiler = false;
        slow.builds[0].data_file = false;
        slow.builds[0].server_program = false;
        slow.managed_expected = "lan:true\nserver:mc.example.org\n".into();
        slow.managed = Some(slow.managed_expected.clone());
        let finds = verdicts(&slow);
        assert!(!any_blocks(&finds), "{finds:?}");
        for said in ["dxcompiler", "horizon-data.bin", "horizon-server.exe"] {
            assert!(
                finds.iter().any(|f| f.said.contains(said)),
                "{said}: {finds:?}"
            );
        }
    }

    /// A build the file names but `core\` does not hold is said; the club still plays the one
    /// the rules pick.
    #[test]
    fn a_named_build_that_is_not_there_is_said_without_blocking_anyone() {
        let mut absent = facts();
        absent.named_but_absent = Some("club-11-gone".into());
        let finds = verdicts(&absent);
        assert!(!any_blocks(&finds), "{finds:?}");
        assert!(finds
            .iter()
            .any(|f| f.said.contains("club-11-gone") && f.said.contains("club-10-0a1b2c4")));
    }

    /// A spare build is listed, with what it would need before the club can play it.
    #[test]
    fn spare_builds_are_listed_with_what_they_lack() {
        let mut spare = facts();
        spare.builds.push(Build {
            name: "club-9-old".into(),
            game_version: "26.3".into(),
            prepared: false,
            dxcompiler: true,
            data_file: false,
            server_program: true,
        });
        let finds = verdicts(&spare);
        assert!(!any_blocks(&finds), "{finds:?}");
        assert!(finds
            .iter()
            .any(|f| f.said.contains("club-9-old") && f.said.contains("not prepared")));
    }
}
