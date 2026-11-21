---
type: Decision
title: "ADR 0080"
description: "OKF の frontmatter のうち type・title・description・status・superseded_by だけを使い、type を6つに固定する（ADR 0088 で `Convention` の範囲を、ADR 0089 で type の数を変えた）"
---

- 日付: 2026-11-20
- 関連仕様: [frontmatter の書き方](../conventions/frontmatter.md)
- 置き換え: なし

## 背景

ドキュメントを OKF v0.3 のバンドルにした（0079）。OKF の frontmatter のキーのうちどれを使うかと、`type` にどんな値を使うかを決める必要があった。ユーザーは、OKF を採用するのは人間が関わらない範囲だけとし、「verifiedなどは使わない。」と示していた。

## 決定

- 使うキーは `type`（必須）、`title`・`description`（OKF では任意だが、このプロジェクトでは必須）、`status`・`superseded_by`（ADR だけ）にする。
  - `title`・`description` を必須にする理由: `index.md` の 1 行になり、エージェントがファイルを開く前の判断材料になる。
  - `title` と `description` の値は必ず二重引用符で囲む。理由: 日本語の文にはバッククォート、コロン、`#` が入ることがあり、引用符がないと YAML として読めなくなる。
- 次のものは使わない。
  - `verified` と `human:` の actor。理由: 人間が関わる範囲だから（ユーザーの方針）。
  - `stale_after`。理由: 見直し日を決める人がいない。OKF も、分からない日付を作るなと定めている。
  - `generated`。理由: git の履歴と重複し、比較相手の `verified` もない。
  - `sources`、`tags`、Attested Computation の各キー。理由: 根拠は ADR に書く。分類はディレクトリで足りる。
  - `log.md`。理由: OKF では任意で、git の履歴や ADR と重複する。
- `type` の値は `Specification`、`Acceptance Criteria`、`Open Questions`、`Glossary`、`Decision`、`Convention` の 6 つに固定する。エージェントがいつ引くか（実装するとき、実装を確かめるとき、仕様を変える前、文書を書くとき、言葉の意味を確かめるとき）という役割で分けている。
  - 教材の作成ルール（`content-guidelines`）は、教材をこう作るというプロダクト側の決まりなので `Specification` にする。`Convention` は `documents/` の中の文書の書き方だけに使う。
  - 領域（画面、教材、デザインなど）は、`type` ではなく置き場所で区別する。
- `verified` と `human:` の actor を使わないことはユーザーの方針による。それ以外は Claude の提案で、ユーザーが「すべて提案通りで大丈夫です。」「「決めてほしい点」もすべてあなたの推奨通りで問題ないので、この方針でドキュメントの整理をお願いします。」と承認した。

## 検討した他の案

- 案: OKF の公式スキル（okf-generate）の書き方に合わせて、`generated`・`sources`・`log.md` も使う
  - 採用しなかった理由: `generated` は git の履歴と重複し、比較相手の `verified` もない。`sources` については、根拠は ADR に書く。`log.md` は OKF では任意で、git の履歴や ADR と重複する。

## 影響

- どの concept にも `title` と `description` があるので、`index.md` の行から、ファイルを開くかどうかを決められる。
- 領域を表す `type` や `tags` がないので、領域は置き場所（ディレクトリ）で見分ける。
