---
type: Decision
title: "ADR 0124"
description: "アプリが `claude` を起動するときは、引き継いだ環境変数をそのまま渡さず、HOME・USER・LOGNAME・PATH・SHELL・TMPDIR の 6 つだけを渡す"
---

- 日付: 2026-11-21
- 関連仕様: [claude -p の呼び出しの共通ルール](../spec/claude-integration/invocation.md)「環境変数」、[外部ツール（cargo・claude）への依存](../spec/architecture/external-tools.md)「検出」、[非機能要件の未決事項](../spec/non-functional/open-questions.md)
- 置き換え: なし

## 背景

`claude -p` は、学習者のユーザー設定を読み込まずに呼び出し（[ADR 0021](0021-isolated-claude-invocation.md)）、認証は claude.ai のログインだけを使う（[ADR 0106](0106-isolated-claude-flags-and-oauth-only.md)）。アプリが `claude` を子プロセスとして起動するときに、アプリが引き継いだ環境変数をどう扱うかは、仕様に無かった（Issue #43）。

技術検証で、次のことが分かっている。

- Finder から起動したアプリが受け取る環境変数は、`HOME`・`LOGNAME`・`PATH`・`SHELL`・`TMPDIR`・`USER` などの少数で、`CLAUDE*` や `LANG` は無い。ターミナルから起動したアプリには、呼び出し元のシェルの環境変数がそのまま渡る（[claude -p の子プロセスとしての扱いの技術検証](../verification/issue-10-claude-child-process.md)）。
- 環境変数を空にしたうえで `HOME`・`USER`・`LOGNAME`・`PATH`・`SHELL`・`TMPDIR` の 6 つだけを渡して `claude -p` を呼ぶと、claude.ai のログインのまま応答が返り、50KB の日本語の指示文も文字化けせずに届いた（同じ検証。出力形式は json で、`--system-prompt` は付けていない）。
- 隔離起動の検証では、名前が `CLAUDE`・`ANTHROPIC`・`AI_AGENT`・`MCP_`・`OTEL` で始まる環境変数を外して呼び、隔離のフラグが効くことを確かめた（[claude -p の隔離起動の技術検証](../verification/issue-8-claude-isolated-invocation.md)）。
- シェルの環境変数 `ANTHROPIC_API_KEY` は、`claude` の認証に効くと推定している（試していない。同じ検証の「残った事項」）。

Claude は、6 つだけを渡すことを推奨した。ユーザーは「その他の内容は推奨通りで問題ありません。」と示し、推奨を承認した。以下の理由は Claude が書いた。

## 決定

- アプリが `claude` を起動するときは、引き継いだ環境変数をそのまま渡さず、`HOME`・`USER`・`LOGNAME`・`PATH`・`SHELL`・`TMPDIR` の 6 つだけを、受け取った値のまま渡す（[claude -p の呼び出しの共通ルール](../spec/claude-integration/invocation.md)「環境変数」）。
  - 理由: ターミナルから起動したときだけ、シェルの `ANTHROPIC_API_KEY` などが `claude` に効くと、認証を claude.ai のログインだけにする判断（ADR 0106）と、学習者の環境の設定に関係なく同じ条件で呼び出す判断（ADR 0021）が崩れる。6 つだけにすれば、どこから起動しても、Finder から起動したときとほぼ同じ条件になる。この 6 つでログインが通り、指示文が届くことは、検証で確かめてある。
- `claude -p` の呼び出しだけでなく、検出のための `claude --version` と接続テストでも同じにする。
  - 理由: 検出と接続テストが通ったのに実際の呼び出しが失敗する、という食い違いを避ける。
- cargo と学習者のプログラムの起動には、この決まりを当てはめない。
  - 理由: 演習のビルドには、学習者の cargo の設定を効かせると決めている（[ADR 0099](0099-allow-package-cache-wait-during-preparation.md)、[ADR 0105](0105-detect-lost-crates-from-offline-build-errors.md)）。

## 検討した他の案

- 案: 名前の頭（`CLAUDE`・`ANTHROPIC`・`AI_AGENT`・`MCP_`・`OTEL`）で選んで外し、ほかはそのまま渡す
  - 採用しなかった理由: 隔離起動の検証で使った方法だが、Claude Code の版が上がって効く変数が増えるたびに、一覧を直す必要がある。直し忘れると、気づかないうちに呼び出しの条件が変わる
- 案: 環境変数をそのまま引き継ぐ
  - 採用しなかった理由: 「決定」に書いたとおり、ターミナルから起動したときだけ条件が変わり、ADR 0021 と ADR 0106 の判断が崩れる

## 影響

- シェルの環境変数で Claude Code を設定している学習者（プロキシの変数、`ANTHROPIC_API_KEY`、`CLAUDE_CONFIG_DIR` で設定の場所を変えている場合など）は、アプリから `claude` を使えない。Finder から起動したアプリには、もともとシェルの環境変数が渡らない。これらへの対応は、claude.ai のログイン以外の認証への対応と合わせて、一般公開をする場合に決める（[非機能要件の未決事項](../spec/non-functional/open-questions.md)）。
- 6 つだけを渡した呼び出しは、json の出力形式で、`--system-prompt` を付けずに確かめた。仕様の出力形式（stream-json）と `--system-prompt` を付けた呼び出しでも同じように動くことは、実装時に確かめる。
- Claude Code の版を上げるときは、起動のフラグと合わせて、この 6 つで動くことを確かめる。
