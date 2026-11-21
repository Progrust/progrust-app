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
4. テスト実行ファイルを引数 `--list progrust_judge::` で起動し、出力の `{名前}: test` の行から隠しテストの名前を得る（学習者自身が書いたテストは判定に含めない）。名前の `progrust_judge::` より後の部分を、テストケースの関数名とする。
5. 得た名前ごとに、テスト実行ファイルを引数 `--exact {名前} --test-threads=1` で別々のプロセスとして起動する。同時に起動するのは CPU の論理コア数までとする。`--nocapture` は付けない。
6. プロセスごとに、次のとおりテストケースの結果を決める。結果の行は、行末が ` ... ok` または ` ... FAILED` で、その直前に `test {名前}` がある行とする（行頭に学習者の出力が付いていてもよい）。

   | 結果 | 条件 |
   |---|---|
   | `pass` | 終了コード 0 で終わり、` ... ok` の結果の行がある |
   | `fail` | ` ... FAILED` の結果の行がある |
   | `runtime_error` | 上のどちらでもなく終わった（結果の行がないまま終了コード 0 で終わった、シグナルで終わったなど） |
   | `timeout` | テスト全体の制限時間（[実行・判定の制限値](limits.md)）で止めた、または起動する前に制限時間に達した |

7. `fail` の panic メッセージは、`failures:` の行より後で最後の `thread '{名前}' ... panicked at` の行の、次の行から取り出す。`note: run with` で始まる行と、次の節の見出し（`failures:` など）の前で終える。

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
