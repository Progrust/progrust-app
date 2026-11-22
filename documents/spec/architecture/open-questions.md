---
type: Open Questions
title: "アーキテクチャの未決事項"
description: "アーキテクチャの仕様でまだ決まっていない事項（読み込めないファイルを初期状態で作り直したときの通知の出し方）を記す。"
---

- 読み込めないファイル（`settings.json`・`progress.json`・`jobs.json`）を退避して初期状態で作り直したとき（[データの保存場所](data-locations.md)「書き込みの規則」）の、ユーザーへの通知の出し方と文言。通知を運ぶコマンド・イベントも無い（[コマンドとイベント](commands/index.md)）。
