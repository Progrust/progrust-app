---
type: Specification
title: "学習記録・分析の全体フロー"
description: "試行ログへの追記から記録ジョブと分析ジョブを経て、学習記録と `analysis.md` を保存するまでの流れを定める。"
---

```
[演習中] アプリが試行ログ(JSON)にイベントを追記
     │  質問 / 実行 / 提出
     ▼
[レッスン終了] ※spec/learning-record-and-analysis/attempt-log.md「試行セッションのライフサイクル」 の終了条件を満たしたとき
     │  記録ジョブをキューに登録
     ▼
[記録ジョブ] claude -p（記録セッション）
     │  入力: レッスン情報 + 試行ログ + 記録テンプレート
     │  出力: 学習記録 .md → 検証・事実フィールド上書き → records/ に保存
     │  分析ジョブをキューに登録
     ▼
[分析ジョブ] claude -p（分析セッション、記録とは別セッション）
        入力: 現在の analysis.md + 今回の記録 + アプリの集計値・基準レベル
        出力: 新しい analysis.md → 検証・アプリ管理フィールド上書き
              → 旧版を history/ に退避して保存
```

- AI 処理はすべてバックグラウンドのジョブとして実行し、画面操作をブロックしない。
- ジョブキューの仕様は [ジョブキュー（jobs.json）](../claude-integration/job-queue.md) を参照。
- 各ファイルの保存場所は [データの保存場所](../architecture/data-locations.md) を参照。
