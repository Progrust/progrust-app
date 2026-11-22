---
type: Decision
title: "ADR 0119"
description: "周辺知識だけを扱うレッスンを演習のない動画のみのレッスンにし、動画を最後まで再生したら完了とする（ADR 0143 で、動画画面に「レッスン一覧へ」を表示しないことにした）"
---

- 日付: 2026-11-21
- 関連仕様: [レッスンの流れ](../spec/lesson-flow/flow.md)、[レッスンの完了](../spec/lesson-flow/completion.md)、[進捗（progress.json）](../spec/lesson-flow/progress.md)、[レッスン定義の形式（lesson.json）](../spec/content-format/lesson-json.md)、[教材作成におけるカリキュラムの決まり](../spec/content-guidelines/curriculum.md)、[教材チェックツール](../spec/content-guidelines/check-tool.md)、[章とレッスンの一覧の読み方](../spec/curriculum/chapters/how-to-read.md)、[動画画面](../spec/screens/video-screen.md)、[レッスン一覧画面](../spec/screens/lesson-list.md)、[トップページ（ダッシュボード）](../spec/screens/top-page.md)
- 置き換え: なし

## 背景

レッスンは「動画 1 本 + 演習 1 問」で（[ADR 0060](0060-switch-lessons-from-slides-to-videos.md)）、演習に正解するとレッスンが完了する。演習は、主要素を正しく理解していないと解けない問題にする決まりである。

カリキュラムで周辺知識を扱うことにしたとき（[ADR 0118](0118-teach-background-knowledge-hidden-by-gc-languages.md)）、Claude は「概念だけのレッスンでは演習が作りにくい」として、レッスンを増やさずに既存のレッスンへ書き足す案を出した。これに対してユーザーは次のように示した。「演習なしの動画学習だけのレッスンでも良さそうな気がしましたが、それでもできそうですか？」

Claude が、理由は「概念の説明は演習問題にしにくく、動画で理解できれば十分だから」という理解で合っているかを確かめ、ユーザーは「理由もそれで問題ないです」と答えた。

## 決定

- 周辺知識だけを扱うレッスンを、演習のない動画のみのレッスンにする（[教材作成におけるカリキュラムの決まり](../spec/content-guidelines/curriculum.md)、[レッスン定義の形式（lesson.json）](../spec/content-format/lesson-json.md) の `kind`）。
  - 理由: 概念の説明は演習問題にしにくく、動画で理解できれば十分だから。
- 動画のみのレッスンは、動画を最後まで再生したときに完了とする（[レッスンの完了](../spec/lesson-flow/completion.md)）。Claude が提案し、ユーザーが採用した。
  - 理由: 「演習へ」の表示に使っている `video_watched` をそのまま使える。
- 次の細部は Claude が決めた。
  - 動画のみのレッスンでは、動画を最後まで再生したら、動画画面の「演習へ」の位置に「レッスン一覧へ」「次のレッスンへ」を表示する（[レッスンの流れ](../spec/lesson-flow/flow.md)、[動画画面](../spec/screens/video-screen.md)）。理由: これらのボタンは正解の後にだけ表示していて、演習のないレッスンでは次へ進む場所がない。
  - 完了の日時と回数は、最初に最後まで再生したときだけ記録する（[進捗（progress.json）](../spec/lesson-flow/progress.md)）。理由: 動画の見直しを「やり直し」として数える使い道がない。
  - 動画のみのレッスンにできるのは、章とレッスンの一覧で動画のみと定めたレッスンだけとし、教材チェックツールで照合する（[章とレッスンの一覧の読み方](../spec/curriculum/chapters/how-to-read.md)、[教材チェックツール](../spec/content-guidelines/check-tool.md)）。理由: 演習を作る手間を避けるために、Rust の要素を扱うレッスンが動画のみになることを防ぐ。
  - レッスン一覧では「動画のみ」と表示する（[レッスン一覧画面](../spec/screens/lesson-list.md)）。理由: 章末復習レッスンを「復習」と表示しているのに合わせる。
  - 学習記録がないときのトップページの文言を「演習に正解すると、ここに分析が表示されます」に変える（[トップページ（ダッシュボード）](../spec/screens/top-page.md)）。理由: 動画のみのレッスンを完了しても学習記録はできないので（[ADR 0120](0120-video-only-lessons-have-no-elements-or-attempt-sessions.md)）、「レッスンを完了すると」では事実と合わなくなる。

## 検討した他の案

- 案: 周辺知識を扱うレッスンにも演習を付ける
  - 採用しなかった理由: 概念の説明は演習問題にしにくい。動画で理解できれば十分である

## 影響

- 「動画 1 本 + 演習 1 問」（ADR 0060）に当てはまらないレッスンが、章末復習レッスン（演習のみ）のほかにもう 1 種類できる。
- 再生位置を移して動画の末尾に着いた場合も、最後まで再生したものとするので（[レッスンの流れ](../spec/lesson-flow/flow.md)）、動画を見なくても完了にできる。受講順を制限していない（[ADR 0018](0018-no-lesson-order-lock.md)）のと同じく、学習者に任せる。
- 動画のみのレッスンも、完了レッスン数と章ごとの完了数に数える。
- 周辺知識を理解できたかどうかは、そのレッスンでは確かめられない。続くレッスンの演習で間接的に確かめることになる。
