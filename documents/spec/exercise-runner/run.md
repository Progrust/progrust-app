---
type: Specification
title: "コードの実行（run）"
description: "実行の手順、対話的な標準入力の扱い、結果の状態（`success`、`compile_error`、`runtime_error`、`timeout`、`stopped`）を定める。"
---

## 手順

1. `cargo build --offline --message-format=json-diagnostic-rendered-ansi` でビルドする。
2. ビルドに失敗したら、コンパイラの診断を表示して終了する（`compile_error`）。
3. 成功したら、生成された実行ファイルを `project/` を作業ディレクトリとして起動する。
4. 標準出力・標準エラー出力を `run_output` イベントで逐次送る。
5. 終了したら `run_finished` イベントで結果を送り、試行ログに `run` イベントを追記する（[実行・提出の試行ログへの記録](attempt-log.md)）。

- 演習には必ず `src/main.rs` があるものとする（[演習のファイル形式](../content-format/exercise.md)「ファイル」）。
- コンパイラの警告も表示する。cargo 自身の進捗メッセージ（`Compiling ...` など）は表示しない。

## 対話的な標準入力

- 実行中、コンソールの下部に入力欄を表示する。
- Enter で入力した 1 行に改行を付けてプログラムの標準入力に送り（`send_stdin`）、コンソールにも入力として区別できる色で表示する。
- 「入力終了」ボタン（または Ctrl+D）で標準入力を閉じる（`close_stdin`）。
- プログラムが終了したら入力欄を無効にする。

## 結果の状態

| 状態 | 条件 |
|---|---|
| `success` | 終了コード 0 で終了した |
| `compile_error` | ビルドに失敗した |
| `runtime_error` | 終了コードが 0 以外、またはシグナルで終了した（panic を含む）。出力量の上限を超えて停止した場合も含む |
| `timeout` | 制限時間を超えたため停止した |
| `stopped` | 学習者が停止ボタンを押した |
