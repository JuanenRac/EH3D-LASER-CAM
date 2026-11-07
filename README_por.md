<p align="center">
  <img src="images/EH3D_PHOENIX_LASER_BANNER.svg" alt="EH3D-PHOENIX-LASER banner" width="100%">
</p>

# <img src="images/brand/icon-64.png" alt="" width="40" height="40" align="absmiddle"> EH3D-PHOENIX-LASER
<p align="center"><a href="README.md">🇺🇸 English</a> | <a href="README_spa.md">🇪🇸 Español</a> | <a href="README_fra.md">🇫🇷 Français</a> | <a href="README_ita.md">🇮🇹 Italiano</a> | <a href="README_deu.md">🇩🇪 Deutsch</a> | <a href="README_zho.md">🇨🇳 中文</a> | <a href="README_jpn.md">🇯🇵 日本語</a> | 🇵🇹 <b>Português</b></p>

### Gravação a laser a partir de uma imagem e corte a partir de um desenho: tramagem, G-code de trama, contornos SVG e DXF e grelha de calibração

<p align="center">
  <img src="https://img.shields.io/badge/License-GPL%203.0--or--later-blue.svg" alt="GPL 3.0 or later">
  <img src="https://img.shields.io/badge/Stack-Rust-DEA584.svg" alt="Rust">
  <img src="https://img.shields.io/badge/Languages-8-00E5FF.svg" alt="Languages">
  <img src="https://img.shields.io/badge/Version-0.0.4-0f6cbd.svg" alt="Version">
  <img src="https://img.shields.io/badge/Maturity-scaffolding-FFB020.svg" alt="Maturity">
</p>

---

**Verificação de honestidade - o que funciona hoje:** uma biblioteca Rust e um programa de linha de comandos (`phoenix-laser`) com 309 testes que correm num computador: converte uma imagem em pontos por tramagem e escreve o G-code de um laser que queima linha a linha com potência dinâmica e sobrecurso, lê desenhos SVG e DXF e escreve o G-code que corta os seus contornos (os de dentro primeiro, com a largura do feixe, pontes e várias passagens) e grava as suas formas com linhas, com as definições de cada camada num ficheiro, e uma grelha de teste de potência contra avanço. Tem uma janela (compilada com `--features gui`) que abre uma imagem ou um desenho, mostra o percurso do laser, permite alterar as definições e exporta o G-code, mas a janela ainda não foi vista num ecrã; não foi experimentado numa máquina e não foi verificado com uma coleção de ficheiros reais.

---

> **Estado de segurança:** este programa não controla nenhuma máquina e não é um sistema de segurança. O que produz deve ser verificado por uma pessoa antes de se executar algo numa máquina; nada disto correu em hardware real.


## 📋 Num relance

| | |
|---|---|
| Versão | `0.0.4` |
| Maturidade | Estrutura inicial |
| Testes | 309 |
| Tecnologias | Rust |
| Em hardware real | Não experimentado: nada disto correu numa máquina |

**Ir para:** [Visão geral](#-visão-geral) · [Arquitetura](#-arquitetura) · [Início rápido](#-início-rápido) · [Testes](#-testes) · [Estrutura](#-estrutura-do-repositório) · [Relacionados](#-projetos-relacionados)

## 🚦 Estado do projeto

Cada linha é uma parte deste repositório e a luz diz até onde chega hoje. É escrita com o que o repositório consegue demonstrar (testes, compilações, execuções), não com o que está previsto.

🟢 Feito e verificado · 🟡 Funciona, com limites conhecidos · 🟠 Feito a meio ou ainda por verificar · 🔴 Por fazer ou a não funcionar

| Área | Estado | O que significa aqui |
|---|---|---|
| Núcleo | 🟡 | converte uma imagem em pontos por tramagem e escreve o G-code de um laser que queima linha a linha com potência dinâmica e sobrecurso, lê desenhos SVG e DXF e corta-os ou grava-os por camada (contornos ordenados, largura do feixe, pontes, passagens, hachura), e uma grelha de teste de potência contra avanço; testado num computador com entradas normais e danificadas, não com uma coleção de ficheiros reais. |
| Linha de comandos | 🟡 | `phoenix-laser` (`engrave`, `calibrate`, `info`, `cut`, `vector`, `layers`) executa as mesmas funções que a biblioteca e imprime o resultado; cada erro volta como mensagem e código de saída diferente de zero. |
| Testes | 🟡 | 309 testes passam (`cargo test`), com casos normais, casos-limite e entradas danificadas. |
| Segurança | 🟡 | Rust seguro e simples (`forbid(unsafe_code)`): nada entra em pânico com os ficheiros recebidos e um valor mau é um erro com o seu motivo; sem revisão independente. |
| Janela | 🟠 | Escrita (egui, `--features gui`): abrir ou largar uma imagem ou um desenho SVG ou DXF, ver o percurso do laser (o corte numa cor, a deslocação atenuada), editar a tabela de camadas, uma grelha de calibração, zoom e deslocamento, estimar o tempo, exportar; a sua lógica está testada, mas ainda não foi compilada nem vista num ecrã. |
| Hardware real | 🔴 | Não experimentado: nada disto foi executado numa máquina. |
| Documentação | 🟡 | README em oito idiomas, o registo de alterações e as duas imagens desenhadas a partir de `docs/`. |

## 🎯 Visão geral

* **A imagem** (`eh3d-image`): PNG (cinzento, cor, transparente, 8 ou 16 bits) e PGM (texto e binário) tornam-se níveis de cinzento, com as partes transparentes como branco e o tamanho limitado para que um ficheiro hostil não possa pedir gigabytes; `resize` faz a média dos píxeis para que um ponto seja tão grande como o feixe.
* **Os pontos** (`src/dither.rs`): um limiar, ou difusão de erro de Floyd-Steinberg ou Atkinson; os testes verificam que a escuridão de um cinzento se conserva (um cinzento médio queima cerca de metade dos pontos) e que o branco não queima nada e o preto queima tudo.
* **O laser** (`src/raster.rs`, `src/calibrate.rs`): uma linha depois da outra, da esquerda para a direita e depois da direita para a esquerda, `M4` com `S0` enquanto viaja e a potência escolhida onde queima, um sobrecurso fixo ou calculado a partir da aceleração (v ao quadrado sobre 2a) e uma grelha de quadrados com uma potência por coluna e um avanço por linha para achar o que convém a um material.
* **O cabeçalho e os números de todo o ficheiro de G-code** (`src/gcode_out.rs`): o ficheiro começa com linhas de comentário que dizem o gerador e a sua versão, as unidades (mm, G21), a imagem de que foi feito com o seu SHA-256 e as ferramentas (o laser e a sua potência); uma grelha de calibração diz que não tem origem. Um número escreve-se de uma só maneira (no máximo três casas decimais, sem `-0`) e um valor que não é finito é recusado em vez de ser escrito como `NaN` ou `inf`.
* **`--dry-run`:** `engrave` calcula o resultado e conta-o (pontos, linhas, passagens, comprimento queimado, tempo) sem escrever o ficheiro de saída, que então pode ser omitido.
* **Os desenhos** (`src/svg.rs`, `src/dxf.rs`, `src/flatten.rs`, `src/vector.rs`): um subconjunto seguro de SVG (trajetos com todos os seus comandos, formas, transformações, grupos, unidades) e as entidades de um DXF (linhas, polilinhas com arcos, arcos, círculos, splines, unidades) transformam-se em formas em milímetros; os scripts, as imagens, `use`, as entidades declaradas e tudo o que é desconhecido são ignorados ou recusados, o tamanho, a profundidade e o número de elementos e de pontos são limitados pelas definições, e as curvas passam a troços retos dentro de uma tolerância.
* **Corte e gravação** (`src/order.rs`, `src/offset.rs`, `src/cut.rs`): as formas são agrupadas por camada (DXF) ou cor (SVG); cada grupo é cortado (linhas soltas unidas, os contornos que estão dentro de outro primeiro, a menor deslocação, a largura do feixe descontada do lado certo, pontes por cortar, várias passagens com o foco a avançar) ou gravado (formas fechadas preenchidas com linhas num ângulo, pela regra par-ímpar) com as suas próprias definições.
* **O ficheiro de definições** (`src/vconfig.rs`, `docs/vector-settings.md`): todo o valor que se afina (os limites dos leitores, a máquina, as definições de cada camada) está num ficheiro de texto simples verificado linha a linha; nada disso está no código.
* **Um G-code verificado:** o programa de um desenho é verificado linha a linha antes de ser entregue: nenhum laser ligado durante um movimento rápido ou sem velocidade, nenhum `S` acima da potência máxima, nenhum movimento fora da mesa, `M5` no fim; o ficheiro nunca é escrito por cima do desenho nem das definições de que provém.

## 🧩 Papel no EH3D

EH3D-PHOENIX-LASER é um programa da EH3D Fabrication Suite. Funciona como biblioteca e linha de comandos e, compilado com `--features gui`, como janela com o aspeto que os outros programas da suíte partilham. Lê e escreve ficheiros simples, por isso pode ser usado sozinho ou entre os outros programas.

## 🏛️ Arquitetura

<p align="center">
  <img src="images/architecture.svg" alt="Arquitetura de EH3D-PHOENIX-LASER: quem o usa, o que contém e de que depende" width="100%">
</p>


Como as partes deste repositório se encaixam: quem o usa, o que contém e de que depende. A imagem é desenhada a partir de `docs/architecture.json` e verificada com o código.

## ✨ Funcionalidades

* **Biblioteca e linha de comandos:** cada comando de `phoenix-laser` é uma função da biblioteca, por isso outro programa da suíte pode chamá-la sem lançar um processo.
* **Linha de comandos estrita:** uma opção mal escrita (`--powr`) ou uma palavra a mais é um erro que a nomeia, não algo que se ignora; caso contrário o programa faria a sua saída com os valores por omissão.
* **Testes com entradas danificadas:** os testes também dão ficheiros vazios, lixo, números enormes e ficheiros cortados, e nenhum pode entrar em pânico.
* **Sem adivinhar:** um valor sem sentido é um erro que diz qual e porquê, nunca um resultado que apenas parece certo.
* **Sem código unsafe:** `#![forbid(unsafe_code)]` na biblioteca e no programa.

## 🔒 Segurança e proteção

| Ameaça | Defesa |
|---|---|
| Um ficheiro danificado, cortado ou vazio | sem pânico: os testes cortam uma entrada válida em cada posição e dão lixo |
| Um número sem sentido (negativo, não finito, absurdamente grande) | recusado com um erro que nomeia o valor; nada é arredondado para algo plausível |
| Uso inseguro de memória | Rust seguro e simples com `forbid(unsafe_code)` |
| Um desenho hostil (scripts, entidades declaradas, aninhamento profundo, milhões de pontos, números enormes) | os scripts, as imagens e as ligações nunca são executados nem descarregados; uma entidade declarada é um erro; o tamanho, a profundidade, os elementos, as formas, os pontos e os troços de uma curva são limitados; uma forma com um ponto fora do intervalo fica de fora |
| Um programa que poderia ligar o laser sem velocidade ou acima da potência máxima | cada programa é verificado linha a linha antes de ser entregue e o que falha não é escrito |
| O ficheiro de saída escolhido por cima do desenho ou das definições de que provém | recusado, na janela e nos comandos |

**O que não é defendido:** um resultado produzido sem erros não é um resultado verificado. Nada aqui conhece os limites de uma máquina e o programa não teve revisão independente. Comunique uma vulnerabilidade em privado (ver [SECURITY.md](SECURITY.md)).

## 🔌 Interfaces

A biblioteca é `eh3d_phoenix_laser` (`use eh3d_phoenix_laser::...`); a linha de comandos é `phoenix-laser`. Os comandos são estes; cada um imprime o resultado na saída padrão e comunica um erro na saída de erros com um código de saída diferente de zero. Não há acesso à rede.

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

`DRAWING` é um ficheiro SVG ou DXF. `cut` segue os contornos (os de dentro primeiro); `vector` preenche as formas fechadas com linhas e segue as abertas; ambos tomam as definições de cada camada ou cor de `--config FILE` (ver [docs/vector-settings.md](docs/vector-settings.md)) e uma opção da linha de comandos altera-as em todas as camadas. `layers` lista as camadas de um desenho com o que se faria com cada uma.

## ⚙️ Configuração

A gravação de uma imagem define-se com argumentos da linha de comandos (ver Interfaces) e cada valor por omissão está escrito na ajuda do comando (`phoenix-laser help`). O corte e a gravação de um desenho definem-se com um ficheiro de definições (`--config FILE`, ou «Carregar definições» na janela): os limites dos leitores, como se lê um ficheiro, a ordem dos cortes, a máquina e, para cada camada ou cor, o modo (cortar, gravar ou ignorar), a potência, a velocidade, as passagens, o avanço do foco, a largura do feixe e o seu lado, as pontes e a hachura. Cada chave, o seu valor por omissão e o seu intervalo estão em [docs/vector-settings.md](docs/vector-settings.md) e há um exemplo em `examples/plate.cfg`; uma chave desconhecida ou um valor fora do intervalo é um erro que nomeia a sua linha. Os limites fixos do leitor de imagens estão no código como constantes com nome e são cobertos por testes.

## 🚀 Início rápido

Precisa de uma cadeia de ferramentas Rust estável (1.88 ou mais recente). O repositório EH3D-COMMON tem de estar clonado ao lado deste: o leitor de imagens é tomado dele por caminho.

```bash
cargo build --release
cargo test
cargo run --release -- engrave examples/gradient.pgm examples/gradient.nc --width-mm 16 --dot-mm 0.5 --overscan auto
cargo run --release -- cut examples/plate.svg examples/plate.nc --config examples/plate.cfg
cargo run --release --features gui   # the window
cargo run --release --features gui -- --shot screen.png examples/plate.svg   # a picture of the window
```

Para tirar uma captura da janela sem a abrir à mão, execute-o com `--shot FICHEIRO.png` (também `--size WxH`, `--theme forge|dark|light`, `--wait N`, `--lang CODE`, `--open FILE`, `--replace`): abre-se com esse tamanho, guarda o PNG, não guarda definições e fecha-se sozinho (código de saída 0 se o ficheiro foi escrito, 1 se não, 2 se os argumentos estiverem errados).

## 🧪 Testes

O conjunto tem 309 testes (`cargo test`, executados em Windows e Linux no CI). Verificam os casos normais, os casos-limite e as entradas danificadas: ficheiros vazios, lixo e ficheiros cortados nunca provocam pânico. O CI executa ainda `cargo fmt --check`, `cargo clippy` com avisos como erros e `python -I tools/ci_validate.py`.

## 🏗️ Compilação e publicação

A versão está em `Cargo.toml` e em `eh3d.project.json`; cada versão tem a sua entrada em [CHANGELOG.md](CHANGELOG.md). `build.bat` / `build.sh` compilam o programa, `test.bat` / `test.sh` executam os testes e `run.bat` / `run.sh` executam-no com um exemplo. As versões são publicadas no GitHub.

**Transferências:** Com cada versão deste repositório são publicados programas já feitos para cada sistema: Windows (64 e 32 bits), Linux (64 bits) e macOS (Apple silicon e Intel). Cada pacote, `phoenix-laser-<versão>-<sistema>.zip` ou `.tar.gz`, leva o programa, a licença, este README, o registo de alterações, o ícone e os ficheiros de exemplo, e vem com o seu ficheiro `.sha256`; a versão também lista todos os ficheiros em `SHA256SUMS` e `build-manifest.json`. Ainda não foi publicada nenhuma versão.

**Assinados e verificáveis:** os pacotes de uma versão são assinados com a chave de versões da EH3D, e o ficheiro `.sig` de cada pacote é publicado ao lado (uma assinatura OpenSSH, espaço de nomes `eh3d-release`). Verifique uma transferência antes de a executar: compare o seu SHA-256 (`sha256sum -c phoenix-laser-<versão>-<sistema>.tar.gz.sha256`, ou `Get-FileHash` no Windows) e verifique a assinatura com `ssh-keygen -Y verify -f allowed_signers -I eh3d-release -n eh3d-release -s <pacote>.sig < <pacote>`, onde `allowed_signers` contém a linha `eh3d-release <tipo de chave> <chave pública das versões da EH3D>`.

## 📂 Estrutura do repositório

<p align="center">
  <img src="images/repository-structure.svg" alt="Estrutura do repositório EH3D-PHOENIX-LASER" width="100%">
</p>

<details>
<summary>Versão em texto da árvore</summary>

```text
EH3D-PHOENIX-LASER/
├── src/
│   ├── lib.rs                                   a biblioteca: o que cada comando usa
│   ├── main.rs                                  a linha de comandos: engrave, calibrate, info, cut, vector e layers
│   ├── calibrate.rs                             uma parte da lógica
│   ├── cli_args.rs                              os argumentos da linha de comandos, o mesmo ficheiro em cada utilitário pequeno
│   ├── dither.rs                                uma parte da lógica
│   ├── gcode_out.rs                             a única forma de escrever um número de G-code e o cabeçalho padrão
│   ├── raster.rs                                uma parte da lógica
│   ├── shot.rs                                  o modo de captura da janela (--shot): os seus argumentos, lidos e verificados sem ecrã
│   ├── vector.rs                                um desenho lido: formas em mm, camadas, limites e a geometria básica
│   ├── flatten.rs                               curvas em troços retos dentro de uma tolerância (Bézier, arcos)
│   ├── svg.rs                                   o leitor de SVG: um subconjunto seguro, sem scripts, imagens nem entidades
│   ├── dxf.rs                                   o leitor de DXF: linhas, polilinhas, arcos, círculos e splines, com unidades
│   ├── energy.rs                                a energia de uma passagem por milímetro ou milímetro quadrado, a partir da potência ótica
│   ├── offset.rs                                deslocar um contorno de lado pela largura do feixe, verificado aresta a aresta
│   ├── order.rs                                 a ordem dos cortes: linhas unidas, o interior primeiro, pouco deslocamento
│   ├── cut.rs                                   do desenho ao G-code: corte, pontes, hachura, passagens e a verificação de segurança
│   ├── vconfig.rs                               o ficheiro de definições do trabalho vetorial: limites, máquina e cada camada
│   ├── browse.rs                                a lista de ficheiros da janela que abre um ficheiro
│   ├── gui.rs                                   a janela (egui, com a funcionalidade gui): desenha o estado e transmite o que a pessoa faz
│   ├── lang.rs                                  as palavras da janela nos oito idiomas
│   ├── viewport.rs                              zoom e deslocamento da imagem, em números simples
│   └── window.rs                                o estado da janela sem ecrã: imagem ou desenho, definições, camadas, passagens e exportação
├── tests/
│   ├── dry_run.rs                               testes de --dry-run: conta o mesmo e não escreve nada
│   ├── laser.rs                                 testes da biblioteca e da linha de comandos
│   ├── readme.rs                                testes que executam o que o README e os scripts run prometem
│   ├── strict.rs                                testes da linha de comandos: opções e palavras que não conhece
│   ├── vector.rs                                testes do corte a partir de desenhos: comandos, exemplos, ficheiros hostis e a verificação do G-code
│   └── window_cli.rs                            testes de que uma compilação sem a funcionalidade gui continua a ser só linha de comandos
├── examples/
│   ├── gradient.pgm                             um exemplo para experimentar o programa
│   ├── plate.svg                                uma placa com um furo e um painel para gravar (SVG)
│   ├── plate.dxf                                a mesma placa em DXF
│   └── plate.cfg                                as definições das suas camadas: potência, velocidade, passagens, largura do feixe, pontes e hachura
├── docs/
│   ├── architecture.json                        dados da imagem de Arquitetura
│   ├── structure.json                           dados da imagem de Estrutura
│   └── vector-settings.md                       o ficheiro de definições do trabalho vetorial, chave a chave
├── tools/                                       verificação da CI e do README e ferramentas de imagens
├── images/                                      banner e as imagens de Arquitetura e Estrutura do repositório
├── .github/                                     fluxos de CI e de publicação, modelos de issues e pull requests
├── eh3d.project.json                            manifesto: versão, papel, maturidade e comando de compilação
├── Cargo.toml Cargo.lock                        o workspace e as suas dependências fixadas
├── CHANGELOG.md                                 o que mudou em cada versão
├── SECURITY.md SUPPORT.md                       como comunicar uma vulnerabilidade e onde perguntar
└── CONTRIBUTING.md CODE_OF_CONDUCT.md LICENSE   como contribuir, o código de conduta e a licença
```

</details>

## 🗺️ Roteiro e limites conhecidos

**Limites conhecidos hoje**

* A janela ainda não foi vista num ecrã.
* Não foi experimentado numa máquina nem com uma coleção de ficheiros reais e não teve revisão independente.
* Um desenho é lido num subconjunto seguro (sem `use`, texto, gradientes nem estilos com seletores; sem elipses, hachuras nem blocos num DXF); um contorno mais estreito do que o feixe não é cortado e o programa di-lo; o laser é o de um controlador GRBL com `M4` e nada foi experimentado num laser nem num material reais.

**A seguir**

* Mostrar a densidade de potência na placa antes de queimar (`layers` já escreve a energia de uma passagem quando `laser_watts` está definido) e manter constante a potência nos cantos de um corte.
* Ver a janela num ecrã e terminá-la.
* Experimentá-lo com desenhos reais e com os outros programas da suite.

## 🌍 Idiomas

O README e os textos existem em inglês, espanhol, francês, italiano, alemão, chinês, japonês e português europeu.

## 🔗 Projetos relacionados

* [EH3D-COMMON](https://github.com/JuanenRac/EH3D-COMMON): os crates partilhados, entre eles o leitor de imagens
* [EH3D-STENCILCAM](https://github.com/JuanenRac/EH3D-STENCILCAM): os stencils de pasta de solda
* [EH3D-CAM-DEBUGGER](https://github.com/JuanenRac/EH3D-CAM-DEBUGGER): a verificação do G-code antes de chegar a uma máquina
* [EH3D-TOOL-MANAGER](https://github.com/JuanenRac/EH3D-TOOL-MANAGER): o armazém de ferramentas partilhado pelos programas CAM
* [EH3D-MOTIONCNC](https://github.com/JuanenRac/EH3D-MOTIONCNC): o programa de controlo de máquina
* [EH3D-SUITE-LAUNCHER](https://github.com/JuanenRac/EH3D-SUITE-LAUNCHER): o centro que encontra e arranca os programas da suíte

## 📚 Documentação

* [Registo de alterações](CHANGELOG.md)
* [Dados de arquitetura e estrutura](docs/architecture.json): os dados a partir dos quais as duas imagens são desenhadas.
* [Definições do trabalho vetorial](docs/vector-settings.md): cada chave do ficheiro de definições com o seu valor por omissão e o seu intervalo.

## 🤝 Contribuir e comunidade

* Antes de um pull request execute `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test` e `python -I tools/ci_validate.py`: tudo tem de passar. Veja [CONTRIBUTING.md](CONTRIBUTING.md) e o [código de conduta](CODE_OF_CONDUCT.md).
* O README existe em oito idiomas: uma alteração ao texto faz-se nos oito ficheiros, e as duas imagens são desenhadas a partir de `docs/architecture.json` e `docs/structure.json` por `tools/make_architecture.py` e `tools/make_structure.py`.
* Comunique uma vulnerabilidade em privado: [SECURITY.md](SECURITY.md). Perguntas, ideias e relatos: electrohobby3d@gmail.com ([SUPPORT.md](SUPPORT.md)).

## 👤 AUTOR

**JuanenRac (Electro Hobby 3D)** · electrohobby3d@gmail.com

## 📜 LICENÇA

Software: GPL-3.0-or-later. Ver [LICENSE](LICENSE). Os projetos de hardware da suite usam a CERN OHL-S.
