# Horizon Club : le launcher du club

Le launcher de l'édition « club » d'[Horizon](https://github.com/horizon-rust/horizon), pour un
club Minecraft de collège : hors ligne (aucun compte), sans boutique ni télémétrie, prêt à
l'emploi. La documentation complète — le partage, la configuration `club.toml`, le déploiement,
le suivi d'Horizon — est dans [`docs/school-edition/README.md`](../docs/school-edition/README.md)
à la racine du dépôt.

Ce dossier contient :

- le launcher (`src/`, exécutable `HorizonClubLauncher`) : la fenêtre que les élèves ouvrent ;
- `club-admin` (`src/bin/club_admin.rs`) : l'outil du responsable du club, qui seul écrit dans
  le dossier du club sur le partage ;
- `bootstrap/` : l'amorce stable `HorizonClub.exe`, cible du raccourci du bureau, qui démarre la
  version courante du launcher (`app\<version>\`) d'après `app\current.txt` ;
- `scenery/` et `lang/fr.json` : le décor de la fenêtre et le français.

## Construire

Sous Windows, avec Rust et les outils MSVC, depuis ce dossier :

```sh
export RUSTFLAGS=-C target-feature=+crt-static   # aucun composant Visual C++ requis sur les PC
cargo build --release
```

L'amorce se construit depuis son propre dossier (`cd bootstrap && cargo build --release`), qui
porte ce réglage dans son `.cargo/config.toml`. Les vérifications après une modification sont
dans `docs/school-edition/README.md`, « Construire ».

Ce launcher est une œuvre dérivée du launcher d'Horizon (LGPL-3.0-or-later). Minecraft est une
marque de Mojang ; ce logiciel n'est ni approuvé ni associé à Mojang ou Microsoft.
