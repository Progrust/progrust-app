---
type: Specification
title: "claude -p の呼び出しの共通ルール"
description: "`--model` の指定、ツール・ユーザー設定を使わない起動のフラグ、使う認証、既定のシステムプロンプトの置き換え、一時ディレクトリ、標準入力での指示文の渡し方、出力形式、用途ごとの制限時間を定める。"
---

- 設定画面で選択したモデルを `--model` で指定する（[AI モデルの選択](model-selection.md)）。
- ツールを使わせない設定で起動する。
- 学習者の Claude Code のユーザー設定を読み込まない設定で起動する。読み込まない対象は、ユーザー・プロジェクトの `settings.json`、フック、MCP サーバー、アカウントに紐づく claude.ai のコネクター（MCP）、プラグイン、スキル、エージェント、スラッシュコマンド、出力スタイル、`CLAUDE.md`（ユーザー・プロジェクト）、メモリーである。
- 学習者の Claude Code のセッション履歴に、アプリの呼び出しを残さない。
- Claude Code の既定のシステムプロンプトは使わず、アプリが決めた文に置き換える。文面は指示文（[指示文（プロンプト）の置き場所と書き方](prompts.md)）と同じく実装の一部とする。
- 作業ディレクトリは呼び出しごとに作る空の一時ディレクトリとし、終了後に削除する。
- 指示文は標準入力で渡す。
- `--continue` / `--resume` は使わない。会話の続きが必要な場合（質疑応答）も、アプリが履歴を毎回入力に含める。

## 起動のフラグ

すべての呼び出しで、次のフラグを付ける。`--system-prompt` を除き、Claude Code 2.1.329 で効き方を確かめた組み合わせである。

```
-p --model <モデル> --safe-mode --setting-sources "" --strict-mcp-config --tools "" --disable-slash-commands --no-session-persistence --system-prompt <アプリが決めた文>
```

- `--bare` は使わない。`--bare` を付けると claude.ai のログイン（OAuth）を読まず、未ログインになる。
- `--safe-mode` だけでは `settings.json` の値が効き、`--setting-sources ""` だけでは自動のメモリーが有効のまま残るため、両方を付ける。
- 出力形式のフラグは、下の「出力形式」による。
- Claude Code の版を上げるときは、これらのフラグが同じ効果を持つことを確かめる。

## 認証

- 学習者が Claude Code で claude.ai にログインした状態（OAuth）を使う。
- ユーザー設定を読み込まないので、`settings.json` の `apiKeyHelper` や `env` による認証（API キー、Bedrock、Vertex、プロキシなど）は使えない。
- 管理者の設定（macOS では `/Library/Application Support/ClaudeCode/managed-settings.json`）と管理プラグインは、どのフラグでも適用される。アプリでは止めない。

## 出力形式

- ジョブと質疑応答のどちらも、`--output-format stream-json --verbose` を指定し、1 行ずつ JSON として読む。
  - ジョブ: 最後の `result` の行の `result` を、結果のテキストとして取り出す。
  - 質疑応答: `--include-partial-messages` も指定し、`stream_event` の `event.delta.text` を回答の断片として少しずつ表示する。`result` の行で完了とする。
- エラーの判定は [AI 呼び出しのエラーの分類と表示](errors.md) による。
- ツールを使わせない設定でも、AI がツールの呼び出しをまねたテキストを返すことがある。ジョブの出力は、各機能の出力検証で、先頭から形式（frontmatter・JSON）を厳密に確かめる。

## 制限時間

| 用途 | 1 回の呼び出しの制限時間 |
|---|---|
| 質疑応答 | 120 秒 |
| 記録・分析 | 300 秒 |
| 問題生成 | 600 秒 |

制限時間を超えたらプロセスを終了し、エラー（[AI 呼び出しのエラーの分類と表示](errors.md)）とする。
