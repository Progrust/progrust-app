---
type: Decision
title: "ADR 0211"
description: "演習の無いレッスン（動画のみのレッスン）に `save_draft`・`reset_draft` を呼んだときは、下書きを書かずに `invalid_argument` を返す"
---

- 日付: 2026-11-23
- 関連仕様: [進捗のコマンド](../spec/architecture/commands/progress.md)
- 置き換え: なし

## 背景

[進捗のコマンド](../spec/architecture/commands/progress.md) は、`open_lesson` の `draft` を演習の無いレッスンでは `null` にすると定めていたが、演習の無いレッスンに `save_draft`・`reset_draft` を呼んだときの動作は決めていなかった。`reset_draft` は教材の初期コード（`starter/`）で下書きを上書きするが、演習の無いレッスンには初期コードが無い。Issue #154 で、下書きのコマンドを作るときに決める必要があった。

## 決定

Claude が決めた。

- 演習の無いレッスンに `save_draft`・`reset_draft` を呼んだときは、下書きを書かずに `invalid_argument` を返す（[進捗のコマンド](../spec/architecture/commands/progress.md)「save_draft」「reset_draft」）。
  - 理由: 演習の無いレッスンには、演習画面もエディタも無いので、画面がこのコマンドを呼ぶことは無い。呼ばれたときは、引数の `lesson_id` がそのコマンドの条件を満たしていない。これは、[コマンドとイベントの共通の決まり](../spec/architecture/commands/conventions.md)「エラー」の `invalid_argument` に当たる。動画の無いレッスンに `save_video_position` を呼んだときも、`invalid_argument` を返す。

## 検討した他の案

- 案: 演習の無いレッスンでも、`save_draft` は受け取ったファイルを保存する
  - 採用しなかった理由: `open_lesson` が返さない下書きのファイルが、学習者データに残る
- 案: `reset_draft` で、下書きを書かずに空の `files` の下書きを返す
  - 採用しなかった理由: 戻り値が「初期コードで上書きした後の下書き」ではなくなり、呼んだ側の誤りに気づけない
- 案: `not_found` を返す
  - 採用しなかった理由: レッスンはあるので、「指定したレッスンなどが無い」という `not_found` の意味と合わない

## 影響

- 生成問題（M6）は、どれも演習を持つので、この判断の対象にならない。
