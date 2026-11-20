---
type: Specification
title: "初回起動画面"
description: "初回起動画面を表示する条件（`first_launch_completed`）と、Progrust の紹介、AI 機能についての説明、`cargo` と `claude` の環境チェック、クレートの初回準備の開始を定める。"
---

初回起動時（`first_launch_completed` が `false` または `settings.json` がない）に表示する。

1. Progrust の紹介（動画で学び、演習で解く流れ）
2. AI 機能についての説明: 質疑応答・学習記録・分析・問題生成のために、学習者のコード・質問・学習記録の内容が Claude Code を通じて Anthropic に送信されること
3. 環境チェック: `cargo` と `claude` の検出結果。見つからない場合は導入手順へのリンク（見つからなくても先に進める）
4. 「始める」でトップページへ。クレートの初回準備はこの画面を表示した時点でバックグラウンドで開始する
