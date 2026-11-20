---
type: Specification
title: "演習問題生成の流れ"
description: "生成設定画面で「生成」を押してから、generate ジョブによる生成と検証を経て、`ready` または `failed` になるまでの流れを定める。"
---

```
[生成設定画面] 設定を入力して「生成」
     │  generated/{generation_id}/meta.json を status: generating で作成
     │  generate ジョブを登録（spec/claude-integration/job-queue.md）
     ▼
[generate ジョブ] claude -p で問題を生成 → 検証（spec/exercise-generation/output-and-validation.md）
     │  失敗: 修正を依頼して再生成（最大 2 回）
     ▼
 成功: 演習ファイルを保存し status: ready → 通知
 失敗: status: failed → 一覧にエラーを表示
```

生成中も学習者は他の画面で学習を続けられる。
