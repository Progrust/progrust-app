---
type: Decision
title: "ADR 0167"
description: "フロントエンドに許可する Tauri の権限に、バックエンドが送るイベントの受け取り（listen・unlisten）を加え、それ以外の組み込みの API（フロントエンドからのイベントの送信を含む）は許可せず、本アプリのコマンドは権限の一覧に書かずに呼べるままにする"
---

- 日付: 2026-11-22
- 関連仕様: [Tauri の権限](../spec/architecture/tauri-permissions.md)
- 置き換え: なし

## 背景

[Tauri の権限](../spec/architecture/tauri-permissions.md) は、フロントエンドに許可する権限を「本アプリのコマンドの呼び出しと、外部 URL を OS の既定ブラウザで開くこと」に限るとしていた。一方、[コマンドとイベントの共通の決まり](../spec/architecture/commands/conventions.md)「仕組み」は、バックエンドが送るイベントを、フロントエンドが `listen` で受け取ると定めている。

Issue #74（CSP と Tauri の権限を設定するタスク）の実装で、次のことが分かった。

- Tauri 2 では、`listen` と `unlisten` は本アプリのコマンドではなく、Tauri の組み込みの API（`plugin:event|listen`・`plugin:event|unlisten`）である。権限の `core:event:allow-listen`・`core:event:allow-unlisten` が無いと呼べない。仕様の文からは、イベントの受け取りが許可する範囲に入るかが読み取れなかった。
- 雛形の権限 `core:default` は、アプリ情報・ファイルのパス・ウィンドウの操作など、組み込みの API の大半を許可する。イベントの権限の既定（`core:event:default`）も、受け取りのほかに、フロントエンドからの送信（`emit`・`emit_to`）を許可する。
- 本アプリのコマンドは、`build.rs` でコマンドの権限の定義（アプリのマニフェスト）を作らなければ、権限の一覧に書かなくても、アプリのページから呼べる。外部のページからは、権限を書かない限り呼べない（tauri 2.12.7 の `Webview::on_message`）。

プロジェクトの方針（`CLAUDE.md`）で、アプリの操作感に関わらない技術の詳細は、Claude が最適と思う方法で決めることになっている。この判断は、ユーザーがスキル `implement-task` で Issue #74 の実装を頼んだ会話で、Claude が決めた。以下の決定と理由は、すべて Claude の判断である。

## 決定

- フロントエンドに許可する権限に、バックエンドが送るイベントの受け取り（`listen` と `unlisten`）を加える（[Tauri の権限](../spec/architecture/tauri-permissions.md)）。
  - 理由: イベントで経過と結果を受け取ることは、既に仕様が決めている（[コマンドとイベントの共通の決まり](../spec/architecture/commands/conventions.md)）。受け取りを許可しないと、実行・提出・質疑応答・ジョブの表示が作れない。受け取りは、バックエンドが送ると決めたものを読むだけで、フロントエンドができることを増やさない。
- 組み込みの API は、イベントの受け取りのほかは許可しない。フロントエンドからのイベントの送信（`emit`・`emit_to`）も許可しない。権限は `core:default` や `core:event:default` ではなく、`core:event:allow-listen` と `core:event:allow-unlisten` を名指しで書く。
  - [ADR 0147](0147-define-command-and-event-payloads-in-architecture.md) の「決定」の `get_app_version` の項にある「フロントエンドにはアプリのコマンド以外の Tauri の権限を許可しない」を、「アプリのコマンドとイベントの受け取りのほかは許可しない」に変える。アプリ情報の API を許可しないことは変えない。
  - 理由: 権限を最小限にする方針（[セキュリティとプライバシー](../spec/non-functional/security-privacy.md)「アプリ自体」）に合わせる。フロントエンドからバックエンドへの要求は、すべてコマンドで送ると決めてあり、イベントを送る使い道が無い。アプリ情報は `get_app_version` で受け取る（[設定のコマンド](../spec/architecture/commands/settings.md)「get_app_version」）。
- 本アプリのコマンドは、アプリのマニフェストを作らず、権限の一覧に書かずに呼べるままにする。
  - 理由: 仕様は、本アプリのコマンドの呼び出しを、コマンドを区別せずに許可している。マニフェストを作ると、コマンドを足すたびに `build.rs` と権限の一覧の両方を直すことになり、書き漏らすとそのコマンドだけが呼べなくなる。外部のページから呼べないことは、マニフェストが無くても Tauri が守る。

## 検討した他の案

- 案: 雛形のまま `core:default` を許可する
  - 採用しなかった理由: アプリ情報・ファイルのパス・ウィンドウの操作などの組み込みの API も許可してしまい、仕様（[Tauri の権限](../spec/architecture/tauri-permissions.md)、[設定のコマンド](../spec/architecture/commands/settings.md)）に反する
- 案: イベントの権限の既定（`core:event:default`）を許可する
  - 採用しなかった理由: 使い道の無い、フロントエンドからのイベントの送信も許可してしまう
- 案: イベントを使わず、経過と結果を Tauri 2 のチャンネル（`Channel`）で送り、イベントの権限を許可しない
  - 採用しなかった理由: イベントで送ることは [ADR 0147](0147-define-command-and-event-payloads-in-architecture.md) で決めてある。要求に結びつかない知らせ（`job_updated`・`environment_updated`）にはイベントが要る
- 案: アプリのマニフェストを作り、本アプリのコマンドを 1 つずつ権限の一覧に書く
  - 採用しなかった理由: 上の「決定」の 3 つ目の理由のとおり。仕様が許可する範囲は変わらず、手間と書き漏らしだけが増える

## 影響

- コマンドやイベントを足しても、権限の一覧（`src-tauri/capabilities/`）を直さなくてよい。
- 外部 URL を OS の既定ブラウザで開く権限は、まだ許可していない。それを使う画面のタスクで、プラグインとその権限を加える。そのとき、権限の自動テスト（`src-tauri/tests/permissions.rs`）の許可する一覧も直す。
- 組み込みの API が要る機能（ウィンドウの操作など）を作るときは、仕様を変えて、その権限を名指しで加える。
