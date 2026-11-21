---
type: Decision
title: "ADR 0114"
description: "学習者が tests/ の統合テストを書けるようにし、ドキュメントテストも実行して、ch11-05 と ch11-06 のレッスンを残す"
---

- 日付: 2026-11-21
- 関連仕様: [実行・提出時のプロジェクトの組み立て](../spec/exercise-runner/project-assembly.md)、[演習コードの下書き](../spec/lesson-flow/drafts.md)「multi レイアウトのファイル操作」、[演習のファイル形式](../spec/content-format/exercise.md)「判定方式」、[レッスンの演習を作るときの注意](../spec/curriculum/exercise-constraints.md)、[ch11 テスト](../spec/curriculum/chapters/ch11-testing.md)
- 置き換え: なし

## 背景

エディタから受け取るファイルのパスを `src/` 以下に限っていたため、学習者は `tests/` の統合テストを書けなかった。ドキュメントテストは `src/lib.rs` がある演習でしか動かない。統合テストを書けるようにするか、ドキュメントテストを実行するかが未決事項で、決め方によっては統合テスト（ch11-05）とドキュメントテスト（ch11-06）のレッスンを外すことになっていた（Issue #28）。

ユーザーは「私は詳しい技術的な話はわからないので、あなたが推奨するやり方でお願いします。」と示し、判断と理由を Claude に任せた。以下の決定と理由は Claude が書いた。

## 決定

- エディタから受け取るパスに `tests/` 以下の `.rs` を加える（[実行・提出時のプロジェクトの組み立て](../spec/exercise-runner/project-assembly.md)）。`multi` レイアウトでは、`tests/` 以下にもファイルを追加できる（[演習コードの下書き](../spec/lesson-flow/drafts.md)「multi レイアウトのファイル操作」）。
  - 理由: `tests/` に置いたファイルは、cargo が統合テストとして自動で見つけて実行する。アプリに要る変更は、受け取るパスの条件とファイル操作の範囲だけである。パスの検査（`..` と絶対パスを拒否する）はそのまま残すので、`project/` の外に書かれることはない。
- ドキュメントテストも、テスト実行と学習者のテストの判定で実行する。統合テストとドキュメントテストを書かせる演習では、`starter/` に `src/lib.rs` を含める（[演習のファイル形式](../spec/content-format/exercise.md)「判定方式」）。
  - 理由: `src/lib.rs` があれば、`cargo test` がドキュメントテストを自動で実行する。演習の側で `src/lib.rs` を用意すれば済み、アプリに特別な処理は要らない。
- ch11-05 と ch11-06 のレッスンを残す（[ch11 テスト](../spec/curriculum/chapters/ch11-testing.md)）。
  - 理由: 上の 2 つで演習を作れる。統合テストとドキュメントテストは TRPL のテストの章で扱う内容で、カリキュラムは TRPL を網羅の目安にしている（[ADR 0077](0077-curriculum-uses-trpl-as-coverage-guide.md)）。

## 検討した他の案

- 案: 統合テストとドキュメントテストを扱わず、ch11-05 と ch11-06 を外す
  - 採用しなかった理由: 必要な変更が小さく、外す理由にならない。Rust のテストの書き方の一部が抜ける
- 案: 統合テストのファイルを `src/` に置かせ、アプリが `tests/` に移す
  - 採用しなかった理由: 手元の Rust で書くときのファイルの置き場所と違い、学んだことがそのまま使えない

## 影響

- どの演習でも、`multi` レイアウトなら学習者は `tests/` にファイルを作れる。`src/lib.rs` が無い演習で `tests/` に書いても、`progrust_exercise` クレートとしては使えない。
- 演習を作る人は、ch11-05・ch11-06 の `starter/` に `src/lib.rs` を含め、`judge.target` を `src/lib.rs` にする。
