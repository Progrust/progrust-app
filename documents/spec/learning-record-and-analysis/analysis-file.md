---
type: Specification
title: "分析ファイル（analysis/analysis.md）"
description: "分析ファイル `analysis/analysis.md` の習熟レベル、アプリによる `base_level` の計算、AI による `level` の調整の制約、`trend`・`review_due` などの計算方法、frontmatter と本文を定める。"
---

学習者全体の傾向を表す、常に最新状態に保たれるファイル。テンプレートは [analysis.md.tmpl](templates/analysis.md.tmpl)。

## 習熟レベル

| level | 表示名 | 意味 |
|---|---|---|
| 4 | 得意 | 安定して使え、発展的な問題でも躓かない |
| 3 | 安定 | 概ね躓かずに使える |
| 2 | たまに間違える | ときどき躓く |
| 1 | よく間違える | 繰り返し躓く |
| 0 | 未学習 | 演習で扱っていない |

加えて、演習回数が 1 回のみの要素は `provisional: true` とし、UI では「データ不足」バッジを併記する。データ不足の要素はダッシュボードの得意・苦手ランキングには含めないが、問題生成の苦手分野には含める。

## 基準レベルの計算（アプリ）

**関連記録**: 要素が `elements.primary`・`elements.secondary`・`stumbles[].element` のいずれかに含まれる記録。

**直近ウィンドウ W**: 関連記録を `completed_at` の新しい順に最大 3 件。

各記録のその要素に対する躓き度: 躓きなし = 0、`minor` = 1、`major` = 2（同じ記録に複数ある場合は最大値）。`resolved: false` の躓きは `major` として扱う。

以下を上から順に評価し、最初に該当したものを `base_level` とする。

1. 関連記録が 0 件 → `0`
2. W 内の躓き度 2 が 2 件以上、または W 内の 2 件以上の記録に同じ `pattern` がある → `1`
3. W 内の躓き度 2 が 1 件、または躓き度 1 が 2 件以上 → `2`
4. W が 3 件かつ W 内の躓き度がすべて 0、かつ W 内に「高負荷の記録」が 1 件以上ある → `4`
5. それ以外 → `3`

**高負荷の記録**: `kind: chapter_review`、または `kind: generated` かつ `difficulty >= 4` で、その要素が `primary` に含まれる記録。

## AI によるレベル調整

- AI は `base_level` を参考に `level` を決定する。
- 調整幅は `base_level ± 1` かつ 1〜4 の範囲に限る（`base_level: 0` は調整不可）。
- `level` が `base_level` と異なる場合は `level_reason` に理由を必須で記述する。
- 範囲外の値が出力された場合、アプリは `base_level` を採用する。

## 推移と時間軸（アプリ）

| フィールド | 計算方法 |
|---|---|
| `trend` | 直近 3 件と、その前の 3 件の平均躓き度を比較。差が 0.5 以上改善なら `improving`、0.5 以上悪化なら `declining`、それ以外は `stable`。関連記録が 4 件未満なら `unknown` |
| `last_practiced` | 関連記録の最新 `completed_at` の日付 |
| `last_stumbled` | 躓き度 1 以上の関連記録の最新日付（なければ `null`） |
| `review_due` | `last_practiced` からの経過日数が、level 1〜2 で 7 日以上、level 3 で 21 日以上、level 4 で 45 日以上なら `true` |
| `previous_level` | 更新前の analysis.md における `level`（新規要素は `null`） |
| `regressed` | `previous_level >= 3` かつ新しい `level <= 2` なら `true`（定着度の低下） |

## frontmatter

```yaml
---
schema_version: 1
# --- アプリ管理 ---
updated_at: 2026-10-31T13:33:00+09:00
based_on_records: 42
last_record_id: 2026-10-31_ch04-03_01
activity:
  total_records: 42
  total_active_minutes: 812
  last_4_weeks: [5, 7, 3, 6]      # 週ごとの記録数（古い順）
  last_studied: 2026-10-31
elements:
  ownership.move:
    level: 1                       # AI 管理（「AI によるレベル調整」 の制約あり）
    level_reason: ""               # AI 管理
    base_level: 1
    provisional: false
    previous_level: 2
    regressed: false
    trend: declining
    practice_count: 5
    stumble_count: 3
    last_practiced: 2026-10-31
    last_stumbled: 2026-10-31
    recurring_patterns: [use_after_move]
    review_due: false
# --- AI 管理 ---
focus_recommendation: [ownership.move, borrow.mutable]
recommended_difficulty: 2
---
```

- `elements` には関連記録が 1 件以上ある要素のみを含める（未学習の要素は省略し、level 0 として扱う）。
- `recurring_patterns`: 全関連記録のうち 2 件以上に出現した `pattern`。
- `focus_recommendation`: 次に重点的に演習すべき要素 ID（最大 5 件、優先度順）。
- `recommended_difficulty`: 生成問題の推奨難易度（1〜5）。

## 本文

| セクション | 内容 |
|---|---|
| 総評 | 現在の学習状況を 3〜5 行で要約 |
| 苦手分野 | 「よく間違える」「たまに間違える」に分けて、要素ごとにミスのパターン・推移・根拠となる記録 ID を記述 |
| 得意分野 | 「得意」「安定」の要素と、その根拠 |
| 全体に共通する傾向 | 要素をまたいだ傾向（例: エラーメッセージを読まずに試行錯誤する、借用エラーを `clone()` で回避しがち） |
| 時間軸の分析 | 最近扱っていない分野、復習推奨、学習ペース、定着度が低下した分野 |
| 演習問題生成への推奨 | 重点要素、推奨難易度、出題の切り口の提案 |
| 更新履歴 | 直近 5 回分の更新内容（日時、記録 ID、主な変化） |
