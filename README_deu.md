<p align="center">
  <img src="images/EH3D_PHOENIX_LASER_BANNER.svg" alt="EH3D-PHOENIX-LASER banner" width="100%">
</p>

# <img src="images/brand/icon-64.png" alt="" width="40" height="40" align="absmiddle"> EH3D-PHOENIX-LASER
<p align="center"><a href="README.md">🇺🇸 English</a> | <a href="README_spa.md">🇪🇸 Español</a> | <a href="README_fra.md">🇫🇷 Français</a> | <a href="README_ita.md">🇮🇹 Italiano</a> | 🇩🇪 <b>Deutsch</b> | <a href="README_zho.md">🇨🇳 中文</a> | <a href="README_jpn.md">🇯🇵 日本語</a> | <a href="README_por.md">🇵🇹 Português</a></p>

### Lasergravur aus einem Bild und Schneiden aus einer Zeichnung: Dithering, Raster-G-Code, SVG- und DXF-Konturen und Kalibrierraster

<p align="center">
  <img src="https://img.shields.io/badge/License-GPL%203.0--or--later-blue.svg" alt="GPL 3.0 or later">
  <img src="https://img.shields.io/badge/Stack-Rust-DEA584.svg" alt="Rust">
  <img src="https://img.shields.io/badge/Languages-8-00E5FF.svg" alt="Languages">
  <img src="https://img.shields.io/badge/Version-0.0.4-0f6cbd.svg" alt="Version">
  <img src="https://img.shields.io/badge/Maturity-scaffolding-FFB020.svg" alt="Maturity">
</p>

---

**Ehrlichkeitscheck - was heute wirklich läuft:** eine Rust-Bibliothek und ein Kommandozeilenprogramm (`phoenix-laser`) mit 304 Tests, die auf einem Computer laufen: wandelt ein Bild per Dithering in Punkte um und schreibt den G-Code eines Lasers, der Zeile für Zeile mit dynamischer Leistung und Überlauf brennt, liest SVG- und DXF-Zeichnungen und schreibt den G-Code, der ihre Konturen schneidet (die inneren zuerst, mit Strahlbreite, Stegen und mehreren Durchgängen) und ihre Formen mit Linien graviert, mit den Einstellungen jeder Ebene in einer Datei, sowie ein Testraster Leistung gegen Vorschub. Es hat ein Fenster (mit `--features gui` gebaut), das ein Bild oder eine Zeichnung öffnet, den Weg des Lasers zeigt, die Einstellungen ändern lässt und den G-Code exportiert, aber das Fenster wurde noch nicht auf einem Bildschirm angesehen; es wurde nicht an einer Maschine erprobt und nicht mit einer Sammlung echter Dateien geprüft.

---

> **Sicherheitsstatus:** Dieses Programm steuert keine Maschine und ist kein Sicherheitssystem. Was es erzeugt, muss ein Mensch prüfen, bevor etwas an einer Maschine läuft; nichts davon lief auf echter Hardware.


## 📋 Auf einen Blick

| | |
|---|---|
| Version | `0.0.4` |
| Reifegrad | Gerüst |
| Tests | 304 |
| Technologie-Stack | Rust |
| Auf echter Hardware | Nicht erprobt: nichts davon lief an einer Maschine |

**Springe zu:** [Überblick](#-überblick) · [Architektur](#-architektur) · [Schnellstart](#-schnellstart) · [Tests](#-tests) · [Struktur](#-struktur-des-repositorys) · [Verwandte](#-verwandte-projekte)

## 🚦 Projektstatus

Jede Zeile ist ein Teil dieses Repositorys, und die Ampel sagt, wie weit er heute ist. Sie beruht auf dem, was das Repository zeigen kann (Tests, Builds, Läufe), nicht auf dem, was geplant ist.

🟢 Fertig und überprüft · 🟡 Funktioniert, mit bekannten Grenzen · 🟠 Teilweise fertig oder noch nicht überprüft · 🔴 Nicht gemacht oder funktioniert nicht

| Bereich | Stand | Was das hier bedeutet |
|---|---|---|
| Kern | 🟡 | wandelt ein Bild per Dithering in Punkte um und schreibt den G-Code eines Lasers, der Zeile für Zeile mit dynamischer Leistung und Überlauf brennt, liest SVG- und DXF-Zeichnungen und schneidet oder graviert sie ebenenweise (geordnete Konturen, Strahlbreite, Stege, Durchgänge, Schraffur) sowie ein Testraster Leistung gegen Vorschub; auf einem Computer mit normalen und beschädigten Eingaben getestet, nicht mit einer Sammlung echter Dateien. |
| Kommandozeile | 🟡 | `phoenix-laser` (`engrave`, `calibrate`, `info`, `cut`, `vector`, `layers`) führt dieselben Funktionen wie die Bibliothek aus und gibt das Ergebnis aus; jeder Fehler kommt als Meldung und mit einem Exit-Code ungleich null zurück. |
| Tests | 🟡 | 304 Tests bestehen (`cargo test`), mit Normalfällen, Grenzfällen und beschädigten Eingaben. |
| Sicherheit | 🟡 | Reines sicheres Rust (`forbid(unsafe_code)`): nichts gerät bei den übergebenen Dateien in Panik und ein schlechter Wert ist ein Fehler mit Begründung; keine unabhängige Prüfung. |
| Fenster | 🟠 | Geschrieben (egui, `--features gui`): ein Bild oder eine SVG- oder DXF-Zeichnung öffnen oder ablegen, den Weg des Lasers sehen (der Schnitt in einer Farbe, der Leerweg abgeschwächt), die Ebenentabelle bearbeiten, ein Kalibrierraster, Zoom und Verschieben, die Zeit schätzen, exportieren; die Logik ist getestet, aber das Fenster wurde noch nicht gebaut oder auf einem Bildschirm angesehen. |
| Echte Hardware | 🔴 | Nicht erprobt: nichts davon wurde an einer Maschine ausgeführt. |
| Dokumentation | 🟡 | README in acht Sprachen, das Changelog und die zwei aus `docs/` gezeichneten Bilder. |

## 🎯 Überblick

* **Das Bild** (`eh3d-image`): PNG (Grau, Farbe, transparent, 8 oder 16 Bit) und PGM (Text und binär) werden zu Graustufen, transparente Teile als Weiß, die Größe ist begrenzt, damit eine feindliche Datei keine Gigabytes verlangen kann; `resize` mittelt die Pixel, sodass ein Punkt so groß wie der Strahl ist.
* **Die Punkte** (`src/dither.rs`): ein Schwellwert oder Fehlerdiffusion nach Floyd-Steinberg oder Atkinson; die Tests prüfen, dass die Dunkelheit eines Grautons erhalten bleibt (ein mittleres Grau brennt etwa die Hälfte der Punkte) und dass Weiß nichts und Schwarz alles brennt.
* **Der Laser** (`src/raster.rs`, `src/calibrate.rs`): Zeile für Zeile, von links nach rechts und dann von rechts nach links, `M4` mit `S0` beim Verfahren und der gewählten Leistung beim Brennen, ein fester oder aus der Beschleunigung berechneter Überlauf (v zum Quadrat durch 2a) und ein Raster aus Quadraten mit einer Leistung je Spalte und einem Vorschub je Zeile, um das Passende für ein Material zu finden.
* **Der Kopf und die Zahlen jeder G-Code-Datei** (`src/gcode_out.rs`): Die Datei beginnt mit Kommentarzeilen, die den Generator und seine Version, die Einheiten (mm, G21), das Bild, aus dem sie entstand, mit seinem SHA-256 und die Werkzeuge (den Laser und seine Leistung) nennen; ein Kalibrierraster gibt an, dass es keine Quelle hat. Eine Zahl wird nur auf eine Weise geschrieben (höchstens drei Dezimalstellen, kein `-0`), und ein Wert, der nicht endlich ist, wird abgelehnt, statt als `NaN` oder `inf` geschrieben zu werden.
* **`--dry-run`:** `engrave` berechnet das Ergebnis und zählt es (Punkte, Zeilen, Läufe, gebrannte Länge, Zeit), ohne die Ausgabedatei zu schreiben, die dann entfallen darf.
* **Die Zeichnungen** (`src/svg.rs`, `src/dxf.rs`, `src/flatten.rs`, `src/vector.rs`): eine sichere Teilmenge von SVG (Pfade mit allen Befehlen, Formen, Transformationen, Gruppen, Einheiten) und die Elemente einer DXF (Linien, Polylinien mit Bögen, Bögen, Kreise, Splines, Einheiten) werden zu Formen in Millimetern; Skripte, Bilder, `use`, deklarierte Entitäten und alles Unbekannte werden übersprungen oder abgelehnt, Größe, Tiefe sowie Zahl der Elemente und Punkte sind durch die Einstellungen begrenzt, und Kurven werden innerhalb einer Toleranz zu geraden Stücken.
* **Schneiden und Gravieren** (`src/order.rs`, `src/offset.rs`, `src/cut.rs`): Die Formen werden nach Ebene (DXF) oder Farbe (SVG) gruppiert; jede Gruppe wird mit ihren eigenen Einstellungen geschnitten (lose Linien verbunden, die Konturen innerhalb einer anderen zuerst, kürzester Leerweg, die Strahlbreite auf der richtigen Seite abgezogen, Stege ungeschnitten, mehrere Durchgänge mit nachgeführtem Fokus) oder graviert (geschlossene Formen mit Linien in einem Winkel gefüllt, nach der Gerade-Ungerade-Regel).
* **Die Einstellungsdatei** (`src/vconfig.rs`, `docs/vector-settings.md`): Jeder abgestimmte Wert (die Grenzen der Leser, die Maschine, die Einstellungen jeder Ebene) steht in einer einfachen Textdatei, die Zeile für Zeile geprüft wird; nichts davon steckt im Code.
* **Ein geprüfter G-Code:** Das Programm einer Zeichnung wird vor der Ausgabe Zeile für Zeile geprüft: kein Laser an bei einer Eilbewegung oder ohne Geschwindigkeit, kein `S` über voller Leistung, keine Bewegung außerhalb des Bettes, `M5` am Ende; die Datei wird nie über die Zeichnung oder die Einstellungen geschrieben, aus denen sie stammt.

## 🧩 Rolle in EH3D

EH3D-PHOENIX-LASER ist ein Programm der EH3D Fabrication Suite. Es arbeitet als Bibliothek und Kommandozeile und, mit `--features gui` gebaut, als Fenster im gemeinsamen Aussehen der Suite. Es liest und schreibt einfache Dateien und lässt sich allein oder zwischen den anderen Programmen nutzen.

## 🏛️ Architektur

<p align="center">
  <img src="images/architecture.svg" alt="Architektur von EH3D-PHOENIX-LASER: wer es nutzt, was es enthält und wovon es abhängt" width="100%">
</p>


Wie die Teile dieses Repositorys zusammenpassen: wer es nutzt, was es enthält und wovon es abhängt. Das Bild wird aus `docs/architecture.json` gezeichnet und gegen den Code geprüft.

## ✨ Funktionen

* **Bibliothek und Kommandozeile:** Jeder Befehl von `phoenix-laser` ist eine Funktion der Bibliothek, sodass ein anderes Programm der Suite ihn ohne Prozessstart aufrufen kann.
* **Strenge Kommandozeile:** Eine falsch geschriebene Option (`--powr`) oder ein überzähliges Wort ist ein Fehler, der sie nennt, und wird nicht ignoriert; sonst würde das Programm seine Ausgabe mit den Standardwerten erzeugen.
* **Tests mit beschädigten Eingaben:** Die Tests füttern auch leere Dateien, Datenmüll, riesige Zahlen und abgeschnittene Dateien; keiner darf in Panik geraten.
* **Kein Raten:** Ein unsinniger Wert ist ein Fehler, der sagt welcher und warum, nie ein Ergebnis, das nur richtig aussieht.
* **Kein unsafe-Code:** `#![forbid(unsafe_code)]` in Bibliothek und Programm.

## 🔒 Sicherheit und Schutz

| Bedrohung | Schutz |
|---|---|
| Eine beschädigte, abgeschnittene oder leere Datei | keine Panik: die Tests schneiden gültige Eingaben an jeder Stelle ab und füttern Datenmüll |
| Eine unsinnige Zahl (negativ, nicht endlich, absurd groß) | wird mit einem Fehler abgelehnt, der den Wert nennt; nichts wird zu etwas Plausiblem gerundet |
| Unsichere Speichernutzung | reines sicheres Rust mit `forbid(unsafe_code)` |
| Eine feindliche Zeichnung (Skripte, deklarierte Entitäten, tiefe Verschachtelung, Millionen Punkte, riesige Zahlen) | Skripte, Bilder und Links werden nie ausgeführt oder geladen; eine deklarierte Entität ist ein Fehler; Größe, Tiefe, Elemente, Formen, Punkte und die Stücke einer Kurve sind begrenzt; eine Form mit einem Punkt außerhalb des Bereichs wird weggelassen |
| Ein Programm, das den Laser ohne Geschwindigkeit oder über voller Leistung einschalten könnte | jedes Programm wird vor der Ausgabe Zeile für Zeile geprüft, und eines, das durchfällt, wird nicht geschrieben |
| Die Ausgabedatei, über die Zeichnung oder die Einstellungen gewählt, aus denen sie entsteht | wird abgelehnt, im Fenster und in den Befehlen |

**Was nicht abgesichert ist:** Ein fehlerfrei erzeugtes Ergebnis ist kein geprüftes Ergebnis. Nichts hier kennt die Grenzen einer Maschine, und das Programm hatte keine unabhängige Prüfung. Eine Schwachstelle bitte privat melden (siehe [SECURITY.md](SECURITY.md)).

## 🔌 Schnittstellen

Die Bibliothek ist `eh3d_phoenix_laser` (`use eh3d_phoenix_laser::...`); die Kommandozeile ist `phoenix-laser`. Die Befehle sind diese; jeder gibt sein Ergebnis auf der Standardausgabe aus und meldet einen Fehler auf der Fehlerausgabe mit einem Exit-Code ungleich null. Es gibt keinen Netzwerkzugriff.

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

`DRAWING` ist eine SVG- oder DXF-Datei. `cut` folgt den Konturen (die inneren zuerst); `vector` füllt die geschlossenen Formen mit Linien und folgt den offenen; beide nehmen die Einstellungen jeder Ebene oder Farbe aus `--config FILE` (siehe [docs/vector-settings.md](docs/vector-settings.md)), und eine Option der Kommandozeile ändert sie für alle Ebenen. `layers` listet die Ebenen einer Zeichnung mit dem auf, was mit jeder geschehen würde.

## ⚙️ Konfiguration

Die Gravur eines Bildes wird mit Argumenten der Kommandozeile eingestellt (siehe Schnittstellen), und jeder Standardwert steht in der Hilfe des Befehls (`phoenix-laser help`). Das Schneiden und Gravieren einer Zeichnung wird mit einer Einstellungsdatei eingestellt (`--config FILE` oder „Einstellungen laden“ im Fenster): die Grenzen der Leser, wie eine Datei gelesen wird, die Reihenfolge der Schnitte, die Maschine und für jede Ebene oder Farbe der Modus (schneiden, gravieren oder auslassen), die Leistung, die Geschwindigkeit, die Durchgänge, der Fokusschritt, die Strahlbreite und ihre Seite, die Stege und die Schraffur. Jeder Schlüssel, sein Standardwert und sein Bereich stehen in [docs/vector-settings.md](docs/vector-settings.md), ein Beispiel in `examples/plate.cfg`; ein unbekannter Schlüssel oder ein Wert außerhalb des Bereichs ist ein Fehler, der seine Zeile nennt. Die festen Grenzen des Bildlesers stehen als benannte Konstanten im Code und sind durch Tests abgedeckt.

## 🚀 Schnellstart

Sie brauchen eine stabile Rust-Toolchain (1.88 oder neuer). Das Repository EH3D-COMMON muss neben diesem geklont sein: der Bildleser wird daraus per Pfad übernommen.

```bash
cargo build --release
cargo test
cargo run --release -- engrave examples/gradient.pgm examples/gradient.nc --width-mm 16 --dot-mm 0.5 --overscan auto
cargo run --release -- cut examples/plate.svg examples/plate.nc --config examples/plate.cfg
cargo run --release --features gui   # the window
cargo run --release --features gui -- --shot screen.png examples/plate.svg   # a picture of the window
```

Um ein Bild des Fensters aufzunehmen, ohne es von Hand zu öffnen, starten Sie es mit `--shot DATEI.png` (auch `--size WxH`, `--theme forge|dark|light`, `--wait N`, `--lang CODE`, `--open FILE`, `--replace`): Es öffnet sich in dieser Größe, speichert das PNG, behält keine Einstellungen und schließt sich selbst (Exit-Code 0, wenn die Datei geschrieben wurde, 1, wenn nicht, 2 bei falschen Argumenten).

## 🧪 Tests

Die Suite hat 304 Tests (`cargo test`, in der CI unter Windows und Linux). Sie prüfen Normalfälle, Grenzfälle und beschädigte Eingaben: leere Dateien, Datenmüll und abgeschnittene Dateien lösen nie eine Panik aus. Die CI führt außerdem `cargo fmt --check`, `cargo clippy` mit Warnungen als Fehlern und `python -I tools/ci_validate.py` aus.

## 🏗️ Build und Veröffentlichung

Die Version steht in `Cargo.toml` und in `eh3d.project.json`; jede Version hat ihren Eintrag in [CHANGELOG.md](CHANGELOG.md). `build.bat` / `build.sh` bauen das Programm, `test.bat` / `test.sh` führen die Tests aus und `run.bat` / `run.sh` starten es mit einem Beispiel. Releases erscheinen auf GitHub.

**Downloads:** Mit jeder Version dieses Repositorys werden fertige Programme für jedes System veröffentlicht: Windows (64 und 32 Bit), Linux (64 Bit) und macOS (Apple Silicon und Intel). Jedes Paket, `phoenix-laser-<Version>-<System>.zip` oder `.tar.gz`, enthält das Programm, die Lizenz, diese README, das Änderungsprotokoll, das Symbol und die Beispieldateien und kommt mit seiner `.sha256`-Datei; die Version listet außerdem jede Datei in `SHA256SUMS` und `build-manifest.json` auf. Bisher wurde noch keine Version veröffentlicht.

**Signiert und prüfbar:** Die Pakete einer Version werden mit dem Release-Schlüssel von EH3D signiert, und die `.sig`-Datei jedes Pakets liegt daneben (eine OpenSSH-Signatur, Namespace `eh3d-release`). Prüfe einen Download, bevor du ihn startest: Vergleiche seine SHA-256 (`sha256sum -c phoenix-laser-<Version>-<System>.tar.gz.sha256`, unter Windows `Get-FileHash`) und prüfe die Signatur mit `ssh-keygen -Y verify -f allowed_signers -I eh3d-release -n eh3d-release -s <Paket>.sig < <Paket>`; `allowed_signers` enthält die Zeile `eh3d-release <Schlüsseltyp> <öffentlicher Schlüssel der EH3D-Releases>`.

## 📂 Struktur des Repositorys

<p align="center">
  <img src="images/repository-structure.svg" alt="Struktur des Repositorys EH3D-PHOENIX-LASER" width="100%">
</p>

<details>
<summary>Textversion des Baums</summary>

```text
EH3D-PHOENIX-LASER/
├── src/
│   ├── lib.rs                                   die Bibliothek: was jeder Befehl nutzt
│   ├── main.rs                                  die Befehlszeile: engrave, calibrate, info, cut, vector und layers
│   ├── calibrate.rs                             ein Teil der Logik
│   ├── cli_args.rs                              die Argumente der Kommandozeile, dieselbe Datei in jedem kleinen Werkzeug
│   ├── dither.rs                                ein Teil der Logik
│   ├── gcode_out.rs                             die einzige Schreibweise einer G-Code-Zahl und der Standardkopf
│   ├── raster.rs                                ein Teil der Logik
│   ├── shot.rs                                  der Bildmodus des Fensters (--shot): seine Argumente, ohne Bildschirm gelesen und geprüft
│   ├── vector.rs                                eine gelesene Zeichnung: Formen in mm, Ebenen, Grenzen und einfache Geometrie
│   ├── flatten.rs                               Kurven in gerade Stücke innerhalb einer Toleranz (Bézier, Bögen)
│   ├── svg.rs                                   der SVG-Leser: eine sichere Teilmenge, ohne Skripte, Bilder oder Entitäten
│   ├── dxf.rs                                   der DXF-Leser: Linien, Polylinien, Bögen, Kreise und Splines, mit Einheiten
│   ├── offset.rs                                Kontur um die Strahlbreite seitlich versetzen, Kante für Kante geprüft
│   ├── order.rs                                 die Reihenfolge der Schnitte: lose Linien verbunden, Inneres zuerst, kurzer Leerweg
│   ├── cut.rs                                   von der Zeichnung zum G-Code: Schnitt, Stege, Schraffur, Durchgänge und Sicherheitsprüfung
│   ├── vconfig.rs                               die Einstellungsdatei der Vektorarbeit: Grenzen, Maschine und jede Ebene
│   ├── browse.rs                                die Dateiliste des Fensters zum Öffnen einer Datei
│   ├── gui.rs                                   das Fenster (egui, mit dem Feature gui): zeichnet den Zustand und gibt weiter, was die Person tut
│   ├── lang.rs                                  die Wörter des Fensters in den acht Sprachen
│   ├── viewport.rs                              Zoom und Verschiebung des Bildes, in einfachen Zahlen
│   └── window.rs                                der Zustand des Fensters ohne Bildschirm: Bild oder Zeichnung, Einstellungen, Ebenen, Durchgänge und Export
├── tests/
│   ├── dry_run.rs                               Tests von --dry-run: zählt dasselbe und schreibt nichts
│   ├── laser.rs                                 Tests von Bibliothek und Kommandozeile
│   ├── readme.rs                                Tests, die ausführen, was README und run-Skripte versprechen
│   ├── strict.rs                                Tests der Kommandozeile: Optionen und Wörter, die sie nicht kennt
│   ├── vector.rs                                Tests des Schneidens aus Zeichnungen: Befehle, Beispiele, feindliche Dateien und die Prüfung des G-Codes
│   └── window_cli.rs                            Tests, dass ein Build ohne das Feature gui eine reine Kommandozeile bleibt
├── examples/
│   ├── gradient.pgm                             ein Beispiel zum Ausprobieren
│   ├── plate.svg                                eine Platte mit Loch und einer Fläche zum Gravieren (SVG)
│   ├── plate.dxf                                dieselbe Platte als DXF
│   └── plate.cfg                                die Einstellungen seiner Ebenen: Leistung, Geschwindigkeit, Durchgänge, Strahlbreite, Stege und Schraffur
├── docs/
│   ├── architecture.json                        Daten des Architektur-Bildes
│   ├── structure.json                           Daten des Struktur-Bildes
│   └── vector-settings.md                       die Einstellungsdatei der Vektorarbeit, Schlüssel für Schlüssel
├── tools/                                       CI-Prüfung, README-Prüfung und die Bildwerkzeuge
├── images/                                      Banner und die Bilder von Architektur und Repository-Struktur
├── .github/                                     CI- und Release-Workflows, Vorlagen für Issues und Pull Requests
├── eh3d.project.json                            Manifest: Version, Rolle, Reifegrad und Build-Befehl
├── Cargo.toml Cargo.lock                        der Workspace und seine festgelegten Abhängigkeiten
├── CHANGELOG.md                                 was sich in jeder Version geändert hat
├── SECURITY.md SUPPORT.md                       Schwachstellen melden und Fragen stellen
└── CONTRIBUTING.md CODE_OF_CONDUCT.md LICENSE   Mitwirken, Verhaltenskodex und Lizenz
```

</details>

## 🗺️ Roadmap und bekannte Grenzen

**Bekannte Grenzen heute**

* Das Fenster wurde noch nicht auf einem Bildschirm angesehen.
* Es wurde nicht an einer Maschine und nicht mit einer Sammlung echter Dateien erprobt und hatte keine unabhängige Prüfung.
* Eine Zeichnung wird in einer sicheren Teilmenge gelesen (kein `use`, kein Text, keine Verläufe oder Stile mit Selektoren; keine Ellipsen, Schraffuren oder Blöcke in einer DXF); eine Kontur, die schmaler als der Strahl ist, wird nicht geschnitten, und das Programm sagt es; der Laser ist der eines GRBL-Controllers mit `M4`, und nichts wurde an einem echten Laser oder Material erprobt.

**Als Nächstes**

* Die Leistungsdichte vor dem Brennen auf der Platine zeigen und die Leistung in den Ecken eines Schnitts konstant halten.
* Das Fenster auf einem Bildschirm ansehen und fertigstellen.
* Es mit echten Zeichnungen und mit den anderen Programmen der Suite erproben.

## 🌍 Sprachen

README und Texte gibt es auf Englisch, Spanisch, Französisch, Italienisch, Deutsch, Chinesisch, Japanisch und europäischem Portugiesisch.

## 🔗 Verwandte Projekte

* [EH3D-COMMON](https://github.com/JuanenRac/EH3D-COMMON): die gemeinsamen Crates, darunter der Bildleser
* [EH3D-STENCILCAM](https://github.com/JuanenRac/EH3D-STENCILCAM): Lotpasten-Schablonen
* [EH3D-CAM-DEBUGGER](https://github.com/JuanenRac/EH3D-CAM-DEBUGGER): die Prüfung von G-Code, bevor er eine Maschine erreicht
* [EH3D-TOOL-MANAGER](https://github.com/JuanenRac/EH3D-TOOL-MANAGER): das Werkzeuglager der CAM-Programme
* [EH3D-MOTIONCNC](https://github.com/JuanenRac/EH3D-MOTIONCNC): das Maschinensteuerungsprogramm
* [EH3D-SUITE-LAUNCHER](https://github.com/JuanenRac/EH3D-SUITE-LAUNCHER): die Zentrale, die die Programme der Suite findet und startet

## 📚 Dokumentation

* [Changelog](CHANGELOG.md)
* [Architektur- und Strukturdaten](docs/architecture.json): die Daten, aus denen die beiden Bilder gezeichnet werden.
* [Einstellungen der Vektorarbeit](docs/vector-settings.md): jeder Schlüssel der Einstellungsdatei mit Standardwert und Bereich.

## 🤝 Mitwirken und Community

* Vor einem Pull Request `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test` und `python -I tools/ci_validate.py` ausführen: alles muss bestehen. Siehe [CONTRIBUTING.md](CONTRIBUTING.md) und den [Verhaltenskodex](CODE_OF_CONDUCT.md).
* Die README gibt es in acht Sprachen: Eine Textänderung erfolgt in allen acht Dateien, und die zwei Bilder werden aus `docs/architecture.json` und `docs/structure.json` mit `tools/make_architecture.py` und `tools/make_structure.py` gezeichnet.
* Eine Schwachstelle privat melden: [SECURITY.md](SECURITY.md). Fragen, Ideen und Berichte: electrohobby3d@gmail.com ([SUPPORT.md](SUPPORT.md)).

## 👤 AUTOR

**JuanenRac (Electro Hobby 3D)** · electrohobby3d@gmail.com

## 📜 LIZENZ

Software: GPL-3.0-or-later. Siehe [LICENSE](LICENSE). Die Hardware-Entwürfe der Suite verwenden die CERN OHL-S.
