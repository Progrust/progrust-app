---
type: Specification
title: "学習記録ファイル（records/*.md）"
description: "学習記録ファイル `records/*.md` のファイル名（`record_id`）、frontmatter のアプリ管理フィールドと AI 管理フィールド（`outcome`、`stumbles` など）、本文のセクションを定める。"
---

1 セッションにつき 1 ファイル。テンプレートは [record.md.tmpl](templates/record.md.tmpl)。

## ファイル名

`{record_id}.md`、`record_id` = `{YYYY-MM-DD}_{lesson_id}_{連番2桁}`
（同日に同じレッスンを複数回行った場合は連番を増やす）

## frontmatter

```yaml
---
schema_version: 1
record_id: 2026-10-31_ch04-04_01
session_id: 20261031T130200_ch04-04
lesson_id: ch04-04
kind: lesson
title: 所有権のムーブ
difficulty: null
elements:
  primary: [ownership.move]
  secondary: [fn.basic, string.basic]
# --- 事実（アプリ管理） ---
started_at: 2026-10-31T13:02:00+09:00
completed_at: 2026-10-31T13:25:00+09:00
active_minutes: 18
run_count: 6
submit_count: 2
first_submit_passed: false
error_codes: [E0382, E0499]
questions_asked: 1
# --- 解釈（AI 管理） ---
outcome: struggled_resolved
stumbles:
  - element: ownership.move
    severity: major
    pattern: use_after_move
    resolved: true
smooth_elements: [fn.basic]
---
```

### アプリ管理フィールド

AI の出力値に関わらず、アプリが試行ログから計算した値で上書きする。

| フィールド | 計算方法 |
|---|---|
| `schema_version` 〜 `elements` | セッション・レッスン定義から設定 |
| `completed_at` | `passed_at` |
| `active_minutes` | `started_at`〜`passed_at` のうち、イベント間隔が 10 分を超える区間を除いた合計 |
| `run_count` / `submit_count` | 該当イベントの件数 |
| `first_submit_passed` | 最初の `submit` が `passed` か |
| `error_codes` | 全イベントのエラーコード（重複除去、出現順） |
| `questions_asked` | `chat` イベントの件数 |

### AI 管理フィールド

| フィールド | 型 | 説明 |
|---|---|---|
| `outcome` | enum | 総合評価（下表） |
| `stumbles` | array | 躓きのリスト。躓きがなければ `[]` |
| `stumbles[].element` | string | 要素 ID（要素マスタに存在するもの） |
| `stumbles[].severity` | enum | `minor`: 1〜2 回の試行で自力解決 / `major`: 3 回以上の試行、質問が必要だった、または誤った方法で回避した |
| `stumbles[].pattern` | string | 躓きの種類を表す snake_case ラベル。再発の判定に使う |
| `stumbles[].resolved` | bool | 最終コードで正しく解決できているか（`clone()` 等で回避しただけなら `false`） |
| `smooth_elements` | string[] | 躓きなく使えていた要素 ID |

`outcome`:

| 値 | 意味 |
|---|---|
| `smooth` | 躓きなし、または軽微な typo 程度 |
| `minor_struggle` | `minor` の躓きのみ |
| `struggled_resolved` | `major` の躓きがあったが最終的に解決 |
| `heavy_struggle` | `major` の躓きが複数、または未解決の躓きが残っている |

### pattern ラベルの一貫性

記録セッションには**その要素について過去に使われた pattern ラベルの一覧**を入力として渡し、同じ種類の躓きには既存ラベルを再利用させる。

## 本文

| セクション | 内容 |
|---|---|
| 概要 | どのように解いたかを 2〜3 行で要約 |
| 解答の流れ | 試行ごとの変更内容と結果を時系列で記述 |
| 躓いたポイント | 躓きごとに、根拠となる試行・原因の推測・解決方法を記述 |
| スムーズに解けた点 | 迷わず書けていた箇所 |
| 最終コードの所見 | Rust らしい書き方か、冗長な箇所、回避的な実装の有無、模範解答との違い |
| 質問から見える理解度 | 質問内容から推測される理解度・誤解（質問がなければ「質問なし」） |
| 次の学習への示唆 | 復習すべき点、次に挑戦すると良い内容 |
