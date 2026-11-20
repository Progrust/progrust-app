---
type: Decision
title: "ADR 0074"
description: "動画の視聴を試行ログに記録しない"
---

- 日付: 2026-11-19
- 関連仕様: [試行ログ（attempts/*.json）](../spec/learning-record-and-analysis/attempt-log.md)、[試行ログ（attempts/*.json）](../spec/learning-record-and-analysis/attempt-log.md)「スキーマ」
- 置き換え: なし

## 背景

スライドでは、スライドから離れたときに、スライドの番号と表示秒数を試行ログに記録していた（`slide_view`）。提案では、これを動画の視聴のイベント（`video_view`）に替えるとしていた。

## 決定

- 動画の視聴は試行ログに記録しない。`slide_view` に当たるイベントをなくす（`learning-record-and-analysis.md`「スキーマ」）。
- 試行ログそのものと、質問・実行・提出のイベントは残す。
- 動画では再生速度もあり、聞き逃しただけということもありうる。そうした情報を識別して使うのは大変そうで、たいしてメリットもなさそうである。

## 検討した他の案

- 案: `slide_view` を `video_view` に替えて記録する
  - 採用しなかった理由: 再生速度や聞き逃しを識別して使うのは大変そうで、たいしてメリットもなさそう

## 影響

- 学習記録と分析に、動画をどれだけ見たかを使えない。
- 動画の視聴中は試行ログにイベントが記録されないので、`active_minutes` で動画の視聴に使った時間をどう扱うかを決める必要がある（`learning-record-and-analysis.md`「未決事項」）。
- 質問のイベントには、質問したときに再生していた文が残る（`learning-record-and-analysis.md`「スキーマ」）。
- アプリは未実装なので、試行ログの `schema_version` は 1 のままにする。
