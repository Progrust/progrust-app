---
type: Specification
title: "デザインシステムのファイルと変更の手順"
description: "デザインシステム「Progrust」「Progrust Video」のファイルの置き場所（`design-system/progrust/`・`design-system/progrust-video/`）とリポジトリのファイルだけを正としてリポジトリの外に表示用のページを置かないこと、ディレクトリの中身（`tokens.json` から作る `tokens.css`、索引の `design-system.json` など）とリポジトリに置かないファイル、デザインシステムを変えるときの手順（`tokens.css` の作り直し、アイコンの追加）、`python3 tools/design-system.py check` が確かめることを定める。"
---

## 置き場所

- デザインシステム「Progrust」のファイルはリポジトリの `design-system/progrust/` に、動画のデザインシステム「Progrust Video」のファイルは `design-system/progrust-video/` に置く（[リポジトリ構成](../architecture/repository.md)）。
- リポジトリのファイルだけを正とする。claude.ai のアーティファクトなど、リポジトリの外に、写しや表示用のページを置かない。

## ディレクトリの中身

- 各ディレクトリのファイルは、claude.ai のアーティファクトの種類「Design System」の形で置く。ただし、索引には、その種類のページのためだけの項目（ページを読み取り専用にする `editing`、アイコンのアセットの id の `blob`、最後の変更の記録の `lastChange`）を置かない。

| ファイル | 内容 |
|---|---|
| `README.md` | デザインシステムの使い方のルール |
| `tokens.json` | 色・書体・余白などの値（トークン） |
| `tokens.css` | トークンの CSS。`tokens.json` から作るファイルで、手で書かない（下の「変更の手順」） |
| `design-system.json` | 索引。アイコンの記録（`assetGroups`）を持つ |
| `components/bundle.css` | 部品のクラスの CSS |
| `components/{部品}/` | 部品ごとの見本（`preview.html`）と使い方（`README.md`） |
| `assets/Icons/` | アイコンの SVG と、そのライセンスと説明。「Progrust」だけにある |
| `chapter-colors.md` | 章カラーごとの実際の色の早見表。「Progrust Video」だけにある |

- 部品の見本（`preview.html`）は、デザインシステムの CSS（`tokens.css`・`components/bundle.css`）を読み込まない。ブラウザでそのまま開いても、デザインシステムの見た目では表示されない。部品の組み立て方を読むのに使う。
- 次のファイルは、リポジトリに置かない。
  - アーティファクトの種類が持つファイル（`index.html`、`SKILL.md`、`artifact-type/`）
  - アーティファクトのページが作るファイル（`api/`、`manifest.json`）

## 変更の手順

デザインシステムを変えるときは、次の順に行う。`{名前}` は `progrust` か `progrust-video` である。

1. リポジトリの `design-system/{名前}/` のファイルを直す。
2. `tokens.json` を変えたら、`python3 tools/design-system.py tokens-css {名前} --write` で `tokens.css` を作り直す。
3. アイコンを足す・替えるときは、ファイルを `assets/Icons/` に置き、索引の記録（`assetGroups.Icons.files.{ファイル名}`）に `name`・`size`・`type` を書き、`order` にファイル名を足す。
4. `python3 tools/design-system.py check` を実行し、指摘をすべて直す（下の「検査」）。
5. デザインシステムの変更は、それに伴う仕様の変更と同じコミットに入れる。

- 「Progrust」の、動画と共通の値を変えたときは、「Progrust Video」の写しも合わせる（[画面の見た目とテーマ](appearance.md)「デザインシステム」）。

## 検査

`python3 tools/design-system.py check` は、次のことを確かめる。

- `tokens.css` が、`tokens.json` から作った内容と一致すること
- 索引（`design-system.json`）が読めること
- 索引のアイコンの記録と、`assets/` のアイコンのファイルが、過不足なく合うこと
- アーティファクトのページが作るファイル（`api/`、`manifest.json`）が置かれていないこと
- CSS のファイルが、外部（`http:`・`https:`・`//` で始まる URL）から `@import` で読み込んでいないこと（[画面の見た目とテーマ](appearance.md)「デザインシステムの CSS とアイコン」）
- デザインシステムの中の `.md` のファイル（README など）で、インラインコードのパスで書いた参照（例: `` `screens/appearance.md`「書体のファイル」 ``）が、実在する文書と、その文書の小見出しを指すこと
  - `/` を含むパスは、`documents/spec/` からのパスとして確かめる。ただし、`components/`・`assets/` で始まるパスと、`/` を含まないパス（`` `chapter-colors.md` `` など）は、そのデザインシステムのディレクトリからのパスとして確かめる。
  - 小見出しは、文書の中の見出し（`## 見出し` など）か、1 行だけの太字（`**見出し**`）と照らす。
  - 参照の書き方の決まり（パスをインラインコードで書くこと）を守っていない参照と、文の中身（「未決事項」と書いた事項が仕様で決まったかどうか、など）は、確かめない。
- 「Progrust Video」の `tokens.json` の、名前が `video-` で始まらないトークンと書体の族（`type.families`）が、「Progrust」の `tokens.json` に同じ名前で、同じ値であること（[画面の見た目とテーマ](appearance.md)「デザインシステム」の、画面と共通の値の写し）
  - 「Progrust」にあって「Progrust Video」に無いトークンは、指摘しない。
  - 書体のスタイル（`type.groups`）は比べない。
