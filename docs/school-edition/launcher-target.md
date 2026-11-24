# Launcher target behaviour (the spec the trimming steps build to)

Working spec for the agents that trim `launcher/`. The product is described for the club's
supervisor in [README.md](README.md); this page says what the code must do. Read it with
[launcher-trim-map.md](launcher-trim-map.md).

## 1. What the launcher is

A windowed program for a middle-school club. A student double-clicks the shortcut, the launcher
opens on its Home page, the student presses Play. Nothing is installed on the student's PC: the
launcher, the game and the game's files live on a read-only network share. The launcher makes
**no network connection of its own** and takes **no command-line argument**: the only thing it
starts is the game, and the servers the student hosts from its Servers page.

## 2. Folders

| What | Where |
|---|---|
| Club root | the folder `club.toml` is in. Found by: `HORIZON_CLUB_ROOT` if set; else three levels up from the launcher's own executable (`root\app\<version>\HorizonClubLauncher.exe`); else walking up from the current directory, four levels at most |
| Launcher data | `HORIZON_LAUNCHER_HOME` if set, else `%LOCALAPPDATA%\HorizonClub\launcher` |
| Game home | `HORIZON_HOME` if set, else `%LOCALAPPDATA%\HorizonClub\game` |
| Game builds | every `<root>\core\<tag>\` that holds `horizon-client.exe` |
| Rule of the game | `<root>\managed.txt`, handed to the game with `--managed` |
| Online hosted servers | `<root>\online\`, one JSON file per running server |

On start the launcher makes its data folder the current directory (the share is read-only, and a
UNC path is a poor working directory). It never writes inside the club root except in `online\`.

The launcher's location pointer (`data_location.rs`), the per-user installer and the uninstall
entry have no place here: delete them. `HORIZON_LAUNCHER_LOCATION_FILE` goes with them.

## 3. `club.toml`

Every key is optional; the defaults make a bare root work. The file is the supervisor's, read-only
for students. Parsed with the `toml` crate (already a dependency).

```toml
[club]
name = "Club Minecraft"          # shown as the window title and on Home
language = "fr"                  # launcher language the first time a student opens it
identity = "windows"             # how a student is named: "windows" is the only mode
shared_accounts = ["eleve"]      # Windows sessions several students share: with one of them the
                                 # launcher refuses to play and says why

[[servers]]                      # the servers the club offers, in this order
name = "Serveur du club"
address = "mc-club.example.edu"  # a port after a colon only when it is not 25565

[game]                           # written into a student's options at their first Play
render_distance = 8
simulation_distance = 5
max_fps = 60
gui_scale = 2                    # the interface's size: 0 lets the game choose, else 1 to 4

[lan]
open = true                      # a world may be opened to the LAN (vanilla's Open to LAN)

[hosting]                        # the Servers page
enabled = true
first_port = 25565               # hosted servers take a port from this one up
last_port = 25575
eula_accepted = true             # the supervisor accepts Minecraft's EULA for hosted servers

[versions]
active = "26.3-20261122"         # the build the club plays; else the only build in core\, else
                                 # the newest folder name
```

`managed.txt` (the game reads it, `crates/horizon-client/src/managed.rs`) is written from the
same file when it changes: `lan:true|false`, and one `server:<address>` line for each
`[[servers]]` entry whose host is not on the local network.

## 4. Who plays

There are no accounts. The student's name comes from their Windows session:

- the login is `USERNAME` (fallback: the part after the last `\` of `USERDOMAIN\USERNAME`);
- it is folded to ASCII (accents dropped), lower-cased, and only letters, digits and `_` are kept;
  the first letter is capitalised: `j.dupont` becomes `Jdupont`;
- longer than 16 characters: the first 12, `_`, and three hexadecimal characters of a stable
  hash of the folded login (FNV-1a, 32 bits: its low 12 bits), so the same login gives the same
  name on every PC and two long logins that begin the same stay apart;
- shorter than 3: an error;
- a login listed in `shared_accounts` (whatever its case): the launcher does not play and shows
  that this session is shared.

The name is passed to the game with `--name`. Nothing else identifies a student.

## 5. Play

1. Read `club.toml`; find the active build in `core\`.
2. Build the server list (section 6) and write `servers.dat` in the game home.
3. At the student's first Play, write their `options.txt` (section 8).
4. Start `core\<tag>\horizon-client.exe` through the existing Windows staging (the stable copy
   in the launcher's data folder, copied again only when the build changed), with the working
   directory the game home and:
   ```text
   --resources <core>\<tag>\.local\horizon-client\<Minecraft version> --managed <root>\managed.txt --name <Name>
   ```
   plus `--server <address>` (Multiplayer tab, Join) or `--world <name>` (Worlds tab, Play).
5. The Console tab shows the game's output as today. Nothing is uploaded.

A missing build, or one without its prepared files, gives a sentence on Home saying to ask the
supervisor to install or prepare the game (`club-admin`). The launcher never downloads, never
prepares and never updates anything.

## 6. The server list

`servers.dat` (vanilla's format, `server_list.rs`) is rewritten before every Play and whenever
the Multiplayer tab refreshes: the `[[servers]]` of `club.toml`, then the hosted servers found in
`<root>\online\` whose announcement is under 45 seconds old. An entry accepts no server resource
pack (`acceptTextures: false`). The Multiplayer tab lists the same entries with their status and
offers Join; there is no Add, no Remove.

Announcement, written by the launcher that hosts (section 7) every 10 seconds while the server
runs and deleted when it stops: `online\<COMPUTERNAME>.json`:

```json
{"name": "Monde de Jdupont", "owner": "Jdupont", "host": "192.168.10.42", "port": 25565, "at": 1763800000}
```

`host` is the PC's address on the network the club's servers are on (the local address of a UDP
socket connected towards the first club server, else towards `255.255.255.255`).

Who has the launcher open is said the same way, in `online\people\<player name>.json`, while the
launcher runs:

```json
{"name": "Jdupont", "at": 1763800000}
```

A name that is not a Minecraft name (3 to 16 letters, digits or `_`) is not read: the folder is
every student's to write, and these names go into commands.

The servers a student was invited to are remembered on their own computer, in
`%LOCALAPPDATA%\HorizonClub\launcher\invited.json`: a hosted server they joined, and one a
friend hosts. The sidebar's second group lists them, the announced ones first; the game's list
holds those that are off too, to be joined from the game when they come back. One that nobody
announced for 120 days is forgotten.

## 7. Hosting (the Servers page)

Kept as it works today, with these changes:

- binaries and game files come from the active build in `core\`, read in place: nothing is
  copied into a `server-runtime`;
- the hosted server's `server.toml` points `[engine] resources` at the build's prepared folder
  and plays offline (`online_mode = false`), with the ports of `[hosting]`;
- the EULA file is written when `eula_accepted` is true; with it false the page says the
  supervisor has to accept it;
- no Tailscale, no internet panel, no partner banner;
- the page exists only when `hosting.enabled` is true;
- who may join is picked from cards, never typed: one for each friend and one for each student
  whose launcher is open now. A card's switch makes a friend (`friends.json` in the student's
  launcher folder), and every server the student creates lets their friends in from the start,
  the student being the operator. The Players tab has the same cards, with the buttons that
  make an operator, let in, remove, kick and ban;
- the server's folder is never shown nor opened from the window: the lists say its mode, how
  many players it takes and how many backups it holds, and a game that did not start shows its
  log in the window;
- the world is backed up automatically every 10 minutes while the server runs (12 kept): the
  supervisor's own 6 hours would never come during a club's half hour.

## 8. A student's first Play

When `options.txt` is missing from the game home, the launcher writes it: the values of
`[game]`, the game's language (`lang:fr_fr` when the club's language is `fr`), the music off
(`soundCategory_music:0.0`: the club plays together in a room, and every other sound stays), and
the two switches that skip vanilla's first-start screens (`onboardAccessibility:false`,
`skipMultiplayerWarning:true`). `horizon-graphics.txt` gets the lines of Horizon's
"Performance" preset (`graphics_preset.rs`), so weak PCs start quick. `horizon-extras.txt` gets
`noAi:true`. A student may then change anything in the game's own screens.

## 9. The Versions page

Lists the builds of `core\`: tag, the Minecraft version it plays (`research-lock.json` beside
`horizon-client.exe`), and which one the club uses. No Install, no Remove, no patch notes.

## 10. Settings

Four parts: General (language, what the window does while the game runs), Appearance (theme,
accent, glass, blur, performance mode, motion, effects, background), Game (nothing yet: the
values of `[game]` are the supervisor's), About (the launcher's version, the club's name, the
logs folder, the licences and the "not an official Minecraft product" notice). Nothing else.

## 11. Text

Every visible string goes through `tr` or `trf`. The launcher ships French and English only
(`launcher/lang/fr.json`, `en.json`): French must be complete for everything that remains.
The window opens in French unless the student picked another language in Settings.
