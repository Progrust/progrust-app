---
type: Specification
title: "動画を構成するファイル"
description: "`video/` に置くファイル（`script.json`・`index.tsx`・`voice/`・`durations.json`・`cue-reference.json`）とそれを書く人、Remotion の Player での描画、画面の比率とフレームレート、再生時のテーマと章の指定（祖先の要素の属性と `inputProps`）、Player の組み込みの操作を使わないこと、レッスン専用の CSS をそのレッスンの動画の中に閉じ込めることを定める。"
---

1 レッスンの動画を、`video/` の次のファイルで表す。

| ファイル | 内容 | 書くもの |
|---|---|---|
| `script.json` | 台本（[動画の台本（script.json）](script-json.md)） | 作者 |
| `index.tsx` | 動画の入口（[動画のシーンの書き方](scenes.md)） | 作者 |
| そのほかの `.tsx`・`.ts`・`.css` | シーンと、このレッスン専用の部品（[動画のシーンの書き方](scenes.md)） | 作者 |
| `voice/{文の ID}.m4a` | 音声（[動画の音声](voice.md)） | 音声を作るツール |
| `durations.json` | 音声の長さ（[動画の音声](voice.md)） | 音声を作るツール |
| `cue-reference.json` | 合図を合わせたときの音声の長さ（[動画のタイムラインと合図](timeline-and-cues.md)） | 音声を作るツール |

- 動画は、動画ファイルに書き出さない。アプリが Remotion の Player（`@remotion/player`）で描画する。
- 画面は 16:9、30fps とする。作者は 1280×720 の基準サイズで内容を作り、アプリが表示領域に合わせて拡大・縮小する。
- 動画は、シーンを台本の `scenes` の順に並べたものである。シーンと文の位置は、音声の長さから決まる（[動画のタイムラインと合図](timeline-and-cues.md)）。
- アプリは、再生するときにテーマ（ライト／ダーク）、章、レッスン定義（[レッスン定義の形式（lesson.json）](../lesson-json.md)）を指定する。動画のコードには、テーマも章も書かない。
  - テーマと章カラーは、動画を描く要素の祖先の要素の属性で指定する。テーマは `data-theme`（`light`・`dark`）、章カラーは `data-chapter`（章の番号から [画面の見た目とテーマ](../../screens/appearance.md)「デザインシステム」の決まりで選ぶ値）とし、動画のデザインシステムの CSS 変数を、動画の中へ継承させる。テーマと章カラーを変えても、Player を描き直さない。
  - 動画を 1 つずつ、`data-chapter` を付けた要素で囲む。同じ画面に違う章の動画を置くとき（動画画面とレッスン検索のモーダルなど）も、それぞれの動画の章カラーになる。章に属さずに表示するとき（[動画の色の指定](colors.md)）は、`data-chapter` を付けない。
  - 章の番号・章名・レッスンの番号・レッスン名（共通部品 `useLesson()`。[動画のシーンの書き方](scenes.md)）の材料になる章とレッスン定義は、Player の `inputProps` で渡す。
- アプリは、Remotion の Player の組み込みの操作を使わない。Player の操作の表示（`controls`）、Space キーでの再生と一時停止（`spaceKeyToPlayOrPause`）、クリックでの再生と一時停止（`clickToPlay`）、ダブルクリックでの全画面表示（`doubleClickToFullscreen`）を無効にする。末尾まで再生したら、先頭に戻さず（`moveToBeginningWhenEnded` を無効にする）、繰り返さない（`loop` を無効にする）。再生の操作は、すべてアプリの部品（[動画画面](../../screens/video-screen.md)）で行う。
- 見た目（書体・色・余白・部品）は、動画のデザインシステム（[画面の見た目とテーマ](../../screens/appearance.md)「デザインシステム」）に従う。アプリは、動画のデザインシステムの CSS（トークンと部品のクラス）を動画に適用する。アプリの画面の CSS と動画のデザインシステムの CSS がそれぞれ効く範囲は、[画面の見た目とテーマ](../../screens/appearance.md)「デザインシステムの CSS とアイコン」に従う。
- アプリは、各レッスンの動画を、`data-lesson="{レッスン ID}"` の属性を持つ要素の中に描く。
- レッスン専用の CSS は、ビルド時に、すべてのセレクタの前に `[data-lesson="{レッスン ID}"] ` を付けたものに変える。`:root` は `[data-lesson="{レッスン ID}"]` に置き換える。レッスン専用の CSS は、そのレッスンの動画の中だけに効き、ほかのレッスンの動画とアプリの画面には効かない。
- 再生の操作と字幕は [動画画面](../../screens/video-screen.md) に定める。
