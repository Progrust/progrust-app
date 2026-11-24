---
type: Specification
title: "初回起動画面"
description: "初回起動画面を表示する条件（`first_launch_completed`）と、Progrust の紹介、AI 機能についての説明、`cargo` と `claude` の環境チェックと再検出、外部クレートの初回準備の状態と再試行、「始める」の動作を定める。"
---

初回起動時（`first_launch_completed` が `false` または `settings.json` がない）に表示する。

1. Progrust の紹介（動画で学び、演習で解く流れ）
2. AI 機能についての説明: 質疑応答・学習記録・分析・問題生成のために、学習者のコード・質問・学習記録の内容が Claude Code を通じて Anthropic に送信されること
3. 環境チェック: `cargo` と `claude` の検出結果。見つからない場合は導入手順へのリンク（[外部ツール（cargo・claude）への依存](../architecture/external-tools.md)「見つからない場合」。見つからなくても先に進める）。`rustc` のバージョンが足りない場合は、今のバージョンと必要なバージョンと、`rustup update stable` で更新する手順を示す。「再検出」ボタンで、検出し直す（[環境のコマンドとイベント](../architecture/commands/environment.md)「`redetect_tools`」）
4. 外部クレートの初回準備の状態（準備中・準備できた・失敗）。失敗のときは再試行ボタンを表示する（[許可クレートと作業領域](../exercise-runner/crates-and-workspace.md)「初回準備」）
5. 「始める」で `first_launch_completed` を `true` にして保存し、トップページへ。初回準備の終わりは待たない。初回準備はアプリの起動の時点でバックグラウンドで開始する

- 初回起動画面のヘッダーには、ロゴだけを表示する。メインメニュー・レッスン検索・ジョブの表示は出さない。
