<p align="center">
  <img src="images/EH3D_BANNER.svg" alt="EH3D-LASER-CAM banner" width="100%">
</p>

# 📖 EH3D-LASER-CAM

<p align="center"><a href="README.md">🇺🇸 English</a> | <a href="README_spa.md">🇪🇸 Español</a> | <a href="README_fra.md">🇫🇷 Français</a> | <a href="README_ita.md">🇮🇹 Italiano</a> | <a href="README_deu.md">🇩🇪 Deutsch</a> | <a href="README_zho.md">🇨🇳 中文</a> | <a href="README_jpn.md">🇯🇵 日本語</a> | 🇵🇹 <b>Português</b></p>

### Gravação a laser a partir de uma imagem: tramagem, G-code de trama e grelha de calibração

<p align="center">
  <img src="https://img.shields.io/badge/License-GPL%203.0--or--later-blue.svg" alt="GPL 3.0 or later">
  <img src="https://img.shields.io/badge/Stack-Rust-DEA584.svg" alt="Rust">
  <img src="https://img.shields.io/badge/Languages-8-00E5FF.svg" alt="Languages">
  <img src="https://img.shields.io/badge/Version-0.0.2-0f6cbd.svg" alt="Version">
  <img src="https://img.shields.io/badge/Maturity-scaffolding-FFB020.svg" alt="Maturity">
</p>

---

**Verificação de honestidade - o que funciona hoje:** uma biblioteca Rust e um programa de linha de comandos (`eh3d-lasercam`) com 53 testes que correm num computador: transforma uma imagem em pontos por tramagem e escreve o G-code de um laser que queima linha a linha com potência dinâmica e sobrecurso, mais uma grelha de teste de potência contra avanço. Ainda não tem janela, não foi experimentado numa máquina e não foi verificado com uma coleção de ficheiros reais.

---

> **Estado de segurança:** este programa não controla nenhuma máquina e não é um sistema de segurança. O que produz deve ser verificado por uma pessoa antes de se executar algo numa máquina; nada disto correu em hardware real.


## 📋 Num relance

| | |
|---|---|
| Versão | `0.0.2` |
| Maturidade | Estrutura inicial |
| Testes | 53 |
| Tecnologias | Rust |
| Em hardware real | Não experimentado: nada disto correu numa máquina |

**Ir para:** [Visão geral](#-visão-geral) · [Arquitetura](#-arquitetura) · [Início rápido](#-início-rápido) · [Testes](#-testes) · [Estrutura](#-estrutura-do-repositório) · [Relacionados](#-projetos-relacionados)

## 🚦 Estado do projeto

Cada linha é uma parte deste repositório e a luz diz até onde chega hoje. É escrita com o que o repositório consegue demonstrar (testes, compilações, execuções), não com o que está previsto.

🟢 Feito e verificado · 🟡 Funciona, com limites conhecidos · 🟠 Feito a meio ou ainda por verificar · 🔴 Por fazer ou a não funcionar

| Área | Estado | O que significa aqui |
|---|---|---|
| Núcleo | 🟡 | transforma uma imagem em pontos por tramagem e escreve o G-code de um laser que queima linha a linha com potência dinâmica e sobrecurso, mais uma grelha de teste de potência contra avanço; testado num computador com entradas normais e danificadas, não com uma coleção de ficheiros reais. |
| Linha de comandos | 🟡 | `eh3d-lasercam` executa as mesmas funções da biblioteca e imprime o resultado; cada erro volta como mensagem e código de saída diferente de zero. |
| Testes | 🟡 | 53 testes passam (`cargo test`), com casos normais, casos-limite e entradas danificadas. |
| Segurança | 🟡 | Rust seguro e simples (`forbid(unsafe_code)`): nada entra em pânico com os ficheiros recebidos e um valor mau é um erro com o seu motivo; sem revisão independente. |
| Janela | 🔴 | Previsto: a janela egui comum à suíte; hoje existe apenas a linha de comandos. |
| Hardware real | 🔴 | Não experimentado: nada disto foi executado numa máquina. |
| Documentação | 🟡 | README em oito idiomas, o registo de alterações e as duas imagens desenhadas a partir de `docs/`. |

## 🎯 Visão geral

* **A imagem** (`eh3d-image`): PNG (cinzento, cor, transparente, 8 ou 16 bits) e PGM (texto e binário) tornam-se níveis de cinzento, com as partes transparentes como branco e o tamanho limitado para que um ficheiro hostil não possa pedir gigabytes; `resize` faz a média dos píxeis para que um ponto seja tão grande como o feixe.
* **Os pontos** (`src/dither.rs`): um limiar, ou difusão de erro de Floyd-Steinberg ou Atkinson; os testes verificam que a escuridão de um cinzento se conserva (um cinzento médio queima cerca de metade dos pontos) e que o branco não queima nada e o preto queima tudo.
* **O laser** (`src/raster.rs`, `src/calibrate.rs`): uma linha depois da outra, da esquerda para a direita e depois da direita para a esquerda, `M4` com `S0` enquanto viaja e a potência escolhida onde queima, um sobrecurso fixo ou calculado a partir da aceleração (v ao quadrado sobre 2a) e uma grelha de quadrados com uma potência por coluna e um avanço por linha para achar o que convém a um material.
* **O cabeçalho e os números de todo o ficheiro de G-code** (`src/gcode_out.rs`): o ficheiro começa com linhas de comentário que dizem o gerador e a sua versão, as unidades (mm, G21), a imagem de que foi feito com o seu SHA-256 e as ferramentas (o laser e a sua potência); uma grelha de calibração diz que não tem origem. Um número escreve-se de uma só maneira (no máximo três casas decimais, sem `-0`) e um valor que não é finito é recusado em vez de ser escrito como `NaN` ou `inf`.
* **`--dry-run`:** `engrave` calcula o resultado e conta-o (pontos, linhas, passagens, comprimento queimado, tempo) sem escrever o ficheiro de saída, que então pode ser omitido.

## 🧩 Papel no EH3D

EH3D-LASER-CAM é um programa da EH3D Fabrication Suite. Hoje funciona como biblioteca e linha de comandos; a sua janela, com o aspeto que os outros programas da suíte partilham, está prevista. Lê e escreve ficheiros simples, por isso pode ser usado sozinho ou entre os outros programas.

## 🏛️ Arquitetura

<p align="center">
  <img src="images/architecture.svg" alt="Arquitetura de EH3D-LASER-CAM: quem o usa, o que contém e de que depende" width="100%">
</p>


Como as partes deste repositório se encaixam: quem o usa, o que contém e de que depende. A imagem é desenhada a partir de `docs/architecture.json` e verificada com o código.

## ✨ Funcionalidades

* **Biblioteca e linha de comandos:** cada comando de `eh3d-lasercam` é uma função da biblioteca, por isso outro programa da suíte pode chamá-la sem lançar um processo.
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

**O que não é defendido:** um resultado produzido sem erros não é um resultado verificado. Nada aqui conhece os limites de uma máquina e o programa não teve revisão independente. Comunique uma vulnerabilidade em privado (ver [SECURITY.md](SECURITY.md)).

## 🔌 Interfaces

A biblioteca é `eh3d_laser_cam` (`use eh3d_laser_cam::...`); a linha de comandos é `eh3d-lasercam`. Os comandos são estes; cada um imprime o resultado na saída padrão e comunica um erro na saída de erros com um código de saída diferente de zero. Não há acesso à rede.

```text
EH3D-LASER-CAM/
├── src/
│   ├── lib.rs                                   a biblioteca: o que cada comando usa
│   ├── main.rs                                  a linha de comandos
│   ├── calibrate.rs                             uma parte da lógica
│   ├── cli_args.rs                              os argumentos da linha de comandos, o mesmo ficheiro em cada utilitário pequeno
│   ├── dither.rs                                uma parte da lógica
│   ├── gcode_out.rs                             a única forma de escrever um número de G-code e o cabeçalho padrão
│   └── raster.rs                                uma parte da lógica
├── tests/
│   ├── dry_run.rs                               testes de --dry-run: conta o mesmo e não escreve nada
│   ├── laser.rs                                 testes da biblioteca e da linha de comandos
│   ├── readme.rs                                testes que executam o que o README e os scripts run prometem
│   └── strict.rs                                testes da linha de comandos: opções e palavras que não conhece
├── examples/
│   └── gradient.pgm                             um exemplo para experimentar o programa
├── docs/
│   ├── architecture.json                        dados da imagem de Arquitetura
│   └── structure.json                           dados da imagem de Estrutura
├── tools/                                       verificação da CI e do README e ferramentas de imagens
├── images/                                      banner e as imagens de Arquitetura e Estrutura do repositório
├── .github/                                     fluxos de CI e de publicação, modelos de issues e pull requests
├── eh3d.project.json                            manifesto: versão, papel, maturidade e comando de compilação
├── Cargo.toml Cargo.lock                        o workspace e as suas dependências fixadas
├── CHANGELOG.md                                 o que mudou em cada versão
├── SECURITY.md SUPPORT.md                       como comunicar uma vulnerabilidade e onde perguntar
└── CONTRIBUTING.md CODE_OF_CONDUCT.md LICENSE   como contribuir, o código de conduta e a licença
```

## ⚙️ Configuração

Ainda não há ficheiro de configuração: o que a pessoa escolhe é dado como argumentos da linha de comandos (ver Interfaces) e cada valor por omissão está escrito na ajuda do comando (`eh3d-lasercam help`). Os limites fixos são constantes com nome no código, cobertas por testes.

## 🚀 Início rápido

Precisa de uma cadeia de ferramentas Rust estável (1.75 ou mais recente). O repositório EH3D-COMMON tem de estar clonado ao lado deste: o leitor de imagens é tomado dele por caminho.

```bash
cargo build --release
cargo test
cargo run --release -- engrave examples/gradient.pgm examples/gradient.nc --width-mm 16 --dot-mm 0.5 --overscan auto
```

## 🧪 Testes

O conjunto tem 53 testes (`cargo test`, executados em Windows e Linux no CI). Verificam os casos normais, os casos-limite e as entradas danificadas: ficheiros vazios, lixo e ficheiros cortados nunca provocam pânico. O CI executa ainda `cargo fmt --check`, `cargo clippy` com avisos como erros e `python -I tools/ci_validate.py`.

## 🏗️ Compilação e publicação

A versão está em `Cargo.toml` e em `eh3d.project.json`; cada versão tem a sua entrada em [CHANGELOG.md](CHANGELOG.md). `build.bat` / `build.sh` compilam o programa, `test.bat` / `test.sh` executam os testes e `run.bat` / `run.sh` executam-no com um exemplo. As versões são publicadas no GitHub.

## 📂 Estrutura do repositório

<p align="center">
  <img src="images/repository-structure.svg" alt="Estrutura do repositório EH3D-LASER-CAM" width="100%">
</p>

<details>
<summary>Versão em texto da árvore</summary>

```text
EH3D-LASER-CAM/
├── src/
│   ├── lib.rs                                   a biblioteca: o que cada comando usa
│   ├── main.rs                                  a linha de comandos
│   ├── calibrate.rs                             uma parte da lógica
│   ├── cli_args.rs                              os argumentos da linha de comandos, o mesmo ficheiro em cada utilitário pequeno
│   ├── dither.rs                                uma parte da lógica
│   ├── gcode_out.rs                             a única forma de escrever um número de G-code e o cabeçalho padrão
│   └── raster.rs                                uma parte da lógica
├── tests/
│   ├── dry_run.rs                               testes de --dry-run: conta o mesmo e não escreve nada
│   ├── laser.rs                                 testes da biblioteca e da linha de comandos
│   ├── readme.rs                                testes que executam o que o README e os scripts run prometem
│   └── strict.rs                                testes da linha de comandos: opções e palavras que não conhece
├── examples/
│   └── gradient.pgm                             um exemplo para experimentar o programa
├── docs/
│   ├── architecture.json                        dados da imagem de Arquitetura
│   └── structure.json                           dados da imagem de Estrutura
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

* Ainda não tem janela: existem apenas a biblioteca e a linha de comandos.
* Não foi experimentado numa máquina nem com uma coleção de ficheiros reais e não teve revisão independente.
* Queima pontos, não vetores (ainda sem corte de contorno), o laser é o de um controlador GRBL com `M4` e nada foi experimentado num laser ou material reais.

**A seguir**

* Cortar contornos vetoriais com potência que segue a velocidade, mostrar a densidade de potência na placa antes de queimar e acrescentar a janela da suíte.
* Acrescentar a janela com o aspeto partilhado pelos programas da suíte, nos oito idiomas.
* Experimentá-lo com ficheiros reais e com os outros programas da suíte.

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

## 🤝 Contribuir e comunidade

* Antes de um pull request execute `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test` e `python -I tools/ci_validate.py`: tudo tem de passar. Veja [CONTRIBUTING.md](CONTRIBUTING.md) e o [código de conduta](CODE_OF_CONDUCT.md).
* O README existe em oito idiomas: uma alteração ao texto faz-se nos oito ficheiros, e as duas imagens são desenhadas a partir de `docs/architecture.json` e `docs/structure.json` por `tools/make_architecture.py` e `tools/make_structure.py`.
* Comunique uma vulnerabilidade em privado: [SECURITY.md](SECURITY.md). Perguntas, ideias e relatos: electrohobby3d@gmail.com ([SUPPORT.md](SUPPORT.md)).

## 👤 AUTOR

**JuanenRac (Electro Hobby 3D)** · electrohobby3d@gmail.com

## 📜 LICENÇA

Software: GPL-3.0-or-later. Ver [LICENSE](LICENSE). Os projetos de hardware da suite usam a CERN OHL-S.
