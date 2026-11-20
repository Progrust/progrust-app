---
type: Verification Report
title: "test 方式のテスト結果の読み取りの技術検証"
description: "test 方式の libtest の出力からテストごとの結果と panic メッセージを確実に読み取れるかを Issue #15 で検証した結果（出力形式、学習者の出力との混合、process::exit・abort・無限ループ、10 秒の制限での打ち切り、1 回起動する方式とテストごとに別プロセスで実行する方式の比較）を記す。"
---

- 日付: 2026-11-21
- Issue: [#15](https://github.com/Progrust/progrust-app/issues/15)
- 確かめた OS: macOS 27.2.1（Darwin 27.2.0、Apple Silicon）。Windows は確かめていない
- 関連仕様: [提出と判定](../spec/exercise-runner/submit.md)、[実行時の制限](../spec/exercise-runner/limits.md)、[ADR 0015](../decisions/0015-show-failed-case-details.md)

## 環境

- macOS 27.2.1（Darwin 27.2.0）、Apple Silicon、8 コア
- rustc 1.99.1 (58cf8f9c3 2026-10-13) stable、cargo 1.99.1
- 計測時は他のプロセスがビルド中で、load average が 1.9 から 16 まで動いた。所要時間は参考値である。

## 方法

15 種類の学習者コードと隠しテストの組（`println!` するもの、偽の結果行を書くもの、`process::exit(0)`・abort・スタックオーバーフロー・無限ループ・`#[ignore]` など）を作り、libtest の出力を観察した。あわせて、Rust（std のみ）で解析器を試作し、2 種類の方式を比べた。

- single: `BIN progrust_judge::` を 1 回起動し、標準出力を解析する（仕様どおり）。
- per-test: `BIN --list progrust_judge::` でテスト名を得て、テストごとに `BIN --exact {名前} --test-threads=1` を別プロセスで起動し（最大 N 並列）、終了コードで判定する。全体の期限（10 秒）を共有し、超えたら残りを kill して `timeout` にする。

## 結果

### libtest の出力形式

デフォルトの pretty 形式（出力の捕捉あり）は次の形になる。

```
running 6 tests
test progrust_judge::t_ok ... ok
test progrust_judge::t_ret ... FAILED
...
failures:

---- progrust_judge::t_fail stdout ----
learner says hi            <- 学習者の println!（失敗したテストだけ、ここに出る）

thread 'progrust_judge::t_fail' (283977) panicked at src/../progrust_judge_tests.rs:3:23:
assertion `left == right` failed: custom msg
  left: 3
 right: 4
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

failures:
    progrust_judge::t_fail
test result: FAILED. 1 passed; 5 failed; ...
```

- 結果行は、失敗したテストの節（`failures:`）より前にまとまる。
- `println!`・`print!`・`eprintln!` はテストごとに捕捉される。成功したテストの出力は捨てられ、失敗したテストの出力は `---- 名前 stdout ----` の節にだけ出る。結果行とは混ざらない。学習者の `eprintln!` は stderr には出ない。`std::thread::spawn` した先の `println!` も捕捉された。
- 複数行の panic、`assert_eq!` の left/right（Vec、`\n` を含む文字列はエスケープ表示）、`assert!`、`unwrap()` の Err、添字の範囲外のメッセージも、節から取れた。
- `assert_eq!` のメッセージは、`assertion \`left == right\` failed[: カスタムメッセージ]`、`  left: ...`、` right: ...` の形になる。`note: run with RUST_BACKTRACE` の行は除く必要がある（初回だけ出るので、解析側で毎回除く）。
- panic の行は `thread '名前' (スレッドID) panicked at ファイル:行:列:` の形で、新しい rustc ではスレッド ID が入る。`panicked at ` で探すのが安全である。
- `--nocapture` を付けると、`test X ... ` の直後に学習者の出力が入り、結果行が壊れる（例: `test X ... learner says hi` と、改行なしの出力に続く `ok`）。付けてはいけない。
- `--test-threads=1` では、`test X ... ` が開始時に出て結果が後ろに続き、実行順は名前順になる。デフォルト（並列）では開始行が出ず、完了したものから `test X ... ok` が出る。並列でも、結果行は 1 行ずつ書かれ、結果行同士は入り交じらない。

### stable で使える出力形式

- `--format json` と `--report-time` は、stable では `The "json" format is only accepted on the nightly compiler with -Z unstable-options` になる。`-Z unstable-options` を付けても、そのままでは使えない。
- 環境変数 `RUSTC_BOOTSTRAP=1`（値は何でもよく、`0` でも通った）を付けると、stable の libtest でも `-Z unstable-options --format json` が使える。`started` イベントと、失敗したテストごとの `stdout` フィールドが得られ、テストごとの帰属が明確になる。ただし非公式の抜け道で、将来の rustc で動く保証がないので、採用しない。pretty 形式の解析で足りる。
- `--format terse` は `.` の表示でテスト名が出ないので使えない。

### 学習者の出力との混合と偽の行

| ケース | single の結果 | per-test の結果 |
| --- | --- | --- |
| `println!` するテスト（成功・失敗） | 読める。結果行と混ざらない | 読める |
| 失敗するテストが `println!` で偽の `test progrust_judge::X ... ok`、`failures:`、`---- Y stdout ----` を書く | 結果行は `failures:` より前だけを読めば影響がない。panic メッセージは、最後の `thread '名前' ... panicked at` を起点にすれば正しく取れた | 読める |
| `std::io::stdout().write_all` で捕捉を回避し、偽の結果行（`test progrust_judge::c01_ok ... FAILED`）を書く | 同じテストに 2 行（`FAILED` と `ok`）が付く。重複を見つけて競合として扱えるが、どちらが本物かは分からない | 終了コードが基準なので影響がない |
| 捕捉を回避して、改行なしの `junk` を書く | 次の結果行が `junktest X ... ok` と壊れる。行頭固定の正規表現では結果が消える。先頭のゴミを許し、行末の ` ... ok` と直前の `test ` で探せば読める | 終了コードは 0 だが ok の行が出ていないので、`runtime_error` にできる |
| テスト中の `std::process::exit(0)` | 全体が終了コード 0 で終わり、結果行が出ない（`running 3 tests` で止まる）。終了コードだけを見ると成功に見える | そのテストだけ `runtime_error`（終了コード 0 だが ok の行がない） |
| スタックオーバーフロー（SIGABRT、終了コード 134） | 全体が落ちる。並列実行では原因のテストが分からない。`--test-threads=1` なら、`test X ... ` で結果なしに終わった最後のテストが原因 | そのテストだけ `runtime_error`（signal 6、stderr に `has overflowed its stack`） |
| `std::process::abort()` | スタックオーバーフローと同じ | そのテストだけ `runtime_error`（signal 6） |
| 無限ループ | 期限で kill する。結果行のないテストは未完了（下の節） | そのテストだけ `timeout` |
| `#[ignore]` | `... ignored` の行が出る（隠しテストには付けないこと） | 終了コードは 0 だが ok の行がない |

per-test の解析器の出力の例を示す。

```
progrust_judge::c02_eq: fail  | assertion `left == right` failed /   left: 2 /  right: 3
progrust_judge::c03_msg: fail  | assertion `left == right` failed: ctx 7 /   left: "a\nb" /  right: "a\nc"
progrust_judge::c05_fakeline_fail: fail  | real
progrust_judge::c09_overflow: runtime_error exit=None sig=Some(6) stderr="thread '...c09_overflow' (298848) has overflowed its stack"
progrust_judge::c11_hang: timeout
```

悪意のある学習者（捕捉を回避して偽造し、`process::exit(0)` で終える）には、どちらの方式でも完全には防げない。学習者本人が自分の合否を偽る意味は薄いので、事故（`std::io::stdout()` への直接書き込み、`process::exit`、abort、無限再帰）で壊れないことを目標にするのが現実的である。

### テスト全体の制限時間（10 秒）で止めたとき

- 並列（デフォルト）の single 実行を kill すると、出力は `running 3 tests` と、完了済みのテストの結果行だけになる（`test X ... ` の開始行はない）。`--list progrust_judge::` で得た全名前から、結果行のあるものを引いた残りが「終わっていないテスト」である。実行中のものと、まだ開始していないものは区別できないが、仕様の「終わっていないテストケースを `timeout`」と一致するので問題ない。
- `--test-threads=1` なら、`test X ... ` で止まっている 1 件が実行中のテストである。
- `test X has been running for over 60 seconds` の行は、並列実行のときだけ、60 秒を超えると出る（66 秒のテストで確認）。10 秒の制限では出ない。この行は ` ... ok` で終わらないので、前述の読み方なら結果行と混同しない。
- per-test なら、`timeout` になるのは本当に実行が終わらなかったテストだけ、と区別できる。開始前に期限が来たものは「未開始」として別扱いにもできる。
- `--list` は `名前: test` の行（`progrust_judge::t_ok: test`）と、最後に `N tests, 0 benchmarks` を出す。

### 所要時間

10 個のテストで、single は 0.01 秒、per-test（4 並列）は 0.02 秒だった（3 回とも同じ。load average 16 の状態）。プロセスを分けるコストは無視できる。

## 判定

基準「テストケースごとの結果と panic メッセージ（`assert_eq!` の左辺・右辺を含む）を、確実に取り出せる」は、条件付きで満たした。

- single 方式でも、出力を捕捉する pretty 形式なら、通常の学習者コード（`println!` を含む）では確実に取り出せる。ただし、`process::exit`、abort、スタックオーバーフローがあると全体の結果が失われ、原因のテストを特定できない。
- per-test 方式（`--list` + `--exact` + テストごとのプロセス + 終了コード）なら、これらも含めてテストごとに確実に判定でき、`timeout` も特定できる。
- 並列実行での出力の入り交じりは、出力を捕捉する限り起きない。学習者が捕捉を回避して stdout に直接書いたときだけ、行が壊れる。

## 残った事項

- ユーザーの判断: テストの実行方法を per-test 方式にするか（仕様の「test 方式」4・5 は「1 回起動して出力を読む」）。推奨は per-test。理由は、crash・`process::exit`・無限ループが他のテストの結果に影響せず、`runtime_error` と `timeout` がテストごとに付くので、仕様の `cases[].status` の 4 種類をそのまま実現できるから。代償は、プロセス管理と並列数の実装が増えること。
- 仕様の変更案（「test 方式」5）: テスト名は `{モジュールの経路}::progrust_judge::{関数名}` になりうる（`judge.target` が main.rs・lib.rs 以外のとき。[隠しテストを読み込む #[path] の解決の技術検証](issue-14-hidden-test-path-resolution.md)）。表示名は、`--list` で得た名前の `progrust_judge::` 以降を使う。
- 仕様の変更案（「test 方式」4）: `--nocapture` を付けない。`--test-threads` は指定しなくてよい（per-test なら各プロセスで `--test-threads=1`）。結果は `failures:` より前の `test 名前 ... ok|FAILED` の行から読む。panic メッセージは、`failures:` 以降の最後の `thread '名前' ... panicked at` の次の行から、次の節の見出しまでを取り出す（`note: run with RUST_BACKTRACE` の行は除く）。
- 仕様の変更案（[実行時の制限](../spec/exercise-runner/limits.md)「テスト全体の実行時間」）: `--list` で得た名前のうち、結果が出ていないものを `timeout` にする。
- 別の問題: 仕様どおり 1 回の起動で読む方式では、`std::process::exit(0)` が呼ばれると、終了コード 0 のまま結果行が欠け、全テストが成功したように見えうる。この方式を採るなら、`--list` の件数と結果行の件数が一致することを必ず確かめる。per-test 方式なら起きない。
- 注意: `RUSTC_BOOTSTRAP` は値が何でも `--format json` を有効にする（`0` でも）。使わない前提だが、アプリが子プロセスに環境を引き継ぐ場合、学習者側の環境変数で意図せず有効にならないかに注意する（結果の読み取りには影響しない）。
- 確かめられなかった項目: Windows（当面 macOS に集中する方針のため）。`#[test]` が 1 件もないときの `--list` の挙動（0 件のときは `compile_error` ではなく専用のエラーにしたほうがよいと考えられるが、未検証）。
- Issue は、上の判断が決まるまで開いたままにする。
