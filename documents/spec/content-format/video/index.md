# レッスンの動画の形式（video/）

* [動画を構成するファイル](structure.md) - `video/` に置くファイル（`script.json`・`index.tsx`・`voice/`・`durations.json`・`cue-reference.json`）とそれを書く人、Remotion の Player での描画、画面の比率とフレームレート、再生時のテーマと章の指定、レッスン専用の CSS をそのレッスンの動画の中に閉じ込めることを定める。
* [動画の台本（script.json）](script-json.md) - `script.json` の `ai_note`・`code_blocks`・`scenes`・`sentences` の各フィールドと、ID・文の並び・文字数の数え方の決まりを定める。
* [動画の音声](voice.md) - VOICEPEAK で作る音声ファイルの形式と置き場所、声の設定（`voice.json`）、読みの対応表（`readings.json`）、音声の長さ（`durations.json`）、音声を作るツールの動作を定める。
* [動画のタイムラインと合図](timeline-and-cues.md) - 台本と `durations.json` からシーンと文の開始位置を決める方法と、絵を動かす位置を指す合図の関数 `at`・`endOf`、合図を合わせたときの音声の長さ（`cue-reference.json`）を定める。
* [動画のシーンの書き方](scenes.md) - シーンを React（TSX）の部品として書くときの決まり（`index.tsx` の default export、現在のフレームだけから絵を決めること、画像と図の扱い）と、共通部品（`useCues()`・`useLesson()`・`LessonTitle`・`CodeBlock`・`Camera`）を定める。
* [動画でのコードの表示](code-display.md) - `code_blocks` の `language` ごとの表示、行番号と Diff の表示、共通部品 `CodeBlock` の指定（`scopes`・`marks`・`errors`・`lines`）とその制約を定める。
* [動画の色の指定](colors.md) - 動画のコードで使う色クラス（`c-primary`・`c-accent`・`c-ok`・`c-ng`・`c-muted`）と SVG 用のクラス、`callout` とマーカーの色、章カラーの決まり方を定める。
* [動画のコードの禁止事項](prohibited-code.md) - 教材チェックツールの lint でエラーにする動画のコード（色の直接指定、Tauri の API・ネットワーク・ファイルへのアクセス）を定める。
