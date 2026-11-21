---
type: Decision
title: "ADR 0107"
description: "ジョブも stream-json で呼び出し、AI 呼び出しのエラーを `is_error` と `assistant` の行の `error` の分類コードで見分ける"
---

- 日付: 2026-11-21
- 関連仕様: [AI 呼び出しのエラーの分類と表示](../spec/claude-integration/errors.md)、[claude -p の呼び出しの共通ルール](../spec/claude-integration/invocation.md)「出力形式」
- 置き換え: なし

## 背景

これまでの仕様では、ジョブは `--output-format json`、質疑応答はストリーミングの出力形式で呼び出し、未ログインは「終了コードと出力」、その他のエラーの詳細は「標準エラー出力の先頭」で扱うとしていた。

Issue #9 の技術検証で、出力の形とエラーの出方を確かめた（[claude -p の出力の形とエラーの見分け方の技術検証](../verification/issue-9-claude-output-and-errors.md)）。

- 終了コードは、未ログインでもモデル指定の誤りでも 1 だった。
- エラーのときも `subtype` は `success` で、エラーかどうかは `is_error` にだけ出た。
- 分類コード（`authentication_failed`・`model_not_found` など）は、stream-json の `assistant` の行の `error` にだけ出た。json には分類コードが無く、`result` の英語の文と `api_error_status` しかない。
- エラーの文は標準出力（json・stream-json では `result`）に出た。標準エラー出力は空か、関係ない警告（ログインの期限の予告など）だった。
- 利用上限は再現していない。`claude` の中のスキーマには、`error` の値として `rate_limit` と `billing_error` があった。

また、Issue #11 の技術検証では、ジョブも stream-json で呼び、所要時間と結果の取り出しに問題が無かった（[AI の応答時間と出力の安定性の技術検証](../verification/issue-11-ai-timing.md)）。

検証の結果、ジョブの出力形式と、利用上限の判定の前提が、ユーザーの判断を要する点として残った。Claude は、ジョブも stream-json にして分類コードで判定し、利用上限は `rate_limit`・`billing_error` で判定する前提にして、実際に上限に達したときに確かめることを推奨した。ユーザーは「私は細かい技術的な話はわからないので、あなたの推奨通りで進めて大丈夫です」と示し、推奨を承認した。以下の理由は Claude が書いた。

## 決定

- ジョブも、質疑応答と同じく stream-json で呼び出す（[claude -p の呼び出しの共通ルール](../spec/claude-integration/invocation.md)「出力形式」）。
  - 理由: 分類コードは stream-json にだけ出る。json のまま英語の文で判定すると、Claude Code の版で文が変わったときに判定が壊れる。読み方を 1 つにそろえられる。
- エラーかどうかは `is_error` で決め、`subtype` と終了コードは使わない。分類は `assistant` の行の `error` の値で決める（[AI 呼び出しのエラーの分類と表示](../spec/claude-integration/errors.md)）。
  - 理由: 検証で、`subtype` はエラーでも `success`、終了コードはどのエラーでも 1 だった。
- 利用上限は、`error` が `rate_limit` または `billing_error` のときとする。
  - 理由: 狙って再現できない。スキーマにある値のうち、利用上限に当たるものはこの 2 つである。
- 「その他」の詳細は、`result` の文を基本にし、標準エラー出力は補助にする。
  - 理由: エラーの文は `result` に出て、標準エラー出力には関係ない警告が出る。

## 検討した他の案

- 案: ジョブは json のまま、`result` の文と `api_error_status` で判定する
  - 採用しなかった理由: 英語の文に頼る判定は、Claude Code の版で文が変わると壊れる
- 案: 利用上限に達するまで、利用上限の判定を決めない
  - 採用しなかった理由: いつ達するか分からず、実装を止めることになる。「その他」として表示しても学習は続けられるので、外れても害が小さい

## 影響

- ジョブの呼び出しには `--verbose` が要り、出力を 1 行ずつ JSON として読む処理が要る。
- 利用上限に達したときの `error` の値と `rate_limit_event` の `status` は確かめていない。実際に上限に達したときに出力を記録して確かめ、違っていれば仕様を直す。それまでは、利用上限が「その他」として表示されることがある。
