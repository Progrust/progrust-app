---
type: Specification
title: "AI 呼び出しのエラーの分類と表示"
description: "エラーかどうかの判定（`is_error`）、未検出、未ログイン、利用上限、タイムアウト、出力不正、その他のエラーの判定条件（`assistant` の行の `error` の値）と、表示・動作、`last_error` への保存を定める。"
---

## エラーかどうかの判定

- 出力形式は stream-json（[claude -p の呼び出しの共通ルール](invocation.md)「出力形式」）。
- 最後の `result` の行の `is_error` が `true` のとき、エラーとする。`subtype` はエラーのときも `success` になるので、判定に使わない。
- `result` の行が出ずに終了したときも、エラーとする。
- 終了コードは、どのエラーでも 1 になるので、分類には使わない。
- エラーの分類は、`result` の行より前の `assistant` の行の `error` の値で決める（下の表）。

## 分類

| 分類 | 判定 | 表示・動作 |
|---|---|---|
| 未検出 | `claude` が見つからない | [claude が使えない場合の動作](unavailable.md) |
| 未ログイン | `assistant` の行の `error` が `authentication_failed` | 「ターミナルで `claude` を起動してログインしてください」 |
| 利用上限 | `assistant` の行の `error` が `rate_limit` または `billing_error` | 「Claude の利用上限に達しました。時間をおいて再試行してください」。ジョブは通常のリトライに従う |
| タイムアウト | [claude -p の呼び出しの共通ルール](invocation.md)「制限時間」 の制限時間を超えた | 「応答がありませんでした」 |
| 出力不正 | 各機能の出力検証に失敗した | 質疑応答では発生しない。ジョブは失敗としてリトライする。ただし、問題生成のジョブは [演習問題生成の出力と検証](../exercise-generation/output-and-validation.md)「検証に失敗した場合」に従う |
| その他 | 上記以外のエラー | 「AI の呼び出しに失敗しました」と、詳細を折りたたんで表示 |

- 「その他」の詳細は、`result` の行の `result` の文（エラーの文はここに出る）を基本にし、標準エラー出力の先頭を補助として添える。標準エラー出力には、エラーと関係ない警告（ログインの期限の予告など）も出る。
- ジョブの `last_error` には分類と詳細を保存する。
- エラーの詳細はログ（[アプリのログ](../non-functional/logging.md)）にも記録する。
