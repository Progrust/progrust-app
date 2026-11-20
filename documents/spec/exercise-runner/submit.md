---
type: Specification
title: "コードの提出（submit）"
description: "`stdout` 方式と `test` 方式それぞれの判定手順、出力の正規化、判定結果の項目（`status`、`passed`、`total`、`cases[]`）と画面に表示する内容を定める。"
---

提出中はコンソールにビルドの診断と判定の進み具合だけを表示し、プログラムの出力は表示しない。

## stdout 方式

1. [コードの実行（run）](run.md)「手順」 と同じ方法でビルドする。失敗したら `compile_error` で終了する。
2. テストケースごとに実行ファイルを起動し、`stdin` を書き込んで標準入力を閉じ、標準出力を受け取る。
3. 正規化した標準出力を `expected_stdout` と比較する。
4. 1 件失敗しても残りのテストケースはすべて実行する。

**正規化**: 改行コード（CRLF → LF）をそろえ、各行の末尾の空白と、末尾の空行を取り除く。期待値にも同じ正規化をかける。

## test 方式

1. `judge_tests.rs` を `project/progrust_judge_tests.rs` に置く。
2. `judge.target` のファイルの末尾に以下を追加する。

   ```rust
   #[cfg(test)]
   #[path = "../progrust_judge_tests.rs"]
   mod progrust_judge;
   ```

3. `cargo test --offline --no-run --message-format=json-diagnostic-rendered-ansi` でビルドする。失敗したら `compile_error` で終了する。隠しテストの側でエラーが出た場合は、「問題文で指定された関数名・型・シグネチャを確認してください」と併せて表示する。
4. テスト実行ファイルを、引数 `progrust_judge::` で起動する（学習者自身が書いたテストは判定に含めない）。
5. 出力の `test progrust_judge::{関数名} ... ok|FAILED` を読み取り、テストケースごとの結果とする。失敗したテストの panic メッセージを詳細として取り出す。

## 判定結果

| 項目 | 内容 |
|---|---|
| `status` | `passed`（全テストケース通過）/ `wrong`（1 件以上失敗）/ `compile_error` |
| `passed` / `total` | 通過したテストケース数 / 全テストケース数 |
| `cases[]` | テストケースごとの `name`（表示名）・`status`（`pass` / `fail` / `timeout` / `runtime_error`）・`detail` |

`detail` の内容:

- stdout 方式の `fail`: 入力・期待する出力・実際の出力
- test 方式の `fail`: panic メッセージ（`assert_eq!` の左辺・右辺を含む）
- `timeout` / `runtime_error`: 標準エラー出力の先頭

画面には、通過数と失敗したテストケースの名前をすべて表示し、最初に失敗したテストケースの `detail` を表示する。表示する場所と、詳細の開閉は [演習画面](../screens/exercise-screen.md) に従う。
