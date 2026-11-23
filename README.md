# Horizon Club

**L'édition collège d'[Horizon](https://github.com/horizon-rust/horizon)** : Minecraft: Java Edition 26.3, réécrit en Rust, préparé pour le club Minecraft d'un collège et ses PC peu puissants. Pas de compte à créer, pas de boutique ni de télémétrie, un réseau fermé à celui du club, et une installation unique sur un partage de fichiers que tous les postes suivent.

## Ce que fait l'édition

- **Une fenêtre, un bouton Jouer.** Le nom du joueur vient de la session Windows de l'élève.
- **Le réseau du club seulement.** Les élèves rejoignent les serveurs que le responsable inscrit dans `club.toml`, et ceux que d'autres élèves hébergent sur le réseau local.
- **Héberger un serveur en quelques clics** depuis la page Serveurs : le créer (survie, créatif, aventure), le démarrer, dire qui peut entrer, changer ses réglages, le sauvegarder.
- **Installé une seule fois.** Tout est sur un partage (`\\serveur\Horizon\`) ; un raccourci sur le bureau de chaque poste suffit. Les mondes et les réglages d'un élève restent sur son poste.
- **Mis à jour au même endroit.** Le launcher vit en un seul exemplaire sur le partage : une mise à jour là-bas, et chaque poste a la nouvelle version à son ouverture suivante.

## Commencer

Tout est expliqué dans **[docs/school-edition/README.md](docs/school-edition/README.md)** : construire, organiser le partage, déployer sur les postes, mettre à jour.

Les exécutables d'une version sont dans [Releases](../../releases) :

| Fichier | Rôle | Où sur le partage |
|---|---|---|
| `HorizonClub.exe` | l'amorce : la cible du raccourci de chaque poste | racine |
| `HorizonClubLauncher.exe` | le launcher | `app\<version>\` |
| `club-admin.exe` | l'outil en ligne de commande du responsable | `admin\` |
| `SHA256SUMS.txt` | l'empreinte de chacun | avec eux |

## Licence et mentions

Le code est sous **LGPL-3.0-or-later**, comme Horizon. Ce dépôt est un fork de [horizon-rust/horizon](https://github.com/horizon-rust/horizon) et en garde l'historique ; l'édition collège (le launcher, la règle du jeu `managed`, les outils de `tools/school/`) est décrite dans `docs/school-edition/`.

Aucun fichier de Minecraft n'est fourni ni compris dans ce dépôt : il faut une installation de **Minecraft: Java Edition**, dont le responsable du club prépare une fois les fichiers du jeu.

<sub>NOT AN OFFICIAL MINECRAFT PRODUCT. NOT APPROVED BY OR ASSOCIATED WITH MOJANG OR MICROSOFT. Minecraft is a trademark of Mojang Synergies AB.</sub>

<sub>Horizon Club est un projet indépendant et non officiel : le projet Horizon ne le soutient pas plus que Mojang ou Microsoft.</sub>
