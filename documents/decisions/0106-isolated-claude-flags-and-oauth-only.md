---
type: Decision
title: "ADR 0106"
description: "`claude -p` を検証で確かめたフラグの組み合わせで隔離して呼び、既定のシステムプロンプトを置き換え、認証は claude.ai のログインだけを使う"
---

- 日付: 2026-11-21
- 関連仕様: [claude -p の呼び出しの共通ルール](../spec/claude-integration/invocation.md)、[非機能要件の未決事項](../spec/non-functional/open-questions.md)
- 置き換え: なし

## 背景

[ADR 0021](0021-isolated-claude-invocation.md) で、`claude -p` を学習者のユーザー設定を読み込まずに呼び出すと決めた。具体的なフラグは実装時に確かめることにしていた。

Issue #8 の技術検証で、フラグの組み合わせごとの効き方を確かめた（[claude -p の隔離起動の技術検証](../verification/issue-8-claude-isolated-invocation.md)）。

- `--safe-mode --setting-sources "" --strict-mcp-config --tools "" --disable-slash-commands --no-session-persistence` で、フック・MCP・claude.ai のコネクター・プラグイン・スキル・エージェント・コマンド・出力スタイル・`CLAUDE.md`（ユーザー・プロジェクト）・メモリーがすべて効かず、ツールも無く、claude.ai のログインのまま応答が返った。`~/.claude/CLAUDE.md` は、2026-11-21 に目印を置いて確かめた。
- `--bare` は claude.ai のログイン（OAuth）を読まず、未ログインになった。
- `--safe-mode` だけでは `settings.json` の値が効き、`--setting-sources ""` だけでは自動のメモリーが有効のまま残った。
- claude.ai のコネクター（MCP）は、隔離しないと読み込まれた。
- 隔離しても、Claude Code の既定のシステムプロンプトは使われ、作業ディレクトリ・実行環境・アカウントのメールアドレスなどがコンテキストに入った。
- このフラグでは `settings.json` を読まないので、`apiKeyHelper` や `env` による認証も効かなくなると推定した（試していない）。管理者の設定と管理プラグインは、どのフラグでも適用される。

検証の結果、次の 3 点がユーザーの判断を要する点として残った。設定ファイルでの認証を捨ててよいか、管理者の設定が効くことを許容するか、既定のシステムプロンプトを置き換えるか。Claude は、claude.ai のログインだけを対象にし、管理者の設定は一般公開する場合に扱い、システムプロンプトを置き換えることを推奨した。ユーザーは「私は細かい技術的な話はわからないので、あなたの推奨通りで進めて大丈夫です」と示し、推奨を承認した。以下の理由は Claude が書いた。

## 決定

- `claude -p` を、検証で確かめたフラグの組み合わせで呼び出す。`--bare` は使わない（[claude -p の呼び出しの共通ルール](../spec/claude-integration/invocation.md)「起動のフラグ」）。
  - 理由: 検証で、この組み合わせだけが、学習者の設定をすべて止めつつ claude.ai のログインを使えた。
- 読み込まない対象に、アカウントに紐づく claude.ai のコネクター（MCP）を明記する。
  - 理由: `settings.json` の MCP とは別の経路で読み込まれ、隔離しないと効く。
- 学習者の Claude Code のセッション履歴に、アプリの呼び出しを残さない（`--no-session-persistence`）。
  - 理由: ADR 0021 で、学習者のセッション履歴にアプリの呼び出しが残ることを避けたい点に挙げていた。
- Claude Code の既定のシステムプロンプトを、アプリが決めた文に置き換える（`--system-prompt`）。
  - 理由: 呼び出しの条件をすべてアプリで決められる。作業ディレクトリや実行環境、メールアドレスのように、記録・分析・問題生成・質疑応答に要らない情報を AI に渡さずに済む。
- 認証は、学習者が Claude Code で claude.ai にログインした状態だけを使う。`settings.json` による認証と、管理者の設定・管理プラグインが効く環境への対応は、一般公開をする場合に決める（[非機能要件の未決事項](../spec/non-functional/open-questions.md)）。
  - 理由: アプリの完成までは、対応環境を作者の Mac だけとしている（[ADR 0090](0090-support-only-authors-mac-until-completion.md)）。作者の Mac は claude.ai のログインで Claude Code を使っており、管理者の設定も無い。
- ジョブの出力は、先頭から形式を厳密に確かめる。
  - 理由: ツールを使わせない設定でも、AI がツールの呼び出しをまねたテキストを返すことがある。

## 検討した他の案

- 案: `--bare` で起動する
  - 採用しなかった理由: claude.ai のログインを読まず、未ログインになる
- 案: `CLAUDE_CONFIG_DIR` や `HOME` を空の一時ディレクトリにして隔離する
  - 採用しなかった理由: ログインの情報も読めなくなり、未ログインになる
- 案: 既定のシステムプロンプトのまま使う
  - 採用しなかった理由: AI に要らない情報が入り、呼び出しの条件の一部が Claude Code の版によって変わる
- 案: `settings.json` の認証を使えるように、`settings.json` だけは読み込む
  - 採用しなかった理由: `settings.json` のフックや権限などの値も効き、ADR 0021 の隔離が崩れる。作者の Mac では不要である

## 影響

- 学習者が claude.ai のログイン以外の方法で Claude Code を使っていると、アプリからは未ログインと同じエラーになる。
- `--system-prompt` で置き換えたときに、claude.ai のログインで応答が返ることと、隔離の効き方が変わらないことは、検証では試していない。実装時に確かめる。
- Claude Code の版を上げると、フラグの名前や効き方が変わることがある。版を上げるときに確かめ直す。
- `~/.claude.json` などの Claude Code 自身のファイルは、隔離しても更新される。
