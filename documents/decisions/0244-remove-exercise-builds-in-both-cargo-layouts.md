---
type: Decision
title: "ADR 0244"
description: "初回準備で target-validate/ から学習者の project/ のビルド結果を消すときは、cargo の版で変わるビルド先の配置によらず、パッケージ progrust_exercise の途中の成果物と fingerprint を、cargo 1.100.0 の置き場所（build/progrust_exercise/）と cargo 1.99.1 の置き場所（deps/・.fingerprint/）の両方から消す"
---

- 日付: 2026-11-25
- 関連仕様: [許可クレートと作業領域](../spec/exercise-runner/crates-and-workspace.md)「初回準備」
- 置き換え: なし

## 背景

[ADR 0098](0098-separate-build-dir-for-validation.md) は、初回準備で `target/` を `target-validate/` に複製した後、`target-validate/` から学習者の `project/` のビルド結果を消すとした。消すものは、パッケージ `progrust_exercise` の実行ファイルと、`deps/` と `.fingerprint/` の `progrust_exercise-*` としていた。この置き場所は、cargo 1.99.1 での技術検証（[許可クレートのビルド結果の共有の技術検証](../verification/issue-12-reuse-prepared-crates.md)、[検証と学習者の実行の並行ビルドの技術検証](../verification/issue-13-parallel-build-lock.md)）のときのものである。

Issue #234 で初回準備の手順を作ったとき、cargo 1.100.0（macOS、`~/.cargo/config.toml` なし、環境変数 `CARGO_*` の設定なし）で、`CARGO_TARGET_DIR` を指定して `warmup/` をビルドすると、ビルド先は次の配置になった。

- 実行ファイルは、cargo 1.99.1 と同じく `debug/progrust_warmup` にあった。
- 途中の成果物と fingerprint は、`debug/build/{パッケージ名}/{ハッシュ}/` の `out/` と `fingerprint/` にあった。`debug/deps/` と `debug/.fingerprint/` は作られなかった。

学習者の環境の rustc の最低バージョンは 1.100.0 である（[対応環境](../spec/non-functional/environments.md)）。仕様の文のとおりに `deps/` と `.fingerprint/` だけを探すと、cargo 1.100.0 では途中の成果物と fingerprint を消せず、ADR 0098 が消すとした目的（学習者の実行ファイルを検証で使う誤りを、確かめなくても起こさない）が成り立たない。

## 決定

以下は、Claude の技術的な判断である。

- `target-validate/` から消すものを、パッケージ `progrust_exercise` の実行ファイルと、そのビルドの途中の成果物と fingerprint とし、cargo 1.100.0 の置き場所（`build/progrust_exercise/`）と cargo 1.99.1 の置き場所（`deps/` と `.fingerprint/` の `progrust_exercise-*`）の両方から消す（[許可クレートと作業領域](../spec/exercise-runner/crates-and-workspace.md)「初回準備」）。ADR 0098 の「決定」の「学習者の `project/` のビルド結果（パッケージ `progrust_exercise` の実行ファイル・`deps/`・`.fingerprint/` の項目）を消す」を、この点で変える。
  - 理由: 置き場所は cargo の版で変わる。両方を消せば、どちらの配置で作られたビルド先でも、学習者のビルド結果が残らない。無い置き場所を消す処理は何もしないので、害がない。
- 自動テストでは、手で置いたファイルではなく、実物の cargo で `progrust_exercise` のプロジェクトを `target/` にビルドしてから初回準備を動かし、`target-validate/` に `progrust_exercise` の名前のものが残らないことを確かめる。
  - 理由: cargo の版で配置がまた変わったときに、テストが失敗して気づける。手で置いたファイルでは、今回の食い違いを見逃した。

## 検討した他の案

- 案: cargo 1.100.0 の置き場所（`build/progrust_exercise/`）だけを消す
  - 採用しなかった理由: 対応する cargo の版の範囲で、cargo 1.99.1 の配置にならないことを確かめていない。両方を消しても手間はほぼ同じである
- 案: 複製の前に、`target/` から学習者のビルド結果を消す
  - 採用しなかった理由: `target/` は学習者の実行・提出のビルド先なので、次の実行で学習者のコードを作り直すことになる

## 影響

- cargo の版でビルド先の配置がまた変わったら、消す置き場所を見直す。手順の 4 のテストは macOS のときだけ実行する（[テストの方針](../spec/non-functional/testing.md)「CI」）ので、気づくのは作者の Mac でテストを実行したときである。
