---
type: Specification
title: "実行・提出の試行ログへの記録"
description: "実行（テスト実行を含む）・提出が終わるたびに試行ログへ `run` / `submit` イベントを追記するときの、`error_codes`、`stdout`、`stderr`、`stdin` の記録のしかたを定める。"
---

実行・提出が終わるたびに、試行ログに `run` / `submit` イベントを追記する。テスト実行（[コードの実行（run）](run.md)「テスト実行」）も `run` イベントとして記録し、`mode` を `test` にする。スキーマは [試行ログ（attempts/*.json）](../learning-record-and-analysis/attempt-log.md)「スキーマ」 に従う。

- `error_codes` は、ビルドの JSON 出力のうちレベルが `error` の診断の `code` から取り出す（重複除去・出現順）。
- `stdout` / `stderr` はそれぞれ先頭 20,000 文字までを保存する。`stderr` にはコンパイラの診断（ANSI エスケープを除去したもの）とプログラムの標準エラー出力を、この順に連結して保存する。
- `run` イベントの `stdin` には、学習者が入力した内容をそのまま連結して保存する。テスト実行では空の文字列にする。
