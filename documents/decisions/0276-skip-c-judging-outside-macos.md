---
type: Decision
title: "ADR 0276"
description: "CI（ubuntu-latest）では C の演習の判定のテストを実行せず、教材チェックツールも macOS でないときは C のレッスンの模範解答・初期コードを判定せずに警告を出し、C の判定は作者の Mac のコミット前の確認と実装のループの機械のチェックで確かめる"
---

- 日付: 2026-11-28
- 関連仕様: [テストの方針](../spec/non-functional/testing.md)「CI」、[教材チェックツール](../spec/content-guidelines/check-tool.md)
- 置き換え: なし

## 背景

C の演習の判定は、macOS の `clang`、AddressSanitizer の報告に行番号を付ける `/usr/bin/atos`、メモリリークの検査のライブラリを差し込む `DYLD_INSERT_LIBRARIES` と dyld の interpose を使う（[C のコードの提出（submit）](../spec/exercise-runner/c-submit.md)）。CI の実行環境（GitHub Actions の ubuntu-latest）には、どれも無い（[確保と解放を数えるライブラリでメモリリークを検査できるかの技術検証](../verification/issue-294-leak-check-library.md)「CI」）。

テストの方針は、実物の `clang` とメモリリークの検査のライブラリを使う C の演習のテストを、macOS に固有の動作を確かめるテストとして扱うとしていた。CI で、C の演習の判定のテストと、教材チェックツールの C のレッスンの判定（模範解答・初期コード）をどう動かすかは、未決事項に残していた（非機能要件と教材作成ガイドラインの 2 か所）。

実装のループは作者の Mac で回り、マージの前に、機械のチェック（`npm run check` と `npm run content:check` を含む）を実行する（[実装のループ](../conventions/implementation-loop.md)「回す場所と始め方」「機械のチェック」）。

実装計画 M5（C 言語編）のタスクを作る前に、Claude が決めた。

## 決定

以下は、どれも Claude が決めた。

- CI では、C の演習の判定のテストを実行しない。これらのテストは、macOS のときだけ実行する（[テストの方針](../spec/non-functional/testing.md)「CI」）。
  - 理由: [ADR 0149](0149-testing-policy.md) の「macOS に固有の動作を確かめるテストは、CI では実行しない」の決まりに当たる。C の判定は、作者の Mac のコミット前の確認と、実装のループの機械のチェックで、マージの前に確かめられる。
- 教材チェックツールは、macOS でないときは、C のレッスンの「模範解答」「初期コード」を判定せず、判定しなかったレッスンを警告の「C の判定」に出す（[教材チェックツール](../spec/content-guidelines/check-tool.md)）。macOS で `clang` が見つからないときは、その検査のエラーにする。
  - 理由: 黙って飛ばすと、CI の結果だけを見て、C の模範解答も確かめたと誤解しやすい。警告なら、CI を失敗にせずに、判定していないことが結果に残る。作者の Mac では `clang` がある前提なので（[ADR 0271](0271-do-not-detect-clang-without-command-line-tools.md)）、macOS で見つからないのは、確かめられていない状態としてエラーにする。

## 検討した他の案

- 案: CI に macOS の実行環境（GitHub Actions の macOS の runner）のジョブを足し、C の判定のテストと教材チェックツールの C の判定を動かす
  - 採用しなかった理由: [ADR 0149](0149-testing-policy.md) が macOS の runner を使わなかった理由のうち、料金は、リポジトリを公開したので（[ADR 0248](0248-publish-repository-on-github.md)）当てはまらなくなった。それでも、CI の役目は、コミット前の確認の実行し忘れと、ロックファイルのとおりの依存で通ることの確かめにとどめている（同 ADR）。C の判定はマージの前に作者の Mac で確かめられ、macOS のジョブを足すと、Tauri のバックエンドのビルドを含む CI の時間が延び、ワークフロー（保護するパス）を変える作業が要る
- 案: CI では、メモリリークの検査だけを省いて、C の判定を動かす
  - 採用しなかった理由: ubuntu-latest の `clang` と AddressSanitizer は、対応環境のもの（Apple clang と `atos`）と報告の書式や行番号の付け方が違いうる。通っても、対応環境での判定の確かめにならない（[テストの方針](../spec/non-functional/testing.md)「CI」）
- 案: 教材チェックツールが、macOS でないときに、黙って C の判定を飛ばす
  - 採用しなかった理由: 上の「決定」の理由のとおり

## 影響

- CI が通っても、C の演習の判定と、C のレッスンの模範解答・初期コードは確かめられていない。作者の Mac の確認とループの機械のチェックが、それを確かめる唯一の場所になる。
- CI の教材チェックツールの結果には、C のレッスンがある限り、警告の「C の判定」が出る。
