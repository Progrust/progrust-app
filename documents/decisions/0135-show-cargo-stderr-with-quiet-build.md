---
type: Decision
title: "ADR 0135"
description: "実行・テスト実行・提出のビルドの cargo に --quiet を付け、cargo 自身の標準エラー出力を、コンパイラの診断と合わせてそのまま表示する"
---

- 日付: 2026-11-22
- 関連仕様: [コードの実行（run）](../spec/exercise-runner/run.md)、[コードの提出（submit）](../spec/exercise-runner/submit.md)
- 置き換え: なし

## 背景

実行・テスト実行・提出のビルドは、`--message-format=json-diagnostic-rendered-ansi` を付けた cargo で行う。仕様は「コンパイラの警告も表示する。cargo 自身の進捗メッセージ（`Compiling ...` など）は表示しない」とだけ書き、進捗メッセージでもコンパイラの診断でもない、cargo 自身が標準エラー出力に出す警告やエラーを表示するかどうかを決めていなかった（Issue #54）。

Issue #54 の仕分け（スキル `triage-issues`）のときに、cargo 1.100.0・rustc 1.100.0（macOS）で、`cargo build --offline --message-format=json-diagnostic-rendered-ansi` と `cargo test --offline --no-run --message-format=json-diagnostic-rendered-ansi` を、`--quiet`（`-q`）を付けた場合と付けない場合とで試し、次のとおり確かめた。どの場合も、build と test の結果は同じだった（`error: could not compile ...` の行の `(bin "progrust_exercise")` が、test では `(bin "progrust_exercise" test)` になる違いだけがあった）。

| 確かめたこと | `--quiet` なし | `--quiet` あり |
|---|---|---|
| 学習者のコードのコンパイルエラー | JSON の `compiler-message` に診断が出た。標準エラー出力には `Compiling ...` と `` error: could not compile `progrust_exercise` (bin "progrust_exercise") due to 1 previous error `` が出た | `compiler-message` は同じ。標準エラー出力は `error: could not compile ...` の行だけになった |
| ビルドの成功（コンパイラの警告あり） | 警告は `compiler-message` に出た。標準エラー出力には `Compiling ...`・`Finished ...` が出た | `compiler-message` は同じ。標準エラー出力には何も出なかった |
| `Cargo.toml` に知らないキーがある | `warning: Cargo.toml: unused manifest key: ...` が標準エラー出力に出た | 出なかった |
| cargo の lint の警告（`cargo::unused_dependencies`） | 標準エラー出力に出た | 同じく出た |
| 学習者の cargo の設定ファイル（`CARGO_HOME` の `config.toml`）が TOML として壊れている | 終了コード 101。`compiler-message` は出ず、`error: could not load Cargo configuration` と、壊れた行を示す文が、標準エラー出力にだけ出た | 同じエラーが出た |
| [許可クレートと作業領域](../spec/exercise-runner/crates-and-workspace.md)「初回準備のやり直しが要る失敗」の 3 つの状態（`.crate` の欠落、レジストリの索引の欠落、ビルド済みの依存の破損） | 終了コード 101。`.crate` と索引の欠落の文は標準エラー出力に、E0786 の診断は JSON の `compiler-message` に出た | 終了コードも、判定に使う文も、文が出る場所も同じだった |

`compiler-message` が 1 つも出ないビルドの失敗（cargo の設定ファイルが壊れているときなど）があり、初回準備のやり直しが要る失敗に当てはまらないものもある。このとき cargo 自身の標準エラー出力を表示しないと、学習者には `compile_error` だけが見え、コンソールに何も出ない。

Claude が決めた。以下の理由は Claude が書いた。

## 決定

- 実行・テスト実行・提出のビルドの cargo に `--quiet` を付ける（[コードの実行（run）](../spec/exercise-runner/run.md)「手順」「テスト実行」、[コードの提出（submit）](../spec/exercise-runner/submit.md)）。
  - 理由: cargo 自身が進捗メッセージを出さなくなるので、アプリが cargo の出力の行を文言で見分けて取り除かずに済む。文言で行を取り除く方法は、cargo の版で文言が変わると外れるおそれがある（[ADR 0134](0134-allow-unused-dependencies-cargo-lint.md) で採用しなかった理由と同じ）。初回準備のやり直しの判定に使う文は、`--quiet` を付けても変わらないことを確かめた。
- ビルドの出力として、コンパイラの診断と、cargo 自身の標準エラー出力を、そのまま表示する。ビルドに成功したときも失敗したときも表示する。初回準備のやり直しが要る失敗（`crates_unavailable`）のときは、どちらも表示しない（[コードの実行（run）](../spec/exercise-runner/run.md)「ビルドの出力の表示」）。
  - 理由: `compiler-message` が出ないビルドの失敗でも、学習者に失敗の文が見え、自分の cargo の設定などを直せる。`--quiet` を付けた後に残るのは cargo のエラーと lint の警告だけで、学習者が手元の端末で `cargo build --quiet` をしたときと同じ表示になる。

## 検討した他の案

- 案: `--quiet` を付けず、cargo 自身の標準エラー出力から、進捗メッセージの行だけを文言で見分けて取り除いて表示する
  - 採用しなかった理由: 進捗メッセージの文言や形は cargo の版で変わりうる。`--quiet` なら、何を出さないかを cargo が決める
- 案: cargo 自身の標準エラー出力を表示せず、コンパイラの診断だけを表示する
  - 採用しなかった理由: cargo の設定ファイルが壊れているときなど、`compiler-message` が出ないビルドの失敗で、コンソールに何も出ない
- 案: cargo 自身のエラーだけを表示し、cargo の lint の警告は表示しない
  - 採用しなかった理由: `--quiet` を付けても lint の警告は出るので、エラーと警告を文言で見分ける必要がある。アプリが組み立てる `Cargo.toml` から学習者に関係の無い警告が出るときは、ADR 0134 のように `Cargo.toml` の側で止める

## 影響

- 学習者のコードのコンパイルエラーのたびに、コンパイラの診断に加えて `error: could not compile ...` の行が表示される。手元の端末での `cargo build` と同じである。
- `Cargo.toml` の読み込みについての警告は、`--quiet` で出なくなる。アプリが組み立てる `Cargo.toml` の誤りは、警告からは気づけない。
- 標準出力（JSON）と標準エラー出力は別々のパイプなので、コンパイラの診断と cargo 自身の出力の前後関係は、cargo が書いた順と違って表示されることがある（[コードの実行（run）](../spec/exercise-runner/run.md)「出力の送り方」）。
- テスト実行の 2 回目の cargo（`cargo test --offline --no-fail-fast`）には `--quiet` を付けない。出力をそのまま表示する（[ADR 0112](0112-run-learner-tests-with-test-button.md)）。
- `--quiet` が出さないものは、cargo の版で変わりうる。確かめたのは cargo 1.100.0 だけである。
