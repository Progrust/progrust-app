---
type: Decision
title: "ADR 0265"
description: "C のレッスンに要素を持たせず、学習記録・分析・演習問題生成の対象にしないが、試行セッションは作って試行ログを残す"
---

- 日付: 2026-11-28
- 関連仕様: [レッスン定義の形式（lesson.json）](../spec/content-format/lesson-json.md)「C のレッスン」、[試行ログ（attempts/*.json）](../spec/learning-record-and-analysis/attempt-log.md)「試行セッションのライフサイクル」、[学習記録・分析の全体フロー](../spec/learning-record-and-analysis/flow.md)、[学習記録・分析の目的とスコープ](../spec/learning-record-and-analysis/scope.md)、[質疑応答を利用できる場面](../spec/qa-chat/availability.md)、[レッスンの完了](../spec/lesson-flow/completion.md)、[演習問題生成の目的とスコープ](../spec/exercise-generation/scope.md)、[問題生成のコマンド](../spec/architecture/commands/generation.md)、[進捗のコマンド](../spec/architecture/commands/progress.md)、[トップページ（ダッシュボード）](../spec/screens/top-page.md)、[問題生成の画面（生成問題一覧、生成設定画面）](../spec/screens/generation-screens.md)、[要素の分け方と要素 ID の付け方](../spec/content-guidelines/elements.md)「要素の分け方」、[実行・提出の試行ログへの記録](../spec/exercise-runner/attempt-log.md)
- 置き換え: なし

## 背景

カリキュラムの最初に C の章を置くことにした（[ADR 0262](0262-teach-c-chapters-before-rust.md)）。レッスン・学習記録・分析・問題生成の設定は、すべて要素マスタの要素 ID で Rust の要素を参照していて、要素 ID は公開したら消せない（[ADR 0006](0006-element-ids-are-permanent.md)）。

Claude は、C の演習を分析や問題生成の対象にすると、ほぼすべての機能に手が入るので、対象にしないことを勧めた。その場合、C の演習での躓きが分析に残らないことも伝えた。ユーザーは次のように答えた。「C の演習を分析・問題生成の対象にはしなくてよいです。質問応答ができれば十分だとおもいます。」

演習のないレッスンには、要素を持たせず、試行セッションも作らない前例がある（[ADR 0120](0120-video-only-lessons-have-no-elements-or-attempt-sessions.md)）。一方で、演習のあるレッスンでは、「前回の正解コードを見る」、正解した後の下書きの削除、解説表示中の質疑応答に渡す「学習者が正解したときのコード」が、試行ログと試行セッションを使っている。

## 決定

- C のレッスンは、学習記録・分析・演習問題生成の対象にしない。質疑応答は使える（[学習記録・分析の全体フロー](../spec/learning-record-and-analysis/flow.md)、[演習問題生成の目的とスコープ](../spec/exercise-generation/scope.md)）。
  - 理由: 上の「背景」のユーザーの発言のとおり。
- 次の細部は Claude が決めた。
  - C のレッスンには、主要素も副要素も持たせない（[レッスン定義の形式（lesson.json）](../spec/content-format/lesson-json.md)「C のレッスン」、[要素の分け方と要素 ID の付け方](../spec/content-guidelines/elements.md)「要素の分け方」）。理由: 要素は、学習記録・分析・問題生成のために置いている。対象にしないレッスンに持たせると、消せない要素 ID が増え、問題生成の設定画面に、生成できない要素が並ぶ。
  - C のレッスンでも、試行セッションを作り、試行ログに実行・提出・質問を記録する。試行セッションが終わっても、記録ジョブは登録しない（[試行ログ（attempts/*.json）](../spec/learning-record-and-analysis/attempt-log.md)「試行セッションのライフサイクル」、[進捗のコマンド](../spec/architecture/commands/progress.md)）。理由: 上の「背景」の 3 つの機能を、Rust のレッスンと同じ作りのまま C のレッスンで使える。試行ログが残っていれば、後から C のレッスンを学習記録の対象にすることになっても、過去の分を使える。
  - C のレッスンも、進捗に記録し、完了レッスン数と章の完了数に数える（[レッスンの完了](../spec/lesson-flow/completion.md)、[トップページ（ダッシュボード）](../spec/screens/top-page.md)）。理由: C の章もカリキュラムの一部で、次にやるべきレッスンの決め方も同じである。
  - 学習記録がまだないときのトップページの文言を、「Rust の演習に正解すると、ここに分析が表示されます」に変える（[トップページ（ダッシュボード）](../spec/screens/top-page.md)）。[ADR 0119](0119-video-only-lessons-for-background-knowledge.md) の「決定」が定めた文言「演習に正解すると、ここに分析が表示されます」を、この点で変える。理由: C の演習に正解しても学習記録はできないので、「演習に正解すると」では事実と合わなくなる。
  - 問題生成の設定の要素の選択欄には、C の章を出さない（[問題生成の画面（生成問題一覧、生成設定画面）](../spec/screens/generation-screens.md)、[問題生成のコマンド](../spec/architecture/commands/generation.md)）。理由: C の章には選べる要素が無い。

## 検討した他の案

- 案: C のレッスンでは試行セッションを作らない（動画のみのレッスンと同じ。[ADR 0120](0120-video-only-lessons-have-no-elements-or-attempt-sessions.md)）
  - 採用しなかった理由: 「前回の正解コードを見る」、下書きの削除、解説表示中の質疑応答に渡す正解コードのために、別の仕組みが要る。[ADR 0120](0120-video-only-lessons-have-no-elements-or-attempt-sessions.md) が作らないとした理由（演習がなく、試行セッションが終わらない）は、演習のある C のレッスンには当てはまらない
- 案: C の内容も要素マスタに入れ、学習記録と分析の対象にする
  - 採用しなかった理由: 上の「背景」のユーザーの発言のとおり、対象にしなくてよい。要素マスタ、学習記録、分析、問題生成が Rust の要素を前提にしていて、変える範囲が大きい

## 影響

- C の演習での躓きは、学習記録と分析に残らない。C の章を学んでいる間は、トップページの分析の項目に何も出ない。
- C のレッスンの試行ログは、記録ジョブに使われないまま残る。
- [ADR 0120](0120-video-only-lessons-have-no-elements-or-attempt-sessions.md) は変えない。動画のみのレッスン（c01-01 を含む）は、試行セッションを作らない。
