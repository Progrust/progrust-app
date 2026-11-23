---
type: Decision
title: "ADR 0205"
description: "`test` 方式で隠しテストを入れたテスト実行ファイルを、ビルドの JSON の出力のうち、`judge.target` のファイルを根に持つテスト用の成果物から決める"
---

- 日付: 2026-11-23
- 関連仕様: [コードの提出（submit）](../spec/exercise-runner/submit.md)「test 方式」
- 置き換え: なし

## 背景

`test` 方式の判定では、隠しテストを `judge.target` のファイルに組み込み、テスト実行ファイルを `--list` と `--exact` で起動する（[ADR 0101](0101-run-each-hidden-test-in-own-process.md)）。`src/main.rs` と `src/lib.rs` の両方があるときや、`tests/` に統合テストがあるときは、`cargo test --no-run` がテスト実行ファイルを複数作る。どれを起動するかが、未決事項として残っていた。実装計画の M2 で、`test` 方式の判定を作るために決める必要があった。

## 決定

- ビルド（`cargo test --no-run --message-format=json-diagnostic-rendered-ansi`）の標準出力の JSON のうち、`reason` が `compiler-artifact` で、`profile.test` が `true` で、`target.src_path` が `judge.target` のファイルであるものの `executable` を、隠しテストの入ったテスト実行ファイルとする（[コードの提出（submit）](../spec/exercise-runner/submit.md)「test 方式」）。Claude が決めた。
  - 理由: 隠しテストは `judge.target` のファイルの子モジュールなので、そのファイルをクレートの根にするテスト実行ファイルにだけ入る。cargo は、成果物ごとに、根のファイルのパスとテスト用かどうかを JSON で出すので、ファイル名や並び順から推し量らずに決められる。ビルドには既に JSON の出力を使っている（[コードの実行（run）](../spec/exercise-runner/run.md)「ビルドの出力の表示」）。

## 検討した他の案

- 案: `target/debug/deps/` の実行ファイルを名前（`progrust_exercise-{ハッシュ}`）で探す
  - 採用しなかった理由: `src/main.rs` と `src/lib.rs` のテスト実行ファイルは、どちらも同じ名前で始まり、ハッシュだけが違う。古いビルドの実行ファイルも残る
- 案: 作られたテスト実行ファイルをすべて `--list progrust_judge::` で起動し、隠しテストが出たものを使う
  - 採用しなかった理由: 学習者のコードを、判定に要らない分まで起動する。起動の分だけ判定が遅くなる

## 影響

- 判定の手順 4 で、ビルドの JSON から実行ファイルを決める。見つからないときは、アプリの側の失敗として扱う（`run_finished` の `error`）。
