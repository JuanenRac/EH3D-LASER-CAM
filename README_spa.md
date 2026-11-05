<p align="center">
  <img src="images/EH3D_PHOENIX_LASER_BANNER.svg" alt="EH3D-PHOENIX-LASER banner" width="100%">
</p>

# <img src="images/brand/icon-64.png" alt="" width="40" height="40" align="absmiddle"> EH3D-PHOENIX-LASER
<p align="center"><a href="README.md">🇺🇸 English</a> | 🇪🇸 <b>Español</b> | <a href="README_fra.md">🇫🇷 Français</a> | <a href="README_ita.md">🇮🇹 Italiano</a> | <a href="README_deu.md">🇩🇪 Deutsch</a> | <a href="README_zho.md">🇨🇳 中文</a> | <a href="README_jpn.md">🇯🇵 日本語</a> | <a href="README_por.md">🇵🇹 Português</a></p>

### Grabado láser desde una imagen y corte desde un dibujo: tramado, G-code de trama, contornos SVG y DXF y rejilla de calibración

<p align="center">
  <img src="https://img.shields.io/badge/License-GPL%203.0--or--later-blue.svg" alt="GPL 3.0 or later">
  <img src="https://img.shields.io/badge/Stack-Rust-DEA584.svg" alt="Rust">
  <img src="https://img.shields.io/badge/Languages-8-00E5FF.svg" alt="Languages">
  <img src="https://img.shields.io/badge/Version-0.0.4-0f6cbd.svg" alt="Version">
  <img src="https://img.shields.io/badge/Maturity-scaffolding-FFB020.svg" alt="Maturity">
</p>

---

**Comprobación de honestidad - lo que funciona hoy:** una biblioteca Rust y un programa de línea de comandos (`phoenix-laser`) con 294 pruebas que se ejecutan en un ordenador: convierte una imagen en puntos por tramado y escribe el G-code de un láser que quema línea a línea con potencia dinámica y sobrerrecorrido, lee dibujos SVG y DXF y escribe el G-code que corta sus contornos (los de dentro primero, con el ancho del haz, puentes y varias pasadas) y graba sus formas con líneas, con los ajustes de cada capa en un fichero, y una rejilla de prueba de potencia contra avance. Tiene una ventana (se compila con `--features gui`) que abre una imagen o un dibujo, muestra el recorrido del láser, permite cambiar los ajustes y exporta el G-code, pero todavía no se ha visto en una pantalla; no se ha probado en una máquina y no se ha comprobado con una colección de ficheros reales.

---

> **Estado de seguridad:** este programa no controla ninguna máquina y no es un sistema de seguridad. Lo que produce debe revisarlo una persona antes de ejecutar nada en una máquina; nada de esto se ha ejecutado en hardware real.


## 📋 De un vistazo

| | |
|---|---|
| Versión | `0.0.4` |
| Madurez | Andamiaje |
| Pruebas | 294 |
| Pila tecnológica | Rust |
| En hardware real | No probado: nada de esto se ha ejecutado en una máquina |

**Ir a:** [Resumen](#-resumen) · [Arquitectura](#-arquitectura) · [Inicio rápido](#-inicio-rápido) · [Pruebas](#-pruebas) · [Estructura](#-estructura-del-repositorio) · [Relacionados](#-proyectos-relacionados)

## 🚦 Estado del proyecto

Cada línea es una parte de este repositorio y la luz dice hasta dónde llega hoy. Se escribe con lo que el repositorio puede demostrar (pruebas, compilaciones, ejecuciones), no con lo previsto.

🟢 Hecho y verificado · 🟡 Funciona, con límites conocidos · 🟠 Hecho a medias o sin verificar · 🔴 Sin hacer o sin funcionar

| Área | Estado | Qué significa aquí |
|---|---|---|
| Núcleo | 🟡 | convierte una imagen en puntos por tramado y escribe el G-code de un láser que quema línea a línea con potencia dinámica y sobrerrecorrido, lee dibujos SVG y DXF y los corta o graba por capas (contornos ordenados, ancho del haz, puentes, pasadas, rayado) y una rejilla de prueba de potencia contra avance; probado en un ordenador con entradas normales y dañadas, no con una colección de ficheros reales. |
| Línea de comandos | 🟡 | `phoenix-laser` (`engrave`, `calibrate`, `info`, `cut`, `vector`, `layers`) ejecuta las mismas funciones que la biblioteca e imprime el resultado; cada error vuelve como mensaje y un código de salida distinto de cero. |
| Pruebas | 🟡 | 294 pruebas pasan (`cargo test`), con casos normales, casos límite y entradas dañadas. |
| Seguridad | 🟡 | Rust seguro y simple (`forbid(unsafe_code)`): nada entra en pánico con los ficheros recibidos y un valor malo es un error con su motivo; sin revisión independiente. |
| Ventana | 🟠 | Escrita (egui, `--features gui`): abrir o soltar una imagen o un dibujo SVG o DXF, ver el recorrido del láser (el corte en un color, el desplazamiento atenuado), editar la tabla de capas, una rejilla de calibración, zoom y desplazamiento, estimar el tiempo, exportar; su lógica está probada, pero todavía no se ha compilado ni visto en una pantalla. |
| Hardware real | 🔴 | No probado: nada de esto se ha ejecutado en una máquina. |
| Documentación | 🟡 | README en ocho idiomas, el registro de cambios y las dos imágenes dibujadas desde `docs/`. |

## 🎯 Resumen

* **La imagen** (`eh3d-image`): PNG (gris, color, transparente, 8 o 16 bits) y PGM (texto y binario) se convierten en niveles de gris, con las partes transparentes como blanco y el tamaño limitado para que un fichero hostil no pida gigabytes; `resize` promedia los píxeles para que un punto sea tan grande como el haz.
* **Los puntos** (`src/dither.rs`): umbral, o difusión de error de Floyd-Steinberg o Atkinson; las pruebas comprueban que se conserva la oscuridad de un gris (un gris medio quema la mitad de los puntos) y que el blanco no quema nada y el negro lo quema todo.
* **El láser** (`src/raster.rs`, `src/calibrate.rs`): una línea tras otra, de izquierda a derecha y luego de derecha a izquierda, `M4` con `S0` mientras viaja y la potencia elegida donde quema, un sobrerrecorrido fijo o calculado con la aceleración (v al cuadrado entre 2a) y una rejilla de cuadrados con una potencia por columna y un avance por fila para hallar lo que conviene a un material.
* **La cabecera y los números de todo fichero de G-code** (`src/gcode_out.rs`): el fichero empieza con líneas de comentario que dicen el generador y su versión, las unidades (mm, G21), la imagen de la que se hizo con su SHA-256 y las herramientas (el láser y su potencia); una rejilla de calibración dice que no tiene origen. Un número se escribe de una sola manera (como mucho tres decimales, sin `-0`) y un valor que no es finito se rechaza en vez de escribirse como `NaN` o `inf`.
* **`--dry-run`:** `engrave` calcula el resultado y lo cuenta (puntos, líneas, tramos, longitud quemada, tiempo) sin escribir el fichero de salida, que entonces puede omitirse.
* **Los dibujos** (`src/svg.rs`, `src/dxf.rs`, `src/flatten.rs`, `src/vector.rs`): un subconjunto seguro de SVG (trazados con todas sus órdenes, formas, transformaciones, grupos, unidades) y las entidades de un DXF (líneas, polilíneas con arcos, arcos, círculos, splines, unidades) se convierten en formas en milímetros; los scripts, las imágenes, `use`, las entidades declaradas y todo lo desconocido se omiten o se rechazan, el tamaño, la profundidad y el número de elementos y de puntos están limitados por los ajustes, y las curvas se convierten en tramos rectos dentro de una tolerancia.
* **Corte y grabado** (`src/order.rs`, `src/offset.rs`, `src/cut.rs`): las formas se agrupan por capa (DXF) o color (SVG); cada grupo se corta (líneas sueltas unidas, los contornos que están dentro de otro primero, el menor desplazamiento, el ancho del haz descontado del lado correcto, puentes sin cortar, varias pasadas con el foco avanzando) o se graba (formas cerradas rellenas de líneas con un ángulo, por la regla par-impar) con sus propios ajustes.
* **El fichero de ajustes** (`src/vconfig.rs`, `docs/vector-settings.md`): todo valor que se afina (los límites de los lectores, la máquina, los ajustes de cada capa) está en un fichero de texto simple que se comprueba línea a línea; nada de eso está en el código.
* **Un G-code comprobado:** el programa de un dibujo se comprueba línea a línea antes de entregarse: ningún láser encendido durante un movimiento rápido ni sin velocidad, ninguna `S` por encima de la potencia máxima, ningún movimiento fuera de la cama, `M5` al final; el fichero nunca se escribe sobre el dibujo ni sobre los ajustes de los que procede.

## 🧩 Papel en EH3D

EH3D-PHOENIX-LASER es un programa de la EH3D Fabrication Suite. Funciona como biblioteca y línea de comandos y, compilado con `--features gui`, como ventana con el aspecto que comparten los demás programas de la suite. Lee y escribe ficheros simples, así que puede usarse solo o entre los demás programas.

## 🏛️ Arquitectura

<p align="center">
  <img src="images/architecture.svg" alt="Arquitectura de EH3D-PHOENIX-LASER: quién lo usa, qué contiene y de qué depende" width="100%">
</p>


Cómo encajan las partes de este repositorio: quién lo usa, qué contiene y de qué depende. La imagen se dibuja desde `docs/architecture.json` y se comprueba con el código.

## ✨ Funciones

* **Biblioteca y línea de comandos:** cada orden de `phoenix-laser` es una función de la biblioteca, así que otro programa de la suite puede llamarla sin lanzar un proceso.
* **Línea de comandos estricta:** una opción mal escrita (`--powr`) o una palabra de más es un error que la nombra, no algo que se ignora; si no, el programa haría su salida con los valores por defecto.
* **Pruebas con entradas dañadas:** las pruebas también dan ficheros vacíos, basura, números enormes y ficheros cortados, y ninguna puede provocar un pánico.
* **Sin adivinar:** un valor que no tiene sentido es un error que dice cuál y por qué, nunca un resultado que solo lo parece.
* **Sin código unsafe:** `#![forbid(unsafe_code)]` en la biblioteca y el programa.

## 🔒 Seguridad y protección

| Amenaza | Defensa |
|---|---|
| Un fichero dañado, cortado o vacío | sin pánico: las pruebas cortan una entrada válida en cada posición y dan basura |
| Un número sin sentido (negativo, no finito, absurdamente grande) | se rechaza con un error que nombra el valor; nada se redondea hasta parecer plausible |
| Uso inseguro de memoria | Rust seguro y simple con `forbid(unsafe_code)` |
| Un dibujo hostil (scripts, entidades declaradas, anidamiento profundo, millones de puntos, números enormes) | los scripts, las imágenes y los enlaces nunca se ejecutan ni se descargan; una entidad declarada es un error; el tamaño, la profundidad, los elementos, las formas, los puntos y los tramos de una curva están limitados; una forma con un punto fuera de rango se deja fuera |
| Un programa que podría encender el láser sin velocidad o por encima de la potencia máxima | cada programa se comprueba línea a línea antes de entregarse y el que falla no se escribe |
| El fichero de salida elegido sobre el dibujo o los ajustes de los que procede | se rechaza, en la ventana y en las órdenes |

**Lo que no se defiende:** un resultado producido sin errores no es un resultado verificado. Nada de esto conoce los límites de una máquina y el programa no ha tenido revisión independiente. Informe de una vulnerabilidad en privado (véase [SECURITY.md](SECURITY.md)).

## 🔌 Interfaces

La biblioteca es `eh3d_phoenix_laser` (`use eh3d_phoenix_laser::...`); la línea de comandos es `phoenix-laser`. Las órdenes son estas; cada una imprime su resultado por la salida estándar e informa de un error por la salida de errores con un código de salida distinto de cero. No hay acceso a la red.

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

`DRAWING` es un fichero SVG o DXF. `cut` sigue los contornos (los de dentro primero); `vector` rellena las formas cerradas con líneas y sigue las abiertas; ambos toman los ajustes de cada capa o color de `--config FILE` (véase [docs/vector-settings.md](docs/vector-settings.md)) y una opción de la línea de comandos los cambia en todas las capas. `layers` lista las capas de un dibujo con lo que se haría con cada una.

## ⚙️ Configuración

El grabado de una imagen se ajusta con argumentos de la línea de comandos (véase Interfaces) y cada valor por defecto está escrito en la ayuda de la orden (`phoenix-laser help`). El corte y el grabado de un dibujo se ajustan con un fichero de ajustes (`--config FILE`, o «Cargar ajustes» en la ventana): los límites de los lectores, cómo se lee un fichero, el orden de los cortes, la máquina y, para cada capa o color, el modo (cortar, grabar u omitir), la potencia, la velocidad, las pasadas, el avance del foco, el ancho del haz y su lado, los puentes y el rayado. Cada clave, su valor por defecto y su rango están en [docs/vector-settings.md](docs/vector-settings.md) y hay un ejemplo en `examples/plate.cfg`; una clave desconocida o un valor fuera de rango es un error que nombra su línea. Los límites fijos del lector de imágenes están en el código como constantes con nombre y las pruebas los cubren.

## 🚀 Inicio rápido

Necesita una cadena de herramientas Rust estable (1.88 o más reciente). El repositorio EH3D-COMMON debe estar clonado junto a este: el lector de imágenes se toma de él por ruta.

```bash
cargo build --release
cargo test
cargo run --release -- engrave examples/gradient.pgm examples/gradient.nc --width-mm 16 --dot-mm 0.5 --overscan auto
cargo run --release -- cut examples/plate.svg examples/plate.nc --config examples/plate.cfg
cargo run --release --features gui   # the window
```

## 🧪 Pruebas

El conjunto tiene 294 pruebas (`cargo test`, ejecutadas en Windows y Linux en el CI). Comprueban los casos normales, los casos límite y las entradas dañadas: ficheros vacíos, basura y ficheros cortados nunca provocan un pánico. El CI ejecuta además `cargo fmt --check`, `cargo clippy` con las advertencias como errores y `python -I tools/ci_validate.py`.

## 🏗️ Compilación y publicación

La versión está en `Cargo.toml` y en `eh3d.project.json`; cada versión tiene su entrada en [CHANGELOG.md](CHANGELOG.md). `build.bat` / `build.sh` compilan el programa, `test.bat` / `test.sh` ejecutan las pruebas y `run.bat` / `run.sh` lo ejecutan con un ejemplo. Las versiones se publican en GitHub.

**Descargas:** Con cada versión de este repositorio se publican programas ya hechos para cada sistema: Windows (64 y 32 bits), Linux (64 bits) y macOS (Apple silicon e Intel). Cada paquete, `phoenix-laser-<versión>-<sistema>.zip` o `.tar.gz`, lleva el programa, la licencia, este README, el registro de cambios, el icono y los ficheros de ejemplo, y viene con su fichero `.sha256`; la versión también lista todos los ficheros en `SHA256SUMS` y `build-manifest.json`. Todavía no se ha publicado ninguna versión.

**Firmados y comprobables:** los paquetes de una versión se firman con la clave de versiones de EH3D, y el fichero `.sig` de cada paquete se publica a su lado (una firma OpenSSH, espacio de nombres `eh3d-release`). Comprueba una descarga antes de ejecutarla: compara su SHA-256 (`sha256sum -c phoenix-laser-<versión>-<sistema>.tar.gz.sha256`, o `Get-FileHash` en Windows) y verifica la firma con `ssh-keygen -Y verify -f allowed_signers -I eh3d-release -n eh3d-release -s <paquete>.sig < <paquete>`, donde `allowed_signers` contiene la línea `eh3d-release <tipo de clave> <clave pública de las versiones de EH3D>`.

## 📂 Estructura del repositorio

<p align="center">
  <img src="images/repository-structure.svg" alt="Estructura del repositorio EH3D-PHOENIX-LASER" width="100%">
</p>

<details>
<summary>Versión en texto del árbol</summary>

```text
EH3D-PHOENIX-LASER/
├── src/
│   ├── lib.rs                                   la biblioteca: lo que usa cada orden
│   ├── main.rs                                  la línea de comandos: engrave, calibrate, info, cut, vector y layers
│   ├── calibrate.rs                             una parte de la lógica
│   ├── cli_args.rs                              los argumentos de la línea de comandos, el mismo fichero en cada utilidad pequeña
│   ├── dither.rs                                una parte de la lógica
│   ├── gcode_out.rs                             la única forma de escribir un número de G-code y la cabecera estándar
│   ├── raster.rs                                una parte de la lógica
│   ├── vector.rs                                un dibujo ya leído: formas en mm, capas, límites y la geometría básica
│   ├── flatten.rs                               curvas en tramos rectos dentro de una tolerancia (Bézier, arcos)
│   ├── svg.rs                                   el lector de SVG: un subconjunto seguro, sin scripts, imágenes ni entidades
│   ├── dxf.rs                                   el lector de DXF: líneas, polilíneas, arcos, círculos y splines, con unidades
│   ├── offset.rs                                desplazar un contorno de lado el ancho del haz, comprobado arista a arista
│   ├── order.rs                                 el orden de los cortes: líneas sueltas unidas, lo de dentro primero, poco desplazamiento
│   ├── cut.rs                                   del dibujo al G-code: corte, puentes, rayado, pasadas y la comprobación de seguridad
│   ├── vconfig.rs                               el fichero de ajustes del trabajo vectorial: límites, máquina y cada capa
│   ├── browse.rs                                la lista de ficheros de la ventana que abre un fichero
│   ├── gui.rs                                   la ventana (egui, con la característica gui): dibuja el estado y transmite lo que hace la persona
│   ├── lang.rs                                  las palabras de la ventana en los ocho idiomas
│   ├── viewport.rs                              zoom y desplazamiento de la imagen, en números simples
│   └── window.rs                                el estado de la ventana sin pantalla: imagen o dibujo, ajustes, capas, pasadas y exportación
├── tests/
│   ├── dry_run.rs                               pruebas de --dry-run: cuenta lo mismo y no escribe nada
│   ├── laser.rs                                 pruebas de la biblioteca y la línea de comandos
│   ├── readme.rs                                pruebas que ejecutan lo que prometen el README y los scripts run
│   ├── strict.rs                                pruebas de la línea de comandos: opciones y palabras que no conoce
│   ├── vector.rs                                pruebas de corte desde dibujos: órdenes, ejemplos, ficheros hostiles y la comprobación del G-code
│   └── window_cli.rs                            pruebas de que una compilación sin la característica gui sigue siendo solo línea de comandos
├── examples/
│   ├── gradient.pgm                             una muestra para probar el programa
│   ├── plate.svg                                una placa con un agujero y un panel para grabar (SVG)
│   ├── plate.dxf                                la misma placa como DXF
│   └── plate.cfg                                los ajustes de sus capas: potencia, velocidad, pasadas, ancho del haz, puentes y rayado
├── docs/
│   ├── architecture.json                        datos de la imagen de Arquitectura
│   ├── structure.json                           datos de la imagen de Estructura
│   └── vector-settings.md                       el fichero de ajustes del trabajo vectorial, clave por clave
├── tools/                                       comprobación del CI y del README y herramientas de imágenes
├── images/                                      banner y las imágenes de Arquitectura y Estructura del repositorio
├── .github/                                     flujos de CI y de publicación, plantillas de issues y pull requests
├── eh3d.project.json                            manifiesto: versión, rol, madurez y comando de compilación
├── Cargo.toml Cargo.lock                        el workspace y sus dependencias fijadas
├── CHANGELOG.md                                 qué cambió en cada versión
├── SECURITY.md SUPPORT.md                       cómo informar de una vulnerabilidad y dónde preguntar
└── CONTRIBUTING.md CODE_OF_CONDUCT.md LICENSE   cómo contribuir, el código de conducta y la licencia
```

</details>

## 🗺️ Hoja de ruta y límites conocidos

**Límites conocidos hoy**

* La ventana todavía no se ha visto en una pantalla.
* No se ha probado en una máquina ni con una colección de ficheros reales y no ha tenido revisión independiente.
* Un dibujo se lee en un subconjunto seguro (sin `use`, texto, degradados ni estilos con selectores; sin elipses, rayados ni bloques en un DXF); un contorno más estrecho que el haz no se corta y el programa lo dice; el láser es el de un controlador GRBL con `M4` y nada se ha probado en un láser ni un material reales.

**Siguiente**

* Mostrar la densidad de potencia sobre la placa antes de quemar y mantener constante la potencia en las esquinas de un corte.
* Ver la ventana en una pantalla y terminarla.
* Probarlo con dibujos reales y con los demás programas de la suite.

## 🌍 Idiomas

El README y los textos existen en inglés, español, francés, italiano, alemán, chino, japonés y portugués de Portugal.

## 🔗 Proyectos relacionados

* [EH3D-COMMON](https://github.com/JuanenRac/EH3D-COMMON): los crates compartidos, entre ellos el lector de imágenes
* [EH3D-STENCILCAM](https://github.com/JuanenRac/EH3D-STENCILCAM): las plantillas de pasta de soldadura
* [EH3D-CAM-DEBUGGER](https://github.com/JuanenRac/EH3D-CAM-DEBUGGER): la revisión del G-code antes de llegar a una máquina
* [EH3D-TOOL-MANAGER](https://github.com/JuanenRac/EH3D-TOOL-MANAGER): el almacén de herramientas que comparten los programas CAM
* [EH3D-MOTIONCNC](https://github.com/JuanenRac/EH3D-MOTIONCNC): el programa de control de máquina
* [EH3D-SUITE-LAUNCHER](https://github.com/JuanenRac/EH3D-SUITE-LAUNCHER): el centro que encuentra y arranca los programas de la suite

## 📚 Documentación

* [Registro de cambios](CHANGELOG.md)
* [Datos de arquitectura y estructura](docs/architecture.json): los datos con los que se dibujan las dos imágenes.
* [Ajustes del trabajo vectorial](docs/vector-settings.md): cada clave del fichero de ajustes con su valor por defecto y su rango.

## 🤝 Contribuir y comunidad

* Antes de una pull request ejecute `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test` y `python -I tools/ci_validate.py`: todo debe pasar. Véanse [CONTRIBUTING.md](CONTRIBUTING.md) y el [código de conducta](CODE_OF_CONDUCT.md).
* El README existe en ocho idiomas: un cambio de texto se hace en los ocho ficheros, y las dos imágenes se dibujan desde `docs/architecture.json` y `docs/structure.json` con `tools/make_architecture.py` y `tools/make_structure.py`.
* Informe de una vulnerabilidad en privado: [SECURITY.md](SECURITY.md). Preguntas, ideas e informes: electrohobby3d@gmail.com ([SUPPORT.md](SUPPORT.md)).

## 👤 AUTOR

**JuanenRac (Electro Hobby 3D)** · electrohobby3d@gmail.com

## 📜 LICENCIA

Software: GPL-3.0-or-later. Véase [LICENSE](LICENSE). Los diseños de hardware de la suite usan CERN OHL-S.
