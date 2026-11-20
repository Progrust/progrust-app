---
type: Specification
title: "演習問題生成の出力と検証"
description: "AI に出力させる JSON の形式、出力の検証項目、検証に失敗したときの修正依頼（最大 2 回）と `failed` の扱いを定める。"
---

## 出力形式

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

各フィールドの意味は [レッスン定義の形式（lesson.json）](../content-format/lesson-json.md)・[演習のファイル形式](../content-format/exercise.md) の同名の項目と同じ。`cases` は `stdout` 方式、`judge_tests_rs` は `test` 方式で使う。

## 検証

1. JSON として解析でき、必須フィールドがそろっている。
2. 要素 ID が要素マスタに存在し、選択した要素がすべて `primary_elements` か `secondary_elements` に含まれている。
3. `dependencies` が許可クレートの一覧にある。
4. ファイルパスが [実行・提出時のプロジェクトの組み立て](../exercise-runner/project-assembly.md) の条件を満たし、`src/main.rs` がある。
5. 問題文・解説に必須の見出し（[演習のファイル形式](../content-format/exercise.md)「問題文（problem.md）」・[演習のファイル形式](../content-format/exercise.md)「解説（explanation.md）」）がある。
6. テストケースが 3 件以上ある。
7. 模範解答が判定ですべてのテストケースに通る。
8. 初期コードが判定で正解にならない。

7・8 の判定は [コード実行・判定](../exercise-runner/index.md) の判定処理を使い、`workspace/validate/` で行う。

## 検証に失敗した場合

- 失敗した項目と内容（コンパイルエラー、失敗したテストケースなど）を、前回の出力とともに AI に渡し、修正版を出力させる。修正は最大 2 回とする。
- 修正しても検証に通らない場合は、ジョブを `failed` とする（リトライはしない）。
- `claude` の呼び出し自体が失敗した場合（タイムアウトなど）は、通常のジョブのリトライ（[ジョブキュー（jobs.json）](../claude-integration/job-queue.md)）に従う。
