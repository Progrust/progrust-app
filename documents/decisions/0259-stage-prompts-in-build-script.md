---
type: Decision
title: "ADR 0259"
description: "同梱する指示文を、ビルドスクリプトがリポジトリの `prompts/` の直下の `.md` だけ `src-tauri/gen/bundled/prompts/` に写し、Tauri がそれを `{app_resources}/prompts/` に入れる"
---

- 日付: 2026-11-27
- 関連仕様: [データの保存場所](../spec/architecture/data-locations.md)「アプリ同梱データ（`{app_resources}`）」、[指示文（プロンプト）の置き場所と書き方](../spec/claude-integration/prompts.md)
- 置き換え: なし

## 背景

AI への指示文は、リポジトリの `prompts/` に Markdown で置き、アプリに同梱する（[指示文（プロンプト）の置き場所と書き方](../spec/claude-integration/prompts.md)）。同梱の先は `{app_resources}/prompts/` と決まっていたが（[データの保存場所](../spec/architecture/data-locations.md)）、どう写すかは決まっていなかった。M4 の質疑応答で、初めて指示文を同梱する。

教材は、ビルドスクリプトが同梱するファイルだけを選んで写し、Tauri はそのディレクトリだけを入れる（[ADR 0187](0187-stage-bundled-content-in-build-script.md)）。許可クレートの定義（`runner/`）も、同じ形で決まったファイルだけを写している。

M4 の実装計画で、Claude が技術的な判断で決めた。

## 決定

- ビルドスクリプトが、リポジトリの `prompts/` の直下の `.md` のファイルだけを `src-tauri/gen/bundled/prompts/` に写し、Tauri の `bundle.resources` がそれを `{app_resources}/prompts/` に入れる（[データの保存場所](../spec/architecture/data-locations.md)「アプリ同梱データ（`{app_resources}`）」）。
  - 理由: 教材・許可クレートと同じ形にそろえる。`prompts/` に置いた下書きや、`.DS_Store` のようなファイルが同梱に混ざらない。

## 検討した他の案

- 案: `bundle.resources` で、リポジトリの `prompts/` をそのまま入れる
  - 採用しなかった理由: ADR 0187 で教材について採用しなかった理由と同じく、同梱したくないファイルも入る
- 案: 指示文を Rust のコードに埋め込む（`include_str!`）
  - 採用しなかった理由: 仕様は、指示文をアプリ同梱データ（`{app_resources}/prompts/`）のファイルとして置くとしている（[データの保存場所](../spec/architecture/data-locations.md)）

## 影響

- 指示文のファイルを足すときは、`prompts/` の直下に `.md` で置けば同梱される。
