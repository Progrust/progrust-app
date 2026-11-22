---
type: Specification
title: "デザインシステムのファイルと変更の手順"
description: "デザインシステム「Progrust」「Progrust Video」のファイルの置き場所（`design-system/progrust/`・`design-system/progrust-video/`）とリポジトリのファイルを正とすること、ディレクトリの中身（`tokens.json` から作る `tokens.css`、索引の `design-system.json` など）とリポジトリに置かないファイル、表示用のアーティファクトの URL と読み取り専用にすること、デザインシステムを変えるときの手順（`tokens.css` の作り直し、アイコンの追加、アーティファクトへの送信と中身の確認、画面デザインの中の写しの取り直し）、`python3 tools/design-system.py check` が確かめることを定める。"
---

## 置き場所

- デザインシステム「Progrust」のファイルはリポジトリの `design-system/progrust/` に、動画のデザインシステム「Progrust Video」のファイルは `design-system/progrust-video/` に置く（[リポジトリ構成](../architecture/repository.md)）。
- リポジトリのファイルを正とする。claude.ai のアーティファクト（下の「表示用のアーティファクト」）は、リポジトリのファイルを送ったもので、表示に使う。

## ディレクトリの中身

- 各ディレクトリには、アーティファクトの `project/` の下と同じ形でファイルを置く。形は、アーティファクトの種類「Design System」の決まりに従う。

| ファイル | 内容 |
|---|---|
| `README.md` | デザインシステムの使い方のルール |
| `tokens.json` | 色・書体・余白などの値（トークン） |
| `tokens.css` | トークンの CSS。`tokens.json` から作るファイルで、手で書かない（下の「変更の手順」） |
| `design-system.json` | 索引。アイコンの記録（`assetGroups`）に、表示用のアーティファクトのアセットの id（`blob`）を持つ。ページを読み取り専用にする指定（`"editing": "code"`）も持つ |
| `components/bundle.css` | 部品のクラスの CSS |
| `components/{部品}/` | 部品ごとの見本（`preview.html`）と使い方（`README.md`） |
| `assets/Icons/` | アイコンの SVG と、そのライセンスと説明。「Progrust」だけにある |
| `chapter-colors.md` | 章カラーごとの実際の色の早見表。「Progrust Video」だけにある |

- 次のファイルは、リポジトリに置かない。
  - アーティファクトの種類が持つファイル（`index.html`、`SKILL.md`、`artifact-type/`）
  - アーティファクトのページが作るファイル（`api/`、`manifest.json`）

## 表示用のアーティファクト

| デザインシステム | 表示用のアーティファクト |
|---|---|
| 「Progrust」 | https://claude.ai/artifact/3Pn15TZ4nZ1H2u8cAfLosA |
| 「Progrust Video」 | https://claude.ai/artifact/QGt9Cb1cFYaA8TU7kY9bPq |

- 表示用のアーティファクトは、リポジトリのファイルを送ったものである。部品の見本は、このページで見る。
- ページは読み取り専用にする（索引の `"editing": "code"`）。ページでは編集しない。
- Design System の種類の説明（表示用のアーティファクトの `SKILL.md` など）は、`"editing": "code"` のデザインシステムを `/design-sync` という仕組みで送る前提で書かれている（`tokens.css` を書くのは `/design-sync` だけ、など）。このプロジェクトは `/design-sync` を使わない。下の「変更の手順」がその代わりで、`tokens.css` と索引も、この手順で送る。

## 変更の手順

デザインシステムを変えるときは、次の順に行う。`{名前}` は `progrust` か `progrust-video` である。

1. リポジトリの `design-system/{名前}/` のファイルを直す。
2. `tokens.json` を変えたら、`python3 tools/design-system.py tokens-css {名前} --write` で `tokens.css` を作り直す。
3. アイコンを足す・替えるときは、ファイルを `assets/Icons/` に置き、Claude Code の Artifact ツールで表示用のアーティファクトのアセットストアへ上げる（`action: "publish"`、`url`、`asset: true`、`file_paths`。1 回に 25 件まで）。返った id を、索引の記録（`assetGroups.Icons.files.{ファイル名}` の `blob`）に書く。あわせて、同じ記録の `name`・`size`・`type` と、`order` も書く。
4. 索引の `lastChange`（`by`・`at`・`via`・`note`）を書き換える。
5. `python3 tools/design-system.py check` を実行し、指摘をすべて直す（下の「検査」）。
6. Artifact ツールで、表示用のアーティファクトへ送る。
   - 送る前に、表示用のアーティファクトを Artifact ツールで読む（`action: "read"` に `url` だけを渡す）。その会話でまだ読んでいないアーティファクトへの送信は断られ、`path` を付けた read だけでは足りない。断られても `force` は使わない。
   - `python3 tools/design-system.py files {名前}` が、渡す引数（`root`・`file_path`・`files`）を JSON で出す。`url` には表示用のアーティファクトの URL を渡す。
   - 出力の `files` をそのまま（全ファイル）渡してもよく、変えたファイルだけを送ってもよい。変えたファイルだけを送るときも、索引の項目（`"project/design-system.json": "design-system.json"`）は `files` に残す（`file_path` に索引を渡すだけにしない）。何回かに分けて送るときは、索引を最後の呼び出しで送る。
   - 出力の `assets` は、Artifact ツールの引数ではない。アイコンの一覧（`blob` が、表示用のアーティファクトでのアセットの id）で、7 でアイコンを取得するときに使う。
7. 送った後、表示用のアーティファクトのファイルを Artifact ツールで取得し、`python3 tools/design-system.py compare {名前} {取得したディレクトリ}` で、中身が同じことを確かめる。
   - 取得には `action: "read"`、`url` を使い、`out_dir` に空のディレクトリを渡す。
   - 公開ファイルは、`paths` に全部のパスを渡して 1 回で取得できる。
   - アイコンは、`path` に id（`files {名前}` の出力の `assets` の `blob`）を渡して 1 つずつ取得する。`out_dir` の直下に `{id}.svg` の名前で保存され、`compare` はその配置のまま読む。
   - アイコンを変えていないときは、アイコンの取得を省き、`compare` に `--skip-assets` を付けてよい（アイコンを比べない）。
   - あわせて、表示用のアーティファクトのファイルの一覧（`action: "list"`、`scope: "files"`）の `project/` の下のパスを、`files {名前}` の出力の `files` のキーと照らし、リポジトリに無いファイルが無いことを確かめる。
8. `tokens.css`・`tokens.json`・`components/bundle.css` のどれかを変えたときだけ、表示用のアーティファクトへ送った後に、画面デザイン「Progrust UIデザイン」（[画面の見た目とテーマ](appearance.md)「画面デザイン」）の中の写し（`project/ds/{名前}/` の同じ名前のファイル）を取り直す。
   - ほかのファイルだけを変えたときは、取り直さない。画面デザインの索引の `designSystems` の記録の `version` は、写しを取ったときの表示用のアーティファクトの版を表すもので、いまの版と違っていてよい。
   - 取り直しは、画面デザインのアーティファクトの種類「Design」の、デザインシステムを入れる手順をやり直して行う。この手順では、サーバーが表示用のアーティファクトからファイルを写し、画面デザインの索引 `project/canvas.json` の `designSystems` の記録も置き換える（Artifact ツールの 1 回の呼び出しで、写す元を表示用のアーティファクトにした `files` の項目と、索引を送る）。手順の細部は、画面デザインのアーティファクトの `SKILL.md` と `artifact-type/reference/design-system-components.md` に従う。
   - 種類の文書からは決まらないことは、次のようにする。
     - 送る前に、画面デザインのアーティファクトを Artifact ツールで読む（`action: "read"` に `url` だけを渡す）。
     - 写すファイルは、デザインシステムごとに `tokens.css`・`tokens.json`・`components/bundle.css` の 3 つだけにする。
     - 写しのフォルダの名前は、いまある `progrust`・`progrust-video` のままにする。
     - 索引 `project/canvas.json` を取得し、`designSystems` の中の、そのデザインシステムの記録の `version` を表示用のアーティファクトのいまの版の id（送信の結果か、表示用のアーティファクトの一覧に出る）に、`copiedAt` をいまの UTC の時刻にする。ほかのキーは変えない。
   - `project/ds/` の下のファイルを、直接送って直さない。
   - 取り直した後、7 と同じように取得し、`python3 tools/design-system.py compare {名前} {取得したディレクトリ} --prefix project/ds/{名前} --partial` で確かめる。
9. デザインシステムの変更は、それに伴う仕様の変更と同じコミットに入れる。

- 「Progrust」の、動画と共通の値を変えたときは、「Progrust Video」の写しも合わせる（[画面の見た目とテーマ](appearance.md)「デザインシステム」）。

## 検査

`python3 tools/design-system.py check` は、次のことを確かめる。

- `tokens.css` が、`tokens.json` から作った内容と一致すること
- 索引（`design-system.json`）が読めて、`"editing": "code"` を持つこと
- 索引のアイコンの記録と、`assets/` のアイコンのファイルが、過不足なく合うこと
- アーティファクトのページが作るファイル（`api/`、`manifest.json`）が置かれていないこと
- デザインシステムの中の `.md` のファイル（README など）で、インラインコードのパスで書いた参照（例: `` `screens/appearance.md`「書体のファイル」 ``）が、実在する文書と、その文書の小見出しを指すこと
  - `/` を含むパスは、`documents/spec/` からのパスとして確かめる。ただし、`components/`・`assets/` で始まるパスと、`/` を含まないパス（`` `chapter-colors.md` `` など）は、そのデザインシステムのディレクトリからのパスとして確かめる。
  - 小見出しは、文書の中の見出し（`## 見出し` など）か、1 行だけの太字（`**見出し**`）と照らす。
  - 参照の書き方の決まり（パスをインラインコードで書くこと）を守っていない参照と、文の中身（「未決事項」と書いた事項が仕様で決まったかどうか、など）は、確かめない。
- 「Progrust Video」の `tokens.json` の、名前が `video-` で始まらないトークンと書体の族（`type.families`）が、「Progrust」の `tokens.json` に同じ名前で、同じ値であること（[画面の見た目とテーマ](appearance.md)「デザインシステム」の、画面と共通の値の写し）
  - 「Progrust」にあって「Progrust Video」に無いトークンは、指摘しない。
  - 書体のスタイル（`type.groups`）は比べない。
