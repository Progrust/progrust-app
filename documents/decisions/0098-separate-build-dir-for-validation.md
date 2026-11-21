---
type: Decision
title: "ADR 0098"
description: "生成問題の検証のビルド先を学習者のものと分け、許可クレートのビルド結果は初回準備の後に APFS のクローンで複製する"
---

- 日付: 2026-11-21
- 関連仕様: [許可クレートと作業領域](../spec/exercise-runner/crates-and-workspace.md)「作業領域」、[許可クレートと作業領域](../spec/exercise-runner/crates-and-workspace.md)「初回準備」、[実行・提出時のプロジェクトの組み立て](../spec/exercise-runner/project-assembly.md)「生成問題の検証（validate/）」、[コード実行・判定の概要](../spec/exercise-runner/overview.md)、[演習問題生成の出力と検証](../spec/exercise-generation/output-and-validation.md)「検証」
- 置き換え: なし

## 背景

生成問題の検証は、学習者の `project/` とは別の `validate/` で行い、学習者の実行と並行して行えるとしていた。ビルド先は、許可クレートを使うかどうかで `target/` と `target-std/` に分けていた（[ADR 0012](0012-separate-build-dir-without-crates.md)）が、`validate/` も同じビルド先を使っていた。

Issue #13 の技術検証で、同じビルド先を使うと次のことが起きた（[検証と学習者の実行の並行ビルドの技術検証](../verification/issue-13-parallel-build-lock.md)）。

- 後から始めた cargo は、ビルド先のロックで待たされる。重い検証のビルドと重なると、学習者の実行は約 5.4 秒待った。
- `project/` と `validate/` はパッケージ名が同じ `progrust_exercise` なので、実行ファイルの置き場所が同じになる。後からビルドした方が置き換え、cargo が相手のビルド結果を自分のものと判定して使うこともある。そのため、検証が学習者のコードを実行したり、学習者の提出が検証のコードで判定されたりする（Issue #32）。依存の組み合わせによらず、`target-std/` でも起きる。
- `validate/` のパッケージ名を変えるか、ビルド先を分けると、この誤りは起きなくなる。ロックの待ちは、ビルド先を分けたときだけ無くなる。
- 初回準備の後に `target/` を APFS のクローンで複製すると、0.13 秒で済み、追加の容量はほぼ 0 で、複製した先でも依存を作り直さずに使えた。依存の作り直しが無くなるのは、全許可クレートに依存する空のライブラリへの path 依存（[ADR 0097](0097-share-crate-builds-via-allcrates-library.md)）と組み合わせたときである。

検証の後、Claude は、ビルド先を分けて APFS のクローンで複製することを推奨した。ユーザーは「私は細かい技術的な話はわからないので、あなたの推奨通りで進めて大丈夫です」と示し、判断と理由を Claude に任せた。以下の理由は Claude が書いた。

## 決定

- `validate/` のビルド先を、許可クレートを使う問題は `target-validate/`、使わない問題は `target-std-validate/` にし、学習者の `project/` のビルド先と分ける（[許可クレートと作業領域](../spec/exercise-runner/crates-and-workspace.md)「作業領域」、[実行・提出時のプロジェクトの組み立て](../spec/exercise-runner/project-assembly.md)「生成問題の検証（validate/）」）。
  - 理由: 誤った実行ファイルを使う問題と、ロックの待ちの両方が無くなる。仕様の「検証は学習者の実行と並行して行える」が成り立つ。
- `target-validate/` は、初回準備の最後に `target/` を APFS のクローンで複製して作る（[許可クレートと作業領域](../spec/exercise-runner/crates-and-workspace.md)「初回準備」）。
  - 理由: 許可クレートを検証のためにもう一度ビルドすると、初回準備の時間が 2 倍になる。クローンなら一瞬で、容量もほぼ増えない。対応環境は作者の Mac（APFS）だけである（[ADR 0090](0090-support-only-authors-mac-until-completion.md)）。
- 複製した後、`target-validate/` から、学習者の `project/` のビルド結果（パッケージ `progrust_exercise` の実行ファイル・`deps/`・`.fingerprint/` の項目）を消す。
  - 理由: 初回準備をやり直すとき、`target/` には学習者のビルド結果が残っている。パッケージ名が同じなので、cargo はこれを `validate/` のビルド結果と区別しない。検証のビルドの前に `validate/` のソースを書き直すので、mtime の比較で作り直されると考えられるが、確かめていない。消しておけば、学習者の実行ファイルを検証で使う誤りは、確かめなくても起きない。
- `validate/` のパッケージ名は、`project/` と同じ `progrust_exercise` のままにする。
  - 理由: ビルド先を分ければ、名前が同じでも誤りは起きない。ライブラリクレートの演習では、`src/main.rs` から `use progrust_exercise::...` でライブラリを使う（[レッスンの演習を作るときの注意](../spec/curriculum/exercise-constraints.md)）。生成問題も同じ書き方をするので、名前を変えると、学習者の環境では通るコードが検証では通らなくなる。

## 検討した他の案

- 案: `validate/` のパッケージ名だけを変え、ビルド先は共有する（検証の回避策）
  - 採用しなかった理由: ロックの待ちが残り、重い検証と重なると学習者の実行が数秒待つ。ライブラリをパッケージ名で使う生成問題が、検証で通らなくなる
- 案: 検証専用に、許可クレートをもう一度ビルドする
  - 採用しなかった理由: 初回準備の時間とディスク使用量が 2 倍になる
- 案: ビルド先は分けずに、検証と学習者の実行を直列にする
  - 採用しなかった理由: 検証のビルドの時間だけ学習者が待たされる
- 案: ハードリンクで複製する
  - 採用しなかった理由: 検証で、元のビルド先のビルドの後に複製先の中身が変わったことがあった。書き込み時にコピーするクローンの方が安全である
- 案: cargo の細かいロック（`-Z fine-grain-locking`）を使う
  - 採用しなかった理由: nightly でしか使えない

## 影響

- 作業領域に `target-validate/`・`target-std-validate/` が増える。クローンなので、`target-validate/` の容量はビルドが進むにつれて増える分だけである。
- 初回準備をやり直すたびに、`target-validate/` を作り直す。初回準備の間は、許可クレートを使う生成問題の検証を始めない。
- APFS のクローンは macOS の APFS でしか使えない。対応環境を広げるとき（[ADR 0090](0090-support-only-authors-mac-until-completion.md)）は、複製の方法を見直す。
- ADR 0012 は、`project/` の中でのビルド先の分け方として、そのまま有効である。
