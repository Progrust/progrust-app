---
type: Decision
title: "ADR 0257"
description: "`claude -p` を新しいプロセスグループで起動し、制限時間を超えたときと中断のときは、グループ全体に SIGTERM を送り、3 秒で終わらなければ SIGKILL を送る"
---

- 日付: 2026-11-27
- 関連仕様: [claude -p の呼び出しの共通ルール](../spec/claude-integration/invocation.md)「プロセスの止め方」
- 置き換え: なし

## 背景

`claude -p` の呼び出しには制限時間があり（[claude -p の呼び出しの共通ルール](../spec/claude-integration/invocation.md)「制限時間」）、質疑応答は「停止」と画面の移動で中断する（[質疑応答の画面](../spec/qa-chat/screen.md)）。このときの、子・孫のプロセスまで止める方法が決まっていなかった（Claude Code 連携の未決事項）。

Issue #10 の技術検証で、次のことが分かっている（[claude -p の子プロセスとしての扱いの技術検証](../verification/issue-10-claude-child-process.md)）。

- 新しいプロセスグループで起動し、グループに SIGTERM、続けて SIGKILL を送ると、`claude` と、その子・孫のプロセスがすべて止まった。`claude` の pid だけに送ると、孫のプロセスが残った。
- 隔離して呼んだ `claude` には子プロセスが無かった。制限時間 3 秒でグループに SIGTERM を送ると、起動から 3.81 秒で、SIGTERM による終了コード 143 で終わった。止めたときは `result` の行が出なかった。
- 検証は、最大 3 秒待ってから SIGKILL を送る形で行い、仕様の変更案として、新しいプロセスグループで起動してグループへ kill することを挙げていた。

M4 の実装計画で、Claude が技術的な判断で決めた。

## 決定

- `claude -p` は、新しいプロセスグループで起動する（[claude -p の呼び出しの共通ルール](../spec/claude-integration/invocation.md)「プロセスの止め方」）。
  - 理由: 検証で、グループへの kill だけが孫のプロセスまで止められた。隔離した呼び出しにはいま子プロセスが無いが、Claude Code の版が変わって子プロセスを作るようになっても残さない。
- 止めるときは、グループ全体に SIGTERM を送り、3 秒たっても終わらなければ SIGKILL を送る。
  - 理由: `claude` に終わりの処理（自分のファイルの書き込みなど）をする機会を与え、応答しないときも 3 秒で必ず止める。検証で確かめた形である。
- 止めた呼び出しの終わりは、`claude` のプロセスが終わってから知らせる。
  - 理由: 終わったと知らせた後に、止めたプロセスが動き続けたり、出力が届いたりしないようにする。

## 検討した他の案

- 案: 学習者のプログラムと同じく、すぐに SIGKILL を送る（[コードの実行（run）](../spec/exercise-runner/run.md)「停止」）
  - 採用しなかった理由: 学習者のプログラムと違い、`claude` は自分の設定ディレクトリのファイルを書くことがある。途中で殺すと、書きかけのファイルが残るおそれがある
- 案: `claude` の pid だけに送る
  - 採用しなかった理由: 検証で、孫のプロセスが残った

## 影響

- 中断してから `chat_finished`（`cancelled`）が届くまで、最大で 3 秒ほどかかる。
- 学習者のプログラムを動かす部品（`src-tauri/src/runner/process.rs`）は、アプリの環境変数を引き継ぎ、すぐに SIGKILL を送るので、`claude` の起動には使わない。`claude` の起動は、渡す環境変数を決める部品（[claude -p の呼び出しの共通ルール](../spec/claude-integration/invocation.md)「環境変数」）を通す。
