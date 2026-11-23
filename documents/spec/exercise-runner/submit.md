---
type: Specification
title: "コードの提出（submit）"
description: "提出を学習者が途中で止められないこと、`stdout` 方式と `test` 方式それぞれの判定手順とテストケースの結果の決め方（`stdout` 方式で出力が一致しても終了コード 0 以外なら `runtime_error`、`test` 方式で隠しテストの入ったテスト実行ファイルの選び方）、`test` 方式で学習者が書いたテストを判定する手順、出力の正規化、判定結果の項目（`status`、`passed`、`total`、`cases[]`）と画面に表示する内容、判定をしなかった提出（`crates_unavailable`）と途中で止めた提出（`stopped`）の扱いを定める。"
---

提出中はコンソールにビルドの出力（[コードの実行（run）](run.md)「ビルドの出力の表示」）と判定の進み具合だけを表示し、プログラムの出力は表示しない。

- 提出は、学習者が途中で止められない。提出中は「停止」ボタンを無効にする。
- 提出の途中で演習画面を離れたときと、アプリを終了したときは、提出を止める（下の「途中で止めた提出」）。

## stdout 方式

1. [コードの実行（run）](run.md)「手順」 と同じ方法でビルドする。失敗したら `compile_error` で終了する。ただし、失敗が [許可クレートと作業領域](crates-and-workspace.md)「初回準備のやり直しが要る失敗」に当てはまるときは、`crates_unavailable` で終了する。
2. テストケースごとに実行ファイルを起動し、`stdin` を書き込んで標準入力を閉じ、標準出力を受け取る。標準エラー出力は、`detail`（下の「判定結果」）に使う先頭の部分だけを受け取り、それより後は、プログラムを止めずに捨てる。
3. 次のとおりテストケースの結果を決める。上の行から順に当てはめる。

   | 結果 | 条件 |
   |---|---|
   | `timeout` | 1 テストケースの実行時間の制限（[実行・判定の制限値](limits.md)）で止めた |
   | `fail` | 標準出力が 1 テストケースの出力量の上限（[実行・判定の制限値](limits.md)）を超えた（出力過多） |
   | `runtime_error` | 終了コードが 0 以外で終わった、またはシグナルで終わった（panic を含む）。標準出力が期待値と一致していても `runtime_error` にする |
   | `pass` | 終了コード 0 で終わり、正規化した標準出力が、正規化した `expected_stdout` と一致した |
   | `fail` | 終了コード 0 で終わり、正規化した標準出力が、正規化した `expected_stdout` と一致しない |

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

3. `cargo test --offline --quiet --no-run --message-format=json-diagnostic-rendered-ansi` でビルドする。失敗したら `compile_error` で終了する。ただし、失敗が [許可クレートと作業領域](crates-and-workspace.md)「初回準備のやり直しが要る失敗」に当てはまるときは、`crates_unavailable` で終了する。隠しテストの側でエラーが出た場合は、「問題文で指定された関数名・型・シグネチャを確認してください」と併せて表示する。
4. 隠しテストの入ったテスト実行ファイルを、3 のビルドの出力から決める。cargo の標準出力の JSON のうち、`reason` が `compiler-artifact` で、`profile.test` が `true` で、`target.src_path` が `judge.target` のファイル（`project/` の中の絶対パス）であるものの `executable` とする。`src/main.rs` と `src/lib.rs` の両方がある演習や、`tests/` に統合テストがある演習でも、ほかのテスト実行ファイルは使わない。見つからないときは、アプリの側の失敗（[実行のコマンドとイベント](../architecture/commands/run.md)「イベント」の `run_finished` の `error`）として終える。そのテスト実行ファイルを引数 `--list progrust_judge::` で起動し、出力の `{名前}: test` の行から隠しテストの名前を得る（学習者自身が書いたテストは判定に含めない）。名前の `progrust_judge::` より後の部分を、テストケースの関数名とする。
5. 得た名前ごとに、テスト実行ファイルを引数 `--exact {名前} --test-threads=1` で別々のプロセスとして起動する。同時に起動するのは CPU の論理コア数までとする。`--nocapture` は付けない。
6. プロセスごとに、次のとおりテストケースの結果を決める。結果の行は、行末が ` ... ok` または ` ... FAILED` で、その直前に `test {名前}` がある行とする（行頭に学習者の出力が付いていてもよい）。

   | 結果 | 条件 |
   |---|---|
   | `pass` | 終了コード 0 で終わり、` ... ok` の結果の行がある |
   | `fail` | ` ... FAILED` の結果の行がある |
   | `runtime_error` | 上のどちらでもなく終わった（結果の行がないまま終了コード 0 で終わった、シグナルで終わったなど） |
   | `timeout` | テスト全体の制限時間（[実行・判定の制限値](limits.md)）で止めた、または起動する前に制限時間に達した |

7. `fail` の panic メッセージは、`failures:` の行より後で最後の `thread '{名前}' ... panicked at` の行の、次の行から取り出す。`note: run with` で始まる行と、次の節の見出し（`failures:` など）の前で終える。
8. `exercise.judge.min_learner_tests`（[レッスン定義の形式（lesson.json）](../content-format/lesson-json.md)）がある演習では、続けて「学習者のテストの判定」を行う。無い演習では、学習者が書いたテストを判定に含めない。

### 学習者のテストの判定

学習者が書いたテスト（単体テスト、`tests/` の統合テスト、ドキュメントテスト）を、テストケース 1 件として判定する。

1. 3 でビルドした `project/` で、`cargo test --offline --no-fail-fast -- --skip progrust_judge::` を新しいプロセスグループで起動する。隠しテストは `--skip` で除く。
2. 標準出力から、行頭が `test ` で、行末が ` ... ok` または ` ... FAILED` の行を、学習者のテストの結果の行として数える。` ... ignored` の行は数えない。
3. 次のとおり結果を決める。

   | 結果 | 条件 |
   |---|---|
   | `pass` | cargo が終了コード 0 で終わり、` ... ok` の行が `min_learner_tests` 件以上ある |
   | `fail` | ` ... FAILED` の行がある、または ` ... ok` の行が `min_learner_tests` 件より少ない |
   | `runtime_error` | 上のどちらでもなく、cargo が終了コード 0 以外で終わった |
   | `timeout` | 学習者のテスト全体の制限時間（[実行・判定の制限値](limits.md)）で止めた |

- このテストケースの表示名は「自分で書いたテスト（{min_learner_tests} 件以上）」とし、`cases[]` の最後に置く。
- `fail` の `detail` には、通ったテストの件数と必要な件数、失敗したテストの名前（` ... FAILED` の行の `test ` と ` ... FAILED` の間）を入れる。失敗の詳しい内容は、テスト実行（[コードの実行（run）](run.md)「テスト実行」）で確かめる。

## 判定結果

| 項目 | 内容 |
|---|---|
| `status` | `passed`（全テストケース通過）/ `wrong`（1 件以上失敗）/ `compile_error` / `crates_unavailable`（判定をしていない。下の「判定をしなかった提出」）/ `stopped`（途中で止めた。下の「途中で止めた提出」） |
| `passed` / `total` | 通過したテストケース数 / 全テストケース数 |
| `cases[]` | テストケースごとの `name`（表示名）・`status`（`pass` / `fail` / `timeout` / `runtime_error`）・`detail` |

`detail` の内容:

- stdout 方式の `fail`: 入力・期待する出力・実際の出力
- test 方式の `fail`: panic メッセージ（`assert_eq!` の左辺・右辺を含む）。学習者のテストの判定の `fail` は「学習者のテストの判定」に従う
- `timeout` / `runtime_error`: 標準エラー出力の先頭

画面には、通過数と失敗したテストケースの名前をすべて表示し、最初に失敗したテストケースの `detail` を表示する。表示する場所と、詳細の開閉は [演習画面](../screens/exercise-screen.md) に従う。

## 判定をしなかった提出

`status` が `crates_unavailable` の提出は、テストケースを 1 件も実行していない。

- `passed` と `total` は 0、`cases[]` は空にする。
- 提出として数えない。試行ログに `submit` イベントを追記しない（[実行・提出の試行ログへの記録](attempt-log.md)）。
- 画面での扱いは [演習画面](../screens/exercise-screen.md)「判定結果」に従う。

## 途中で止めた提出

提出の途中で演習画面を離れたとき（[演習画面](../screens/exercise-screen.md)）と、アプリを終了したときは、ビルドかテストケースの実行のプロセスグループ全体に SIGKILL を送って止める（止め方は [コードの実行（run）](run.md)「停止」と同じ）。

- `status` を `stopped` とし、`passed` と `total` は 0、`cases[]` は空にする。まだ送っていない出力は捨てる。
- 提出として数えない。試行ログに `submit` イベントを追記しない（[実行・提出の試行ログへの記録](attempt-log.md)）。正解になる前に止めたので、レッスンを完了にしない。
