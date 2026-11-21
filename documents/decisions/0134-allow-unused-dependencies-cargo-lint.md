---
type: Decision
title: "ADR 0134"
description: "許可クレートを使う演習のプロジェクトの Cargo.toml で、cargo の「使っていない依存」の警告（cargo::unused_dependencies）を止める"
---

- 日付: 2026-11-22
- 関連仕様: [実行・提出時のプロジェクトの組み立て](../spec/exercise-runner/project-assembly.md)
- 置き換え: なし

## 背景

許可クレートを使う演習の `project/`・`validate/` は、全許可クレートに依存する空のライブラリ `allcrates/`（パッケージ名 `progrust_allowed_crates`）に path 依存する（[ADR 0097](0097-share-crate-builds-via-allcrates-library.md)、[ADR 0132](0132-no-allcrates-dependency-without-crates.md)）。演習のコードは、このライブラリを使わない。

Issue #53 で、cargo 1.100.0 がこの path 依存に「使っていない依存」の警告を出すと起票された。仕分けのときに、cargo 1.100.0・rustc 1.100.0（macOS 27.2.1）で、`allcrates/` を `rand = "=0.9.1"` だけに依存する空のライブラリにし、rand と path 依存の両方を書いた演習のプロジェクトを `--offline` でビルドして、次のとおり確かめた。

| 確かめたこと | 結果 |
|---|---|
| 警告の出方 | `` warning: unused dependency `progrust_allowed_crates` `` と、`` `cargo::unused_dependencies` is set to `warn` by default ``、`help: consider removing the dependency` が、cargo 自身の標準エラー出力に出た。`--message-format=json-diagnostic-rendered-ansi` でも、JSON の標準出力には出なかった。ビルドは成功した |
| 演習のコードが rand を使わないとき | 警告は `progrust_allowed_crates` の 1 件だけで、rand には出なかった |
| `cargo test --offline --no-fail-fast` | 同じ警告が、テストの結果の前に出た |
| `Cargo.toml` に `[lints.cargo]` の `unused_dependencies = "allow"` を足したとき | `cargo build --offline --message-format=json-diagnostic-rendered-ansi`、`cargo test --offline --no-run --message-format=json-diagnostic-rendered-ansi`、`cargo test --offline --no-fail-fast` のどれも、終了コード 0 で、警告が出なかった。rustc が学習者のコードに出す警告（使っていない変数など）は、JSON の `compiler-message` として出たままだった |
| 依存の作り直し | 仕様どおりの `warmup/`（`[lints.cargo]` なし）で `cargo build` と `cargo test --no-run` をした `target/` に、`[lints.cargo]` を足したプロジェクトを build と test の両方でビルドすると、作り直したのは `progrust_exercise` だけで、rand とその依存は作り直さなかった |

この警告が学習者や判定に与える影響は、次のとおりである。

- テスト実行（[コードの実行（run）](../spec/exercise-runner/run.md)「テスト実行」）は、cargo の標準エラー出力をそのまま表示するので、許可クレートを使う演習では、テストのたびに学習者に見える。
- 提出の、学習者のテストの判定（[コードの提出（submit）](../spec/exercise-runner/submit.md)「学習者のテストの判定」）は、結果の行を標準出力から数えるので、判定は変わらない。ただし `runtime_error` の `detail` は標準エラー出力の先頭なので、この警告が先頭に入りうる。
- 初回準備のやり直しが要る失敗の判定（[許可クレートと作業領域](../spec/exercise-runner/crates-and-workspace.md)「初回準備のやり直しが要る失敗」）は、終了コード 101 と決まった語の組で判定するので、当てはまらない。
- 学習者は `Cargo.toml` を編集できない（エディタで扱うのは `src/**`・`tests/**` だけ）。警告の「依存を消す」という助言に、学習者は従えない。

Issue #53 の仕分け（スキル `triage-issues`）で、Claude が決めた。以下の理由は Claude が書いた。

## 決定

- 許可クレートを使う演習の `project/`・`validate/` の `Cargo.toml` に、`[lints.cargo]` の `unused_dependencies = "allow"` を書く（[実行・提出時のプロジェクトの組み立て](../spec/exercise-runner/project-assembly.md)）。
  - 理由: `progrust_allowed_crates` への path 依存は、ビルド結果を共有するためのアプリの内部の仕組みで、学習者のコードとは関係が無い。学習者が直せない警告をテストのたびに見せることになり、提出の `detail` にも入りうる。`Cargo.toml` で止めれば、cargo の出力を加工せずに済み、rustc が学習者のコードに出す警告は残る。確かめた範囲では、依存の作り直しも起きない。
- 許可クレートを使わない演習の `Cargo.toml` には書かない。初回準備の `warmup/` にも書かない。
  - 理由: 許可クレートを使わない演習には依存が無い（ADR 0132）。`warmup/` の出力は学習者に見せない。書く場所を、警告が学習者に見えるプロジェクトだけに絞る。

## 検討した他の案

- 案: 何もしない
  - 採用しなかった理由: 許可クレートを使う演習では、テスト実行のたびに、学習者が直せない警告が出る。演習と関係の無い `progrust_allowed_crates` という名前が見え、学習者を迷わせる
- 案: cargo の標準エラー出力から、この警告の行を取り除いてから表示する
  - 採用しなかった理由: 警告の文言や行の数は cargo の版で変わりうるので、取り除き損ねたり、関係の無い行まで消したりするおそれがある。テスト実行は cargo の出力をそのまま表示すると決めている（[ADR 0112](0112-run-learner-tests-with-test-button.md)）
- 案: 学習者のコードに、`progrust_allowed_crates` を使ったことにする行（`use progrust_allowed_crates as _;` など）を足す
  - 採用しなかった理由: 実行・テストのたびに学習者が書いたファイルを書き換えることになり、`src/main.rs`・`src/lib.rs` のどこに足すかも決める必要がある。その行で cargo が警告を出さなくなるかも確かめていない。`Cargo.toml` の 2 行で済む案のほうが簡単である

## 影響

- 許可クレートを使う演習で、演習の `dependencies` に挙げたクレートを学習者のコードが使わなくても、cargo の「使っていない依存」の警告は出ない（確かめた cargo 1.100.0 では、もともと rand には出なかった）。
- lint の名前（`unused_dependencies`）や `[lints.cargo]` の扱いは、cargo の版で変わりうる。確かめたのは cargo 1.100.0 だけである。いまの rustc の最低バージョンは 1.100.0 なので（[外部ツール（cargo・claude）への依存](../spec/architecture/external-tools.md)「検出」）、どの学習者の環境でも使える。最低バージョンを上げるときや、cargo の版で警告が戻ったときは、確かめ直す。
- 演習を組み立てるときに、許可クレートを使う演習の `Cargo.toml` に 2 行を足す。
