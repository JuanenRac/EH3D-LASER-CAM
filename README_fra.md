<p align="center">
  <img src="images/EH3D_PHOENIX_LASER_BANNER.svg" alt="EH3D-PHOENIX-LASER banner" width="100%">
</p>

# <img src="images/brand/icon-64.png" alt="" width="40" height="40" align="absmiddle"> EH3D-PHOENIX-LASER
<p align="center"><a href="README.md">🇺🇸 English</a> | <a href="README_spa.md">🇪🇸 Español</a> | 🇫🇷 <b>Français</b> | <a href="README_ita.md">🇮🇹 Italiano</a> | <a href="README_deu.md">🇩🇪 Deutsch</a> | <a href="README_zho.md">🇨🇳 中文</a> | <a href="README_jpn.md">🇯🇵 日本語</a> | <a href="README_por.md">🇵🇹 Português</a></p>

### Gravure laser à partir d'une image et découpe à partir d'un dessin : tramage, G-code raster, contours SVG et DXF et grille d'étalonnage

<p align="center">
  <img src="https://img.shields.io/badge/License-GPL%203.0--or--later-blue.svg" alt="GPL 3.0 or later">
  <img src="https://img.shields.io/badge/Stack-Rust-DEA584.svg" alt="Rust">
  <img src="https://img.shields.io/badge/Languages-8-00E5FF.svg" alt="Languages">
  <img src="https://img.shields.io/badge/Version-0.0.4-0f6cbd.svg" alt="Version">
  <img src="https://img.shields.io/badge/Maturity-scaffolding-FFB020.svg" alt="Maturity">
</p>

---

**Vérification d'honnêteté - ce qui fonctionne aujourd'hui :** une bibliothèque Rust et un programme en ligne de commande (`phoenix-laser`) avec 294 tests qui s'exécutent sur un ordinateur : il transforme une image en points par tramage et écrit le G-code d'un laser qui brûle ligne par ligne avec une puissance dynamique et un dépassement, lit des dessins SVG et DXF et écrit le G-code qui découpe leurs contours (ceux de l'intérieur d'abord, avec la largeur du faisceau, des ponts et plusieurs passes) et grave leurs formes avec des lignes, avec les réglages de chaque calque dans un fichier, et une grille d'essai puissance contre avance. Il a une fenêtre (compilée avec `--features gui`) qui ouvre une image ou un dessin, montre le trajet du laser, permet de changer les réglages et exporte le G-code, mais la fenêtre n'a pas encore été vue sur un écran ; il n'a pas été essayé sur une machine et n'a pas été comparé à une collection de fichiers réels.

---

> **État de sécurité :** ce programme ne commande aucune machine et n'est pas un système de sécurité. Ce qu'il produit doit être vérifié par une personne avant tout essai sur une machine ; rien ici n'a fonctionné sur du matériel réel.


## 📋 En un coup d'œil

| | |
|---|---|
| Version | `0.0.4` |
| Maturité | Échafaudage |
| Tests | 294 |
| Pile technique | Rust |
| Sur du matériel réel | Non essayé : rien ici n'a fonctionné sur une machine |

**Aller à  :** [Présentation](#-présentation) · [Architecture](#-architecture) · [Démarrage rapide](#-démarrage-rapide) · [Tests](#-tests) · [Structure](#-structure-du-dépôt) · [Liés](#-projets-liés)

## 🚦 État du projet

Chaque ligne est une partie de ce dépôt et le feu dit où elle en est aujourd'hui. Elle est écrite d'après ce que le dépôt peut montrer (tests, compilations, exécutions), pas d'après ce qui est prévu.

🟢 Fait et vérifié · 🟡 Fonctionne, avec des limites connues · 🟠 À moitié fait ou pas encore vérifié · 🔴 Pas fait ou ne fonctionne pas

| Domaine | État | Ce que cela veut dire ici |
|---|---|---|
| Cœur | 🟡 | transforme une image en points par tramage et écrit le G-code d'un laser qui brûle ligne par ligne avec une puissance dynamique et un dépassement, lit des dessins SVG et DXF et les découpe ou les grave par calque (contours ordonnés, largeur du faisceau, ponts, passes, hachures), et une grille d'essai puissance contre avance ; testé sur un ordinateur avec des entrées normales et abîmées, pas avec une collection de fichiers réels. |
| Ligne de commande | 🟡 | `phoenix-laser` (`engrave`, `calibrate`, `info`, `cut`, `vector`, `layers`) exécute les mêmes fonctions que la bibliothèque et affiche le résultat ; chaque erreur revient sous forme de message et d'un code de sortie non nul. |
| Tests | 🟡 | 294 tests passent (`cargo test`), avec des cas normaux, des cas limites et des entrées abîmées. |
| Sécurité | 🟡 | Rust sûr et simple (`forbid(unsafe_code)`) : rien ne panique sur les fichiers reçus et une mauvaise valeur est une erreur avec sa raison ; pas de revue indépendante. |
| Fenêtre | 🟠 | Écrite (egui, `--features gui`) : ouvrir ou déposer une image ou un dessin SVG ou DXF, voir le trajet du laser (la découpe d'une couleur, le déplacement atténué), modifier le tableau des calques, une grille d'étalonnage, zoom et déplacement, estimer la durée, exporter ; sa logique est testée, mais elle n'a pas encore été compilée ni vue sur un écran. |
| Matériel réel | 🔴 | Non essayé : rien ici n'a été exécuté sur une machine. |
| Documentation | 🟡 | README en huit langues, le journal des modifications et les deux images dessinées depuis `docs/`. |

## 🎯 Présentation

* **L'image** (`eh3d-image`) : PNG (gris, couleur, transparent, 8 ou 16 bits) et PGM (texte et binaire) deviennent des niveaux de gris, les parties transparentes comptant comme blanc et la taille étant limitée pour qu'un fichier hostile ne puisse pas réclamer des gigaoctets ; `resize` moyenne les pixels pour qu'un point soit aussi gros que le faisceau.
* **Les points** (`src/dither.rs`) : un seuil ou une diffusion d'erreur de Floyd-Steinberg ou d'Atkinson ; les tests vérifient que l'obscurité d'un gris est conservée (un gris moyen brûle environ la moitié des points) et que le blanc ne brûle rien et le noir tout.
* **Le laser** (`src/raster.rs`, `src/calibrate.rs`) : une ligne après l'autre, de gauche à droite puis de droite à gauche, `M4` avec `S0` pendant le déplacement et la puissance choisie là où il brûle, un dépassement fixe ou calculé avec l'accélération (v au carré sur 2a) et une grille de carrés avec une puissance par colonne et une avance par ligne pour trouver ce qui convient à un matériau.
* **L'en-tête et les nombres de tout fichier de G-code** (`src/gcode_out.rs`) : le fichier s'ouvre sur des lignes de commentaire qui disent le générateur et sa version, les unités (mm, G21), l'image dont il vient avec son SHA-256 et les outils (le laser et sa puissance) ; une grille d'étalonnage dit qu'elle n'a pas de source. Un nombre s'écrit d'une seule façon (trois décimales au plus, pas de `-0`) et une valeur non finie est refusée au lieu d'être écrite `NaN` ou `inf`.
* **`--dry-run` :** `engrave` calcule le résultat et le compte (points, lignes, passages, longueur brûlée, temps) sans écrire le fichier de sortie, qui peut alors être omis.
* **Les dessins** (`src/svg.rs`, `src/dxf.rs`, `src/flatten.rs`, `src/vector.rs`) : un sous-ensemble sûr de SVG (tracés avec toutes leurs commandes, formes, transformations, groupes, unités) et les entités d'un DXF (lignes, polylignes avec arcs, arcs, cercles, splines, unités) deviennent des formes en millimètres ; les scripts, les images, `use`, les entités déclarées et tout ce qui est inconnu sont ignorés ou refusés, la taille, la profondeur et le nombre d'éléments et de points sont limités par les réglages, et les courbes deviennent des segments droits dans une tolérance.
* **Découpe et gravure** (`src/order.rs`, `src/offset.rs`, `src/cut.rs`) : les formes sont groupées par calque (DXF) ou par couleur (SVG) ; chaque groupe est découpé (lignes libres jointes, les contours à l'intérieur d'un autre d'abord, le plus court déplacement, la largeur du faisceau retirée du bon côté, ponts laissés, plusieurs passes avec la mise au point qui avance) ou gravé (formes fermées remplies de lignes selon un angle, par la règle pair-impair) avec ses propres réglages.
* **Le fichier de réglages** (`src/vconfig.rs`, `docs/vector-settings.md`) : toute valeur ajustée (les limites des lecteurs, la machine, les réglages de chaque calque) est dans un fichier texte simple vérifié ligne par ligne ; rien de cela n'est dans le code.
* **Un G-code vérifié :** le programme d'un dessin est vérifié ligne par ligne avant d'être livré : aucun laser allumé pendant un déplacement rapide ou sans vitesse, aucun `S` au-dessus de la pleine puissance, aucun déplacement hors du plateau, `M5` à la fin ; le fichier n'est jamais écrit sur le dessin ni sur les réglages dont il provient.

## 🧩 Rôle dans EH3D

EH3D-PHOENIX-LASER est un programme de l'EH3D Fabrication Suite. Il fonctionne comme bibliothèque et ligne de commande et, compilé avec `--features gui`, comme fenêtre avec l'aspect commun aux autres programmes de la suite. Il lit et écrit des fichiers simples et peut donc s'utiliser seul ou entre les autres programmes.

## 🏛️ Architecture

<p align="center">
  <img src="images/architecture.svg" alt="Architecture de EH3D-PHOENIX-LASER : qui l'utilise, ce qu'il contient et de quoi il dépend" width="100%">
</p>


Comment les parties de ce dépôt s'articulent : qui l'utilise, ce qu'il contient et de quoi il dépend. L'image est dessinée depuis `docs/architecture.json` et vérifiée avec le code.

## ✨ Fonctionnalités

* **Une bibliothèque et une ligne de commande :** chaque commande de `phoenix-laser` est une fonction de la bibliothèque ; un autre programme de la suite peut l'appeler sans lancer de processus.
* **Ligne de commande stricte :** une option mal écrite (`--powr`) ou un mot en trop est une erreur qui la nomme, et non quelque chose d'ignoré ; sinon le programme ferait sa sortie avec les valeurs par défaut.
* **Tests avec entrées abîmées :** les tests fournissent aussi des fichiers vides, du bruit, des nombres énormes et des fichiers coupés, et aucun ne doit paniquer.
* **Sans deviner :** une valeur absurde est une erreur qui dit laquelle et pourquoi, jamais un résultat qui n'en a que l'air.
* **Pas de code unsafe :** `#![forbid(unsafe_code)]` dans la bibliothèque et le programme.

## 🔒 Sécurité et protection

| Menace | Défense |
|---|---|
| Un fichier abîmé, coupé ou vide | pas de panique : les tests coupent une entrée valide à chaque position et fournissent du bruit |
| Un nombre absurde (négatif, non fini, démesuré) | refusé avec une erreur qui nomme la valeur ; rien n'est arrondi en quelque chose de plausible |
| Usage non sûr de la mémoire | Rust sûr et simple avec `forbid(unsafe_code)` |
| Un dessin hostile (scripts, entités déclarées, imbrication profonde, millions de points, nombres énormes) | les scripts, les images et les liens ne sont jamais exécutés ni téléchargés ; une entité déclarée est une erreur ; la taille, la profondeur, les éléments, les formes, les points et les morceaux d'une courbe sont limités ; une forme avec un point hors limites est laissée de côté |
| Un programme qui pourrait allumer le laser sans vitesse ou au-dessus de la pleine puissance | chaque programme est vérifié ligne par ligne avant d'être livré, et celui qui échoue n'est pas écrit |
| Le fichier de sortie choisi sur le dessin ou sur les réglages dont il provient | refusé, dans la fenêtre et dans les commandes |

**Ce qui n'est pas défendu :** un résultat produit sans erreur n'est pas un résultat vérifié. Rien ici ne connaît les limites d'une machine et le programme n'a pas eu de revue indépendante. Signalez une vulnérabilité en privé (voir [SECURITY.md](SECURITY.md)).

## 🔌 Interfaces

La bibliothèque est `eh3d_phoenix_laser` (`use eh3d_phoenix_laser::...`) ; la ligne de commande est `phoenix-laser`. Les commandes sont celles-ci ; chacune affiche son résultat sur la sortie standard et signale une erreur sur la sortie d'erreur avec un code de sortie non nul. Il n'y a pas d'accès réseau.

```text
phoenix-laser engrave IMAGE OUT.nc --width-mm 50 [--dot-mm 0.1] [--feed 3000] [--power 50] [--smax 1000]
              [--dither fs|atkinson|threshold] [--threshold 128] [--overscan MM|auto] [--accel 1000] [--origin X,Y] [--dry-run]
phoenix-laser calibrate OUT.nc --powers 20,40,60,80 --feeds 1000,2000,3000 [--smax 1000]
phoenix-laser info IMAGE
phoenix-laser cut DRAWING OUT.nc [--config FILE] [--power P] [--speed MM/MIN] [--passes N] [--focus-step MM] [--kerf MM] [--kerf-side none|auto|inside|outside]
              [--tabs N] [--tab-width MM] [--smax N] [--overscan MM] [--origin X,Y] [--bed X,Y] [--dpi N] [--tolerance MM] [--dry-run]
phoenix-laser vector DRAWING OUT.nc [the options of cut] [--angle DEG] [--spacing MM]
phoenix-laser layers DRAWING [--config FILE] [--dpi N]
phoenix-laser [gui] [FILE]                       the window (a build with --features gui); with no arguments it opens
```

`DRAWING` est un fichier SVG ou DXF. `cut` suit les contours (ceux de l'intérieur d'abord) ; `vector` remplit les formes fermées de lignes et suit les ouvertes ; les deux prennent les réglages de chaque calque ou couleur dans `--config FILE` (voir [docs/vector-settings.md](docs/vector-settings.md)) et une option de la ligne de commande les change pour tous les calques. `layers` liste les calques d'un dessin avec ce qui serait fait de chacun.

## ⚙️ Configuration

La gravure d'une image se règle avec des arguments de la ligne de commande (voir Interfaces) et chaque valeur par défaut est écrite dans l'aide de la commande (`phoenix-laser help`). La découpe et la gravure d'un dessin se règlent avec un fichier de réglages (`--config FILE`, ou « Charger les réglages » dans la fenêtre) : les limites des lecteurs, la façon de lire un fichier, l'ordre des découpes, la machine et, pour chaque calque ou couleur, le mode (découper, graver ou ignorer), la puissance, la vitesse, les passes, le pas de la mise au point, la largeur du faisceau et son côté, les ponts et les hachures. Chaque clé, sa valeur par défaut et sa plage sont dans [docs/vector-settings.md](docs/vector-settings.md) et un exemple est dans `examples/plate.cfg` ; une clé inconnue ou une valeur hors plage est une erreur qui nomme sa ligne. Les limites fixes du lecteur d'images sont des constantes nommées dans le code, couvertes par des tests.

## 🚀 Démarrage rapide

Il faut une chaîne d'outils Rust stable (1.88 ou plus récente). Le dépôt EH3D-COMMON doit être cloné à côté de celui-ci : le lecteur d'images en est pris par chemin.

```bash
cargo build --release
cargo test
cargo run --release -- engrave examples/gradient.pgm examples/gradient.nc --width-mm 16 --dot-mm 0.5 --overscan auto
cargo run --release -- cut examples/plate.svg examples/plate.nc --config examples/plate.cfg
cargo run --release --features gui   # the window
```

## 🧪 Tests

La suite compte 294 tests (`cargo test`, exécutés sous Windows et Linux dans le CI). Ils vérifient les cas normaux, les cas limites et les entrées abîmées : fichiers vides, bruit et fichiers coupés ne font jamais paniquer. Le CI exécute aussi `cargo fmt --check`, `cargo clippy` avec les avertissements comme erreurs et `python -I tools/ci_validate.py`.

## 🏗️ Compilation et publication

La version se trouve dans `Cargo.toml` et `eh3d.project.json` ; chaque version a son entrée dans [CHANGELOG.md](CHANGELOG.md). `build.bat` / `build.sh` compilent le programme, `test.bat` / `test.sh` lancent les tests et `run.bat` / `run.sh` l'exécutent avec un exemple. Les versions sont publiées sur GitHub.

**Téléchargements :** Avec chaque version de ce dépôt sont publiés des programmes prêts à l'emploi pour chaque système : Windows (64 et 32 bits), Linux (64 bits) et macOS (Apple silicon et Intel). Chaque paquet, `phoenix-laser-<version>-<système>.zip` ou `.tar.gz`, contient le programme, la licence, ce README, le journal des modifications, l'icône et les fichiers d'exemple, et vient avec son fichier `.sha256` ; la version liste aussi tous les fichiers dans `SHA256SUMS` et `build-manifest.json`. Aucune version n'a encore été publiée.

**Signés et vérifiables :** les paquets d'une version sont signés avec la clé de publication d'EH3D, et le fichier `.sig` de chaque paquet est publié à côté (une signature OpenSSH, espace de noms `eh3d-release`). Vérifiez un téléchargement avant de l'exécuter : comparez son SHA-256 (`sha256sum -c phoenix-laser-<version>-<système>.tar.gz.sha256`, ou `Get-FileHash` sous Windows) et vérifiez la signature avec `ssh-keygen -Y verify -f allowed_signers -I eh3d-release -n eh3d-release -s <paquet>.sig < <paquet>`, où `allowed_signers` contient la ligne `eh3d-release <type de clé> <clé publique des versions d'EH3D>`.

## 📂 Structure du dépôt

<p align="center">
  <img src="images/repository-structure.svg" alt="Structure du dépôt EH3D-PHOENIX-LASER" width="100%">
</p>

<details>
<summary>Version texte de l'arborescence</summary>

```text
EH3D-PHOENIX-LASER/
├── src/
│   ├── lib.rs                                   la bibliothèque : ce que chaque commande utilise
│   ├── main.rs                                  la ligne de commande : engrave, calibrate, info, cut, vector et layers
│   ├── calibrate.rs                             une partie de la logique
│   ├── cli_args.rs                              les arguments de la ligne de commande, le même fichier dans chaque petit utilitaire
│   ├── dither.rs                                une partie de la logique
│   ├── gcode_out.rs                             la seule façon d'écrire un nombre de G-code et l'en-tête standard
│   ├── raster.rs                                une partie de la logique
│   ├── vector.rs                                un dessin lu : formes en mm, calques, limites et géométrie de base
│   ├── flatten.rs                               courbes en segments droits dans une tolérance (Bézier, arcs)
│   ├── svg.rs                                   le lecteur SVG : un sous-ensemble sûr, sans scripts, images ni entités
│   ├── dxf.rs                                   le lecteur DXF : lignes, polylignes, arcs, cercles et splines, avec unités
│   ├── offset.rs                                décaler un contour de la largeur du faisceau, vérifié arête par arête
│   ├── order.rs                                 l'ordre des découpes : lignes jointes, l'intérieur d'abord, peu de déplacement
│   ├── cut.rs                                   du dessin au G-code : découpe, ponts, hachures, passes et vérification de sécurité
│   ├── vconfig.rs                               le fichier de réglages du travail vectoriel : limites, machine et chaque calque
│   ├── browse.rs                                la liste des fichiers de la fenêtre d'ouverture d'un fichier
│   ├── gui.rs                                   la fenêtre (egui, avec la fonctionnalité gui) : dessine l'état et transmet ce que fait la personne
│   ├── lang.rs                                  les mots de la fenêtre dans les huit langues
│   ├── viewport.rs                              zoom et déplacement de l'image, en simples nombres
│   └── window.rs                                l'état de la fenêtre sans écran : image ou dessin, réglages, calques, passes et export
├── tests/
│   ├── dry_run.rs                               tests de --dry-run : compte la même chose et n'écrit rien
│   ├── laser.rs                                 tests de la bibliothèque et de la ligne de commande
│   ├── readme.rs                                tests qui exécutent ce que promettent le README et les scripts run
│   ├── strict.rs                                tests de la ligne de commande : options et mots qu'elle ne connaît pas
│   ├── vector.rs                                tests de la découpe depuis des dessins : commandes, exemples, fichiers hostiles et vérification du G-code
│   └── window_cli.rs                            tests qu'une compilation sans la fonctionnalité gui reste une ligne de commande
├── examples/
│   ├── gradient.pgm                             un exemple pour essayer le programme
│   ├── plate.svg                                une plaque avec un trou et un panneau à graver (SVG)
│   ├── plate.dxf                                la même plaque en DXF
│   └── plate.cfg                                les réglages de ses calques : puissance, vitesse, passes, largeur du faisceau, ponts et hachures
├── docs/
│   ├── architecture.json                        données de l'image d'Architecture
│   ├── structure.json                           données de l'image de Structure
│   └── vector-settings.md                       le fichier de réglages du travail vectoriel, clé par clé
├── tools/                                       contrôle de la CI et du README et outils d'images
├── images/                                      bannière et images d'Architecture et de Structure du dépôt
├── .github/                                     workflows de CI et de publication, modèles d'issues et de pull requests
├── eh3d.project.json                            manifeste : version, rôle, maturité et commande de compilation
├── Cargo.toml Cargo.lock                        le workspace et ses dépendances verrouillées
├── CHANGELOG.md                                 ce qui a changé dans chaque version
├── SECURITY.md SUPPORT.md                       signaler une vulnérabilité et poser des questions
└── CONTRIBUTING.md CODE_OF_CONDUCT.md LICENSE   contribuer, code de conduite et licence
```

</details>

## 🗺️ Feuille de route et limites connues

**Limites connues aujourd'hui**

* La fenêtre n'a pas encore été vue sur un écran.
* Il n'a pas été essayé sur une machine ni avec une collection de fichiers réels et n'a pas eu de revue indépendante.
* Un dessin est lu dans un sous-ensemble sûr (pas de `use`, de texte, de dégradés ni de styles à sélecteurs ; pas d'ellipses, de hachures ni de blocs dans un DXF) ; un contour plus étroit que le faisceau n'est pas découpé et le programme le dit ; le laser est celui d'un contrôleur GRBL avec `M4`, et rien n'a été essayé sur un vrai laser ni un vrai matériau.

**Ensuite**

* Montrer la densité de puissance sur la carte avant de brûler et garder la puissance constante dans les angles d'une découpe.
* Voir la fenêtre sur un écran et la terminer.
* L'essayer avec de vrais dessins et avec les autres programmes de la suite.

## 🌍 Langues

Le README et les textes existent en anglais, espagnol, français, italien, allemand, chinois, japonais et portugais européen.

## 🔗 Projets liés

* [EH3D-COMMON](https://github.com/JuanenRac/EH3D-COMMON) : les crates partagés, dont le lecteur d'images
* [EH3D-STENCILCAM](https://github.com/JuanenRac/EH3D-STENCILCAM): les pochoirs de pâte à braser
* [EH3D-CAM-DEBUGGER](https://github.com/JuanenRac/EH3D-CAM-DEBUGGER): la vérification du G-code avant qu'il n'atteigne une machine
* [EH3D-TOOL-MANAGER](https://github.com/JuanenRac/EH3D-TOOL-MANAGER): le magasin d'outils partagé par les programmes CAM
* [EH3D-MOTIONCNC](https://github.com/JuanenRac/EH3D-MOTIONCNC): le programme de commande de machine
* [EH3D-SUITE-LAUNCHER](https://github.com/JuanenRac/EH3D-SUITE-LAUNCHER): le centre qui trouve et lance les programmes de la suite

## 📚 Documentation

* [Journal des modifications](CHANGELOG.md)
* [Données d'architecture et de structure](docs/architecture.json) : les données qui servent à dessiner les deux images.
* [Réglages du travail vectoriel](docs/vector-settings.md) : chaque clé du fichier de réglages avec sa valeur par défaut et sa plage.

## 🤝 Contribuer et communauté

* Avant une pull request, lancez `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test` et `python -I tools/ci_validate.py` : tout doit passer. Voir [CONTRIBUTING.md](CONTRIBUTING.md) et le [code de conduite](CODE_OF_CONDUCT.md).
* Le README existe en huit langues : un changement de texte se fait dans les huit fichiers, et les deux images sont dessinées depuis `docs/architecture.json` et `docs/structure.json` par `tools/make_architecture.py` et `tools/make_structure.py`.
* Signalez une vulnérabilité en privé : [SECURITY.md](SECURITY.md). Questions, idées et rapports : electrohobby3d@gmail.com ([SUPPORT.md](SUPPORT.md)).

## 👤 AUTEUR

**JuanenRac (Electro Hobby 3D)** · electrohobby3d@gmail.com

## 📜 LICENCE

Logiciel : GPL-3.0-or-later. Voir [LICENSE](LICENSE). Les conceptions matérielles de la suite utilisent la CERN OHL-S.
