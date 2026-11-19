# 演習問題生成 仕様書

学習者の設定と分析ファイルをもとに、AI が演習問題を生成する機能を定める。
生成した問題はレッスンの演習と同じ形式（`content-format.md` §4・§6）で保存し、同じ仕組みで実行・判定・記録する。

## 1. 流れ

```
[生成設定画面] 設定を入力して「生成」
     │  generated/{generation_id}/meta.json を status: generating で作成
     │  generate ジョブを登録（claude-integration.md §5）
     ▼
[generate ジョブ] claude -p で問題を生成 → 検証（§4）
     │  失敗: 修正を依頼して再生成（最大 2 回）
     ▼
 成功: 演習ファイルを保存し status: ready → 通知
 失敗: status: failed → 一覧にエラーを表示
```

生成中も学習者は他の画面で学習を続けられる。

## 2. 生成設定

| 項目 | 内容 | 初期値 |
|---|---|---|
| 苦手分野を中心にする | オン/オフ。分析ファイルに要素が 1 件もない場合はオフに固定する | 分析ファイルに要素があればオン |
| 要素 | 要素マスタから最大 5 件選ぶ。章ごとにまとめて表示し、各要素に習熟レベルを表示する。未学習の要素には「未学習」と表示する（選択は可能） | 下記 |
| 難易度 | 1〜5（§2.1） | 分析ファイルの `recommended_difficulty`。分析ファイルがなければ 2 |
| AI へのメッセージ | 自由入力（最大 1,000 文字）。他の設定より優先される | 空 |

**要素の初期値**: 「苦手分野を中心にする」がオンのとき、以下の順に重複を除いて最大 5 件を選択状態にする。オフのときは空とする。

1. 分析ファイルの `focus_recommendation`（その順序）
2. `level <= 2` の要素（level が低い順）
3. `review_due: true` の要素（`last_practiced` が古い順）

要素を 1 件も選ばずに生成した場合、AI が既習の要素から選ぶ。

### 2.1 難易度

難易度は、問題の難しさとコードの規模（複雑さ）をまとめて表す。

| 難易度 | 目安 |
|---|---|
| 1 | 主要素を 1 つ使う短い問題。解答は 20 行程度 |
| 2 | 主要素に既習の要素を 1〜2 個組み合わせる。通常のレッスンの演習と同程度 |
| 3 | 複数の要素を組み合わせ、境界のケースへの配慮が必要。解答は 50 行程度 |
| 4 | 型やトレイトの設計を伴う。複数ファイル（`multi`）にしてもよい |
| 5 | 章末復習レッスン以上。設計判断を含む複数ファイルの問題 |

## 3. AI への入力

1. 指示文（`prompts/generate.md`）
2. 生成設定（選択した要素の `name`・`description`、難易度とその目安、AI へのメッセージ）
3. 既習の要素の一覧（完了済みレッスンの `primary_elements` に含まれる要素）
4. 「苦手分野を中心にする」がオンの場合: 分析ファイルの本文のうち「苦手分野」「全体に共通する傾向」「演習問題生成への推奨」セクションと、選択した要素の `level`・`recurring_patterns`
5. 直近 20 件の生成問題のタイトルと概要
6. 演習の作成ルール（`content-guidelines.md` §5）
7. 許可クレートの一覧
8. 出力形式（§4.1）

指示文で以下を指示する。

- AI へのメッセージを最優先とし、他の設定と矛盾する場合はメッセージに従う。
- 選択した要素はすべて問題に登場させる（主要素または副要素）。
- 選択した要素と既習の要素以外は使わない。
- 直近の生成問題と同じ内容の問題にしない。
- 苦手分野を中心にする場合、`recurring_patterns` の躓きを克服できる問題にする。

## 4. 出力と検証

### 4.1 出力形式

JSON オブジェクト 1 つを出力させる（コードフェンスで囲まれていた場合はアプリが除去する）。

```json
{
  "title": "在庫リストの集計",
  "summary": "所有権を意識しながらベクタを集計する",
  "primary_elements": ["ownership.move"],
  "secondary_elements": ["vec.basic"],
  "layout": "single",
  "judge": { "mode": "stdout", "target": "src/main.rs" },
  "dependencies": [],
  "readonly_files": [],
  "case_labels": {},
  "problem_md": "## 問題\n...",
  "explanation_md": "## 考え方\n...",
  "starter_files": { "src/main.rs": "..." },
  "solution_files": { "src/main.rs": "..." },
  "cases": [ { "name": "...", "stdin": "...", "expected_stdout": "..." } ],
  "judge_tests_rs": null
}
```

各フィールドの意味は `content-format.md` §4・§6 の同名の項目と同じ。`cases` は `stdout` 方式、`judge_tests_rs` は `test` 方式で使う。

### 4.2 検証

1. JSON として解析でき、必須フィールドがそろっている。
2. 要素 ID が要素マスタに存在し、選択した要素がすべて `primary_elements` か `secondary_elements` に含まれている。
3. `dependencies` が許可クレートの一覧にある。
4. ファイルパスが `exercise-runner.md` §3 の条件を満たし、`src/main.rs` がある。
5. 問題文・解説に必須の見出し（`content-format.md` §6.2・§6.3）がある。
6. テストケースが 3 件以上ある。
7. 模範解答が判定ですべてのテストケースに通る。
8. 初期コードが判定で正解にならない。

7・8 の判定は `exercise-runner.md` の判定処理を使い、`workspace/validate/` で行う。

### 4.3 検証に失敗した場合

- 失敗した項目と内容（コンパイルエラー、失敗したテストケースなど）を、前回の出力とともに AI に渡し、修正版を出力させる。修正は最大 2 回とする。
- 修正しても検証に通らない場合は、ジョブを `failed` とする（リトライはしない）。
- `claude` の呼び出し自体が失敗した場合（タイムアウトなど）は、通常のジョブのリトライ（`claude-integration.md` §5）に従う。

## 5. 保存（generated/{generation_id}/）

`generation_id` は生成を依頼した日時 `YYYYMMDDTHHMMSS` とする。レッスン ID として扱うときは `gen-{generation_id}` とする。

```
generated/{generation_id}/
├── meta.json
├── lesson.json        # content-format.md §4 の形式（kind は generated）
└── exercise/          # content-format.md §6 の形式
```

`meta.json`:

```json
{
  "generation_id": "20261031T140000",
  "status": "ready",
  "settings": {
    "focus_weak": true,
    "elements": ["ownership.move"],
    "difficulty": 2,
    "message": ""
  },
  "created_at": "2026-10-31T14:00:00+09:00",
  "ready_at": "2026-10-31T14:02:10+09:00",
  "title": "在庫リストの集計",
  "summary": "所有権を意識しながらベクタを集計する",
  "error": null,
  "completion_count": 0,
  "last_completed_at": null,
  "last_passed_session_id": null
}
```

| フィールド | 説明 |
|---|---|
| `status` | `generating` / `ready` / `failed` |
| `error` | `failed` のときの理由（検証で失敗した項目など） |
| `completion_count` 〜 `last_passed_session_id` | `lesson-flow.md` §3 の同名の項目と同じ意味 |

## 6. 生成問題一覧と演習

- 生成問題一覧は新しい順に表示する。各問題にタイトル・概要・要素・難易度・状態（生成中 / 未着手 / 完了 / 失敗）・作成日時を表示する。
- 操作:
  - 「開く」: 演習画面を開く（`ready` のみ）。
  - 「同じ設定で生成」: `settings` を初期値として生成設定画面を開く。
  - 「削除」: 確認ダイアログの後、`generated/{generation_id}/` と下書きを削除する。学習記録・試行ログは削除しない。
  - 失敗した問題は、エラーの内容と「再生成」ボタン（同じ設定で新しく生成）を表示する。
- 生成が完了したら、どの画面にいても通知（トースト）を表示し、「開く」で演習画面に移れるようにする。
- 生成問題の演習は、動画がないこと以外はレッスンと同じ流れとする（`lesson-flow.md` §1）。解説表示の後は「生成問題一覧へ」を表示する。
- 生成問題の試行も、レッスンと同じく記録・分析する（試行ログの `kind: generated`、`difficulty` に難易度を記録する）。

## 7. 受け入れ条件

- 生成中に他のレッスンを進められ、完了するとトーストで通知される。
- 生成された問題の模範解答は必ず判定に通り、初期コードは必ず判定に通らない。
- 選択した要素が、生成された問題の `primary_elements` か `secondary_elements` に必ず含まれる。
- 分析ファイルに要素がない状態では「苦手分野を中心にする」をオンにできない。
- 「苦手分野を中心にする」をオンにすると、§2 の規則どおりに要素の初期値が設定される。
- 生成問題を削除しても、その問題の学習記録と分析への反映は残る。

## 8. 未決事項

- なし
