---
type: Specification
title: "動画を構成するファイル"
description: "`video/` に置くファイル（`script.json`・`index.tsx`・`voice/`・`durations.json`・`cue-reference.json`）とそれを書く人、Remotion の Player での描画、画面の比率とフレームレート、再生時のテーマと章の指定を定める。"
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
- 見た目（書体・色・余白・部品）は、動画のデザインシステム（[画面の見た目とテーマ](../../screens/appearance.md)「デザインシステム」）に従う。アプリは、動画のデザインシステムの CSS（トークンと部品のクラス）を動画に適用する。
- 再生の操作と字幕は [動画画面](../../screens/video-screen.md) に定める。
