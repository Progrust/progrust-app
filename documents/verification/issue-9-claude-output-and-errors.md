---
type: Verification Report
title: "claude -p の出力の形とエラーの見分け方の技術検証"
description: "claude -p の json 出力から回答の本文を取り出せるか、stream-json で回答を少しずつ表示できるか、未ログインとモデル指定の誤りで終了コードと出力がどうなるかを Issue #9 で検証した結果（subtype が success のままになること、分類コードは stream-json にだけ出ること、エラーの文が標準出力に出ること、利用上限は未再現）を記す。"
---

- 日付: 2026-11-20
- Issue: [#9](https://github.com/Progrust/progrust-app/issues/9)
- 確かめた OS: macOS 27.2.1（Apple Silicon、arm64）。Windows は確かめていない（当面 macOS に集中する方針のため）
- 関連仕様: [claude -p の呼び出し](../spec/claude-integration/invocation.md)、[Claude Code 連携のエラー](../spec/claude-integration/errors.md)、[質疑応答の画面](../spec/qa-chat/screen.md)

## 環境

| 項目 | 値 |
|---|---|
| OS | macOS 27.2.1、Apple Silicon（arm64） |
| Claude Code | 2.1.329（ネイティブインストーラーで導入） |
| ログイン | claude.ai のサブスクリプション（OAuth）。API キーは使っていない |
| node / python3 | node v25.9.0 / python3（出力の整形に使用） |
| 測定時の負荷 | load average 18〜19（他のエージェントが同時に動いていた） |

入れ子で `claude` を実行するため、検証を動かしている環境から引き継いだ `CLAUDE*` などの環境変数を取り除いて実行した。モデルはすべて `haiku`。

## 方法

- 作業ディレクトリは OS の一時ディレクトリの下に作った。各実行の標準出力・標準エラー出力・終了コードを別々に保存した。
- 次のフラグ（[隔離起動の技術検証](issue-8-claude-isolated-invocation.md)の推奨の組み合わせ）で実行した。ことわりのあるものを除く。

```
-p --model haiku --safe-mode --setting-sources "" --strict-mcp-config --tools "" --disable-slash-commands --no-session-persistence
```

- 未ログインは、`CLAUDE_CONFIG_DIR` を空の一時ディレクトリにして作った（`claude auth status` が `loggedIn: false` になることを確認）。未ログインの text・json・stream-json は、隔離のフラグなしで実行し、上のフラグでも json と stream-json で同じ出力になることを確かめた。
- モデル指定の誤りは `--model claude-nonexistent-model-9` で作った。
- 利用上限は、ユーザー判断で再現していない。

## 結果

### `--output-format json` から回答の本文を取り出せるか

取り出せる。標準出力に JSON オブジェクトが 1 つ出る。`result` が回答の本文。主な項目（usage などは省略）:

```json
{"type":"result","subtype":"success","is_error":false,"result":"Rustの所有権とは、各値にはただ一つの所有者変数が存在し、…","stop_reason":"end_turn","terminal_reason":"completed","api_error_status":null,"num_turns":1,"duration_ms":2865,"total_cost_usd":0.00020118,"usage":{...},"modelUsage":{"claude-haiku-5-5":{...}},"permission_denials":[],"session_id":"...","credential_expires_at":"<日時>"}
```

標準エラー出力は空、終了コードは 0。

### ストリーミングで回答を少しずつ表示できるか（`chat_delta`）

できる。

- `--output-format stream-json` は `-p` のとき `--verbose` が必須。無いと終了コード 1、標準エラー出力に `Error: When using --print, --output-format=stream-json requires --verbose`。
- `--include-partial-messages` を付けると、回答の断片が `stream_event` として 1 行ずつ出る。パイプで読み、行ごとに到着時刻を記録した。イベントの種類と数:

```
1 system/init
1 system/status          {"status":"requesting"}
1 stream_event/message_start
1 stream_event/content_block_start
77 stream_event/content_block_delta/text_delta   ← 1.912 秒〜3.436 秒にかけて少しずつ届いた
1 assistant              （完成したメッセージ全体）
1 stream_event/content_block_stop, message_delta, message_stop
1 rate_limit_event
1 result/success          （result に全文）
```

断片の 1 行:

```json
{"type":"stream_event","event":{"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"**所有権**"}},"session_id":"...","parent_tool_use_id":null,"uuid":"..."}
```

`event.delta.text` を `chat_delta` として送り、`result` で完了とすればよい。隔離していない実行では、ほかに `system/hook_started`・`hook_response`、モデルによって `system/thinking_tokens` も出る（無視してよい）。

### エラーごとの終了コードと出力

| 状態 | 再現方法 | 終了コード | 標準エラー出力 | text | json | stream-json |
|---|---|---|---|---|---|---|
| 未ログイン | 空の `CLAUDE_CONFIG_DIR` | 1 | 空 | 標準出力に `Not logged in · Please run /login` | `is_error: true`、`subtype: "success"`、`result: "Not logged in · Please run /login"`、`api_error_status: null`、`terminal_reason: "api_error"` | `assistant` の行に `"error":"authentication_failed"`、`"is_api_error_message":true`、`model: "<synthetic>"`。続く `result` は json と同じ |
| モデル指定の誤り | `--model claude-nonexistent-model-9` | 1 | `[claude-code:unrecognized_model] {"model":"claude-nonexistent-model-9","query_source":"sdk"}` | 標準出力に `There's an issue with the selected model (claude-nonexistent-model-9). It may not exist or you may not have access to it. Run --model to pick a different model.` | `is_error: true`、`subtype: "success"`、`result` は左と同じ文、`api_error_status: 404`、`terminal_reason: "api_error"` | `assistant` の行に `"error":"model_not_found"`。`result` は json と同じ |
| 利用上限 | 再現していない | 未確認 | 未確認 | 未確認 | 未確認 | 未確認 |
| `--bare`（OAuth を読まない） | `--bare` | 1 | 空 | — | 未ログインと同じ | — |

補足:
- `subtype` はエラーのときも `"success"` だった。エラーかどうかは `is_error` で見る必要がある。
- json の出力には、機械的な分類コード（`authentication_failed` など）が無く、`result` の文と `api_error_status` しかない。分類コードは stream-json の `assistant` の行の `error` にだけ出る。
- 利用上限について、観測ではなく参考として: `claude` のバイナリの中のスキーマに、`assistant` の `error` の値の一覧 `authentication_failed, oauth_org_not_allowed, account_on_hold, verification_required, billing_error, rate_limit, overloaded, invalid_request, model_not_found, server_error, unknown, max_output_tokens, cloud_credential_error` があった。また、正常な実行でも毎回 `rate_limit_event` が出る（下に抜粋）。上限に達したときに `rate_limit` か `billing_error` になるのか、`status` がどう変わるのかは観測していない。

```json
{"rate_limit_info":{"status":"allowed","resetsAt":...,"rateLimitType":"five_hour",...,"unifiedWindows":{"five_hour":{"utilization":<0〜1>,...},"seven_day":{...}}}}
```

- 標準エラー出力には、エラーと関係ない警告も出る。text 形式で `--safe-mode --tools ""` だけを付けたモデル指定の誤りの実行では、次が出た（json・stream-json と、推奨のフラグでは出なかった。形式とフラグのどちらによるかは切り分けていない）。
  - `Claude Code doesn't recognize the model "...", so it assumes a context window of 1m tokens. ...`
  - `Warning: this sign-in ends between <日付> and <日付> (in N days at the earliest). ... Run claude auth login before then, or use claude setup-token for unattended jobs.`
  - `Warning: Advisor disabled — ...`（ユーザーの settings.json の `advisorModel` による）

## 判定

**一部合格（利用上限が未確認）**。

- json から本文を取り出せる: 合格。ストリーミングで少しずつ表示できる: 合格。
- 未ログインとモデル指定の誤りは、記録した出力から見分けられる（stream-json の `error`、または json の `result` の文と `api_error_status`）。
- 利用上限は再現していないので、「利用上限」と「その他」の見分けは判定できない。

## 残った事項

確かめられなかった項目:
- 利用上限に達したときの終了コードと出力（`rate_limit`・`billing_error` のどちらになるか、`rate_limit_event` の `status` がどう変わるか）。
- Windows。
- text 形式での標準エラー出力の警告が、形式とフラグのどちらによるか。

ユーザーの判断が要る点:
1. ジョブの出力形式。仕様ではジョブは `--output-format json`。json には分類コードが無く、エラーの判定は `result` の英語の文（`Not logged in · Please run /login` など）と `api_error_status` に頼ることになる。選択肢は、(a) ジョブも stream-json にして `assistant` の `error` で分類する、(b) json のまま文で判定する。(a) は `--verbose` が必須で行ごとの処理が要るが、分類コードで判定できる。
2. 利用上限の判定。再現していないので、`rate_limit`・`billing_error`（と `rate_limit_event` の `status`）で判定する前提にしてよいか、上限に達したときに改めて確かめるか。

仕様と食い違う点（[Claude Code 連携のエラー](../spec/claude-integration/errors.md)）:
- 「その他」の詳細を「標準エラー出力の先頭」としているが、エラーの文は標準出力（text では本文、json・stream-json では `result`）に出て、標準エラー出力は空か、関係ない警告（ログインの期限の予告など）だった。詳細は `result` の文を基本にし、標準エラー出力は補助にするのがよい。
- 「未ログイン」を「終了コードと出力から認証エラーと判定」としている。終了コードはどのエラーでも 1 で見分けに使えない。判定は出力（`error: "authentication_failed"` または `result` の文）による。
- エラーの判定には `subtype` ではなく `is_error` を使う必要がある（エラーでも `subtype: "success"`）。
