---
type: Verification Report
title: "レッスンごとの動画の遅延読み込みの技術検証"
description: "レッスンごとに動画のコード・音声・画像を遅延読み込みできるかを Issue #4 で検証した結果（Vite のプラグインで作るレッスンごとの仮想モジュール、塊の一覧、tauri:// で配信されたファイルの記録、音声の置き方 2 通りの比較、tauri:// が Range に 200 で全体を返すこと）を記す。"
---

- 日付: 2026-11-21
- Issue: [#4](https://github.com/Progrust/progrust-app/issues/4)
- 確かめた OS: macOS 27.2.1（Apple M1）だけ。macOS 13 の実機と Windows（WebView2）は確かめていない
- 関連仕様: [責務の分担](../spec/architecture/responsibilities.md)、[動画のコードの構成](../spec/content-format/video/structure.md)、[動画のシーン](../spec/content-format/video/scenes.md)、[教材のディレクトリ構成](../spec/content-format/directory-structure.md)、[データの置き場所](../spec/architecture/data-locations.md)、[ADR 0061](../decisions/0061-render-videos-with-remotion-player.md)、[ADR 0062](../decisions/0062-write-video-scenes-in-unrestricted-tsx.md)

## 環境

- macOS 27.2.1、Apple M1。WebKit.framework 22625.2.8.11.9（Safari 27.2 相当）。rAF 58.8Hz。
- rustc 1.99.1、node v25.9.0、Tauri CLI 2.12.6、tauri 2.12.7（feature `protocol-asset`）、@tauri-apps/api 2.12.5。
- Vite 8.3.8（rolldown 1.2.19、lightningcss 1.33.0）、React 19.3.1、TypeScript 6.0。
- remotion・@remotion/player・@remotion/media 4.0.545。
- 計測はすべて release ビルド（`npx tauri build --bundles app`）の .app の実行ファイルを、環境変数を付けて起動して行った。ページ（tauri://localhost）が自分で計測し、Tauri のコマンドで JSON に書いて終了する。
- 計測の前に毎回、他の rustc が動いていないことを確かめた。ウィンドウは前面に出してフォーカスした（最前面への固定はしていない）。全回で `visibilityState=visible`、`hasFocus=true`、rAF の間隔の中央値は 17ms で、間引かれた回はなかった。

## 方法

### 作ったもの

Issue #1〜#3 の試作のアプリを写し、仕様（教材のディレクトリ構成、リポジトリの構成）の形に並べ直した。

```
content/assets/common-mark.png          全レッスン共通の画像（59KB）
content/lessons/ch04-move/              試作のレッスン（第4章）
  lesson.json  assets/scene-ch04.png（7.5KB）
  video/index.tsx scenes/ components/ timeline.ts script.json durations.json voice/*.m4a（21 文）
content/lessons/ch02-move-b/            複製 B（題名・画像（0.9KB）・音声を変えた）
content/lessons/ch07-short/             複製 C（表紙とまとめの 2 シーン、5 文）
src/video/                              共通部品（Stage・Camera・CodeBlock・anim・fonts・CSS）と LessonComposition.tsx
```

- レッスンのシーンは `@video/anim` などの共通部品を別名 `@video` → `src/video` で import し、画像は `../../assets/x.png` と `../../../../assets/common-mark.png` で import する（仕様どおり）。
- アプリ側の `LessonComposition.tsx` が、シーンを台本の順に `Sequence` で並べ、音声（`@remotion/media` の `Audio`、または `Html5Audio`）を鳴らす。
- 台本は、仕様ではバックエンドが `get_lesson` で渡すが、この試作ではレッスンの中で import した（合図の計算をモジュールの評価時に行うため）。

### レッスンの塊を作る Vite のプラグイン

`vite.config.ts` に約 60 行のプラグインを置き、仮想モジュールを 2 種類作った。1 つはレッスンの一覧（本体に入る）、もう 1 つはレッスンごとの塊の入口である。

```ts
// virtual:progrust-lessons（本体に入る一覧。中身を読み込むのは load() を呼んだときだけ）
export const lessons = {
  "ch04-move": { id: "ch04-move", chapter: 4, title: "…", load: () => import("virtual:progrust-lesson/ch04-move") },
  …
};
// virtual:progrust-lesson/ch04-move（レッスンの塊の入口。プラグインが content/lessons/*/ を読んで作る）
import * as mod from "/content/lessons/ch04-move/video/index.tsx";
import v0 from "/content/lessons/ch04-move/video/voice/h1.m4a?url";   // … 21 文
import c0 from "/content/lessons/ch04-move/video/probe.shadow.css?inline";
export const voice = { "h1": v0, … };
export const shadowCss = [c0];
```

- アプリは `await lessons[id].load()` の後に Player を出す（`durationInFrames` を timeline から決めるため）。
- 音声の URL を `?url` で import するので、その URL はレッスンの塊の中に入り、本体には入らない。

### 計測

アプリに計測用の画面を作り、次の順に動かした。

- レッスンを出さずに 1 秒待つ。
- ch04・ch02・ch07 を順に表示する（表紙で一時停止し、2.5 秒再生する）。
- 最後に ch04 をもう一度表示する。
- 続けて、音声の置き方 2 通り × 音声の部品 2 通りで再生する。

この WKWebView では `performance.getEntriesByType('resource')` が tauri:// のファイルを 1 件も記録しなかった（全段階で 0 件）。そのため、読み込んだファイルの証拠は次の 3 つにした。

- バックエンドの `on_web_resource_request`（tauri:// で配信したファイルの記録）。主な証拠。
- ページの `fetch` の記録。
- レッスンのモジュールが評価された記録（`window.__lessonEval`）。

## 結果

### レッスンごとの塊に分けられるか

分けられた。`vite build` の出力（`.vite/manifest.json` と、各レッスンの index.tsx に入れた目印の文字列 `LESSON-MARKER-{id}` の grep）は次のとおり。

| ファイル | 大きさ | 中身 |
|---|---|---|
| `index-*.js`（本体） | 1,204KB | アプリ・Remotion・共通部品・デザインシステムの CSS。目印の文字列はなく、レッスンの一覧（id・章・題名・`import()`）だけ |
| `_progrust-lesson_ch04-move-*.js` / `.css` | 26.5KB / 0.55KB | ch04 のシーン・部品・台本・音声の URL 21 件・画像の URL。レッスン専用の CSS |
| `_progrust-lesson_ch02-move-b-*.js` / `.css` | 27.7KB / 0.56KB | 同上。4KB 未満の画像（0.9KB）は data: URI でこの塊の中に入った（`assetsInlineLimit` 4096） |
| `_progrust-lesson_ch07-short-*.js` / `.css` | 12.2KB / 0.55KB | 同上（5 文） |
| `CodeBlock-*.js` | 1.1KB | ch04 と ch02 が共有する共通部品（ch07 は使わないので読まない） |
| `common-mark-*.js` | 0.4KB | 全レッスンが使う anim と、共通の画像の URL |
| `assets/*.m4a`・`*.png` | | 音声 47 個と 4KB 以上の画像。ハッシュ付きの名前で 1 ファイルずつ |

- 共通部品は塊ごとに重複しない（rolldown が共有の塊にまとめた）。
- 同じ中身のファイルは同じハッシュ名になるので、2 つのレッスンに同じ音声ファイルを置くと 1 つにまとまる。そのため検証では、音声の中身をレッスンごとに変えた。

### そのレッスンの動画を表示するときだけ読み込まれるか

読み込まれた。バックエンドが tauri:// で配信したファイルの記録は次のとおり。

| 段階 | 配信されたファイル | 評価されたレッスン |
|---|---|---|
| 起動直後 | `index.html`、`index-*.css`、`index-*.js`、書体 2 つ | なし |
| 1 秒後（まだ表示していない） | なし | なし |
| 1. ch04 を表示（表紙） | `common-mark-*.js`、`CodeBlock-*.js`、`_progrust-lesson_ch04-move-*.css`・`.js`、`scene-ch04-*.png`、`common-mark-*.png` | ch04 |
| 1. ch04 を再生 | `h1-*.m4a`（再生した文だけ） | ch04 |
| 2. ch02 を表示 | `_progrust-lesson_ch02-move-b-*.css`・`.js`（共有の塊は読み直さない。画像は data: URI） | ch04・ch02 |
| 2. ch02 を再生 | ch02 の `h1-*.m4a` | |
| 3. ch07 を表示 | `_progrust-lesson_ch07-short-*.css`・`.js` | ch04・ch02・ch07 |
| 4. ch04 をもう一度表示 | なし（モジュールはキャッシュ済み） | |

- 5 回流し、5 回とも段階ごとの配信と評価が上の表と同じだった。画像の表示と音（RMS の最大 0.16〜0.21）も同じだった。
- 表示までの時間（`load()` から Player が出るまで）は、最初の ch04 が 45〜56ms、それ以降は 9〜23ms だった。
- 一度読んだレッスンの JS と CSS は、閉じても文書に残る（`<link rel=stylesheet>` が head に残る。[Issue #5 の検証結果](issue-5-scope-lesson-css.md)に関係する）。

### 本番ビルドでアセットの URL が解決でき、再生できるか

できた。

- 画像: `tauri://localhost/assets/scene-ch04-CD7Qp2Ys.png`、`common-mark-Cr4VpRNL.png`、小さい画像の data: URI のどれも、`complete=true` で、`naturalWidth` が元の幅（320・240）だった。
- 音声: 3 本とも `@remotion/media` の `Audio` で鳴った（2.5 秒の再生で AnalyserNode の RMS の最大が 0.16〜0.21、RMS > 0.01 の rAF が 61〜87%）。フレームは 194 → 268 と進んだ。

### 音声の置き方 2 通りの比較

ch04 の文 h1（52,755 バイト）を 2.5 秒再生した。

| | Vite のアセット（`?url`。`tauri://localhost/assets/h1-*.m4a`） | Tauri のリソース + `convertFileSrc`（`asset://localhost/…/Contents/Resources/content/lessons/ch04-move/video/voice/h1.m4a`） |
|---|---|---|
| `@remotion/media` の Audio | 鳴る（RMS 最大 0.164）。`fetch` は `Range: bytes=0-` に **200** で全体を返す。mediabunny が警告を出す（「did not respond to a range request with 206 Partial Content…」） | 鳴る（RMS 最大 0.164）。**206**（`Content-Range: bytes 0-52754/52755`）。警告なし |
| `Html5Audio`（`<audio>`） | 鳴る（currentTime が 2.46〜2.47 秒進む）。ただし、Player を出してシークし 2.5 秒再生する間（約 3 秒）に、同じファイル（52KB）の全体を **81 回**配信した（5 回とも 81 回）。`<audio>` が範囲を要求し、tauri:// が毎回 200 で全体を返すためとみられる（推測） | 鳴る（2.46 秒進む）。配信の繰り返しは観測していない（asset:// は `on_web_resource_request` の対象外なので回数は数えられない。Range には 206 で答える） |
| `bytes=0-99` の要求 | 200・52,755 バイト（範囲を無視） | 206・100 バイト |
| 必要な設定 | なし | `app.security.assetProtocol`（`enable: true`、`scope: ["$RESOURCE/**"]`）、Cargo の `tauri` の feature `protocol-asset`、`bundle.resources`。URL はアプリの絶対パスを含む |
| 仕様との関係 | 仕様（責務の分担）どおり | 仕様（音声はフロントエンドのビルドに含める）と違う。フロントエンドがリソースのパスを知る必要がある |

- 繰り返しは、読み込みとシークのときに起きているとみられる。読み込みとシークが済んだ後の 5 秒の再生中の配信は、[Issue #6 の検証結果](issue-6-switch-theme-during-playback.md)の計測で 3 回だった。
- 1 文 30〜60KB では、どちらの置き方でも再生に問題は見えなかった。

### 音声の URL の持ち方（ビルドだけで比べた。47 個）

- アプリ側の `import.meta.glob` で `eager: true`: 全レッスンの URL（1 件約 77 バイト、計 3.6KB）が本体に入る。
- `eager` なし: 音声 1 つにつき 54 バイトの小さな塊が 47 個できる。本体には 6.8KB の表が入る。
- 上のプラグインの方法: URL はそのレッスンの塊に入り、本体には入らない。

### バックエンドからのリソースの読み取り

バックエンドがリソースの `content/lessons/ch04-move/video/script.json` を読めることも確かめた（`get_lesson` の代わりのコマンド）。`bundle.resources` に `content/` を丸ごと入れたので、TSX と音声もリソースに入った。本番では [データの置き場所](../spec/architecture/data-locations.md)のとおり除く必要がある。

## 判定

基準「本番ビルドで、開いたレッスンの塊だけが読み込まれ、動画が再生できる」は、macOS で満たした。Windows は確かめていない。

## 残った事項

- Windows（WebView2）は確かめていない。macOS 13 の実機も確かめていない。
- tauri:// は Range リクエストに 200 で全体を返す。`Html5Audio` では、読み込みとシークのときに同じ音声を何十回も全体で配信した（約 3 秒で 81 回、52KB × 81 ≒ 4.3MB）。Issue #2 の結果（1 文 30〜50KB のファイルでは問題は出なかった）には、配信の回数の観点がなかった。
- ユーザーの判断が要る点: 音声の置き方。仕様どおり Vite のアセット（tauri://）にするか、Tauri のリソース + `convertFileSrc`（asset://）にするか。asset:// は 206 で答えるが、仕様の役割分担（音声はフロントエンドのビルドに含める）とアセットのスコープの設定が変わる。
- 仕様の変更案
  - [動画のコードの構成](../spec/content-format/video/structure.md)・[動画のシーン](../spec/content-format/video/scenes.md)に、アプリがシーンと音声を並べるには、レッスンの塊ごとに「音声の URL の表」と（Shadow DOM のときは）「CSS の文字列」が要ることと、それを作るビルドの仕組み（Vite のプラグインで仮想モジュールを作る）を、どこかに書く必要がある（置き場所の候補は技術スタックか責務の分担）。
  - 試作の timeline.ts はレッスンごとに間（LEAD・TAIL・GAP）を持つ。仕様の `useCues()` はアプリが合図を計算する前提で、この検証ではレッスンごとの timeline のままにしたので、未確認。
  - [データの置き場所](../spec/architecture/data-locations.md): リソースに `content/` を入れるとき、動画のコードと音声を除く設定が要る。
- この WKWebView の `performance.getEntriesByType('resource')` は tauri:// のファイルを記録しない。アプリの性能の計測や遅延読み込みの確認で、Resource Timing に頼れない。
- Vite 8 の既定のビルドの対象は `safari16.4`（CSS の lightningcss も同じ）。macOS 13.0〜13.2 の Safari 16.1〜16.3 はこれより古い。macOS 13 を対応環境に含めるなら `build.target` を決める必要がある（Issue #1〜#3 の試作も既定のまま）。
