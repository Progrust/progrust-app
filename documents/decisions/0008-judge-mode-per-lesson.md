---
type: Decision
title: "ADR 0008"
description: "演習の判定方式を、標準出力の比較と隠しテストからレッスンごとに選ぶ"
---

- 日付: 2026-10-31
- 関連仕様: [演習のファイル形式](../spec/content-format/exercise.md)「判定方式」、[コードの提出（submit）](../spec/exercise-runner/submit.md)、[レッスン定義の形式（lesson.json）](../spec/content-format/lesson-json.md)、[演習問題生成の出力と検証](../spec/exercise-generation/output-and-validation.md)
- 置き換え: なし

## 背景

序盤のレッスンは `println!` の出力を確認する問題が中心になる一方、関数や型を実装させる問題では、関数単位の検証や境界ケースの確認が必要になる。

## 決定

標準出力の比較（`stdout` 方式）と、隠しテストを `cargo test` で実行する方式（`test` 方式）の両方を用意し、レッスンごとに指定する（`content-format.md`「判定方式」）。

## 検討した他の案

- 案: 隠しテストのみ
  - 採用しなかった理由: 序盤の問題も関数を作らせる形にする必要があり、まだ関数を学んでいない段階の問題を作れない
- 案: 標準出力の比較のみ
  - 採用しなかった理由: 関数単位の検証や、出力に現れない境界ケースの確認が難しい

## 影響

- 2 つの判定処理を実装・保守する必要がある。
- 問題生成でも AI がどちらかの方式を選んで出力する。
