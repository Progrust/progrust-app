---
type: Specification
title: "学習記録・分析の AI ジョブ"
description: "記録セッションと分析セッションの入力、試行ログの圧縮、分析セッションに出力させるフィールド、出力の検証と保存、分析の再構築（rebuild）の手順を定める。"
---

記録・分析・再構築はジョブとして実行する。`claude -p` の呼び出し方・モデル・ジョブキューは [Claude Code 連携](../claude-integration/index.md) に従う。

- 記録・分析セッションの出力は、frontmatter（`---`）から始まる Markdown のみとするよう指示する。コードフェンスで囲まれていた場合はアプリが除去する。
- 記録と分析は別々の `claude -p` 呼び出しとする。

## 記録セッション

**入力**

1. 指示文（`prompts/record.md`。役割、出力形式、判断基準）
2. 記録テンプレート（`templates/record.md`）。テンプレート中の `{{...}}` はアプリが値を埋め込んでから渡す
3. レッスン情報: 問題文、模範解答、テストケース、`primary_elements` / `secondary_elements` と各要素の説明
4. 要素ごとの既存 pattern ラベル一覧（[学習記録ファイル（records/*.md）](record-file.md)「frontmatter」）
5. エラーコードから推定した要素の候補（`related_errors` による）
6. 試行ログ（後述の圧縮を適用）
7. アプリが計算した事実フィールドの値

**試行ログの圧縮**

- 最初の `run` / `submit` のコードと、最後の提出のコードは全文を渡す。それ以外の `run` / `submit` のコードは、直前のスナップショットとの差分（unified diff）に変換する。直前のスナップショットは、下の件数の上限で残したイベントのうち、そのイベントの直前のものとする。
- `stderr` は 1 イベントあたり 2,000 文字で切り詰める。
- `run` / `submit` が 30 件を超える場合は、最初の 5 件・最後の 10 件・結果の `status` が変化したイベントを優先して残す。

**出力の検証**

- frontmatter を YAML として解析できること。
- `outcome` / `severity` が定義済みの値であること。
- `stumbles[].element` / `smooth_elements` が要素マスタに存在すること。
- 本文に必須セクション（[学習記録ファイル（records/*.md）](record-file.md)「本文」）がすべて存在すること。
- 検証に失敗した場合はジョブを失敗扱いにしてリトライする。
- 検証に成功したら、アプリ管理フィールドを上書きして保存する。

## 分析セッション

**入力**

1. 指示文（`prompts/analysis.md`。役割、出力形式、レベル調整の制約）
2. 分析テンプレート（`templates/analysis.md`）
3. 現在の `analysis.md`（初回はテンプレート）
4. 今回の学習記録（全文）
5. アプリの集計結果: 全要素の `base_level`・`trend`・`review_due` などのアプリ管理フィールドと `activity`
6. 要素マスタ（関連記録のある要素と、今回の記録に含まれる要素のみ）

過去の全記録は渡さず、「現在の分析 + 新しい記録 + 集計値」で差分的に更新する。

**出力**

- AI には、frontmatter のうち AI 管理のフィールド（各要素の `level`・`level_reason`、`focus_recommendation`、`recommended_difficulty`）と、本文を出力させる。アプリ管理のフィールド（[分析ファイル（analysis/analysis.md）](analysis-file.md)「frontmatter」）は出力させない。
- `elements` には、入力 5 の集計結果にあるすべての要素を、`level`・`level_reason` の 2 つだけで出力させる。

**出力の検証**

- frontmatter を解析できること。
- 集計結果にある要素のうち、`elements` に無い要素は、`level` を `base_level`、`level_reason` を空にする。集計結果に無い要素は除去する。
- `level` が [分析ファイル（analysis/analysis.md）](analysis-file.md)「AI によるレベル調整」 の制約を満たすこと（違反時は `base_level` を採用し、`level_reason` を空にする）。
- `focus_recommendation` の要素 ID が要素マスタに存在すること（存在しないものは除去）。
- `recommended_difficulty` が 1〜5 であること（範囲外なら 1〜5 に丸める）。
- 本文に必須セクション（[分析ファイル（analysis/analysis.md）](analysis-file.md)「本文」）がすべて存在すること。
- 検証に成功したら、AI の出力にアプリ管理フィールドを合わせて分析ファイルを組み立て、旧版を `history/` に退避して保存する。`history/` は直近 20 件を保持する。

## 分析の再構築（rebuild）

全記録から分析ファイルを作り直す処理。設定画面の「分析を再構築」から実行する。

1. 現在の `analysis.md` を `history/` に退避する。
2. テンプレートから空の分析ファイルを作成する。
3. 全記録を `completed_at` 順に並べ、10 件ずつまとめて分析セッションに渡して更新する（10 件ごとに 1 回の `claude -p`）。
4. 最後にアプリ管理フィールドを全記録から再計算する。
