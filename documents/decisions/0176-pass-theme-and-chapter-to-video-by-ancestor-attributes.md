---
type: Decision
title: "ADR 0176"
description: "再生するときのテーマと章カラーを、動画を描く要素の祖先の `data-theme`・`data-chapter` と CSS 変数の継承で動画に渡し、`useLesson()` の材料になる章とレッスン定義は Player の `inputProps` で渡す"
---

- 日付: 2026-11-23
- 関連仕様: [動画を構成するファイル](../spec/content-format/video/structure.md)
- 置き換え: なし

## 背景

アプリは、再生するときにテーマ、章、レッスン定義を指定し、動画のコードにはテーマも章も書かない（[動画を構成するファイル](../spec/content-format/video/structure.md)）。指定する方法（祖先の要素の属性と CSS 変数の継承で渡すか、Player の入力の値として渡すか）は、未決事項として残っていた。実装計画の M1 で、動画画面を作るために決める必要があった。

[再生中のテーマと章カラーの切り替えの技術検証](../verification/issue-6-switch-theme-during-playback.md) では、CSS 変数の継承、`inputProps`、毎回新しい `inputProps` の 3 通りのどれでも、再生を止めずにテーマと章カラーが切り替わった。継承は React の描き直しがなく最も軽いが、章の違う 2 本の動画を同じ祖先の下に置くときは、Player ごとの囲みが要る。Shadow DOM を使うときは、章を `inputProps` で渡す必要がある。レッスン専用の CSS は、Shadow DOM ではなく、ビルド時に属性セレクタを付けて閉じ込めると決まっている（[ADR 0093](0093-scope-lesson-css-with-build-time-prefix.md)）。

## 決定

- テーマと章カラーは、動画を描く要素の祖先の `data-theme`・`data-chapter` で指定し、動画のデザインシステムの CSS 変数を継承させる。動画は 1 つずつ `data-chapter` を付けた要素で囲む（[動画を構成するファイル](../spec/content-format/video/structure.md)）。Claude が決めた。
  - 理由: 技術検証で最も軽く、テーマを変えても Player を描き直さない。動画を使わない画面と同じ `data-theme`・`data-chapter` の仕組み（デザインシステムの README「章カラー」）で色が決まる。Shadow DOM を使わないので、継承で章カラーが届く。
- 共通部品 `useLesson()` が返す章の番号・章名・レッスンの番号・レッスン名の材料（章とレッスン定義）は、Player の `inputProps` で渡す。Claude が決めた。
  - 理由: これらは CSS ではなく、シーンの部品が値として読むものなので、CSS 変数では渡せない。レッスンを開いている間は変わらないので、描き直しは起きない。

## 検討した他の案

- 案: テーマと章カラーも `inputProps` で渡す
  - 採用しなかった理由: テーマを変えるたびに Player の中が描き直される。動画の中で、CSS 変数を自分で付け直す処理が要る
- 案: 章を `inputProps` で渡し、テーマだけを継承で渡す
  - 採用しなかった理由: 章カラーのためだけに、動画の中で `data-chapter` を付け直す処理が要る。囲みに `data-chapter` を付ければ足りる

## 影響

- 動画画面とレッスン検索のモーダルなど、同じ画面に違う章の動画を置くときも、囲みごとに章カラーが決まる。
- 動画の中の色が、祖先の要素の CSS 変数で変わる。アプリの画面の CSS が動画の中に効かないようにする決まりは、[ADR 0177](0177-load-both-design-systems-and-write-app-css-with-classes.md) にある。
