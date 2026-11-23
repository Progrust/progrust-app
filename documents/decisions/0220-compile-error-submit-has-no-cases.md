---
type: Decision
title: "ADR 0220"
description: "`compile_error` の提出の判定結果は、`passed` と `total` を 0、`cases[]` を空にする"
---

- 日付: 2026-11-24
- 関連仕様: [コードの提出（submit）](../spec/exercise-runner/submit.md)「判定結果」
- 置き換え: なし

## 背景

[コードの提出（submit）](../spec/exercise-runner/submit.md)「判定結果」は、`passed`・`total` を通過したテストケースの数と全テストケースの数と定め、判定をしなかった提出（`crates_unavailable`）と途中で止めた提出（`stopped`）では 0 と空にすると定めていた。ビルドに失敗した提出（`compile_error`）で、これらを何にするかは書かれていなかった。判定結果は、イベント `run_finished` の `submit` と、試行ログの `submit` イベントに入る。実装計画の M2 で、提出のコマンド `submit_code` を作るときに決める必要があった。

この判断は、Claude が技術的な判断として決めた。

## 決定

- `compile_error` の提出の判定結果は、`passed` と `total` を 0、`cases[]` を空にする（[コードの提出（submit）](../spec/exercise-runner/submit.md)「判定結果」）。試行ログの `submit` イベントの `passed`・`total` も 0、`failed_cases` も空になる。
  - 理由: ビルドに失敗した提出は、テストケースを 1 件も実行していないので、テストケースの結果が無い。`test` 方式では、全テストケースの数は、ビルドしたテスト実行ファイルから隠しテストの名前を得るまで決まらないので、ビルドに失敗したときは数えられない。判定をしなかった提出・途中で止めた提出と同じ形にすれば、どちらの判定方式でも同じに扱える。

## 検討した他の案

- 案: `stdout` 方式では `total` を `cases.json` のテストケースの数にし、`test` 方式では 0 にする
  - 採用しなかった理由: 同じ `compile_error` なのに、判定方式で `total` の意味が変わる。画面は、ビルドに失敗したときは、テストを実行していないことを示すだけで（[演習画面](../spec/screens/exercise-screen.md)「判定結果」）、テストケースの数を使わない
- 案: `cases[]` に、すべてのテストケースを、実行していないことを示す状態で並べる
  - 採用しなかった理由: テストケースの結果の状態（`pass`・`fail`・`timeout`・`runtime_error`）に、実行していないことを表す値を足す必要がある。`test` 方式では、並べるテストケースの名前も得られない

## 影響

- 試行ログから学習記録を作るときは、`compile_error` の `submit` イベントの `passed`・`total` が 0 であることを、テストが 1 件も通らなかったこととして読まない必要がある。`status` で見分けられる。
