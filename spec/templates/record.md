---
schema_version: 1
record_id: "{{record_id}}"
session_id: "{{session_id}}"
lesson_id: "{{lesson_id}}"
kind: "{{kind}}"
title: "{{title}}"
difficulty: {{difficulty}}
elements:
  primary: {{primary_elements}}
  secondary: {{secondary_elements}}
# --- 事実（アプリが記入・上書きする。値を変更しないこと） ---
started_at: "{{started_at}}"
completed_at: "{{completed_at}}"
active_minutes: {{active_minutes}}
run_count: {{run_count}}
submit_count: {{submit_count}}
first_submit_passed: {{first_submit_passed}}
error_codes: {{error_codes}}
questions_asked: {{questions_asked}}
# --- 解釈（AIが記入する） ---
# outcome: smooth | minor_struggle | struggled_resolved | heavy_struggle
outcome:
# 躓きがなければ stumbles: []
# severity: minor（1〜2回の試行で自力解決） | major（3回以上の試行、質問が必要、または誤った方法で回避）
# pattern: 躓きの種類を表す snake_case ラベル。既存ラベル一覧に同じ種類があれば必ず再利用する
# resolved: 最終コードで正しく解決できていれば true（clone() 等で回避しただけなら false）
stumbles:
  - element:
    severity:
    pattern:
    resolved:
# 躓きなく使えていた要素ID
smooth_elements: []
---

# {{title}} 学習記録

## 概要

<!-- どのように解いたかを2〜3行で要約する -->

## 解答の流れ

<!--
試行ごとに、何を変更し、どういう結果になったかを時系列で書く。
形式: 「1. 試行N（HH:MM 実行|提出）: 変更内容 → 結果」
-->

## 躓いたポイント

<!--
躓きごとに見出しを作る。躓きがなければ「特になし」と書く。
### 躓きの内容（要素ID / severity）
- 根拠: どの試行から判断したか
- 原因の推測: 何を誤解していたと考えられるか
- 解決方法: どのように解決したか（未解決・回避の場合はその旨）
-->

## スムーズに解けた点

<!-- 迷わず書けていた箇所と、そう判断した根拠 -->

## 最終コードの所見

<!--
- Rustらしい書き方になっているか
- 冗長な箇所、不要な clone() や unwrap() などの回避的な実装の有無
- 模範解答との主な違い（優劣ではなく、考え方の違いとして書く）
-->

## 質問から見える理解度

<!-- 質問内容から推測される理解度・誤解。質問がなければ「質問なし」と書く -->

## 次の学習への示唆

<!-- 復習すべき点、次に挑戦すると良い内容 -->
