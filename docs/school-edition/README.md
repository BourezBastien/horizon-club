# Horizon Club : édition collège d'Horizon

Plan approuvé le 22 novembre 2026. Ce dossier porte les documents de travail du fork :

- [audit.md](audit.md) : ce que l'audit du dépôt d'origine a établi, avec les emplacements dans le code ;
- [launcher-trim-map.md](launcher-trim-map.md) : la carte d'élagage du launcher (modules à supprimer, références à reprendre, ordre conseillé).

## Le but

Un fork fermé d'Horizon (Minecraft Java 26.3 en Rust) pour un club de collège sur des PC peu puissants : hors ligne (aucun compte Microsoft ou Mojang), sans boutique ni télémétrie, prêt à l'emploi, installé une seule fois sur le partage du serveur de fichiers et mis à jour centralement.

Décisions prises avec le responsable du club :

- onglets gardés dans le launcher : Console (journal du jeu), Mondes, Multijoueur (rejoindre), et la page Serveurs (héberger) ;
- chaque PC peut héberger un serveur : des élèves jouent ensemble sur le leur ;
- le nom du joueur vient du compte Windows ;
- le serveur principal du club est un serveur vanilla ou Paper plus ancien que 26.3.

## Organisation du partage

```
\\SERVEUR\Horizon\                   élèves : lecture seule ; responsable : écriture
├─ HorizonClub.exe                   amorce stable, cible du raccourci bureau
├─ club.toml                         la configuration du club
├─ app\current.txt, app\<version>\   le launcher, versionné : une mise à jour = une ligne
├─ core\<build>\                     horizon-client.exe, horizon-server.exe, horizon-prepare.exe
│   └─ .local\horizon-client\26.3\   les fichiers du jeu : une seule copie pour le collège
├─ online\                           les serveurs en ligne créés par les élèves, et online\people\,
│                                    qui dit qui a ouvert son launcher
└─ admin\club-admin.exe              les outils du responsable

%LOCALAPPDATA%\HorizonClub\          par poste et par session Windows
├─ launcher\                         réglages, journaux, copie locale du jeu, serveurs hébergés
│   (servers\<nom>\), amis (friends.json), serveurs invités (invited.json)
└─ game\                             mondes, options, servers.dat, captures, plantages
```

Un serveur hébergé depuis la page Serveurs vit donc **sur l'ordinateur de l'élève qui l'a créé**, dans `%LOCALAPPDATA%\HorizonClub\launcher\servers\<nom>\` : il n'est pas sur le partage, et il ne tourne que tant que son launcher est ouvert.

## La règle du jeu (la feature `managed`)

Le jeu est construit sans les extras d'Horizon (`--no-default-features`) et avec `managed`. Le launcher lui passe un fichier de règle par `--managed`. Sans règle, un build géré ne joue qu'en solo. Le format et les décisions sont documentés en tête de `crates/horizon-client/src/managed.rs` :

```text
lan:true                       # un monde peut être ouvert au LAN
server:mc-club.example.edu     # une adresse hors du réseau local que les joueurs peuvent rejoindre
                               # (`host` ou `host:port` : 25565 quand le port n'est pas écrit)
```

Ailleurs, la règle est le réseau local : le jeu ne se connecte qu'à une adresse privée (10/8, 172.16/12, 192.168/16, boucle, lien-local, et leurs équivalents IPv6), vérifiée sur l'adresse résolue. Les serveurs hébergés par les élèves depuis la page Serveurs sont annoncés dans `online\` et ne sont acceptés que sur une adresse du réseau local : une annonce y est écrite par n'importe quel élève, elle ne peut donc pas ouvrir Internet.

Sont aussi fermés dans un build géré : Realms, Mods et Amis au menu titre, Connexion directe et Ajouter un serveur, le navigateur de packs, tout appel aux services de Mojang, et la socket de contrôle pour agents du jeu d'un joueur (un serveur dédié garde celle que `horizon-server` utilise pour le piloter).

## Construire

Les quatre programmes. Sous Windows, posez d'abord **`RUSTFLAGS=-C target-feature=+crt-static`** : cela lie l'exécutable C dans le programme. Sans lui, le jeu demande `VCRUNTIME140.dll`, qu'un PC sans composant Visual C++ n'a pas, et il ne démarre pas. C'est le réglage des livraisons Windows d'Horizon, et celui du workflow du fork.

```sh
export RUSTFLAGS=-C target-feature=+crt-static
# le jeu, sans les extras d'Horizon et avec la règle, puis le serveur et l'outil de préparation
cargo build --profile fast -p horizon-game --bin horizon-client --no-default-features --features audio,controllers,managed
cargo build --profile fast -p horizon-prepare -p horizon-server
# le launcher
cargo build --release --manifest-path launcher/Cargo.toml
```

L'amorce porte le réglage dans son propre `.cargo/config.toml`, que cargo ne lit que lorsqu'on lance la compilation depuis son dossier :

```sh
cd launcher/bootstrap && cargo build --release
```

Les vérifications à passer après une modification :

```sh
cargo check -p horizon-game --no-default-features --features audio,controllers,managed
cargo test -p horizon-client-lib --lib managed::
cargo check --manifest-path launcher/Cargo.toml --all-targets
```

`--profile fast` lie vite et sert à essayer. Le profil `release` (que la CI d'Horizon utilise) est plus lent à construire et plus rapide à l'usage.

### Un cœur de jeu (`core\<build>\`)

Le dossier qu'un poste lit sur le partage : `horizon-client.exe`, `horizon-server.exe`, `horizon-prepare.exe`, `research-lock.json`, `LICENSE`, `LICENSES\`, `dxcompiler.dll` et ses trois fichiers de licence (`python tools/ci/fetch_dxc.py <dossier>` : DirectX 12 compile ses shaders avec sur les puces graphiques intégrées, celles des petits PC, où Horizon a mesuré 44 à 65 images par seconde), `exporters\` (les exportateurs Java compilés, facultatif : `horizon-prepare compile-exporters`, qui demande un kit de développement Java 25) et `.local\horizon-client\26.3\` (les fichiers du jeu, environ 650 Mo avec les sons). C'est ce que `cargo xtask package` assemble pour Horizon, moins `horizon-ctl` et la bibliothèque des mods, que l'édition ne livre pas.

Les fichiers du jeu viennent de l'installation Minecraft 26.3 du responsable. `horizon-prepare` importe le client, ses bibliothèques et ses ressources, ainsi que le fichier serveur, exécute les exportateurs et le générateur de données de Mojang, puis recopie les sons :

```sh
horizon-prepare --root <dépôt> --prepare --server-jar <server.jar de 26.3> --minecraft-dir <dossier .minecraft> \
  --output <core>\<build>\.local\horizon-client\26.3 --complete
horizon-prepare --check --root <core>\<build> --output <le même dossier>
```

Horizon ne télécharge jamais les fichiers de Mojang. Une installation du jeu lancée une fois avec le launcher officiel les a tous. Une installation incomplète (celle d'un autre launcher) demande `--libraries-dir` et `--assets-dir` : `club-admin prepare` les complète depuis Mojang, chaque fichier vérifié par l'empreinte que Mojang publie.

## Jalons

| Jalon | Contenu | État au 22/11/2026 |
|---|---|---|
| J0 | Fourche, branche `club`, outillage, audit consigné | en cours |
| J1 | Launcher « Club » : élagage, règle du club, nom Windows, lancement hors ligne, lecteurs réseau | à faire |
| J2 | Cœur de jeu géré : `managed.rs` et ses points d'insertion | écrit, en vérification |
| J3 | Exécutable de test dans `dist\HorizonClub\` | à faire |
| J4 | Déploiement : amorce, `club-admin publish`, raccourci, droits | à faire |
| J5 | Serveurs des élèves, test du serveur principal | à faire |
| J6 | CI du fork et procédure de fusion avec Horizon | à faire |

## Suivre Horizon

Le launcher, les workflows et les outils du club sont propres au fork. Horizon les change tous les jours ; une fusion de `upstream/main` ne doit donc toucher que le moteur :

```sh
git remote add upstream https://github.com/horizon-rust/horizon.git   # une fois
tools/school/merge-upstream.sh                                        # branche main d'upstream, ou passe une autre
```

Le script lance la fusion puis remet `launcher/`, `.github/`, `docs/school-edition/` et `tools/school/` exactement à l'état de la branche `club`. Il reste à résoudre à la main les conflits du moteur, attendus autour de la seule fonctionnalité `managed` (`managed.rs` et ses appels), puis à passer les trois vérifications de la page et à committer. La recette a été éprouvée sur un dépôt-jeton avec les quatre sortes de conflits qu'une fusion produit (fichier modifié des deux côtés, supprimé par le fork, ajouté par Horizon, conflit réel dans le moteur). Committer ou ranger tout avant de l'utiliser : ce qu'aucune branche ne suit est supprimé sous ces dossiers.

La CI du fork (`.github/workflows/club-core.yml`) construit le cœur de jeu Windows à chaque changement du moteur sur `club` : profil `release`, dossier `core\club-<numéro>-<commit>\` en artefact. Elle ne prépare pas les fichiers du jeu (ils viennent de Mojang, sur l'ordinateur du responsable).

À surveiller côté Horizon : #18054 et la PR #18055 (jouer sans compte Microsoft), #17377 (mode portable), #13012 (le launcher quitte le dépôt moteur), #18916 (serveurs anciens par défaut).

## Déployer

Trois étapes, la première et la troisième avec les droits d'un administrateur.

1. **Le partage** (`tools/school/setup-share.ps1`, sur le serveur de fichiers, dans le dossier que le partage montre) : il crée `app\`, `core\`, `online\` et `admin\`, et règle qui écrit où. Le partage lui-même (son nom, qui peut l'ouvrir) se règle avec l'outil du serveur. Le script a été essayé sur un dossier de test ; il accepte `-WhatIf` pour montrer ce qu'il ferait.

   | Dossier | Contenu | Responsable | Élèves |
   |---|---|---|---|
   | racine | `HorizonClub.exe`, `club.toml`, `managed.txt` | écrit | lecture |
   | `app\` | le launcher, version par version, et `current.txt` | écrit | lecture |
   | `core\` | les builds du jeu et leurs fichiers | écrit | lecture |
   | `online\` | les serveurs hébergés par les élèves, annoncés là, et `online\people\` | écrit | écrit |
   | `admin\` | `club-admin.exe` | seul | aucun droit |

2. **Installer** : ouvrir `HorizonClubSetup.exe` (dans la release, voir plus bas), et lui seul. C'est le seul fichier à télécharger, et il ne pose aucune question. Les autres fichiers de la release ne se lancent pas seuls : `HorizonClub.exe` cherche `app\current.txt` à côté de lui, et `HorizonClubLauncher.exe` a besoin de `club.toml` et du jeu ; c'est le dossier entier qui fait Horizon Club. Horizon Club s'installe une seule fois, sur le partage, jamais sur un poste d'élève.

   Il s'installe dans le dossier où il se trouve : le poser dans le dossier du partage (un chemin `\\SERVEUR\Horizon` ou un lecteur réseau mappé) et l'ouvrir là. Il pose l'amorce à la racine, le launcher dans `app\<version>\` avec `current.txt`, `club-admin.exe` dans `admin\`, écrit `managed.txt`, un `club.toml` modèle s'il n'y en a pas, et crée `core\` et `online\people\`. Les fichiers de la release posés à côté de lui servent tels quels (installation sans réseau) ; sinon il les télécharge de GitHub ; dans les deux cas chacun est vérifié par son empreinte.

   **Le jeu**, il le cherche seul, dans cet ordre : déjà dans `core\` du dossier, il le garde ; sinon sur cet ordinateur, dans un dossier de club qui l'a (un dossier où `HorizonClub.exe` a été ouvert), et il le copie dans `core\` ; sinon il le dit en une ligne à la fin : recopier le dossier `core` d'un dossier qui a le jeu à côté de `HorizonClubSetup.exe`, puis rouvrir l'installateur. La release n'en contient aucun fichier : le jeu vient d'un build préparé (par exemple le `core\26.3-…` d'une autre installation). Il écrit son `horizon-data.bin` pour le dossier du club : quelques minutes en tout.

   Rouvert dans un dossier déjà installé, il met le launcher à jour et garde le jeu.

   Pour un script : `admin\club-admin.exe install --root \\SERVEUR\Horizon --game <dossier du jeu> --yes`. `--root` installe dans un autre dossier que celui de l'outil, `--game` dit où prendre le jeu, `--yes` ferme la fenêtre sans attendre Entrée.

   Il reste alors, et l'installateur le dit : corriger `club.toml` puis `club-admin sync`, `tools/school/setup-share.ps1` pour les droits, et le raccourci sur chaque poste. `admin\club-admin.exe check` dit ensuite si le dossier est prêt.

3. **Chaque poste** (`tools/school/setup-pc.ps1`, à la main ou par stratégie de groupe) : le raccourci sur le bureau commun et deux règles de pare-feu, entrantes depuis le réseau local seulement : TCP 25565 à 25575 (les serveurs hébergés depuis la page Serveurs) et UDP 4445 (la découverte des mondes ouverts au réseau local). **Ce script n'a pas pu être essayé** : il demande les droits d'administrateur et modifie la machine.

Une limite à connaître : depuis le menu pause du jeu, « Ouvrir au LAN » prend un port que Windows choisit au hasard, et le jeu a un chemin différent dans le profil de chaque élève. Sans règle dédiée, les autres élèves ne peuvent pas rejoindre un monde ouvert ainsi. L'hébergement depuis la page Serveurs n'a pas cette limite. `[lan] open = false` dans `club.toml` masque l'option.

## Mettre à jour

Le launcher est une seule fois sur le partage : `app\<version>\`, et `app\current.txt` dit la version que les postes démarrent. Une mise à jour se fait donc **une fois, sur le partage** : chaque poste a la nouvelle version à l'ouverture suivante, et rien ne s'installe sur les postes des élèves.

Les versions sont les *releases* du dépôt public du club. Une release porte cinq fichiers : `HorizonClubLauncher.exe`, `HorizonClub.exe` (l'amorce), `club-admin.exe`, `HorizonClubSetup.exe` (l'installateur : le même programme que `club-admin.exe`, sous le nom qui l'ouvre en installant), et `SHA256SUMS.txt`, l'empreinte SHA-256 de chacun. Pour installer, un seul est à télécharger : `HorizonClubSetup.exe`, qui prend les autres lui-même. Les cinq restent dans la release parce que la mise à jour du launcher les y cherche.

- **Depuis le launcher** : à son ouverture, si la session peut écrire dans `app\` (celle du responsable, jamais celle d'un élève), le launcher cherche la dernière release. Quand elle est plus récente que la sienne, il demande « Mettre à jour » : il la télécharge, vérifie son empreinte, la pose dans `app\<version>\` et change `current.txt`. La version d'avant reste ; « Relancer maintenant » rouvre le launcher sur la nouvelle.
- **Sans fenêtre** : `admin\club-admin.exe update` fait la même chose depuis une ligne de commande (sur le serveur de fichiers, à la main ou par une tâche planifiée) ; `--check` dit seulement s'il y en a une plus récente.
- **Revenir en arrière** : remettre l'ancien numéro de version dans `app\current.txt`.

Un élève ne peut rien mettre à jour : sa session lit le partage seulement, et son launcher ne demande rien à Internet. Le téléchargement passe par le moteur de téléchargement du launcher (ses délais, ses reprises, les réglages de proxy de `HORIZON_DOWNLOAD_*`), sous une règle qui n'admet que les serveurs de GitHub (`horizon-addresses`), chaque redirection revérifiée. L'adresse de la release est `CLUB_RELEASES` dans ce même crate ; `HORIZON_CLUB_RELEASES` la remplace.

Seul le launcher se met à jour ainsi : le cœur de jeu (`core\`) se dépose toujours avec `club-admin install-core`, et `club-admin.exe` se recopie à la main depuis la release.

### Publier une nouvelle version

1. Changer `version` dans `launcher/Cargo.toml` et committer : c'est le numéro que le launcher compare à celui de la release.
2. Pousser le tag : `git tag -a v<version> -m "…" && git push origin v<version>`. Le tag est `v` suivi du numéro de `Cargo.toml`, sans quoi les launchers ne voient pas la mise à jour ; une *prerelease* ou un brouillon n'est jamais proposé.
3. Le workflow `club release` (.github/workflows/club-release.yml) construit les trois programmes sur GitHub et les met, avec `SHA256SUMS.txt`, dans la release du tag ; seule la version la plus récente devient « latest ». S'il ne démarre pas tout seul (le jour de la création du dépôt, GitHub ne réagissait pas encore aux envois), le lancer à la main : `gh workflow run club-release.yml --ref v<version>`, puis suivre la page *Actions* du dépôt.

Le jeu, lui, se construit avec le workflow `club core` (`gh workflow run club-core.yml --ref club`) : de 35 à 80 minutes sur un poste prêté par GitHub, en artefact à déposer dans `core\` avec `club-admin install-core`.

Sans le workflow, une release se fait à la main, avec les mêmes fichiers (c'est ce qu'ont fait les versions 0.3.0 et 0.3.1) : construire sous `RUSTFLAGS=-C target-feature=+crt-static` le launcher et `club-admin` (`cargo build --release --manifest-path launcher/Cargo.toml --bin HorizonClubLauncher --bin club-admin`) et l'amorce (`cd launcher/bootstrap && cargo build --release`) ; recopier `club-admin.exe` sous le nom `HorizonClubSetup.exe` ; écrire `SHA256SUMS.txt`, une ligne `<empreinte>  <nom du fichier>` par exécutable (`sha256sum HorizonClubLauncher.exe HorizonClub.exe club-admin.exe HorizonClubSetup.exe > SHA256SUMS.txt`) ; puis `gh release create v<version> --verify-tag --title <version> --notes-file <notes> --latest` avec ces cinq fichiers.

## Points de vigilance

- Aucun fichier Mojang n'entre dans le dépôt. Le responsable les prépare une fois avec `club-admin prepare`, depuis sa propre installation ou depuis Mojang.
- L'identifiant Azure d'Horizon est interdit aux forks. L'édition n'a pas de comptes : le point ne se pose pas.
- Le verrouillage absolu est réseau : le pare-feu ou le proxy du collège, plus une règle Windows entrante TCP 25565-25575 pour les serveurs hébergés.
- `horizon-data.bin` mémorise le chemin exact de son dossier : il doit être écrit sur le partage, par le chemin que voient les élèves. Un build posé avec `club-admin install-core` ou l'installateur n'emporte donc pas celui de son ancien dossier : il en reçoit un nouveau. Un dossier copé à la main garde l'ancien, que le jeu ignore : chaque démarrage lit alors ses 22 000 fichiers un par un, des minutes au lieu de quelques secondes (mesuré sur un même PC : 104 s contre 3 s). `club-admin check` le signale, et `club-admin data-file --tag <build>` le réécrit.
- Les exécutions de test du jeu et du launcher se font dans des dossiers temporaires (`HORIZON_HOME`, `HORIZON_LAUNCHER_HOME`, `XDG_*`), jamais dans les données d'un vrai joueur.
