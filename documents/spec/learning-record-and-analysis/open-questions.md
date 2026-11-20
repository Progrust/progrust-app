---
type: Open Questions
title: "学習記録・分析の未決事項"
description: "学習記録・分析の仕様でまだ決まっていない事項（`active_minutes` で動画の視聴に使った時間をどう扱うか）を記す。"
---

- `active_minutes`（[学習記録ファイル（records/*.md）](record-file.md)「frontmatter」）で、動画の視聴に使った時間をどう扱うか。動画の視聴中は試行ログにイベントが記録されない。この時間を、イベント間隔が 10 分を超える区間を除く決まりの中でどう扱うかは決めていない。
