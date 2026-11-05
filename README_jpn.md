<p align="center">
  <img src="images/EH3D_PHOENIX_LASER_BANNER.svg" alt="EH3D-PHOENIX-LASER banner" width="100%">
</p>

# <img src="images/brand/icon-64.png" alt="" width="40" height="40" align="absmiddle"> EH3D-PHOENIX-LASER
<p align="center"><a href="README.md">🇺🇸 English</a> | <a href="README_spa.md">🇪🇸 Español</a> | <a href="README_fra.md">🇫🇷 Français</a> | <a href="README_ita.md">🇮🇹 Italiano</a> | <a href="README_deu.md">🇩🇪 Deutsch</a> | <a href="README_zho.md">🇨🇳 中文</a> | 🇯🇵 <b>日本語</b> | <a href="README_por.md">🇵🇹 Português</a></p>

### 画像からのレーザー彫刻と図面からの切断：ディザリング、ラスター G コード、SVG・DXF の輪郭、キャリブレーション格子

<p align="center">
  <img src="https://img.shields.io/badge/License-GPL%203.0--or--later-blue.svg" alt="GPL 3.0 or later">
  <img src="https://img.shields.io/badge/Stack-Rust-DEA584.svg" alt="Rust">
  <img src="https://img.shields.io/badge/Languages-8-00E5FF.svg" alt="Languages">
  <img src="https://img.shields.io/badge/Version-0.0.3-0f6cbd.svg" alt="Version">
  <img src="https://img.shields.io/badge/Maturity-scaffolding-FFB020.svg" alt="Maturity">
</p>

---

**正直さの確認 - 現在実際に動くもの：** Rust ライブラリとコマンドラインプログラム（`phoenix-laser`）。コンピューター上で動く 294 件のテストがあります。画像をディザリングで点に変換し、動的出力とオーバースキャンで1行ずつ照射するレーザーの G コードを書き、SVG と DXF の図面を読んで輪郭を切断する G コード（内側から先に、ビーム幅、タブ、複数パスを考慮）と図形を線で彫刻する G コードを書き（各レイヤーの設定は1つのファイルに）、出力対送り速度のテスト格子も作ります。ウィンドウ（`--features gui` でビルド）は画像または図面を開き、レーザーの経路を表示し、設定を変更でき、G コードを書き出しますが、画面ではまだ確認していません。実機では試しておらず、実際のファイルの集まりでも確認していません。

---

> **安全とセキュリティの状況：** このプログラムは機械を制御せず、安全システムでもありません。出力は、機械で何かを実行する前に人が確認する必要があります。ここにあるものは実機で動かしたことがありません。


## 📋 ひと目で

| | |
|---|---|
| バージョン | `0.0.3` |
| 成熟度 | 足場 |
| テスト | 294 |
| 技術スタック | Rust |
| 実機での動作 | 未試験：ここにあるものは機械で動かしたことがありません |

**移動先：** [概要](#-概要) · [アーキテクチャ](#-アーキテクチャ) · [クイックスタート](#-クイックスタート) · [テスト](#-テスト) · [構成](#-リポジトリの構成) · [関連](#-関連プロジェクト)

## 🚦 プロジェクトの状況

各行はこのリポジトリの一部分で、信号は今日どこまで進んでいるかを示します。計画ではなく、リポジトリが示せるもの（テスト、ビルド、実行）に基づいて書かれています。

🟢 完了・検証済み · 🟡 動作する（既知の制限あり） · 🟠 一部のみ、または未検証 · 🔴 未実装または動作しない

| 領域 | 状態 | ここでの意味 |
|---|---|---|
| コア | 🟡 | 画像をディザリングで点に変換し、動的出力とオーバースキャンで1行ずつ照射するレーザーの G コードを書き、SVG と DXF の図面を読んでレイヤーごとに切断または彫刻し（輪郭の順序、ビーム幅、タブ、パス、ハッチ）、出力対送り速度のテスト格子も作ります。コンピューター上で通常入力と壊れた入力を使ってテスト済みで、実際のファイルの集まりでは確認していません。 |
| コマンドライン | 🟡 | `phoenix-laser`（`engrave`、`calibrate`、`info`、`cut`、`vector`、`layers`）はライブラリと同じ関数を実行して結果を表示します。エラーはすべてメッセージと 0 以外の終了コードで返ります。 |
| テスト | 🟡 | 294 件のテストが通ります（`cargo test`）。通常ケース、境界ケース、破損入力を含みます。 |
| セキュリティ | 🟡 | 安全な Rust のみ（`forbid(unsafe_code)`）。受け取ったファイルでパニックせず、不正な値は理由付きのエラーになります。独立したレビューはありません。 |
| ウィンドウ | 🟠 | 作成済み（egui、`--features gui`）：画像または SVG・DXF の図面を開く／ドロップする、レーザーの経路（切断は1色、移動は薄く）を見る、レイヤーの表を編集する、キャリブレーション格子、拡大縮小と移動、時間の見積り、書き出し。ロジックはテスト済みですが、まだビルドも画面での確認もしていません。 |
| 実機 | 🔴 | 未試験：ここにあるものは機械で実行したことがありません。 |
| ドキュメント | 🟡 | 8 言語の README、変更履歴、`docs/` から描いた 2 枚の図。 |

## 🎯 概要

* **画像**（`eh3d-image`）：PNG（グレー、カラー、透明、8 または 16 ビット）と PGM（テキスト、バイナリ）を濃淡に変換し、透明部分は白として扱い、悪意あるファイルがギガバイトを要求できないようサイズを制限します。`resize` は画素を平均し、1 点をビーム径の大きさにします。
* **点**（`src/dither.rs`）：しきい値、または Floyd-Steinberg か Atkinson の誤差拡散。テストで、灰色の濃さが保たれること（中間の灰色は約半分の点を焼く）、白は何も焼かず黒はすべて焼くことを確認します。
* **レーザー**（`src/raster.rs`、`src/calibrate.rs`）：1 行ずつ、左から右、次は右から左へ、移動中は `M4` と `S0`、焼く所では選んだ出力、固定または加速度から求めたオーバースキャン（v の 2 乗を 2a で割る）、そして材料に合う値を探すための、列ごとに出力・行ごとに送りを変えた正方形の格子。
* **すべての G コードファイルのヘッダーと数値**（`src/gcode_out.rs`）：ファイルの先頭にコメント行があり、生成元とそのバージョン、単位（mm、G21）、元の画像とその SHA-256、工具（レーザーと出力）が書かれます。校正グリッドには元がないと書かれます。数値の書き方は一つだけ（小数は最大 3 桁、`-0` なし）で、有限でない値は `NaN` や `inf` として書かれずに拒否されます。
* **`--dry-run`：** `engrave` は結果を計算して集計し（ドット、行、走査、焼く長さ、時間）、出力ファイルは書き出しません（その場合は省略できます）。
* **図面**（`src/svg.rs`、`src/dxf.rs`、`src/flatten.rs`、`src/vector.rs`）：SVG の安全なサブセット（すべてのコマンドのパス、図形、変換、グループ、単位）と DXF のエンティティ（線、円弧つきポリライン、円弧、円、スプライン、単位）をミリメートルの図形にします。スクリプト、画像、`use`、宣言されたエンティティ、未知のものは無視または拒否され、サイズ、深さ、要素数と点数は設定で制限され、曲線は許容差内の直線に分割されます。
* **切断と彫刻**（`src/order.rs`、`src/offset.rs`、`src/cut.rs`）：図形はレイヤー（DXF）または色（SVG）でまとめられ、各グループはそれぞれの設定で切断（ばらばらの線を結合、他の輪郭の内側を先に、移動が最短、ビーム幅を正しい側で差し引き、タブを残し、焦点を進めながら複数パス）または彫刻（閉じた図形を角度つきの線で塗りつぶし、偶奇規則）されます。
* **設定ファイル**（`src/vconfig.rs`、`docs/vector-settings.md`）：調整するすべての値（リーダーの上限、機械、各レイヤーの設定）は1行ずつ検証されるプレーンテキストにあり、コードには何も入っていません。
* **検証済みの G コード：**図面のプログラムは渡す前に1行ずつ検証されます。早送り中や速度なしでレーザーが点灯せず、`S` は全出力を超えず、ベッド外への移動はなく、最後は `M5` です。元の図面や設定の上にファイルが書かれることはありません。

## 🧩 EH3D での役割

EH3D-PHOENIX-LASER は EH3D Fabrication Suite のプログラムの一つです。ライブラリとコマンドラインとして動作し、`--features gui` でビルドすると、スイートの他のプログラムと共通の外観のウィンドウとしても動作します。単純なファイルを読み書きするため、単独でも他のプログラムの間でも使えます。

## 🏛️ アーキテクチャ

<p align="center">
  <img src="images/architecture.svg" alt="EH3D-PHOENIX-LASER のアーキテクチャ：誰が使い、何を持ち、何に依存するか" width="100%">
</p>


このリポジトリの各部分の関係：誰が使い、何を持ち、何に依存するか。図は `docs/architecture.json` から描かれ、コードと照合されます。

## ✨ 機能

* **ライブラリとコマンドライン：** `phoenix-laser` の各コマンドはライブラリの関数なので、スイートの他のプログラムがプロセスを起動せずに呼び出せます。
* **厳密なコマンドライン：** 書き間違えたオプション（`--powr`）や余分な語は無視されず、それを名指しするエラーになります。そうでないと、プログラムは既定値で出力を作ってしまいます。
* **破損入力のテスト：** 空のファイル、ゴミデータ、巨大な数値、途中で切れたファイルも与え、どれもパニックしてはいけません。
* **推測しない：** 意味のない値は、どの値かと理由を示すエラーになり、見かけだけ正しい結果にはなりません。
* **unsafe コードなし：** ライブラリとプログラムに `#![forbid(unsafe_code)]`。

## 🔒 セキュリティと安全

| 脅威 | 対策 |
|---|---|
| 破損・途中切れ・空のファイル | パニックなし：テストは有効な入力をあらゆる位置で切り、ゴミデータも与えます |
| 意味のない数値（負、無限、極端に大きい） | 値を示すエラーで拒否され、もっともらしい値に丸められることはありません |
| 安全でないメモリ使用 | `forbid(unsafe_code)` の安全な Rust のみ |
| 悪意のある図面（スクリプト、宣言されたエンティティ、深い入れ子、数百万の点、巨大な数値） | スクリプト、画像、リンクは実行も取得もされません。宣言されたエンティティはエラーです。サイズ、深さ、要素、図形、点、曲線の分割数は制限されます。範囲外の点を含む図形は除外されます |
| 速度なしで、または全出力を超えてレーザーを点灯させかねないプログラム | すべてのプログラムは渡す前に1行ずつ検証され、不合格のものは書き出されません |
| 元の図面や設定の上に選ばれた出力ファイル | ウィンドウでもコマンドでも拒否されます |

**防御していないこと：** エラーなく作られた結果は、検証された結果ではありません。ここには機械の限界を知るものはなく、プログラムは独立したレビューを受けていません。脆弱性は非公開で報告してください（[SECURITY.md](SECURITY.md) 参照）。

## 🔌 インターフェース

ライブラリは `eh3d_phoenix_laser`（`use eh3d_phoenix_laser::...`）、コマンドラインは `phoenix-laser` です。コマンドは次のとおりで、結果は標準出力に表示され、エラーは標準エラー出力に 0 以外の終了コードで報告されます。ネットワークアクセスはありません。

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

`DRAWING` は SVG または DXF ファイルです。`cut` は輪郭をたどり（内側から先に）、`vector` は閉じた図形を線で塗りつぶして開いた図形をたどります。どちらも各レイヤーまたは色の設定を `--config FILE` から読み込み（[docs/vector-settings.md](docs/vector-settings.md) を参照）、コマンドラインのオプションはすべてのレイヤーの設定を上書きします。`layers` は図面のレイヤーと、それぞれに行われる処理を一覧表示します。

## ⚙️ 設定

画像の彫刻はコマンドラインの引数で設定し（「インターフェース」を参照）、各既定値はコマンドのヘルプ（`phoenix-laser help`）に書かれています。図面の切断と彫刻は設定ファイル（`--config FILE`、またはウィンドウの「設定を読み込む」）で設定します。リーダーの上限、ファイルの読み方、切断の順序、機械、そして各レイヤーまたは色ごとのモード（切断・彫刻・除外）、出力、速度、パス数、焦点の移動量、ビーム幅とその側、タブ、ハッチです。すべてのキー、既定値、範囲は [docs/vector-settings.md](docs/vector-settings.md) にあり、例は `examples/plate.cfg` にあります。未知のキーや範囲外の値は、その行を示すエラーになります。画像リーダーの固定の上限は名前つき定数としてコードにあり、テストで確認されています。

## 🚀 クイックスタート

安定版の Rust ツールチェーン（1.88 以降）が必要です。EH3D-COMMON リポジトリをこのリポジトリの隣にクローンしておく必要があります。画像リーダーはそこからパスで取り込みます。

```bash
cargo build --release
cargo test
cargo run --release -- engrave examples/gradient.pgm examples/gradient.nc --width-mm 16 --dot-mm 0.5 --overscan auto
cargo run --release -- cut examples/plate.svg examples/plate.nc --config examples/plate.cfg
cargo run --release --features gui   # the window
```

## 🧪 テスト

テストは 294 件あります（`cargo test`、CI では Windows と Linux で実行）。通常ケース、境界ケース、破損入力を確認し、空のファイル、ゴミデータ、途中で切れたファイルでもパニックしません。CI は `cargo fmt --check`、警告をエラーにした `cargo clippy`、`python -I tools/ci_validate.py` も実行します。

## 🏗️ ビルドとリリース

バージョンは `Cargo.toml` と `eh3d.project.json` にあり、各バージョンは [CHANGELOG.md](CHANGELOG.md) に項目があります。`build.bat` / `build.sh` でビルド、`test.bat` / `test.sh` でテスト、`run.bat` / `run.sh` でサンプル実行します。リリースは GitHub で公開されます。

## 📂 リポジトリの構成

<p align="center">
  <img src="images/repository-structure.svg" alt="EH3D-PHOENIX-LASER のリポジトリ構成" width="100%">
</p>

<details>
<summary>ツリーのテキスト版</summary>

```text
EH3D-PHOENIX-LASER/
├── src/
│   ├── lib.rs                                   ライブラリ：各コマンドが使うもの
│   ├── main.rs                                  コマンドライン：engrave、calibrate、info、cut、vector、layers
│   ├── calibrate.rs                             ロジックの一部
│   ├── cli_args.rs                              コマンドライン引数：小さなツールすべてで同じファイル
│   ├── dither.rs                                ロジックの一部
│   ├── gcode_out.rs                             G コードの数値の書き方とヘッダーを一つに
│   ├── raster.rs                                ロジックの一部
│   ├── vector.rs                                読み込んだ図面：mm の図形、レイヤー、上限、基本の幾何
│   ├── flatten.rs                               許容差内で曲線を直線に分割（ベジェ、円弧）
│   ├── svg.rs                                   SVG リーダー：安全なサブセット（スクリプト・画像・エンティティなし）
│   ├── dxf.rs                                   DXF リーダー：線、ポリライン、円弧、円、スプライン（単位つき）
│   ├── offset.rs                                ビーム幅だけ輪郭を横にずらす（辺ごとに検証）
│   ├── order.rs                                 切断の順序：線の結合、内側から、移動を短く
│   ├── cut.rs                                   図面から G-code へ：切断、タブ、ハッチ、パス、安全確認
│   ├── vconfig.rs                               ベクター作業の設定ファイル：上限、機械、各レイヤー
│   ├── browse.rs                                ファイルを開くウィンドウのファイル一覧
│   ├── gui.rs                                   ウィンドウ（egui、gui 機能）：状態を描き、操作を伝える
│   ├── lang.rs                                  ウィンドウの文言（8言語）
│   ├── viewport.rs                              画像の拡大縮小と移動（単純な数値）
│   └── window.rs                                画面なしのウィンドウの状態：画像または図面、設定、レイヤー、パス、書き出し
├── tests/
│   ├── dry_run.rs                               --dry-runのテスト：同じ数を数え、何も書かない
│   ├── laser.rs                                 ライブラリとコマンドラインのテスト
│   ├── readme.rs                                READMEとrunスクリプトが約束する内容を実行するテスト
│   ├── strict.rs                                コマンドラインのテスト：知らないオプションと語
│   ├── vector.rs                                図面からの切断のテスト：コマンド、サンプル、悪意あるファイル、G-code の確認
│   └── window_cli.rs                            gui 機能なしのビルドがコマンドラインのままであることのテスト
├── examples/
│   ├── gradient.pgm                             試すためのサンプル
│   ├── plate.svg                                穴と彫刻パネルのある板（SVG）
│   ├── plate.dxf                                同じ板の DXF
│   └── plate.cfg                                各レイヤーの設定：出力、速度、パス数、ビーム幅、タブ、ハッチ
├── docs/
│   ├── architecture.json                        アーキテクチャ図のデータ
│   ├── structure.json                           構成図のデータ
│   └── vector-settings.md                       ベクター作業の設定ファイルをキーごとに説明
├── tools/                                       CI と README のチェック、図のツール
├── images/                                      バナーとアーキテクチャ・リポジトリ構成の図
├── .github/                                     CI とリリースのワークフロー、Issue と PR のテンプレート
├── eh3d.project.json                            マニフェスト：バージョン、役割、成熟度、ビルドコマンド
├── Cargo.toml Cargo.lock                        ワークスペースと固定された依存関係
├── CHANGELOG.md                                 各バージョンの変更点
├── SECURITY.md SUPPORT.md                       脆弱性の報告方法と問い合わせ先
└── CONTRIBUTING.md CODE_OF_CONDUCT.md LICENSE   貢献方法、行動規範、ライセンス
```

</details>

## 🗺️ ロードマップと既知の制限

**現時点で分かっている制限**

* ウィンドウは画面でまだ確認していません。
* 実機でも実際のファイルの集まりでも試しておらず、独立したレビューも受けていません。
* 図面は安全なサブセットで読み込みます（`use`、テキスト、グラデーション、セレクターつきスタイルは対象外。DXF の楕円、ハッチ、ブロックも対象外）。ビームより細い輪郭は切断されず、プログラムがそう伝えます。レーザーは `M4` を使う GRBL コントローラーのもので、実際のレーザーや材料では何も試していません。

**次のステップ**

* 照射の前に基板上の出力密度を表示し、切断の角で出力を一定に保つ。
* ウィンドウを画面で確認して完成させる。
* 実際の図面やスイートの他のプログラムで試す。

## 🌍 言語

README とテキストは、英語、スペイン語、フランス語、イタリア語、ドイツ語、中国語、日本語、ヨーロッパポルトガル語で提供されています。

## 🔗 関連プロジェクト

* [EH3D-COMMON](https://github.com/JuanenRac/EH3D-COMMON)：共有クレート（画像リーダーを含む）
* [EH3D-STENCILCAM](https://github.com/JuanenRac/EH3D-STENCILCAM): はんだペースト用ステンシル
* [EH3D-CAM-DEBUGGER](https://github.com/JuanenRac/EH3D-CAM-DEBUGGER): 機械に届く前の G コード検査
* [EH3D-TOOL-MANAGER](https://github.com/JuanenRac/EH3D-TOOL-MANAGER): CAM プログラム共通の工具庫
* [EH3D-MOTIONCNC](https://github.com/JuanenRac/EH3D-MOTIONCNC): 機械制御プログラム
* [EH3D-SUITE-LAUNCHER](https://github.com/JuanenRac/EH3D-SUITE-LAUNCHER): スイートのプログラムを見つけて起動するハブ

## 📚 ドキュメント

* [変更履歴](CHANGELOG.md)
* [アーキテクチャと構成のデータ](docs/architecture.json)：2 枚の図を描くためのデータ。
* [ベクター作業の設定](docs/vector-settings.md)：設定ファイルのすべてのキーと、その既定値および範囲。

## 🤝 コントリビュートとコミュニティ

* プルリクエストの前に `cargo fmt --check`、`cargo clippy --all-targets -- -D warnings`、`cargo test`、`python -I tools/ci_validate.py` を実行し、すべて通してください。[CONTRIBUTING.md](CONTRIBUTING.md) と[行動規範](CODE_OF_CONDUCT.md)を参照。
* README は 8 言語あります。文章の変更は 8 ファイルすべてで行い、2 枚の図は `tools/make_architecture.py` と `tools/make_structure.py` が `docs/architecture.json` と `docs/structure.json` から描きます。
* 脆弱性は非公開で報告してください：[SECURITY.md](SECURITY.md)。質問・提案・報告：electrohobby3d@gmail.com（[SUPPORT.md](SUPPORT.md)）。

## 👤 作者

**JuanenRac (Electro Hobby 3D)** · electrohobby3d@gmail.com

## 📜 ライセンス

ソフトウェア：GPL-3.0-or-later。[LICENSE](LICENSE) を参照。スイートのハードウェア設計には CERN OHL-S を使います。
