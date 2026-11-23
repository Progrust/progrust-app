---
type: Specification
title: "画面デザインのファイルと変更の手順"
description: "画面デザイン「Progrust UIデザイン」のファイルの置き場所（`design-system/ui-design/`）とリポジトリのファイルだけを正としてリポジトリの外に表示用のページを置かないこと、ディレクトリの中身（索引の `canvas.json`、画面と状態ごとの `.dc.html`、配置の CSS の `styles/app.css`）とリポジトリに置かないファイル、ファイルの読み方、画面デザインを変えるときの手順、`python3 tools/design-system.py check` が確かめることを定める。"
---

## 置き場所

- 画面デザイン「Progrust UIデザイン」（[画面の見た目とテーマ](appearance.md)「画面デザイン」）のファイルは、リポジトリの `design-system/ui-design/` に置く（[リポジトリ構成](../architecture/repository.md)）。
- リポジトリのファイルだけを正とする。claude.ai のアーティファクトなど、リポジトリの外に、写しや表示用のページを置かない。

## ディレクトリの中身

- ディレクトリのファイルは、claude.ai のアーティファクトの種類「Design」の形で置く。ただし、索引には、中のデザインシステムの写しの記録（`designSystems`）を置かない。

| ファイル | 内容 |
|---|---|
| `canvas.json` | 索引。`.dc.html` のファイルごとの、画面での位置と大きさ、題名（`title`）、載せるページ（`page`）と、ファイルの並び（`order`）、注記（`notes`）を持つ |
| `{名前}.dc.html` | 画面と状態ごとのデザイン |
| `styles/app.css` | 画面ごとの配置の CSS。クラスの接頭辞は `pgx-` |

- 次のファイルは、リポジトリに置かない。
  - 画面デザインの中のデザインシステムの写し（`ds/` の下）。`.dc.html` の中の `ds/` への参照は、リポジトリのデザインシステムのファイルに当たる（下の「読み方」）。
  - アーティファクトの種類が持つファイル（`index.html`、`SKILL.md`、`artifact-type/`、`.dc.html` が読み込む `support.js`）

## 読み方

- 画面デザインは、ファイルの中身を読んで使う。`.dc.html` は、ブラウザでそのまま開いても、画面デザインどおりには表示されない（`support.js` と `ds/` の下の写しが無い）。
- どの `.dc.html` がどの画面のどの状態かは、`canvas.json` の `boards` の `title` に書いてある。
- 中身が数十行だけの `.dc.html` は、別の `.dc.html` を `<dc-import>` で取り込み、状態を表す属性（例: `state="wrong"`）を渡したものである。配置は、取り込まれる側のファイルを読む。
- `.dc.html` の中の `ds/progrust/…`・`ds/progrust-video/…` への参照は、リポジトリの `design-system/progrust/`・`design-system/progrust-video/` の同じパスのファイルに当たる（[デザインシステムのファイルと変更の手順](design-system-files.md)）。
- `styles/app.css` は、画面の配置を読むためのものである。アプリの画面の CSS は、[画面の見た目とテーマ](appearance.md)「デザインシステムの CSS とアイコン」の書き方に従って書く。`styles/app.css` には、その書き方に合わない規則（`.app p` のように、中に動画の枠を含む画面全体のクラスの下で要素を指す規則）と、画面デザインの中でだけ使う規則（`.pgx-sheet` などの部品の見本のページの配置、`.pgx-video-fit` などの動画の中の絵を縮めて映す配置）があるので、そのまま写さない。
- 画面を実装した後の、その画面の扱いは、[画面の見た目とテーマ](appearance.md)「画面デザイン」に従う。

## 変更の手順

画面デザインを変えるときは、次の順に行う。

1. リポジトリの `design-system/ui-design/` のファイルを直す。
   - `.dc.html` のファイルを足す・消すとき、ページを変えるとき、位置や大きさを変えるときは、`canvas.json` の `boards` と `order` も直す。`boards` の記録は、いまある記録と同じ形で書く。
2. `python3 tools/design-system.py check` を実行し、指摘をすべて直す（下の「検査」）。
3. 画面デザインの変更は、それに伴う仕様の変更と同じコミットに入れる。

## 検査

`python3 tools/design-system.py check` は、名前を省くと、デザインシステムとあわせて、画面デザインについて次のことを確かめる。

- `canvas.json` が読めて、`boards` と `order` を持つこと
- ディレクトリの中の `.dc.html` のファイルと、`canvas.json` の `boards` のキーが、過不足なく合うこと
- `order` が、`boards` のキーを過不足なく、重複なく並べていること
- `.dc.html` の中の、リポジトリのファイルへの参照（`href`・`src` と、`<dc-import>` で取り込むファイル）が、実在するファイルを指すこと
  - `support.js`、`ds/` で始まる参照、外部の URL、ページの中の位置（`#` で始まるもの）と、`{{` を含む参照は確かめない。
- リポジトリに置かないファイル（上の「ディレクトリの中身」）が置かれていないこと
