---
type: Specification
title: "claude -p の呼び出しの共通ルール"
description: "`--model` の指定、ツール・ユーザー設定を使わない起動のフラグ、`claude` に渡す環境変数、使う認証、既定のシステムプロンプトの置き換え、一時ディレクトリ、標準入力での指示文の渡し方、出力形式、用途ごとの制限時間と、制限時間を超えたときと中断のときのプロセスの止め方、終わった後に同じプロセスグループに残ったプロセスの止め方を定める。"
---

- 設定画面で選択したモデルを `--model` で指定する（[AI モデルの選択](model-selection.md)）。モデルの値が [設定画面と settings.json](../screens/settings-screen.md)「settings.json」の条件を満たさないとき（`settings.json` を手で書き換えたときなど）は、`claude` を起動せず、その他のエラー（[AI 呼び出しのエラーの分類と表示](errors.md)）とする。
- ツールを使わせない設定で起動する。
- 学習者の Claude Code のユーザー設定を読み込まない設定で起動する。読み込まない対象は、ユーザー・プロジェクトの `settings.json`、フック、MCP サーバー、アカウントに紐づく claude.ai のコネクター（MCP）、プラグイン、スキル、エージェント、スラッシュコマンド、出力スタイル、`CLAUDE.md`（ユーザー・プロジェクト）、メモリーである。
- 学習者の Claude Code のセッション履歴に、アプリの呼び出しを残さない。
- Claude Code の既定のシステムプロンプトは使わず、アプリが決めた文に置き換える。文面は指示文（[指示文（プロンプト）の置き場所と書き方](prompts.md)）と同じく実装の一部とする。
- 作業ディレクトリは呼び出しごとに作る空の一時ディレクトリとし、終了後に削除する。
- 指示文は標準入力で渡す。
- `--continue` / `--resume` は使わない。会話の続きが必要な場合（質疑応答）も、アプリが履歴を毎回入力に含める。

## 起動のフラグ

すべての呼び出しで、次のフラグを付ける。Claude Code 2.1.329 で効き方を確かめた組み合わせである。

```
-p --model <モデル> --safe-mode --setting-sources "" --strict-mcp-config --tools "" --disable-slash-commands --no-session-persistence --system-prompt <アプリが決めた文>
```

- `--bare` は使わない。`--bare` を付けると claude.ai のログイン（OAuth）を読まず、未ログインになる。
- `--safe-mode` だけでは `settings.json` の値が効き、`--setting-sources ""` だけでは自動のメモリーが有効のまま残るため、両方を付ける。
- `--system-prompt` で置き換えても、Claude Code は、作業ディレクトリのパス、OS の版、日付などの実行環境の情報をコンテキストに加える。
- 出力形式のフラグは、下の「出力形式」による。
- Claude Code の版を上げるときは、これらのフラグが同じ効果を持つことを確かめる。

## 環境変数

- `claude` を起動するときは、アプリが引き継いだ環境変数をそのまま渡さない。次の 6 つだけを渡す。

```
HOME USER LOGNAME PATH SHELL TMPDIR
```

- 値は、アプリが受け取ったものをそのまま渡す。アプリが受け取っていない変数は渡さない。
- `claude -p` の呼び出しだけでなく、アプリが `claude` を起動するすべての場合に同じにする。検出のための `claude --version`（[外部ツール（cargo・claude）への依存](../architecture/external-tools.md)「検出」）と、接続テスト（[claude が使えない場合の動作](unavailable.md)）を含む。
- cargo、`clang`（[C のツール（clang）への依存](../architecture/c-toolchain.md)。検出のための `clang --version` を含む）と、学習者のプログラムの起動（C の演習の提出で、メモリリークの検査のライブラリを差し込む起動を含む。[C のコードの提出（submit）](../exercise-runner/c-submit.md)「メモリリークの検査」）には、この決まりを当てはめない。

## 認証

- 学習者が Claude Code で claude.ai にログインした状態（OAuth）を使う。
- ユーザー設定を読み込まないので、`settings.json` の `apiKeyHelper` や `env` による認証（API キー、Bedrock、Vertex、プロキシなど）は使えない。上の「環境変数」のとおり、シェルの環境変数（`ANTHROPIC_API_KEY`、プロキシの変数、`CLAUDE_CONFIG_DIR` など）も `claude` に渡らないので、それらによる認証と設定も使えない。
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

## プロセスの止め方

- `claude -p` は、新しいプロセスグループで起動する。
- 制限時間を超えたときと、呼び出しを中断するとき（質疑応答の「停止」と、回答中に画面を離れるとき。[質疑応答の画面](../qa-chat/screen.md)）は、そのプロセスグループ全体に SIGTERM を送る。3 秒たってもプロセスが終わらないときは、プロセスグループ全体に SIGKILL を送る。
- 止めた呼び出しの終わり（質疑応答の `chat_finished` など）は、`claude` のプロセスが終わってから知らせる。
- `claude` のプロセスが終わったら（止めたときも、自分で終わったときも）、同じプロセスグループに残ったプロセスに SIGKILL を送る。その後に届く出力は最大 300ms だけ待って受け取り、パイプの終わり（EOF）は待たない。
