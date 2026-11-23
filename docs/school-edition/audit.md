# Audit de l'édition collège d'Horizon (22 novembre 2026)

Lecture seule du dépôt `horizon-rust/horizon` au commit `f1bb66f353` (main, 22/11/2026) et de son amont, par huit agents. Rien n'a été construit ni lancé pendant l'audit. Les chemins sont relatifs à la racine du dépôt. Conventions : `L/` = `launcher/src/`, `C/` = `crates/horizon-client/src/`, `K/` = `crates/horizon-client-render/src/`, `P/` = `crates/horizon-protocol/src/`.

Tout ce qui suit a été lu dans le code sauf mention « déduit ». La carte détaillée de l'élagage du launcher est dans [launcher-trim-map.md](launcher-trim-map.md).

## 1. Le jeu

### 1.1 Interrupteurs de compilation

- `crates/horizon-client/Cargo.toml:130-173` : `default = ["audio", "controllers", "horizon-extras"]`. `horizon-extras` implique `friends`.
- `crates/horizon-client-lib/Cargo.toml:131-151` : `default = ["audio", "controllers"]`, aucun extra.
- `crates/horizon-game/Cargo.toml:28` expose les mêmes features.
- Build sans la couche Horizon : `cargo build --release -p horizon-game --no-default-features --features audio,controllers` ([docs/forking.md](../forking.md)). « Meilleur effort » : la CI d'amont ne la teste pas comme client.
- `cfg!(feature = "horizon-extras")` n'existe que dans trois fichiers : `C/entry.rs:116-119`, `C/game/interface/cosmetics/mod.rs:334`, `C/game/interface/studio.rs:30`.
- Les parties liées seulement par l'exécutable officiel : `crates/horizon-official/src/lib.rs:15-30` enregistre cinq hooks (heartbeat, friends, serveurs officiels, Discord, partage de captures), puis `main.rs:7-13` appelle `horizon_game::main()`. L'exécutable de `horizon-game` (`lib.rs:28-34`) n'enregistre rien.
- Trait et registres : `crates/horizon-hooks/src/lib.rs` ; scellés au démarrage dans `C/entry.rs:131`.

### 1.2 Ce que la build nue montre encore

- Titre (`C/title/preset_buttons.rs:20-97`) : Solo (1), Multijoueur (4, selon `allows_multiplayer`), Realms (8), Options (3), Quitter (2) ; icônes Mods (9), Amis (5), Langue (6), Accessibilité (7). L'icône Mods reste visible et `C/title.rs:2576` ouvre `Page::Mods`. Amis est dessinée mais inactive sans résolveur de tunnel.
- Options (`K/options_ui/layout.rs:105-159`) : FOV, En ligne, Skin, Son, Vidéo (ouvre l'écran graphique d'Horizon), Commandes, Langue, Chat, Packs de ressources, Accessibilité, puis deux lignes grisées « Telemetry Data » et « Credits & Attribution » quand les extras sont coupés (`:154-166`).
- Pause (`C/game/interface/screens.rs:149-202`) : les boutons Signaler un bug et Envoyer un retour ouvrent des liens aka.ms (`:210-217`).
- Bandeau « RUST EDITION » (`K/title_edition.rs`), ligne playhorizon.dev dans les crédits.
- Navigateur de packs Modrinth : bouton « Add » sur l'écran des packs si `packBrowser:true` dans `horizon-extras.txt` (`K/pack_screen.rs:400-403`). Faux par défaut.
- Aucune boutique ni monnaie dans le jeu.

### 1.3 Toutes les connexions sortantes

| Départ | Chemin | Arrive dans |
|---|---|---|
| Liste, connexion directe, ligne LAN découverte, Realms, `--server` | `C/title.rs:2552` ou `C/game/runtime/loading_window.rs:420-425` → `C/game/world/world_session.rs:58-68` → `C/game/network/remote/connection.rs:92-135` → `Game::join_server` (`C/game/interface/developer_launch.rs:46-61`) → `connection.rs:136-466` → `Session::connect` / `connect_lan` (`C/net.rs:257`, `:272`) → `run` (`:967`) → `connect_tcp` (`:903-919`) | `Connection::connect_server_with_address` (`P/connection.rs:184-192`) |
| Transfert demandé par un serveur | `C/game/network/remote/raw_ingress.rs:246-254` → `Session::connect_transferred` (`C/net.rs:284`) → même `run` | idem, sans passer par `Game::join_server` |
| Solo | `Session::connect_local` (`C/net.rs:309`) | pas de TCP |
| Ping de statut de la liste | `ping_all` (`C/multiplayer.rs:267-366`) → `net::ping` (`C/net.rs:1827`), puis `ping_legacy` (`:1916`) | `Connection::connect_server` |
| Sonde « serveur plus ancien » | `older_server` (`C/net.rs:934`) | `Connection::connect_server` |
| Packs de ressources poussés par un serveur | `C/game/world/server_packs.rs:238-271` | téléchargement HTTP(S) d'une URL arbitraire |
| Realms | `C/realms/client.rs:14` | HTTPS vers `pc.realms.minecraft.net`, seulement avec une session |
| Clés de profil Mojang | `connection.rs:288-305` | HTTPS vers `discovery.minecraft.com`, à chaque connexion, solo compris |

Sans session, le démarrage ne contacte ni Mojang ni Microsoft (`C/title.rs:3687-3698`), et Realms répond « non connecté » sans requête (`C/realms/availability.rs:199-204`).

- `Game::join_server`, ligne 55, refuse déjà toute connexion non locale sous `--developer-single-player` : c'est le modèle pour un refus.
- `parse_address` (`P/servers.rs:228-263`) : hôte, port par défaut 25565, un point final toléré, casse conservée. La recherche SRV n'a lieu que dans `connect_server_with_address` (`P/resolver.rs:36-49`), quand le port est 25565 et le nom n'est ni une IP ni `localhost`.
- Un refus dans `join_server` s'affiche par `Title::failed` (`C/title.rs:1396-1403`) en écran d'erreur avec Retour.

### 1.4 Écran multijoueur (`C/multiplayer.rs`)

- `buttons()` `:644-831`. Identifiants : 40 Rejoindre, 41 Connexion directe, 42 Ajouter, 43 Modifier, 44 Supprimer, 45 Actualiser, 46 Retour. Clics dans `:1477-1743` (41 en `:1504`, 42 en `:1510`, 43 en `:1519`, 44 en `:1536`, 52 « Rejoindre » de la connexion directe en `:1707`).
- Détection LAN : `Scanner::start()` à l'ouverture de la liste (`:638`), UDP 4445, multicast 224.0.2.60 (`C/lan/discovery.rs:136-156`).
- `servers.dat` au format NBT vanilla, à côté de `saves/` ou dans `--player-dir` (`:234-246`). Codec : `P/servers.rs` (champs `name`, `ip`, `icon`, `acceptTextures`, `acceptedCodeOfConduct`, `hidden`).

### 1.5 Ouvrir au LAN

Un seul point de refus : `Game::publish`, `C/game/world/integrated.rs:1296-1315`, qui refuse déjà `developer_single_player`. Toutes les routes y passent (Options du monde, `/publish`, Play with Friends).

### 1.6 Lancement

`C/game/runtime/launch_options.rs` : `--developer-single-player` (`:260`), `--control` (`:269`), `--server` (`:347`), `--name` (`:348`), `--control-socket` (`:350`), `--game-dir` (`:143-147`, change le dossier courant), ressources par défaut (`:183-191`). `--home` et `--player-dir` dans `C/entry.rs:135`, `:145-150`.

- Nom sans compte : `--name`, sinon `HORIZON_PLAYER_NAME`, sinon `horizon-player.txt` (`C/offline_name.rs:1-41`). UUID = MD5 de « OfflinePlayer:nom » (`C/net.rs:1040-1047`).
- Contrôle pour agents : `C/game/runtime/agent_settings.rs:20-44`. `noAi:true` dans `horizon-extras.txt` l'interdit, mais cette ligne est modifiable par l'élève ; `--control` rouvre la socket si elle est absente.

### 1.7 Réglages pour petits PC

`options.txt` (`K/options_file.rs:109-252`) : `renderDistance` 12, `simulationDistance` 12, `graphicsPreset` « fancy », `ao` vrai, `renderClouds` « true », `particles` 0, `entityShadows` vrai, `maxFps` 120, `onboardAccessibility` vrai, `skipMultiplayerWarning` faux.

`horizon-graphics.txt` (`K/graphics.rs:976-1037`) : `preset`, `lodDistance`, `farDetail`, `resolutionScale`, `batteryMode`, `workers`. Les cinq profils sont dans `K/graphics_preset.rs:37-126` ; « Full Performance » vaut 6 chunks, simulation 5, pas de nuages, particules minimales, 75 % de résolution. Un profil ne s'applique que si le joueur le choisit.

`horizon-extras.txt` (`K/extras.rs:305-390`) : `noAi`, `agentControl`, `packBrowser`, `autoReconnect`, `splitscreen`, etc., tous faux par défaut.

### 1.8 Fichiers écrits

- `horizon-data.bin` (`crates/horizon-core/src/prepared_files.rs`) regroupe environ 22 000 petits fichiers. Son en-tête contient le chemin canonique de sa racine (`:19`, `:280-282`) : déplacé ou vu par un autre chemin, il est ignoré en silence. Écrit par le jeu (`C/game/runtime/resource_load.rs:907-936`) et par le launcher (`L/backend.rs:1839-1845`). Un dossier en lecture seule n'est pas fatal.
- Hors du dossier du jeu : `%TEMP%\horizon-control.json` (contrôle ouvert seulement), journaux et rapports de plantage.

### 1.9 Version et protocole

- `crates/horizon-core/src/version.rs` : 26.3, protocole 777, version de monde 5023, Java 25. Une seule version par build.
- Traducteur de protocole `P/adapter.rs` : 14 versions de 1.20.2 à 26.2. Activé par défaut depuis le 21/11/2026 (`C/net.rs:921`, PR #18919) ; `HORIZON_OLDER_SERVERS=0` le coupe.
- Côté serveur, `accept_older_clients` laisse des clients plus anciens entrer.

## 2. Le launcher

### 2.1 Forme

Environ 150 000 lignes : `L/` à plat ~110 000, sous-dossiers ~41 000, `launcher/scenery/` ~3 900.

- Pages : `enum Page` (`L/main.rs:131-143`), `Page::ALL` (`:171-179`), barre haute (`:183-189`) : Accueil, Mods, Serveurs, Versions, Réglages, plus Garde-robe et Boutique quand le service de cosmétiques les montre.
- Onglets d'accueil (`L/home_tabs.rs:7-18`) : Vue d'ensemble, Mods, Mondes, Multijoueur, Packs, Shaders, Captures, Console. Le look « glass » nomme Multijoueur « Saved servers » et Console « Log » (`L/glass_home_tabs.rs:32-41`).
- `struct Launcher` (`L/main.rs:254-638`) porte environ 150 champs. Superpositions (`L/launcher_render.rs:325-416`) : dialogue d'instance, réglages, palette, consentement natif, bienvenue, consentement aux données, porte du compte, logo d'ouverture, création de serveur.
- Démarrage (`L/startup.rs:55-165`) puis `Launcher::new` (`L/launcher_setup.rs`, ~:150-370) : réglages, comptes, trousseau, ressources, têtes de profil, GitHub, mises à jour de mods, `links.json`, boutique, télémétrie.

### 2.2 Comptes

- Porte du propriétaire : `account::launch` (`L/account.rs:1962-2084`), appelée par `L/launcher_play.rs:479-483` (fenêtre) et `L/backend/developer_play.rs:5-51` (CLI, MCP). Le téléchargement Mojang est aussi gated (`L/backend.rs:2135-2153`, `:2268-2311`).
- Sans `HORIZON_MSA_CLIENT_ID` : « non configuré » (`L/account.rs:27`). Cet identifiant est interdit aux forks ([docs/forking.md](../forking.md)).
- Le seul jeu sans compte passe par `L/developer_offline.rs` : builds de développement uniquement, solo uniquement, `--developer-single-player` (`:12-34`, `:93-129`).
- Identifiant Azure et flux détaillés : [docs/microsoft-account.md](../microsoft-account.md).

### 2.3 Lancement du jeu

- Bouton (`L/glass_home.rs:687-693`) → `Launcher::play` (`L/launcher_play.rs:276-305`) → `start_client` (`:406-487`) → `handle_game_session` (`L/developer_play.rs:5-115`) → `backend::spawn` (`L/backend.rs:4466-4532`).
- Commande (`client_command`, `L/backend.rs:3082-3111`) : `horizon-client.exe`, dossier courant = dossier du jeu, `HORIZON_HOME`, `--resources <dossier préparé>`, `--session-file`, arguments du réglage `game_args`, puis `--world` ou `--server`. Jamais `--name`, `--game-dir`, `--player-dir`.
- Sous Windows l'exécutable et ses voisins sont copiés à chaque lancement dans `<données>/run/<empreinte>/game/` (`L/windows_runtime.rs:20-48`, `:93-148`), sans comparaison de taille ni d'empreinte. But : un chemin stable pour le pare-feu.

### 2.4 Installation et préparation

- Listes et archives depuis `api.github.com` et `github.com` (`REPO`, `L/backend.rs:14`, `:673-675`). Sommes vérifiées par une signature Ed25519 dont la clé publique est dans `crates/horizon-release-sign/release-keys.txt` (`L/signed.rs:11-46`).
- Installation dans `<données>/versions/<tag>/` (`L/backend/installation.rs:4-61`). Reprise des ressources d'un build compatible : `L/backend.rs:1768-1832`.
- Préparation (`preparer`, `L/backend.rs:2500-2658`) : fichiers Minecraft depuis une installation existante (`L/sources.rs:405-469`, dossiers connus `:146-152`) ou depuis Mojang pour un compte propriétaire (`L/mojang.rs`). Jar serveur téléchargé depuis le JSON de version (`:2600-2623`). Java 25 : sur la machine, sinon runtime du launcher officiel, sinon téléchargé depuis Mojang (~100 Mo, `:2317-2372`).
- Sortie : `versions/<tag>/.local/horizon-client/<version>/`. Sons ~380 Mo. Sortie déterministe (comparée octet par octet en CI, [docs/launcher-preparation.md](../launcher-preparation.md)).
- Auto-mise à jour du launcher : `stage_update` (`L/backend.rs:4185-4205`), remplacement par renommage (`:3783-3815`), relais `L/update_handoff.rs`. `HORIZON_LAUNCHER_NO_SELF_UPDATE` la coupe (`:3732`). Nouveaux builds du jeu installés automatiquement, contrôle toutes les 30 minutes (`:850`).

### 2.5 Données

- Launcher : `HORIZON_LAUNCHER_HOME`, sinon `%APPDATA%\horizon-launcher` (itinérant) (`L/backend.rs:940-955`). Jeu : `HORIZON_HOME`, sinon `%APPDATA%\horizon` (itinérant) (`:2721-2736`). Pointeur : `%APPDATA%\horizon-launcher-location\location.json` (`L/data_location.rs:32-44`), ignoré quand les deux variables sont posées.
- Réglages : `settings.json` (`L/backend/settings.rs:61-183`), sections de l'écran (`L/glass_settings.rs:16-27`).
- Instances : `instances.json`, une instance par défaut toujours présente (`L/instances.rs:354-362`).
- Programme : `%LOCALAPPDATA%\Programs\Horizon Launcher`, clé `HKCU`, raccourcis (`L/installer.rs:108-260`, `L/shortcut.rs:254-292`).
- Chemin de l'exécutable : utilisé seulement pour la mise à jour, les raccourcis et `HORIZON_LAUNCHER_EXE`. Aucune dépendance au dossier courant.

### 2.6 Lecteurs réseau

- Refus de tout dossier dont le disque n'est pas amovible, fixe, CD-ROM ou RAM : `crates/horizon-mod-package/src/runtime_leases/filesystem.rs:32-75`. Appliqué au dossier de données, au dossier du jeu, aux journaux, au dossier de version et à `--resources` (`L/runtime_leases.rs:104-194`, `:199-233`), message dans `L/refusals.rs:115-135`.
- Les baux servent au nettoyage (ne pas détacher un dossier utilisé par un jeu qui tourne). Le jeu ne participe que si un dossier `run-leases` existe au-dessus de son exécutable (`crates/horizon-mod-package/src/holder/game_entry.rs:19-41`, `:85-101`).
- `is_horizon`, `mark`, `snapshot` (`L/run_leases.rs:12-15`) servent aussi à distinguer le jeu Horizon de l'édition Java : à garder.

### 2.7 Hébergement (page Serveurs)

- `horizon-server.exe` vient du build installé (`can_host`, `L/servers.rs:532-545`). `set_up_in` (`:549-602`) le copie dans `<données>/server-runtime/releases/<tag>` et recopie les fichiers du jeu.
- Ports : jeu à partir de 25565, panneau web à partir de 8130, API de contrôle sur 127.0.0.1 (`:1122-1124`, `:981-991`). Aucune gestion du pare-feu.
- Console et commandes (`L/servers_page.rs:1222-1226`), joueurs, kick, ban, op (`:2080-2291`). EULA écrit par le launcher (`L/servers.rs:3138-3148`). Tailscale facultatif (`L/internet.rs`, `L/tailscale.rs`).

### 2.8 Ligne de commande et MCP

- `enum Cmd` (`L/cli.rs:697-1036`) : `play --server`, `play --arg`, `instance launch --pre-launch <commande>`, `multiplayer add`, `settings`, `install`, `update`, `import-build --trust-unsigned`, etc. Entrée `cli::run` (`:1166-1196`), sans drapeau ni droit.
- MCP : presque chaque commande devient un outil (`L/cli_tools.rs:12-92`, `L/mcp.rs:34-59`).

### 2.9 Traductions et construction

- `tr` et `trf` (`L/i18n.rs:644-660`) ; tables dans `launcher/lang/`, toutes analysées au premier appel (`:622-641`). Le français manque les chaînes récentes de l'accueil et des réglages.
- Build : `cargo build --release --locked` dans `launcher/`. La CI ajoute `RUSTFLAGS=-C target-feature=+crt-static` (`.github/workflows/launcher.yml:169-177`). Version par `HORIZON_LAUNCHER_VERSION`. Icône par `winresource` (`launcher/build.rs`).

## 3. Serveur dédié

- `horizon-server` supervise `horizon-engine` (le jeu lancé avec `--dedicated`). `server.toml` : `online_mode` vrai par défaut, `whitelist` vrai, distances 10. Hors ligne : `online_mode = false`. EULA obligatoire (`eula.txt`, `HORIZON_EULA=true`).
- Ressources préparées obligatoires : dossier `game-files` de l'installation, `resources/` du serveur, ou `[engine] resources`. `horizon-server import-resources DIR`.
- Windows : oui ; `service install` crée une tâche planifiée. Port TCP 25565. Annonce multicast 224.0.2.60:4445 pour les mondes ouverts au LAN seulement.
- Anticheat et filtre de chat intégrés, activés par défaut.

## 4. Amont (22/11/2026)

- Décision du 17/11/2026 : « Jouer sans compte Microsoft » revient (issue #18054, PR #18055 ouverte). Interdiction du 09/11 annulée.
- Mode portable : #17377, ouverte, sans réponse. Rien sur les écoles ni le lancement depuis un partage.
- Découpage en couches : #13012 ouverte, plus de 20 PR M5/M6 en cours. Le launcher quittera le dépôt moteur.
- Nouveau design du launcher : #3635 à #3637. Instances Java : #3915. Toutes les versions préparées : #4758 (plus tard).
- Petites configurations : repli OpenGL livré en v0.2.63.
- Fichiers très actifs : `launcher/src/account.rs`, `backend.rs`, `main.rs`, `crates/horizon-client/src/net.rs`, `multiplayer.rs`, `title/preset_buttons.rs`.
- `horizon-plain` construit la build nue depuis des Nightlies d'au moins une semaine, sans publication GitHub.

## 5. Licence

LGPL-3.0-or-later (`LICENSE`), anciennes contributions sous MIT (`LICENSES/MIT.txt`). Usage interne à un établissement sans distribution : conserver les avis. Ne jamais réutiliser l'identifiant Azure d'Horizon. Ne jamais engager de fichier Mojang.
