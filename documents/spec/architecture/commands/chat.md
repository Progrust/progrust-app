---
type: Specification
title: "質疑応答のコマンドとイベント"
description: "質疑応答のコマンド `send_chat`（質問の場面 `context`、再生中の文の ID、フロントエンドが持つ会話履歴 `history`、再試行かどうか `retry`）と `cancel_chat` の引数と戻り値、イベント `chat_delta`（回答の断片）と `chat_finished`（`completed`・`cancelled`・`error`）の内容を定める。"
---

動作は [質疑応答](../../qa-chat/index.md) に従う。共通の決まりは [コマンドとイベントの共通の決まり](conventions.md) に従う。

- 質問と回答は、質問ごとに `request_id`（[コマンドとイベントの共通の決まり](conventions.md)「要求を見分ける ID」）で見分ける。
- 会話履歴（[質疑応答の会話履歴](../../qa-chat/history.md)）は、フロントエンドが画面の状態としてメモリに持ち、質問のたびに `send_chat` で渡す。バックエンドは会話履歴を持たない。画面を移ると、フロントエンドが会話履歴を捨てる。

## send_chat

- 引数:

```json
{
  "request_id": "9b1e…",
  "lesson_id": "ch04-04",
  "context": "video",
  "sentence_id": "m5",
  "question": "ムーブ後に使えないのはなぜ？",
  "history": [
    { "question": "所有者とは何ですか？", "answer": "値を持っている変数のことです。…" }
  ],
  "retry": false
}
```

| 項目 | 型 | 内容 |
|---|---|---|
| `request_id` | `string` | この質問の ID |
| `lesson_id` | `string` | レッスン ID（生成問題は `gen-{generation_id}`） |
| `context` | `"video" \| "explanation"` | 質問の場面（[質疑応答を利用できる場面](../../qa-chat/availability.md)）。`video` は動画の再生中、`explanation` は演習の解説表示中 |
| `sentence_id` | `string \| null` | `video` のとき、質問を送信したときに再生中だった文の ID。`explanation` では `null` |
| `question` | `string` | 質問文 |
| `history` | `{ question: string, answer: string }[]` | フロントエンドが持つ会話履歴。古い順に並べる。入れる往復と、止めた往復の `answer` は [質疑応答の会話履歴](../../qa-chat/history.md) に従う |
| `retry` | `boolean` | エラーの後の「再試行」（[質疑応答の画面](../../qa-chat/screen.md)）で送り直すときは `true`。ほかは `false` |

- AI に渡す入力は [質疑応答で AI に渡す入力](../../qa-chat/ai-input.md) に従い、バックエンドが組み立てる。会話履歴は、`history` の末尾から [質疑応答の会話履歴](../../qa-chat/history.md) の往復数までを入れる。`explanation` の「学習者が正解したときのコード」は、バックエンドが試行ログから読む。
- 試行ログの `chat` イベントの追記は [質疑応答の試行ログへの記録](../../qa-chat/attempt-log.md) に従う。`retry` が `true` のときは追記しない。`context` が `video` のときの試行ログの `context` は `video:{sentence_id}` とする（[質疑応答を利用できる場面](../../qa-chat/availability.md)）。
- 戻り値: `null`。呼び出しを始めたら返す。回答は `chat_delta` と `chat_finished` で知らせる。
- エラー:
  - `question` が空のとき、[質疑応答の画面](../../qa-chat/screen.md) の文字数の上限を超えるとき、`context` が `video` で `sentence_id` が `null` のときは `invalid_argument`。
  - その場面で質問できないとき（[質疑応答を利用できる場面](../../qa-chat/availability.md)。例: 正解していない演習の `explanation`）は `invalid_state`。
  - 回答中の質問があるときは `busy`（[質疑応答の画面](../../qa-chat/screen.md)）。
  - `claude` が見つからないときは `claude_unavailable`。

## cancel_chat

回答中の「停止」と、回答中に画面を離れるときに呼ぶ（[質疑応答の画面](../../qa-chat/screen.md)）。

- 引数: `request_id`（`string`）。
- 戻り値: `null`。止めた結果は `chat_finished`（`cancelled`）で知らせる。
- 回答中でない質問の ID（既に終わったもの、無いもの）を渡したときは、何もせずに `null` を返す。

## イベント

**`chat_delta`**

回答の断片（[claude -p の呼び出しの共通ルール](../../claude-integration/invocation.md)「出力形式」の `event.delta.text`）。

| 項目 | 型 | 内容 |
|---|---|---|
| `request_id` | `string` | 質問の ID |
| `text` | `string` | 回答の断片。前の断片に続けてつなぐ。バックエンドは、続けて受け取った断片をまとめて 1 つのイベントで送ってよい |

**`chat_finished`**

回答が終わったときに 1 回送る。

| 項目 | 型 | 内容 |
|---|---|---|
| `request_id` | `string` | 質問の ID |
| `status` | `"completed" \| "cancelled" \| "error"` | `completed` は回答が終わった（`result` の行を受け取った）、`cancelled` は `cancel_chat` で止めた、`error` は失敗した |
| `error` | [コマンドとイベントの共通の決まり](conventions.md)「AI 呼び出しのエラー」の形 `\| null` | `error` のときのエラー。`claude` を起動できなかったなど、AI の応答によらない失敗は、分類を `other` にする。ほかでは `null` |

- 回答の全文は、`chat_delta` の断片をつないだものである。`chat_finished` には回答の文を入れない。
