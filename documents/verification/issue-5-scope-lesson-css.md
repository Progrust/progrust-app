---
type: Verification Report
title: "レッスン専用の CSS の閉じ込め方の技術検証"
description: "レッスン専用の CSS をそのレッスンの動画の中に閉じ込める 5 つの候補（そのまま・CSS Modules・@scope・ビルド時の接頭辞・Shadow DOM）を Issue #5 で検証した結果（候補ごとの閉じ込めの結果の表、デザインシステムの CSS が効くこと、macOS 13 の WebKit の対応表、方法の決定がユーザーの判断になること）を記す。"
---

- 日付: 2026-11-21
- Issue: [#5](https://github.com/Progrust/progrust-app/issues/5)
- 確かめた OS: macOS 27.2.1（Apple M1）
- 関連仕様: [教材データ形式の未決事項](../spec/content-format/open-questions.md)、[動画のシーン](../spec/content-format/video/scenes.md)、[動画のコードの構成](../spec/content-format/video/structure.md)

## 環境

- macOS 27.2.1、Apple M1。WebKit.framework 22625.2.8.11.9（Safari 27.2 相当）。
- rustc 1.99.1、node v25.9.0、Tauri CLI 2.12.6、tauri 2.12.7。
- Vite 8.3.8（lightningcss 1.33.0、target は safari16.4）、React 19.3.1、postcss 8.5.33。
- remotion・@remotion/player 4.0.545。
- 計測は release ビルドの .app を起動して行った（ページが自分で計測して JSON に書く）。ウィンドウは前面に出してフォーカスし、全回で表示・フォーカス済みだった。

## 方法

[Issue #4 の検証結果](issue-4-lazy-load-lesson-video.md)の試作（Vite のプラグインでレッスンごとの塊を作る）に、5 つの候補を載せた。どれも、レッスンの根 `<div data-lesson="{id}">` を Stage が作る前提である。入れ子は `data-theme > data-chapter > data-lesson（レッスンの根） > .slide`。

| 候補 | 作者が書くもの | ビルド・実行時の処理 |
|---|---|---|
| 0. そのまま（比較用） | `x.css` を TSX で import | Vite が塊の CSS として head に入れる |
| 1. CSS Modules | `x.module.css` を `import s from` して `className={s.probe}` | クラス名にハッシュを付ける（`._probe_i6cev_2`） |
| 2. `@scope` | 普通の CSS（`*.scope.css`） | プラグインが `@scope ([data-lesson="{id}"]) { … }` で包む。lightningcss（target safari16.4）の後も `@scope` は残った |
| 3. 接頭辞（ビルド時） | 普通の CSS（`*.prefix.css`） | プラグインが postcss で、全セレクタの前に `[data-lesson="{id}"] ` を付ける。`:root` は `[data-lesson="{id}"]` に置き換える |
| 4. Shadow DOM | 普通の CSS（`*.shadow.css`。TSX では import しない） | アプリが `?inline` で文字列として読む。Player の中のコンポジションで `attachShadow({mode:"open"})` し、`createPortal` でシーンをその中に描く。Shadow Root の中に、デザインシステムの CSS（tokens・slides・video）とレッスンの CSS を `<style>` で入れる |

接頭辞を付けるプラグインの処理の中心は、次のようなものである（概略）。

```ts
// 候補 3: postcss でセレクタに接頭辞を付ける
const sel = `[data-lesson="${id}"]`;
rule.selectors = rule.selectors.map(s =>
  s.trim() === ":root" ? sel : `${sel} ${s}`);
// 候補 2: そのまま包む
css = `@scope (${sel}) {\n${css}\n}`;
```

各候補の CSS には、次の規則を入れた。

- 自分のクラス
- 要素だけのセレクタ（h6・h5・h4・h3・b を候補ごとに分けた）
- デザインシステムの部品のクラス `.callout`（候補ごとに別のプロパティ）
- `:root` の変数
- 候補 0 だけ、アプリのクラス `.app-probe`

レッスンごとに違う値（ch04 = 41、ch02 = 21）を入れ、`getComputedStyle` で、どのレッスンの規則が効いたかを読んだ。手順は 4 つで、ページを読み直さずに続けた。

1. レッスンなし
2. ch04 だけ
3. ch04 を閉じて ch02
4. 2 本同時

## 結果

### 候補ごとの閉じ込め

表の値は、効いた規則のレッスン（41 = ch04、21 = ch02）。「-」は効いていないこと。太字は、自分のレッスン以外（アプリ・別のレッスン）に効いたもの。2 回流し、2 回とも同じだった。

候補 0〜3（light DOM）:

| 手順 | 場所 | 0 クラス | 0 要素 | 0 .callout | 1 クラス | 1 要素 | 2 クラス | 2 要素 | 2 .callout | 3 クラス | 3 要素 | 3 .callout |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| ch04 だけ | アプリ | **41** | **41** | **41** | （名前が違う） | **41** | - | - | - | - | - | - |
| ch04 だけ | ch04 | 41 | 41 | 41 | 41 | 41 | 41 | 41 | 41 | 41 | 41 | 41 |
| ch04 を閉じて ch02 | アプリ | **21** | **21** | **21** | | **21** | - | - | - | - | - | - |
| 2 本同時 | ch04 | **21** | **21** | **21** | 41 | **21** | 41 | 41 | 41 | 41 | 41 | 41 |
| 2 本同時 | ch02 | 21 | 21 | 21 | 21 | 21 | 21 | 21 | 21 | 21 | 21 | 21 |

- `:root { --x }`: 候補 0 は文書全体に効いた（アプリの `<html>` に `"41"`、後から `"21"`）。
- 候補 2 の `@scope` の中の `:root` は、どこにも効かなかった。`:scope` はレッスンの根に効いた。
- 候補 3 の `:root`（`[data-lesson]` に置き換え）は、そのレッスンの根だけに効いた。

候補 4（Shadow DOM）:

| 手順 | 場所 | 4 クラス | 4 要素（b） | 4 .callout | 候補 0〜3 の規則 |
|---|---|---|---|---|---|
| ch04 だけ | アプリ | - | - | - | 候補 0・1 の規則は効く（TSX の import で head に入るため） |
| ch04 だけ | ch04（Shadow の中） | 41 | 41 | 41 | どれも効かない（自分のレッスンにも） |
| 2 本同時 | ch04 / ch02 | 41 / 21 | 41 / 21 | 41 / 21 | 効かない |

- Shadow の中の `:root { --x }` はどこにも効かなかった。`:host { --x }` は Shadow のホストに効いた。
- 文書の `:root` のカスタムプロパティは、継承で Shadow の中にも届いた。

### 候補ごとの分かったこと

- 候補 0（そのまま）: ほかのレッスンの動画とアプリの画面に効く。
  - 一度表示したレッスンの CSS は、閉じても head に残る（Vite の動的 import の CSS は外されない）。3 本を順に開いて閉じた後の `document.styleSheets` に、3 本の `_progrust-lesson_*.css` がすべて残っていた。
  - 表の「ch04 を閉じて ch02」のアプリが 21 なのは、同じ詳細度の ch02 の規則が後から入って勝ったためで、ch04 の規則が消えたからではない。
  - あとから読んだレッスンの規則が、前のレッスンの動画を上書きした（2 本同時の ch04 が 21 になった）。
- 候補 1（CSS Modules）: クラスは閉じ込められる。ただし、要素だけのセレクタ（`h5 {}`）はアプリとほかのレッスンに効いた。
  - デザインシステムのクラス（`.callout` など）を書き換えるには `:global()` が要り、そうするとそれも全体に効く。
  - 作者が `.module.css` だけを使い、要素のセレクタを書かないことを、lint で守る必要がある。
- 候補 2（`@scope`）と候補 3（接頭辞）: 自分のクラス、要素のセレクタ、デザインシステムのクラスの上書きのどれも、そのレッスンの根の中にだけ効いた。アプリとほかのレッスンには、閉じた後・2 本同時でも効かなかった。
  - 候補 2 は `:root` が無効になる。候補 3 は `:root` をレッスンの根に置き換えられる。
- 候補 4（Shadow DOM）: 閉じ込めは最も強い。アプリのグローバルな規則（App.css の `h6 { word-spacing: 7px }`）も Shadow の中には効かなかった（light DOM では 7px が効いた）。ただし次の制約がある。
  - レッスンの CSS は、TSX で import せず、アプリが文字列として集めて Shadow の中に入れる必要がある。TSX で import すると Vite が head に入れて全体に効く。
  - デザインシステムの CSS（約 34KB）を、Shadow ごとに `<style>` で入れる必要がある（`adoptedStyleSheets` で共有もできる。Safari 16.4 から）。この試作では本体の JS に `?inline` の写しが入り、文書の CSS と二重になった。
  - デザインシステムの CSS の `:root` の規則は、Shadow の中では効かない（`:host` への書き換えが要る）。この試作で効いたのは、文書の `:root` の値が継承で届いたため。
  - 祖先の `data-chapter` を見る規則（`[data-chapter="N"] .slide`）は Shadow の境界を越えない（[Issue #6 の検証結果](issue-6-switch-theme-during-playback.md)）。`:host-context()` はこの WebKit でも使えない（`CSS.supports` が false）。

### デザインシステムの CSS（トークンと部品のクラス）は閉じ込めた中でも効くか

どの候補でも効いた。

- `c-primary` の色は、ch04 で `rgb(166, 30, 85)`（#A61E55）、ch02 で `rgb(0, 101, 72)`（#006548）。
- `.callout` の背景は `rgb(228, 240, 255)`（#E4F0FF）。
- 書体: 同じ文字列（40px）の幅が、Murecho の 461px（Hiragino Sans なら 508px）だった。Shadow の中でも同じで、`@remotion/fonts`（`document.fonts`）の書体が使えた。

### この Mac の WebKit

すべての候補が動いた。`CSSScopeRule` があり、`@scope` が解釈される。CSS の入れ子、`adoptedStyleSheets`、`:host` も使える。`:host-context()` は使えない。

### macOS 13 の WebKit（実機未確認。互換表で調べた範囲）

- macOS 13.0 には Safari 16.1 が付属する。Ventura 向けには Safari 18.6 まで出ている。
- 古い macOS での Safari の更新は、`/System/Library/StagedFrameworks/Safari` に WebKit を置く形で入る（WebKit Bugzilla の記述）。WKWebView のアプリもこの更新された WebKit を使うかは、Apple の文書では確かめられなかった（未確認）。
- 更新されたものを使うなら、アプリが動く WebKit は 16.1〜18.6 のどれかになる。どちらの場合も、更新していない 13.0 の 16.1 が下限になるので、下限は 16.1 として判断した。

| 機能 | Safari の対応 | macOS 13（16.1〜18.6） | 出典 |
|---|---|---|---|
| `@scope` | 17.4 から | 16.1〜17.3 では使えない。`@scope` の中の規則が丸ごと無視され、レッスンの CSS が効かなくなる（漏れはしない） | [caniuse css-cascade-scope](https://caniuse.com/css-cascade-scope) |
| CSS の入れ子 | 16.5〜17.1 は部分対応、17.2 から完全 | 接頭辞の方法では、ビルド時に平らにすれば関係ない | [caniuse css-nesting](https://caniuse.com/css-nesting) |
| Shadow DOM v1 | 10 から | 使える | [caniuse shadowdomv1](https://caniuse.com/shadowdomv1) |
| `adoptedStyleSheets` | 16.4 から | 16.1〜16.3 では使えない（`<style>` を入れる方法なら関係ない） | [caniuse mdn-api_document_adoptedstylesheets](https://caniuse.com/mdn-api_document_adoptedstylesheets) |
| `:host-context()` | 対応なし（仕様から削除） | 使えない | [MDN](https://developer.mozilla.org/docs/Web/CSS/:host-context) |
| CSS Modules・接頭辞 | ビルド時の変換。出力は普通のセレクタ | 使える | なし |

## 判定

基準「方法が 1 つに決まり、WebView で動く」は、満たしていない。

- 方法の決定はユーザーの判断である。
- どの候補も動いた。

## 残った事項

- ユーザーの判断が要る点: 閉じ込めの方法。推奨は候補 3（ビルド時の接頭辞 `[data-lesson="{id}"]`）。決まったら、[教材データ形式の未決事項](../spec/content-format/open-questions.md)を解消する ADR を追加する。
  - 候補 3 の理由: 閉じ込めが確実（要素のセレクタも閉じる）。作者は普通の CSS を書けばよい。[Issue #6 の検証結果](issue-6-switch-theme-during-playback.md)の CSS 変数の継承もそのまま使える。
  - 候補 3 の弱い点: アプリのグローバルな CSS は動画の中に効く（逆向きの漏れ）。アプリ側の CSS を要素のセレクタで書かない、などの決まりが要る。
  - 候補 2（`@scope`）も、候補 3 と同じ範囲に閉じ込められた。違いは、`@scope` の中の `:root` が無効になることと、Safari 17.4 より前の WebKit では使えないこと。対応環境（macOS 27.2.1）の WebKit では使える。
  - 候補 4（Shadow DOM）は最も強く閉じ込められるが、CSS を文字列で集める仕組み、デザインシステムの CSS の `:root` → `:host`、テーマ・章を Shadow の中に渡すこと、CSS の二重持ちが要る。
  - 候補 1（CSS Modules）は、要素のセレクタが漏れる。lint で禁じる前提になる。
- 合わせて決める必要がある点（候補 3 を選ぶ場合）
  - アプリのグローバルな CSS が動画に効く逆向きの漏れを、どう扱うか。
  - 詳細度が 1 段上がる（`[data-lesson] .x`）ので、デザインシステムの部品を上書きしやすくなる。レッスンの CSS でデザインシステムの部品を上書きしてよいかを決める。
- 仕様の変更案
  - Shadow DOM を選ぶ場合は、[動画のコードの構成](../spec/content-format/video/structure.md)の「アプリは、動画のデザインシステムの CSS を動画に適用する」に従い、デザインシステムの CSS（tokens.css）の `:root` を `:host` でも効くように書く必要がある（デザインシステム側の変更）。
- 一度表示したレッスンの CSS（Vite の動的 import の CSS）は、レッスンを閉じても文書から外れない。閉じ込めない CSS は、その後に開くすべての画面に効き続ける。
