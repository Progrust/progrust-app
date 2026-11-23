---
type: Open Questions
title: "コード実行・判定の未決事項"
description: "コード実行・判定の仕様でまだ決まっていない事項（アプリの起動の後に `cargo` が見つかったときに初回準備を始めるか）を記す。"
---

- アプリの起動時に `cargo` が見つからず、後から「再検出」で見つかったときに、初回準備を始めるかどうか。いまの定めでは、初回準備の状態（[許可クレートと作業領域](crates-and-workspace.md)「初回準備の状態」）が `not_started` のまま変わらず、再試行（[環境のコマンドとイベント](../architecture/commands/environment.md)）も `failed` のときしか受け付けない。
