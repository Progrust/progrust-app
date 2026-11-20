---
type: Specification
title: "動画の台本（script.json）"
description: "`script.json` の `ai_note`・`code_blocks`・`scenes`・`sentences` の各フィールドと、ID・文の並び・文字数の数え方の決まりを定める。"
---

ナレーションを文ごとに並べ、シーンと、画面に出すコードを宣言する。

```json
{
  "ai_note": "学習者には表示しない、AI への補足説明",
  "code_blocks": [
    { "id": "main", "label": "src/main.rs", "language": "rust", "code": "fn main() {\n    let s1 = String::from(\"hi\");\n    let s2 = s1;\n    println!(\"{s1}\");\n}" },
    { "id": "error", "language": "text", "code": "error[E0382]: borrow of moved value: `s1`\n --> src/main.rs:4:16\n…" }
  ],
  "scenes": [
    { "id": "title", "name": "表紙", "lead": 1.3 },
    { "id": "move", "name": "ムーブの図解", "lead": 1.7, "tail": 1.6 }
  ],
  "sentences": [
    { "id": "t1", "scene": "title", "text": "このレッスンでは、所有権のムーブを学びます。", "say": "このレッスンでは、所有権のムーブを学びます。" },
    { "id": "m1", "scene": "move", "text": "2 行目で、変数 s1 に String の値を入れました。", "say": "二行目で、変数エスワンにストリングのあたいを入れました。" },
    { "id": "m2", "scene": "move", "text": "文字列の中身はヒープにあり、s1 がその所有者です。", "say": "文字列のなかみはヒープにあり、エスワンがその所有者です。", "pause_after": 0.8 },
    { "id": "m3", "scene": "move", "text": "3 行目で、s1 を s2 に代入します。", "say": "三行目でエスワンをエスツーに代入します。" }
  ]
}
```

| フィールド | 型 | 説明 |
|---|---|---|
| `ai_note` | string | 画面にも字幕にも出さない補足。質疑応答で AI に渡す（[質疑応答で AI に渡す入力](../../qa-chat/ai-input.md)「動画の再生中」）。省略できる |
| `code_blocks` | object[] | 画面に出すコードとコンパイラの出力（[動画でのコードの表示](code-display.md)）。質疑応答で AI に渡す（[質疑応答で AI に渡す入力](../../qa-chat/ai-input.md)「画面に出すコードの渡し方」）。省略時は空 |
| `scenes` | object[] | シーン。配列の順序が再生の順序である。1 件以上 |
| `sentences` | object[] | 文。配列の順序が再生の順序である |

**`code_blocks` の 1 件**

| フィールド | 型 | 説明 |
|---|---|---|
| `id` | string | コードブロックの ID |
| `label` | string | ラベル（ファイル名・言語名）。省略できる |
| `language` | string | 言語（[動画でのコードの表示](code-display.md)） |
| `code` | string | 表示する文字列。行の区切りは `\n` とする |
| `start_line` | number | 載せた 1 行目の、ファイルでの行番号（1 以上の整数）。省略時は 1 |
| `diff` | boolean | Diff として表示する（[動画でのコードの表示](code-display.md)）。省略時は `false` |

**`scenes` の 1 件**

| フィールド | 型 | 説明 |
|---|---|---|
| `id` | string | シーンの ID |
| `name` | string | シーンの名前（例: 「表紙」「まとめ」） |
| `lead` | number | シーンの先頭から最初の文までの間（秒。0 以上）。省略時は 0.9 |
| `tail` | number | 最後の文の音声が終わってからシーンの末尾までの間（秒。0 以上）。省略時は 0.7 |

**`sentences` の 1 件**

| フィールド | 型 | 説明 |
|---|---|---|
| `id` | string | 文の ID |
| `scene` | string | この文を読み上げるシーンの ID |
| `text` | string | もとの文。字幕（[動画画面](../../screens/video-screen.md)）と質疑応答（[質疑応答で AI に渡す入力](../../qa-chat/ai-input.md)「動画の再生中」）に使う |
| `say` | string | 読ませる文。音声を作るときに VOICEPEAK に渡す（[動画の音声](voice.md)）。140 字以下とする |
| `pause_after` | number | この文の音声が終わってから次の文までの間（秒。0 以上）。省略時は 0.45。シーンの最後の文には書けない |

- ID は、英小文字で始まり、英小文字・数字・`-` からなる文字列とする。`code_blocks`・`scenes`・`sentences` のそれぞれの中で重複しない。
- 同じシーンの文は続けて並べる。シーンが現れる順序は、`scenes` の順序と同じにする。
- どのシーンにも、文が 1 件以上ある。
- `text` と `say` は空にできない。
- 文字数は Unicode のコードポイントで数える。
