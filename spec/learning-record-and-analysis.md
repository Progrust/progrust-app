# 学習記録・分析 仕様書

Claude Code 連携機能のうち「学習記録」「分析」に関する仕様を定める。
`claude -p` の呼び出し方とジョブキューは `claude-integration.md` に、本仕様のデータを画面や問題生成でどう使うかは `screens.md`・`exercise-generation.md` に定める。

## 1. 設計方針

### 1.1 事実と解釈の分離

| 種別 | 例 | 担当 |
|---|---|---|
| 事実 | 実行回数、提出回数、エラーコード、所要時間、最終演習日 | アプリ（機械的に計算） |
| 解釈 | 躓きの原因、理解度の評価、学習者の傾向、次の推奨 | AI（`claude -p`） |

- 数値・日時・回数は AI に計算させない。AI が出力した場合もアプリの値で上書きする。
- AI の判断にはアプリが計算した事実を入力として渡す。

### 1.2 ファイル形式

- 学習記録・分析ファイルは **Markdown + YAML frontmatter** とする。
- frontmatter: アプリが機械的に読み取る構造化データ（ダッシュボード表示、問題生成、集計に使用）。
- 本文: 人と AI が読むための文章。アプリは本文を解析しない。

### 1.3 Rust 要素 ID の共通化

レッスン・学習記録・分析・演習問題生成設定は、すべて共通の要素 ID（例: `ownership.move`）で Rust の要素を参照する。要素 ID の定義は要素マスタ（`content-format.md` §3）に一元化する。

### 1.4 AI にファイルを直接書かせない

`claude -p` には標準出力で結果を返させ、アプリが検証してから書き込む。AI にファイル編集ツールは使わせない。

## 2. 全体フロー

```
[演習中] アプリが試行ログ(JSON)にイベントを追記
     │  質問 / 実行 / 提出
     ▼
[レッスン終了] ※§3.1 の終了条件を満たしたとき
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
- ジョブキューの仕様は `claude-integration.md` §5 を参照。
- 各ファイルの保存場所は `architecture.md` §4 を参照。

## 3. 試行ログ（attempts/*.json）

演習中の行動をアプリが記録する生データ。学習記録の入力となる。
**記録を生成した後も削除せず保持する**。

### 3.1 試行セッションのライフサイクル

- **開始**: レッスンを開いたとき（動画画面。動画のない章末復習レッスン・生成問題では演習画面）。
- **継続**: 未完了のセッションがあるレッスンを再度開いた場合は、同じセッションにイベントを追記する（アプリの再起動をまたいでも継続）。
- **正解**: 提出で正解したとき、`passed_at` を記録する。
- **終了**: 正解後に以下のいずれかが起きたとき、`ended_at` を記録し記録ジョブを登録する。
  - 「レッスン一覧へ」「次のレッスンへ」「生成問題一覧へ」を選択した
  - 解説表示中の演習画面から別画面へ遷移した
  - アプリを終了した
  - アプリ起動時に `passed_at` があり `ended_at` がないセッションが見つかった（異常終了時の救済）
- 解説表示中の質問も、終了までは同じ試行セッションに記録される。
- 完了済みレッスンをやり直した場合は新しいセッションとする。

### 3.2 スキーマ

```json
{
  "session_id": "20261031T130200_ch04-03",
  "lesson_id": "ch04-03",
  "kind": "lesson",
  "difficulty": null,
  "started_at": "2026-10-31T13:02:00+09:00",
  "passed_at": "2026-10-31T13:25:00+09:00",
  "ended_at": "2026-10-31T13:31:00+09:00",
  "events": [
    { "type": "chat", "at": "...", "context": "video:m5",
      "question": "ムーブ後に使えないのはなぜ？", "answer_summary": null },
    { "type": "run", "at": "...",
      "files": { "src/main.rs": "..." },
      "result": { "status": "compile_error", "error_codes": ["E0382"],
                  "stdout": "", "stderr": "...", "exit_code": 1, "stdin": "" } },
    { "type": "submit", "at": "...",
      "files": { "src/main.rs": "..." },
      "result": { "status": "wrong", "error_codes": [], "passed": 3, "total": 5,
                  "failed_cases": ["空の入力"], "stderr": "" } }
  ]
}
```

| フィールド | 説明 |
|---|---|
| `session_id` | `{開始日時 YYYYMMDDTHHMMSS}_{lesson_id}` |
| `lesson_id` | レッスン ID。生成問題は `gen-{生成問題ID}` |
| `kind` | `lesson` / `chapter_review` / `generated` |
| `difficulty` | 生成問題の難易度（1〜5）。レッスンは `null` |

イベント種別:

| type | 記録タイミング | 主な内容 |
|---|---|---|
| `chat` | 質問を送信したとき | `context`（`video:{文の ID}` / `explanation`）、質問文。`answer_summary` は将来の拡張用で、現在は常に `null` |
| `run` | 実行が終わったとき | 全ファイルのスナップショット、実行結果、学習者が入力した標準入力（`stdin`） |
| `submit` | 提出の判定が終わったとき | 全ファイルのスナップショット、判定結果（`failed_cases` は失敗したテストケースの表示名） |

- 動画の視聴（再生・一時停止・シークなど）は試行ログに記録しない。

`result.status`: `run` は `success` / `compile_error` / `runtime_error` / `timeout` / `stopped`、`submit` は `compile_error` / `wrong` / `passed`（意味は `exercise-runner.md` §4.3・§5.3）

- スナップショットはファイル全体を保存する。
- `error_codes` の抽出方法と、`stdout` / `stderr` の保存の上限は `exercise-runner.md` §7 に従う。

## 4. 学習記録ファイル（records/*.md）

1 セッションにつき 1 ファイル。テンプレートは `spec/templates/record.md`。

### 4.1 ファイル名

`{record_id}.md`、`record_id` = `{YYYY-MM-DD}_{lesson_id}_{連番2桁}`
（同日に同じレッスンを複数回行った場合は連番を増やす）

### 4.2 frontmatter

```yaml
---
schema_version: 1
record_id: 2026-10-31_ch04-03_01
session_id: 20261031T130200_ch04-03
lesson_id: ch04-03
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

#### アプリ管理フィールド

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

#### AI 管理フィールド

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

#### pattern ラベルの一貫性

記録セッションには**その要素について過去に使われた pattern ラベルの一覧**を入力として渡し、同じ種類の躓きには既存ラベルを再利用させる。

### 4.3 本文

| セクション | 内容 |
|---|---|
| 概要 | どのように解いたかを 2〜3 行で要約 |
| 解答の流れ | 試行ごとの変更内容と結果を時系列で記述 |
| 躓いたポイント | 躓きごとに、根拠となる試行・原因の推測・解決方法を記述 |
| スムーズに解けた点 | 迷わず書けていた箇所 |
| 最終コードの所見 | Rust らしい書き方か、冗長な箇所、回避的な実装の有無、模範解答との違い |
| 質問から見える理解度 | 質問内容から推測される理解度・誤解（質問がなければ「質問なし」） |
| 次の学習への示唆 | 復習すべき点、次に挑戦すると良い内容 |

## 5. 分析ファイル（analysis/analysis.md）

学習者全体の傾向を表す、常に最新状態に保たれるファイル。テンプレートは `spec/templates/analysis.md`。

### 5.1 習熟レベル

| level | 表示名 | 意味 |
|---|---|---|
| 4 | 得意 | 安定して使え、発展的な問題でも躓かない |
| 3 | 安定 | 概ね躓かずに使える |
| 2 | たまに間違える | ときどき躓く |
| 1 | よく間違える | 繰り返し躓く |
| 0 | 未学習 | 演習で扱っていない |

加えて、演習回数が 1 回のみの要素は `provisional: true` とし、UI では「データ不足」バッジを併記する。データ不足の要素はダッシュボードの得意・苦手ランキングには含めないが、問題生成の苦手分野には含める。

### 5.2 基準レベルの計算（アプリ）

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

### 5.3 AI によるレベル調整

- AI は `base_level` を参考に `level` を決定する。
- 調整幅は `base_level ± 1` かつ 1〜4 の範囲に限る（`base_level: 0` は調整不可）。
- `level` が `base_level` と異なる場合は `level_reason` に理由を必須で記述する。
- 範囲外の値が出力された場合、アプリは `base_level` を採用する。

### 5.4 推移と時間軸（アプリ）

| フィールド | 計算方法 |
|---|---|
| `trend` | 直近 3 件と、その前の 3 件の平均躓き度を比較。差が 0.5 以上改善なら `improving`、0.5 以上悪化なら `declining`、それ以外は `stable`。関連記録が 4 件未満なら `unknown` |
| `last_practiced` | 関連記録の最新 `completed_at` の日付 |
| `last_stumbled` | 躓き度 1 以上の関連記録の最新日付（なければ `null`） |
| `review_due` | `last_practiced` からの経過日数が、level 1〜2 で 7 日以上、level 3 で 21 日以上、level 4 で 45 日以上なら `true` |
| `previous_level` | 更新前の analysis.md における `level`（新規要素は `null`） |
| `regressed` | `previous_level >= 3` かつ新しい `level <= 2` なら `true`（定着度の低下） |

### 5.5 frontmatter

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
    level: 1                       # AI 管理（§5.3 の制約あり）
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

### 5.6 本文

| セクション | 内容 |
|---|---|
| 総評 | 現在の学習状況を 3〜5 行で要約 |
| 苦手分野 | 「よく間違える」「たまに間違える」に分けて、要素ごとにミスのパターン・推移・根拠となる記録 ID を記述 |
| 得意分野 | 「得意」「安定」の要素と、その根拠 |
| 全体に共通する傾向 | 要素をまたいだ傾向（例: エラーメッセージを読まずに試行錯誤する、借用エラーを `clone()` で回避しがち） |
| 時間軸の分析 | 最近扱っていない分野、復習推奨、学習ペース、定着度が低下した分野 |
| 演習問題生成への推奨 | 重点要素、推奨難易度、出題の切り口の提案 |
| 更新履歴 | 直近 5 回分の更新内容（日時、記録 ID、主な変化） |

## 6. AI ジョブ

記録・分析・再構築はジョブとして実行する。`claude -p` の呼び出し方・モデル・ジョブキューは `claude-integration.md` に従う。

- 記録・分析セッションの出力は、frontmatter（`---`）から始まる Markdown のみとするよう指示する。コードフェンスで囲まれていた場合はアプリが除去する。
- 記録と分析は別々の `claude -p` 呼び出しとする。

### 6.1 記録セッション

**入力**

1. 指示文（`prompts/record.md`。役割、出力形式、判断基準）
2. 記録テンプレート（`templates/record.md`）。テンプレート中の `{{...}}` はアプリが値を埋め込んでから渡す
3. レッスン情報: 問題文、模範解答、テストケース、`primary_elements` / `secondary_elements` と各要素の説明
4. 要素ごとの既存 pattern ラベル一覧（§4.2）
5. エラーコードから推定した要素の候補（`related_errors` による）
6. 試行ログ（後述の圧縮を適用）
7. アプリが計算した事実フィールドの値

**試行ログの圧縮**

- 2 回目以降の `run` / `submit` のコードは、直前のスナップショットとの差分（unified diff）に変換する。最初と最後の提出は全文を渡す。
- `stderr` は 1 イベントあたり 2,000 文字で切り詰める。
- `run` / `submit` が 30 件を超える場合は、最初の 5 件・最後の 10 件・結果の `status` が変化したイベントを優先して残す。

**出力の検証**

- frontmatter を YAML として解析できること。
- `outcome` / `severity` が定義済みの値であること。
- `stumbles[].element` / `smooth_elements` が要素マスタに存在すること。
- 本文に必須セクション（§4.3）がすべて存在すること。
- 検証に失敗した場合はジョブを失敗扱いにしてリトライする。
- 検証に成功したら、アプリ管理フィールドを上書きして保存する。

### 6.2 分析セッション

**入力**

1. 指示文（`prompts/analysis.md`。役割、出力形式、レベル調整の制約）
2. 分析テンプレート（`templates/analysis.md`）
3. 現在の `analysis.md`（初回はテンプレート）
4. 今回の学習記録（全文）
5. アプリの集計結果: 全要素の `base_level`・`trend`・`review_due` などのアプリ管理フィールドと `activity`
6. 要素マスタ（関連記録のある要素と、今回の記録に含まれる要素のみ）

過去の全記録は渡さず、「現在の分析 + 新しい記録 + 集計値」で差分的に更新する。

**出力の検証**

- frontmatter を解析できること。
- `level` が §5.3 の制約を満たすこと（違反時は `base_level` を採用し、`level_reason` を空にする）。
- `focus_recommendation` の要素 ID が要素マスタに存在すること（存在しないものは除去）。
- `recommended_difficulty` が 1〜5 であること（範囲外なら 1〜5 に丸める）。
- 本文に必須セクション（§5.6）がすべて存在すること。
- 検証に成功したら、アプリ管理フィールドを上書きし、旧版を `history/` に退避して保存する。`history/` は直近 20 件を保持する。

### 6.3 分析の再構築（rebuild）

全記録から分析ファイルを作り直す処理。設定画面の「分析を再構築」から実行する。

1. 現在の `analysis.md` を `history/` に退避する。
2. テンプレートから空の分析ファイルを作成する。
3. 全記録を `completed_at` 順に並べ、10 件ずつまとめて分析セッションに渡して更新する（10 件ごとに 1 回の `claude -p`）。
4. 最後にアプリ管理フィールドを全記録から再計算する。

## 7. 手動編集への対応

- 学習記録・分析ファイルはユーザーが手で編集することを想定する。
- frontmatter を解析できない記録は集計から除外し、設定画面に警告として表示する（`screens.md` §8）。
- `analysis.md` を解析できない場合は、トップページに「分析ファイルを読み込めません」と表示し、再構築を案内する（`screens.md` §3）。

## 8. スキーマのバージョン管理

- 学習記録・分析ファイルは `schema_version` を持つ。
- スキーマを変更した場合は、アプリ起動時に古いバージョンのファイルを検出し、試行ログからの記録再生成、または分析の再構築を案内する。

## 9. 受け入れ条件

- レッスンに正解して「次のレッスンへ」を押すと、記録ジョブが登録され、完了後に `records/` に学習記録が保存され、続けて分析ジョブが実行される。
- 学習記録のアプリ管理フィールドは、AI が異なる値を出力しても試行ログから計算した値になっている。
- 正解後にアプリを強制終了して再起動すると、その試行セッションの記録ジョブが登録される。
- 分析ファイルの `level` が `base_level ± 1` の範囲を超えることがない。
- `analysis.md` を手で壊した後に「分析を再構築」を実行すると、全記録から分析ファイルが作り直される。
- AI の出力が検証に失敗した場合、`records/`・`analysis/` のファイルは変更されない。

## 10. 未決事項

- `active_minutes`（§4.2）で、動画の視聴に使った時間をどう扱うか。動画の視聴中は試行ログにイベントが記録されない。この時間を、イベント間隔が 10 分を超える区間を除く決まりの中でどう扱うかは決めていない。
