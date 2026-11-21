---
type: Verification Report
title: "Remotion Player の WebView での動作と性能の技術検証"
description: "Remotion Player が Tauri の WKWebView で動き、再生開始 300ms・シーク 200ms の目安を満たすかを Issue #1 で検証した結果（ページ自身が計測して Tauri のコマンドで書き出す方法、再生開始とシークの表、1 倍と 2 倍の通しの再生のコマ落ち、AudioDecoder を消してまねた macOS 13 の状態、2 倍速で 1 フレームずつ飛ぶこと、再生中のシークの後の停止、それらを人が見て問題ないとした判断）を記す。"
---

- 日付: 2026-11-21
- Issue: [#1](https://github.com/Progrust/progrust-app/issues/1)
- 確かめた OS: macOS 27.2.1（Apple M1）
- 関連仕様: [動画を構成するファイル](../spec/content-format/video/structure.md)、[性能の目安](../spec/non-functional/performance.md)、[対応環境](../spec/non-functional/environments.md)、[ADR 0061](../decisions/0061-render-videos-with-remotion-player.md)、[ADR 0062](../decisions/0062-write-video-scenes-in-unrestricted-tsx.md)

## 環境

- macOS 27.2.1、Apple M1、メモリ 16GB。内蔵ディスプレイ（devicePixelRatio 2、rAF 58.8Hz = 60Hz）。
- WebKit.framework 22625.2.8.11.9（Safari 27.2 と同じ）。WKWebView の `navigator.userAgent` は `Intel Mac OS X 10_15_7 ... AppleWebKit/605.1.15` で固定のため、UA からは版が分からない。版は `/System/Library/Frameworks/WebKit.framework/Resources/Info.plist` の `CFBundleVersion` で調べた。
- rustc 1.99.1、node v25.9.0、npm 11.12.1、ffmpeg 9.0.2、afconvert（macOS 付属）。
- Tauri CLI 2.12.6、tauri 2.12.7、wry 0.57.3、@tauri-apps/api 2.12.5、React 19.3.1、Vite 8.3.8、TypeScript 6.0。
- remotion・@remotion/player・@remotion/media 4.0.545、mediabunny 1.66.0（@remotion/media が使う）。
- 計測時の load average は 1.3〜6.8（ほとんどは 2〜4）。各回の前に rustc が動いていないことを確かめた。
- 計測時のウィンドウの状態: 全回（perf 23 回、sync 8 回、smoke 3 回、controls 2 回、pitch 1 回）で、計測の前後とも `visibilityState=visible`、`hasFocus=true`、`NSWindow.occlusionState` が Visible、`NSApp.isActive=true`、rAF 58.8Hz だった。
- 同じ時間帯に、別のエージェントが最前面に固定した別の Tauri ウィンドウで計測していた。他のウィンドウの描画が止まることがあったと報告があったため、全回の rAF の間隔を確かめた。通しの再生の間隔の最大は 19〜45ms、別の計測の全サンプルでも最大 29〜45ms で、50ms を超えた間隔は 0 回だった。描画が止まった回はなく、除外した回はない。

## 方法

### 作ったもの

Tauri v2 + React + TypeScript + Vite の最小アプリ（`npm create tauri-app -- --template react-ts`）に、事前に作ったレッスンの試作（第 4 章 所有権のムーブ、3175 フレーム = 1 分 45.8 秒、台本 21 文）の `src/` の写しを置き、`@remotion/player` の `Player` で再生した。レッスンのコンポーネントは次の 2 点だけ書き直した。

- 音声の部品とファイルを inputProps で切り替える。部品は `@remotion/media` の `Audio`、`remotion` の `Html5Audio`、なし。ファイルは ffmpeg の m4a、afconvert の m4a、WAV。
- 描画の判定用に、現在のフレーム番号を `data-probe-frame` に書く 1px の透明な要素を置いた。

動画画面の試作は、`<Player controls={false} clickToPlay={false} spaceKeyToPlayOrPause={false} moveToBeginningWhenEnded={false} playbackRate={rate} preservePitch />` の外に、仕様の操作を作った（内容は Issue #3 の結果に書く）。

### 計測の仕組み

- 計測は release ビルド（`npx tauri build --bundles app`）の .app の中の実行ファイルを、環境変数（`PROGRUST_BENCH=perf` など）を付けて起動して行った。
- ページ（`tauri://localhost`）が自分で計測し、結果を Tauri のコマンドで JSON ファイルに書き出し、自分でアプリを終了する。外から画面を撮ったり操作したりしない。
- 起動時と計測の前に、ウィンドウを前面に出して常に手前に置く（画面が隠れて描画が止まるのを避けるため）。
- 音声は ffmpeg で変換した（Player で使う形式）。

```sh
ffmpeg -i x.wav -c:a aac -b:a 64k -ac 1 x.m4a        # AAC-LC、モノラル、48kHz、実効 64.3〜64.6kbps
afconvert -f m4af -d aac -b 64000 -c 1 x.wav x.m4a   # 比較用
```

### 「絵が表示された」の定義

DOM が変わった時点ではなく、その後の rAF（その rAF のあとに画面へ描かれる）までを「絵が表示された」とした。具体的には、`data-probe-frame` の値が変わったのを `MutationObserver` で見た時刻を「DOM 反映」、その直後の rAF の時刻を「描画」と呼ぶ。表の「描画」は後者である。画面に出るまでのディスプレイの遅れ（1 フレーム程度）は含まない。

- 再生開始: 一時停止の状態で `player.play()` を呼んだ時刻から、フレームが変わって「描画」になるまで。`play` イベントと `frameupdate` イベントの時刻も記録した。
  - 音の始まりは、`@remotion/media` では AudioContext の出力に並列につないだ AnalyserNode で RMS が 0.01 を超えた時刻、Html5Audio では `<audio>` の `currentTime` が進み始めた時刻。
  - 「コールド」は起動して最初の再生（先頭から。最初の 1.3 秒は無音）。「ウォーム」は 2 回目以降で、10 文の途中の声が出ている位置から。
- シーク: `seekTo(target)` を呼んでから、`data-probe-frame` が target になり、その直後の rAF までを測る。一時停止中に 20 か所（シーンの頭と真ん中、ほかは乱数）、再生中に 10 か所。再生中のシークでは、シーク先でまた絵が動き出すまでも測った。
- コマ落ち: 通しの再生（1 倍で全 3175 フレーム、2 倍で図解のシーンの 900 フレーム）で、rAF ごとに表示中のフレームを読み、表示されたフレームの数と期待の比、フレーム番号の飛び、同じフレームが続いた時間を数えた。
- 繰り返し: 音声の部品ごとに 5 回起動した（`@remotion/media`・Html5Audio が各 5 回、音声なしが 3 回）。表の値は全回を合わせた中央値と最大。

### 人が見た確認

計測のあと、ユーザーが release ビルドの試作を、人が触る画面で起動して操作した。計測で数値の出た 2 点（2 倍速のコマ飛び、再生中のシークの後の停止）が見た目で気になるかを、再生しながら確かめた。速度は試作の速度の選択で 2 倍にし、再生中のシークはシークバーのクリックとキー操作（←→、Shift+←→）で行った。

## 結果

### 動くか

試作のレッスンが、release ビルドの WKWebView で再生できた。音声の部品は、`@remotion/media` の `Audio`、`Html5Audio`、なしのどれでも動いた。

### 再生開始（目安 300ms 以内）

単位は ms。中央値 / 最大。

| 音声の部品 | コールドの描画（5 回） | ウォームの DOM 反映（50 回） | ウォームの描画（50 回） | ウォームの音の始まり |
|---|---|---|---|---|
| `@remotion/media` の Audio | 114 / 130 | 99 / 117 | 114 / 132 | 40.5 / 100（AnalyserNode） |
| Html5Audio | 129 / 130 | 49 / 83 | 65 / 98 | 31 / 50（currentTime が進み始める） |
| なし（3 回、30 回） | 114 / 129 | 84 / 118 | 99 / 131 | なし |

- 最大でも 132ms で、目安の 300ms に収まる。`play` イベントは 1〜5ms で出る。
- `@remotion/media` では、再生開始のときだけ、音が絵より約 60ms 先に出始める。

### シークしてから絵が表示されるまで（目安 200ms 以内）

単位は ms。中央値 / 最大。

| 音声の部品 | 一時停止中の描画（100 回。なしは 60 回） | 再生中の描画（50 回。なしは 30 回） | 再生中: シーク先で絵がまた動き出すまで |
|---|---|---|---|
| `@remotion/media` | 7.5 / 15 | 15 / 19 | 114 / 161 |
| Html5Audio | 7 / 15 | 34 / 74 | 129 / 162 |
| なし | 7 / 15 | 16 / 20 | 112.5 / 132 |

- 最大は 74ms で、目安の 200ms に収まる。
- 再生中にシークすると、シーク先のフレームは 20〜74ms 以内に描かれる。そのあと、絵がまた動き出すまで 110〜160ms 止まる。音声なしでも同じなので、Player 自身の動き（再生の時刻の合わせ直し）とみられる。

### 通しの再生のコマ落ち

| | `@remotion/media`（5 回） | Html5Audio（5 回） | なし（3 回） |
|---|---|---|---|
| 1 倍: 表示されたフレーム / 期待 | 3175 / 3175（全回） | 3175 / 3175（全回） | 3175 / 3175（全回） |
| 1 倍: フレーム番号の飛び | 0（全回） | 0（全回） | 0（全回） |
| 1 倍: 同じフレームが続いた最長 | 52〜99ms | 52〜78ms | 38〜54ms |
| 1 倍: 再生にかかった時間（期待 105.80 秒） | 105.86〜105.92 秒 | 105.83〜105.86 秒 | 105.85〜105.87 秒 |
| 2 倍: 表示されたフレームの比 | 0.74〜0.95 | 0.75〜0.88 | 0.75〜0.79 |
| 2 倍: 飛びの最大 | 2（1 つ飛ばし） | 2 | 2 |

- 1 倍では、どの回もすべてのフレームを順に表示した。同じフレームが 2 フレーム分（約 33ms）を超えて続いたのは、`@remotion/media` と Html5Audio で各 1 回だけ。
- 2 倍では 60Hz の表示で 60 フレーム／秒が必要になり、rAF の揺れで 5〜26% のフレームが 1 つ飛ばしになる。音声なしでも同じなので、Player と表示のリフレッシュレートによるもの。

### 人が見た結果

- ユーザーが実際に操作して見て、2 倍速のコマ飛びも、再生中のシークの後の停止も、特に気にならなかった。ユーザーは、これで問題ないと判断した。

### macOS 13 の見込み（実機は未確認）

- macOS 13 の WKWebView は、システムの Safari と同じ WebKit を使う。macOS 13 で使える Safari は 16 系から、Ventura で最後の 18.6 まで。Safari 26 は Sonoma・Sequoia・Tahoe だけに出ている（Apple の Safari 26.0 リリースノート、Safari 18.6 のセキュリティ情報）。
- WebCodecs の AudioDecoder・AudioEncoder は Safari 26.0 で入った。macOS 13 の WebKit には AudioDecoder がない。
- mediabunny 1.66.0 の `canDecode()` は、AudioDecoder がないと AAC を false にする（ソースで確認）。そのため `@remotion/media` の Audio は Html5Audio に切り替わる。

AudioDecoder がないことだけをまねるために、この Mac で `window.AudioDecoder` を消してから Player を出した（`noAudioDecoder: true`）。全文で次のログが出て、`<audio>` で鳴った。

```
Cannot decode /voice/xx.m4a, falling back to <Html5Audio>
```

性能は Html5Audio とほぼ同じだった。単位は ms。中央値 / 最大。

| 項目 | 値 |
|---|---|
| コールドの描画（5 回） | 113 / 128 |
| ウォームの描画（50 回） | 65 / 94 |
| 一時停止中のシークの描画（100 回） | 10 / 15 |
| 再生中のシークの描画 | 36 / 79 |
| 1 倍の通し（1 回） | 3175 / 3175 フレーム、飛び 0、同じフレームが続いた最長 53ms |
| 2 倍の通し（1 回） | 表示率 0.78 |

- 古い WebKit の描画の速さや不具合は、この方法では分からない。Intel Mac での値も分からない。

## 判定

| 基準 | 結果 |
|---|---|
| 再生開始まで 300ms 以内 | 満たした（最大 132ms） |
| シークから絵の表示まで 200ms 以内 | 満たした（最大 74ms） |
| 通しの再生でコマ落ちが目立たない | 満たした。1 倍は全フレームを表示した。2 倍は 5〜26% が 1 つ飛ばしになるが、人が見て気にならなかった |

基準をすべて満たしたので、Issue を閉じる。

## 残った事項

判断が要る点: なし。2 倍速のコマ飛びと再生中のシークの後の停止は、ユーザーが見て問題ないと判断した（「人が見た結果」）。

確かめられなかった項目: なし。

検証中に見つけた別の問題（事実のみ）

- `tauri://` のアセットは Range 要求に 206 を返さず、200 で全体を返す。mediabunny が「did not respond to a range request with 206 Partial Content … Reads into evicted regions will throw」と警告する。1 文 30〜50KB のファイルでは問題は出なかった。
- `@remotion/media` では、Player が 1 回の通しの再生で文の数と同じ 21 回の `waiting` イベントを出す。絵は止まらない（1 倍で同じフレームが 100ms を超えて続いたことはない）。`waiting` を「読み込み中」の表示に使うと、文ごとにちらつくおそれがある。
- 試作の `Stage` は `data-theme="light"` を固定で書いていた。テーマの切り替えを実装するときは変える必要がある。
