---
type: Specification
title: "画面デザインのファイルと変更の手順"
description: "画面デザイン「Progrust UIデザイン」のファイルの置き場所（`design-system/ui-design/`）とリポジトリのファイルを正とすること、ディレクトリの中身（索引の `canvas.json`、画面と状態ごとの `.dc.html`、配置の CSS の `styles/app.css`）とリポジトリに置かないファイル、ファイルの読み方、表示用のアーティファクトの URL とページで編集しないこと、画面デザインを変えるときの手順（送る前の確認、アーティファクトへの送信と中身の確認）、`python3 tools/design-system.py check` が確かめることを定める。"
---

## 置き場所

- 画面デザイン「Progrust UIデザイン」（[画面の見た目とテーマ](appearance.md)「画面デザイン」）のファイルは、リポジトリの `design-system/ui-design/` に置く（[リポジトリ構成](../architecture/repository.md)）。
- リポジトリのファイルを正とする。claude.ai のアーティファクト（下の「表示用のアーティファクト」）は、リポジトリのファイルを送ったもので、表示に使う。

## ディレクトリの中身

- ディレクトリには、アーティファクトの `project/` の下と同じ形でファイルを置く。形は、アーティファクトの種類「Design」の決まりに従う。

| ファイル | 内容 |
|---|---|
| `canvas.json` | 索引。`.dc.html` のファイルごとの、画面での位置と大きさ、題名（`title`）、載せるページ（`page`）と、ファイルの並び（`order`）、注記（`notes`）、画面デザインの中のデザインシステムの写しの記録（`designSystems`）を持つ |
| `{名前}.dc.html` | 画面と状態ごとのデザイン |
| `styles/app.css` | 画面ごとの配置の CSS。クラスの接頭辞は `pgx-` |

- 次のファイルは、リポジトリに置かない。
  - 画面デザインの中のデザインシステムの写し（アーティファクトの `project/ds/` の下）。表示用のアーティファクトにだけある。写しの元は、リポジトリの `design-system/progrust/`・`design-system/progrust-video/` である（[デザインシステムのファイルと変更の手順](design-system-files.md)）。
  - アーティファクトの種類が持つファイル（`index.html`、`SKILL.md`、`artifact-type/`、`.dc.html` が読み込む `support.js`）

## 読み方

- どの `.dc.html` がどの画面のどの状態かは、`canvas.json` の `boards` の `title` に書いてある。
- 中身が数十行だけの `.dc.html` は、別の `.dc.html` を `<dc-import>` で取り込み、状態を表す属性（例: `state="wrong"`）を渡したものである。配置は、取り込まれる側のファイルを読む。
- `.dc.html` の中の `ds/progrust/…`・`ds/progrust-video/…` への参照は、リポジトリの `design-system/progrust/`・`design-system/progrust-video/` の同じパスのファイルに当たる。
- `styles/app.css` は、画面の配置を読むためのものである。アプリの画面の CSS は、[画面の見た目とテーマ](appearance.md)「デザインシステムの CSS とアイコン」の書き方に従って書く。`styles/app.css` には、その書き方に合わない規則（`.app p` のように、中に動画の枠を含む画面全体のクラスの下で要素を指す規則）と、画面デザインの中でだけ使う規則（`.pgx-sheet` などの部品の見本のページの配置、`.pgx-video-fit` などの動画の中の絵を縮めて映す配置）があるので、そのまま写さない。
- 画面を実装した後の、その画面の扱いは、[画面の見た目とテーマ](appearance.md)「画面デザイン」に従う。

## 表示用のアーティファクト

| 画面デザイン | 表示用のアーティファクト |
|---|---|
| 「Progrust UIデザイン」 | https://claude.ai/artifact/SL7VXW5D7weHs8tLG9zHNx |

- 表示用のアーティファクトは、リポジトリのファイルを送ったものである。画面デザインは、このページで見る。
- ページでは編集しない。種類「Design」には、ページを読み取り専用にする指定が無い。
- 種類「Design」の説明（表示用のアーティファクトの `SKILL.md` など）は、ページで編集されたアーティファクトのファイルを、Claude が読んで直す前提で書かれている。このプロジェクトでは、下の「変更の手順」で、リポジトリのファイルを送る。

## 変更の手順

画面デザインを変えるときは、次の順に行う。

1. 送る前に、表示用のアーティファクトの中身が、リポジトリと同じことを確かめる。
   - 表示用のアーティファクトを Artifact ツールで読む（`action: "read"` に `url` だけを渡す）。その会話でまだ読んでいないアーティファクトへの送信は断られる。
   - 表示用のアーティファクトのファイルの一覧（`action: "list"`、`scope: "files"`）の、`project/` の下のパスのうち `project/ds/` の下を除いたものを、`action: "read"`、`url`、`paths` で取得する。`out_dir` には空のディレクトリを渡す。
   - `python3 tools/design-system.py compare ui-design {取得したディレクトリ}` で比べる。違いがあったら、ページで編集されている。リポジトリのファイルを直す前に、ユーザーに違いを伝え、取り込むかどうかを確かめる。
2. リポジトリの `design-system/ui-design/` のファイルを直す。
   - `.dc.html` のファイルを足す・消すとき、ページを変えるとき、位置や大きさを変えるときは、`canvas.json` の `boards` と `order` も直す。決まりは、種類「Design」の説明に従う。
3. `python3 tools/design-system.py check` を実行し、指摘をすべて直す（下の「検査」）。
4. Artifact ツールで、表示用のアーティファクトへ送る。
   - `python3 tools/design-system.py files ui-design` が、渡す引数（`root`・`file_path`・`files`）を JSON で出す。`url` には表示用のアーティファクトの URL を渡す。
   - 出力の `files` をそのまま（全ファイル）渡してもよく、変えたファイルだけを送ってもよい。変えたファイルだけを送るときも、送るファイルは、`file_path` に渡すものも含めて、すべて `files` に `"project/{パス}": "{パス}"` の形で入れる（`root` の下には `project/` が無いので、`file_path` に渡すだけでは `project/` の下に置かれない）。
   - ファイルを消したときは、`files` に `"project/{パス}": null` を足す。
   - 送信が断られたとき（ページで誰かが保存したとき）は、`force` を使わず、1 に戻る。
5. 送った後、1 と同じように取得し、`python3 tools/design-system.py compare ui-design {取得したディレクトリ}` で、中身が同じことを確かめる。
   - あわせて、表示用のアーティファクトのファイルの一覧の、`project/` の下のパス（`project/ds/` の下を除く）を、`files ui-design` の出力の `files` のキーと照らし、リポジトリに無いファイルが無いことを確かめる。
6. 画面デザインの変更は、それに伴う仕様の変更と同じコミットに入れる。

- 画面デザインの中のデザインシステムの写しは、この手順では送らない。デザインシステムを変えたときに取り直す（[デザインシステムのファイルと変更の手順](design-system-files.md)「変更の手順」）。

## 検査

`python3 tools/design-system.py check` は、名前を省くと、デザインシステムとあわせて、画面デザインについて次のことを確かめる。

- `canvas.json` が読めて、`boards` と `order` を持つこと
- ディレクトリの中の `.dc.html` のファイルと、`canvas.json` の `boards` のキーが、過不足なく合うこと
- `order` が、`boards` のキーを過不足なく、重複なく並べていること
- `.dc.html` の中の、リポジトリのファイルへの参照（`href`・`src` と、`<dc-import>` で取り込むファイル）が、実在するファイルを指すこと
  - `support.js`、`ds/` で始まる参照、外部の URL、ページの中の位置（`#` で始まるもの）と、`{{` を含む参照は確かめない。
- リポジトリに置かないファイル（上の「ディレクトリの中身」）が置かれていないこと
