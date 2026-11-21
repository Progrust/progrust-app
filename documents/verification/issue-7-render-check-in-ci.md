---
type: Verification Report
title: "動画の描画チェックを CI で実行できるかの技術検証"
description: "教材チェックツールの動画の描画チェックと禁止事項の lint が CI で実行できるかを Issue #7 で検証した結果（画面なしの描画とエラー検出、GitHub Actions の ubuntu-latest での実行と所要時間、ネットワークなしでの実行、Studio・チェックツール・Vite の 3 か所で同じコードが動くこと、ESLint と stylelint による禁止事項の検出と限界、ユーザーが Claude の推奨に任せた lint の範囲と CI で検査するレッスンの範囲の判断）を記す。"
---

- 日付: 2026-11-21
- Issue: [#7](https://github.com/Progrust/progrust-app/issues/7)
- 確かめた OS: macOS 27.2.1（Apple Silicon）と、GitHub Actions の ubuntu-latest（Ubuntu 26.04.1 LTS、x86_64）。ubuntu-latest は CI の実行環境で、対応環境ではない
- 関連仕様: [教材チェックツール](../spec/content-guidelines/check-tool.md)、[動画で使えないコード](../spec/content-format/video/prohibited-code.md)、[シーン](../spec/content-format/video/scenes.md)、[ADR 0061](../decisions/0061-render-videos-with-remotion-player.md)、[ADR 0062](../decisions/0062-write-video-scenes-in-unrestricted-tsx.md)

## 環境

- macOS 27.2.1（Apple Silicon）、Node v25.9.0、npm 11.12.1
- remotion・@remotion/cli・@remotion/bundler・@remotion/renderer・@remotion/player はすべて 4.0.545。react 19.3.1、vite 8.3.8、typescript 6.0.3、eslint 10.15.0、typescript-eslint 8.73.0、stylelint 17.16.0、tsx 4.23
- Chrome Headless Shell は Remotion が初回に自動でダウンロードする（mac-arm64、約 140MB。`node_modules/.remotion/` に展開すると 470MB）。
- 測定中は他のエージェントが動いていた（load average 14〜21）。時間の数値は遅めに出ている。
- GitHub Actions: ubuntu-latest（Ubuntu 26.04.1 LTS、x86_64、2 コア、メモリ 7.7GB、ユーザー `runner`）。`actions/setup-node` で Node v25.9.0、npm 11.12.1。ライブラリの版は macOS と同じ（`npm ci`）。日本語のフォント（`fc-list :lang=ja`）は 0 件だった。

## 方法

仕様に沿った最小の再現を作った。

- 共通部品（`src/video/` の想定）: 台本・音声の長さ・合図の基準から、シーンと文の開始フレームを四捨五入のルールで計算する部品、`useCues()`・`useLesson()`・`LessonTitle`・`Camera`、シーンを `Sequence` で並べる `LessonVideo`。`useCues().at/endOf` は、そのシーンの文以外の ID を指すと `Error` を投げる。
- レッスン（`lessons/4-3/video/` の想定）: `script.json`、`durations.json`、`cue-reference.json`、`index.tsx`（`export default { title: TitleScene, move: MoveScene }`）、シーン、CSS（`import "./scene.css"`）、画像（`import dot from "./dot.png"`）。
- チェックツール: 次の手順で描画する。
  1. レッスンごとに一時の entry（`registerRoot` と、`inputProps.sceneId` と `calculateMetadata` でシーンごとに長さを決める `Composition id="Scene"`）を生成し、`@remotion/bundler` の `bundle()` で 1 回だけバンドルする。
  2. `openBrowser("chrome-headless-shell")` を 1 回だけ開き、`puppeteerInstance` に渡して使い回す。
  3. シーンごとに `selectComposition` し、フレーム {0、各文の `localFrom`、最終フレーム} を `renderStill` する。
  4. 例外は `try/catch` で受け、`scene=… frame=… (first|beat m2|last)` を付けて集計する。`onBrowserLog` で console の error と warning（404 など）も拾う。エラーがあれば exit code 1 にする。

```ts
const browser = await openBrowser("chrome-headless-shell", { chromiumOptions: { gl: "swangle" } });
const comp = await selectComposition({ serveUrl, id: "Scene", inputProps: { sceneId }, puppeteerInstance: browser });
const want = new Set([0, ...st.beats.map((b) => b.localFrom), comp.durationInFrames - 1]);
for (const frame of [...want].sort((a, b) => a - b)) {
  try {
    await renderStill({ composition: comp, serveUrl, frame, output, inputProps: { sceneId },
      puppeteerInstance: browser, timeoutInMilliseconds: 15000,
      onBrowserLog: (l) => { if (l.type === "error" || l.type === "warning") logs.push(l.text); } });
  } catch (e) {
    errors.push(`scene=${sceneId} frame=${frame} (${label}): ${String(e.message).split("\n")[0]}`);
  }
}
```

- Studio・Player: 同じシーンと共通部品を、変更なしで Remotion Studio（`npx remotion studio`）、チェックツール、最小の React + `@remotion/player` の Vite アプリの 3 か所から使った。
- lint: ESLint の flat config（typescript-eslint のパーサーで TSX を読む）で `lessons/**/video/**/*.{ts,tsx}` を検査し、CSS には stylelint を使った。違反を並べたファイルと、違反のないファイルで試した。

## 結果

### 描画チェックとエラー検出（確かめた）

正常なレッスン（2 シーン・4 文）は 8 フレームを描画してエラー 0、約 4〜8 秒だった。出力画像を目視して、CSS・画像・SVG が描かれていることを確認した。わざとエラーを入れた結果は次のとおり。

| ケース | 検出 | 出力 |
|---|---|---|
| 台本にない文 ID を指す `at("m9")` | できた | `ERROR scene=move frame=0 (first): 合図が台本にない文 "m9" を指しています（シーン move。文: m1,m2,m3）`（全 5 フレームで） |
| 別のシーンの文を指す `at("t1")`（move シーンで） | できた | 同じ形式で `"t1"` を指しているとして報告 |
| frame 100 以降で throw するコンポーネント | できた。フレームも分かる | `ERROR scene=move frame=141 (beat m3): boom at 100` と `frame=227 (last)`。frame 0・18・84 は通る |
| 存在しない画像の import（`./missing.png`） | できた（バンドルの段階で失敗） | `ERROR bundle/setup: Module not found: Error: Can't resolve './missing.png' in '…/video'`。どのシーンかは分からず、ファイルのパスだけ分かる |
| 存在しない画像を Remotion の `<Img src="/nonexistent.png">` に渡す | できた（ただし遅い） | `Error loading image with src: http://localhost:3000/nonexistent.png. Failed 3 times`。1 フレームごとに数秒のリトライがあり、全体で約 23 秒 |
| 存在しない画像を素の `<img src="/nonexistent.png">` に渡す | `onBrowserLog` を付けたときだけできた | 付けないとエラー 0 で通る。付けると `browser console: Failed to load resource: … 404` |

分かったこと。

- 検出できるのは、描画したフレームで実行された経路のエラーだけである。上の throw の例のように、別のフレームだけで起きるエラーは、そのフレームを描画しないと見つからない。最初・各文の開始・最後の 3 種類では、文の途中だけで通る分岐の誤りは見つからない。
- 各フレームで失敗するとエラーが重複して出る。集計のときに同じメッセージをまとめるとよい。
- `CodeBlock` の制約違反（行番号・Diff）は、`CodeBlock` 部品が例外を投げれば、`useCues` と同じ方法で検出できると思われる。`CodeBlock` は作っていないので確かめていない。
- 「最初のシーンが `LessonTitle` を 1 つ表示する」の検査は、描画した DOM を `page.evaluate` で見るか、`LessonTitle` が描画時に数える方法でできると思われる。確かめていない。
- `renderStill` はフレームごとにページを新しく開く。使い回せるのは Chrome（`puppeteerInstance`）である。`renderFrames` は連続した範囲向きで、飛び飛びのフレームには向かないので使わなかった。

所要時間は、8 シーン・40 文・約 3 分のレッスンに相当する合成レッスン（56 フレーム）で測った。macOS では、`bundle` を含む全体は 3 回で 18.7 秒、14.4 秒、19.8 秒（中央値 18.7 秒）。内訳は bundle 約 2〜4 秒と、描画 1 フレームあたり約 0.25 秒である。ブラウザの起動は 2 回目以降は約 0.2 秒で、初回はダウンロードを含めて約 9 秒だった。1 レッスン約 20 秒なので、全レッスン（数十本）でも、並列にするか、数分から 10 分程度の見込みである。GitHub Actions の runner（2 コア）では、同じレッスンが 3 回で 28.2 秒、22.7 秒、22.1 秒だった（「CI で実行できるか」）。

### CI で実行できるか（確かめた）

試作を一時ブランチに置き、GitHub Actions の ubuntu-latest で実行した（実行後にブランチは消した）。手順は、`actions/checkout`、`actions/setup-node`（Node 25）、`npm ci`、`npx remotion browser ensure`、上のチェックツールの実行、ESLint と stylelint の実行である。

- ubuntu-latest は Ubuntu 26.04.1 LTS だった。Remotion の公式ドキュメント（https://www.remotion.dev/docs/miscellaneous/linux-dependencies）は Ubuntu 24.04 までを書いている。
- Chrome Headless Shell: `npx remotion browser ensure` が `https://remotion.media/chromium-headless-shell-linux-x64-158.0.8127.2.zip` をダウンロードした。ダウンロードと展開は約 1 秒で、`node_modules/.remotion` は 243MB だった。CI でキャッシュしなくても、時間はほとんど変わらない。
- 依存ライブラリ（apt）: 公式ドキュメントの一覧（`libnss3`・`libasound2t64`・`libgbm` など 14 個）は、runner のイメージにすべて入っていた。Chrome Headless Shell の実行ファイルに `ldd` をかけて、見つからないライブラリがないことを確かめた。`apt-get install` をする前の状態で描画でき、エラーは 0 だった。
- ユーザー権限（`runner`）で、Chrome のサンドボックスの設定なしに動いた。
- エラーの検出: 上の表のケース（台本にない文、ほかのシーンの文、途中で throw、存在しない画像の import、`<Img>` と素の `<img>` の 404）を macOS と同じレッスンで実行した。どれも macOS と同じエラーを出し、exit code 1 になった。正常なレッスンは exit code 0 だった。

| レッスン | exit code | 時間 | 出力 |
|---|---|---|---|
| 正常（2 シーン・4 文、8 フレーム） | 0 | 7.7 秒 | `rendered 8 frames, 0 errors` |
| 合成（8 シーン・40 文、56 フレーム）× 3 回 | 0 | 28.2 秒、22.7 秒、22.1 秒 | `rendered 56 frames, 0 errors` |
| 台本にない文 `at("m9")` | 1 | 8.0 秒 | `ERROR scene=move frame=0 (first): 合図が台本にない文 "m9" を指しています（シーン move。文: m1,m2,m3）` ほか 4 件 |
| ほかのシーンの文 `at("t1")` | 1 | 7.7 秒 | 同じ形式で `"t1"` |
| frame 100 以降で throw | 1 | 8.0 秒 | `ERROR scene=move frame=141 (beat m3): boom at 100` と `frame=227 (last)` |
| 存在しない画像の import | 1 | 6.0 秒 | `ERROR bundle/setup: Module not found: Error: Can't resolve './missing.png' …` |
| `<Img src="/nonexistent.png">` | 1 | 27.0 秒 | `Error loading image with src: http://localhost:3000/nonexistent.png. Failed 3 times (maxRetries: 2)` |
| 素の `<img src="/nonexistent.png">` | 1 | 12.2 秒 | `browser console: Failed to load resource: the server responded with a status of 404 (Not Found)` |

- ネットワークなし: `sudo unshare --net` で外部のネットワークのない名前空間を作り、ループバックだけを有効にして（`ip link set lo up`）、ユーザー `runner` で実行した。同じ名前空間で `curl https://example.com` が失敗する（`Could not resolve host`）ことを確かめた。正常なレッスンは `rendered 8 frames, 0 errors` で exit code 0、台本にない文のレッスンはエラー 5 件だった。ループバックを有効にしないと、bundler の localhost のサーバーにつながらず失敗した（`connect ENETUNREACH 127.0.0.1`）。

```
$ sudo unshare --net -- sh -c "ip link set lo up; sudo -u runner … npx tsx tools/render-check.ts lessons/4-3/video"
[8.5s] bundle done / [8.8s] browser open / [10.8s] rendered 8 frames, 0 errors
```

- lint: ESLint は、違反を並べたファイルで 40 件のエラー（exit code 1）、違反のないファイルと正常なレッスンで 0 件（exit code 0）だった。stylelint は、違反を並べた CSS で 9 件のエラー（exit code 2）、違反のない CSS で 0 件（exit code 0）だった。
- フォント: runner には日本語のフォントがなく、描画した画像では日本語が豆腐（□）になった。エラーの検出には影響しなかった。教材が `@remotion/fonts` の `loadFont` でリポジトリ内のフォントを読み込めば、システムのフォントに依存しない（試していない）。文字のはみ出しなどは書体に依存する。
- Remotion は起動時に、ライセンスの警告（`Note: Some companies are required to obtain a license`）を出す。今回の `onBrowserLog` の実装では、macOS でも runner でも誤検出にならなかった。warning を拾う実装に変えるときは、除外が要る。

### Studio・チェックツール・Vite の 3 か所で同じコード（確かめた）

| 場所 | 方法 | 結果 |
|---|---|---|
| Remotion Studio | `npx remotion studio <entry> --no-open`（`Composition` に `LessonVideo` を渡す）。`remotion still` でも描画 | `studio http 200`、`bundle.js 200`、`Built in 1593ms`。still の出力も正常 |
| チェックツール | `bundle()` + `renderStill`（上の結果） | 正常 |
| アプリの Vite ビルド | 最小の React + `@remotion/player` のアプリ。`vite build` のあと `vite preview` を Chrome Headless Shell で開いて DOM を検査 | `tsc` の型検査とビルドが成功（0.1 秒）。Player が `initialFrame=100` で move シーンの 13 フレーム目を表示した。`{"text":"フレーム 13/228\n0:03 / 0:10","big":".../assets/big-f0s8htT8.png nw=64","box":"rgb(0, 170, 119) 72px"…}` で、CSS の `var(--accent)` が効き、画像も読み込まれた |

書き方の注意。

- CSS は `import "./scene.css"` で 3 か所とも動く。
- 画像・JSON は `import dot from "./dot.png"` で 3 か所とも URL の文字列になる。Vite は 4KB 未満を data URI に埋め込み、それ以上は `assets/big-<hash>.png` にする。型のために `declare module "*.png" { const src: string; export default src; }` を `src/` に置く。
- `staticFile("pub.png")` は、Remotion では `public/` から読む。Vite では `/pub.png` を返すだけなので、Vite の `publicDir` を同じ `public/` に合わせると動く（既定の `app/public` のままでは `nw=0` で読めず、`publicDir: "../public"` にすると `nw=64` で読めた）。Tauri の本番ビルドでの配置は確かめていない。教材は仕様どおり import に統一し、`staticFile` は使わない方が安全である。
- 音声は仕様どおり動画のコードに書かず、アプリが再生する。音声の import の差は問題にならない。
- 共通部品の `LessonVideo`（シーンを `Sequence` で並べ、シーンの文脈を与える）を置き、Studio・Player・チェックで共用する構成が、そのまま動いた。チェックツールはシーン単体の `Composition` を作るため、文脈を直接与えている（`Sequence` 経由と同じ値）。
- Remotion のバンドラー（webpack）と Vite の両方で、拡張子なしの相対 import、JSON の import、CSS の import がそのまま動いた。
- WKWebView での再生は、この検証の範囲外である。

### 禁止事項の lint（大部分できた）

違反を並べたファイルの実測で、次のとおり検出できた。

| 禁止事項 | 手段 | 検出できた例 |
|---|---|---|
| `@tauri-apps/` の import | `no-restricted-imports`（patterns の `regex`） | `import { invoke } from "@tauri-apps/api/core"` |
| `window.__TAURI__` など | `no-restricted-properties`（`window`・`globalThis`・`self`・`top`・`parent` と `__TAURI__`・`__TAURI_INTERNALS__` の組）、`no-restricted-globals` | `window.__TAURI__`、`globalThis.fetch`。`(window as any).__TAURI__` は型アサーションで素通りするので、`no-restricted-syntax` で禁止して検出 |
| `fetch`・`XMLHttpRequest`・`WebSocket`・`EventSource` | `no-restricted-globals` | 4 つとも検出。`window.fetch` と、分割代入 `const { fetch } = window` も `no-restricted-properties` が検出 |
| `navigator.sendBeacon` | `no-restricted-properties` | 検出。`navigator["sendBeacon"]` は、computed アクセスを禁止する `no-restricted-syntax` が検出 |
| 外部 URL（`http:`・`https:`・`//`）と `file:` | 独自ルール `progrust/no-external-url`（下）。import は `no-restricted-imports` | `<img src="https://…">`、`"file:///etc/passwd"`、`"//cdn.example.com/a.png"`、`import … from "https://…"` |
| Node.js の組み込みモジュール | `no-restricted-imports`（`builtinModules` から生成した `paths` と、`node:` の regex）、`require()` は `no-restricted-syntax` | `import "fs"`、`import "node:path"`、`require("fs")` |
| 動的 import | `ImportExpression > Literal[value=/…/]` | `import("@tauri-apps/api/event")` |
| 色の直接指定（16 進、色の関数、色の名前） | 独自ルール `progrust/no-hardcoded-color`（下） | `"#fff"`、`` `rgb(${1}, 2, 3)` ``、`{ color: "red" }`、`{ border: "1px solid #00ff00" }`、`linear-gradient(blue, white)`、`<rect fill="red" stroke="#000" />`。`none`・`currentColor`・`transparent`・`inherit`・`var(--…)` は通る |
| CSS の色・外部 URL・`@import` | stylelint（`color-no-hex`、`color-named: never`、`function-disallowed-list`、`function-url-scheme-disallowed-list`、`function-url-no-scheme-relative`、`at-rule-disallowed-list`） | 違反を並べた CSS の 9 件をすべて検出。`var(--fg)`・`transparent`・`currentColor`・`none`・`url(./a.png)` は通る |

違反のないファイルと、試作の `lessons/4-3/video` は 0 件で通った。

外部 URL の独自ルールは、文字列リテラルとテンプレートの文字列部分を正規表現で検査するだけである。

```js
const RE = /^\s*(?:https?:|file:|\/\/)/i;
export default {
  meta: { type: "problem", schema: [], messages: { url: "外部 URL / file: の参照は使えません（{{v}}）" } },
  create(context) {
    const rep = (node, v) => context.report({ node, messageId: "url", data: { v: v.slice(0, 40) } });
    return {
      Literal(n) { if (typeof n.value === "string" && RE.test(n.value)) rep(n, n.value); },
      TemplateElement(n) { const v = n.value.cooked ?? n.value.raw; if (RE.test(v)) rep(n, v); },
    };
  },
};
```

色の独自ルールの決め手は、16 進と色の関数はどの文字列でも検査し、色の名前は文脈の中だけで検査する点である。

```js
const HEX = /#(?:[0-9a-f]{8}|[0-9a-f]{6}|[0-9a-f]{4}|[0-9a-f]{3})(?![0-9a-z_-])/i;
const FUNC = /\b(?:rgba?|hsla?|hwb|lab|lch|oklab|oklch|color|color-mix|device-cmyk)\s*\(/i;
const ALLOWED = new Set(["none", "currentcolor", "transparent", "inherit"]);
const NAMED = new Set(Object.keys(colorName)); // color-name パッケージ
// COLOR_KEYS: color|fill|stroke|background|backgroundColor|border|borderColor|boxShadow|filter|… （style のプロパティ名）
const check = (node, text, namedContext) => {
  const h = text.match(HEX); if (h) return report(node, h[0]);
  const f = text.match(FUNC); if (f) return report(node, f[0]);
  if (namedContext) { const n = namedIn(text); if (n.length) report(node, n.join(",")); }
};
// Literal: 親が COLOR_KEYS の Property なら namedContext = true。
// JSXAttribute: fill・stroke・stopColor・color などの SVG 属性の値が色の名前なら報告（var(--…)・url(#…) は通す）。
```

ESLint の設定のうち、禁止事項ごとの `no-restricted-*` の指定は次のとおり（抜粋）。

```js
"no-restricted-syntax": ["error",
  { selector: "CallExpression[callee.name='require']", message: "require は使えません" },
  { selector: "ImportExpression > Literal[value=/^(?:@tauri-apps\\/|node:|https?:|file:)/]", message: "動的 import でも禁止対象です" },
  { selector: "MemberExpression[computed=true][object.name=/^(?:window|globalThis|self|navigator)$/]", message: "window[...] の動的なアクセスは使えません" },
  { selector: "TSAsExpression > Identifier.expression[name=/^(?:window|globalThis|self|navigator)$/]", message: "window を型アサーションして使う書き方は使えません" },
  { selector: "CallExpression[callee.name='eval'], NewExpression[callee.name='Function']", message: "eval / new Function は使えません" },
],
```

検出できない、または誤検出するもの。

- 実行時に組み立てた文字列: `"r" + "gb(1,2,3)"`、`"#" + ["f","f","f"].join("")`、`"re" + "d"`、`["fe","tch"].join("")` で作った名前での `window[f]`。`window[...]` の computed アクセスは禁止にして、一部は塞げる。
- 別名経由: `const w = window; w.fetch(...)`、`(0, eval)("this")`、`Object.getPrototypeOf(window)` など。
- `src` に変数を渡す（`img.src = src`）場合は、中身が外部 URL かどうか分からない。
- 色の名前（`red` など）は、style のプロパティ名や SVG の色の属性の中の文字列だけを検査する。`const c = "red"; <div style={{ color: c }}>` のように変数を経由すると検出できない。文章の中の "red" を誤検出しないための割り切りである。
- 16 進は、どの文字列でも `#` と 3・4・6・8 桁の 16 進数なら検出するので、`"#123 番"` のような文章を誤検出する（`#1` は大丈夫）。
- ライブラリが内部で行うアクセス（音声の解析ライブラリが内部で `fetch` するなど）は、呼び出し側のコードからは分からない。
- `eval` と `new Function` は検査を逃れる手段なので、この検証では禁止に加えた。仕様にはない。
- CSS の `@import` は、外部だけでなくローカルも全部禁止にした。stylelint の標準のルールでは、外部の URL だけを禁止にできないためである。`<style>{`…`}</style>` や CSS-in-JS の文字列は ESLint 側で見ており、検査するのは 16 進と色の関数だけである。
- `.css` の `url()` の `data:` は通る。
- 動画のコードから import する先（共通部品や、`video/` の外のファイル）は検査の対象にしていない。`video/` の中だけを検査する。

### 検証のあとの判断

検証のあと、ユーザーは、残った判断を Claude の推奨に任せた。Claude は次のように決め、仕様に書いた。

- 禁止事項の lint: `eval`・`new Function`・引数が文字列リテラルでない動的 import・`window` などへの `[...]` でのアクセスと型アサーションを通した使用を禁止に加えた。色の名前は、`style` のプロパティの値・SVG の色の属性・CSS の宣言に直接書いたものだけを検査する。Tauri の名前は `__TAURI__` と `__TAURI_INTERNALS__` にした。CSS の `@import` は、参照先によらず禁じた。`.css` は stylelint で検査する（[ADR 0094](../decisions/0094-close-lint-bypasses-in-video-code.md)）。
- CI の範囲: 描画のチェックでは、毎回すべてのレッスンを描画する。描画のチェックで見つかるのは描画したフレームのエラーだけだと、仕様に書いた（[ADR 0095](../decisions/0095-render-check-all-lessons-in-ci.md)）。
- `staticFile()`: 動画のコードでは使わず、lint でエラーにする（[ADR 0096](../decisions/0096-do-not-use-staticfile-in-video-code.md)）。

## 判定

Issue の基準「CI で、描画のエラー（台本にない文を指す合図など）と禁止事項の違反を検出できる」は、満たした。

- 描画のエラー: GitHub Actions の ubuntu-latest と macOS で、画面なしで最初・各文の開始・最後のフレームを描画し、エラーのシーンとフレームを特定できた。ネットワークなしでも動いた。ただし、描画していないフレームで起きるエラーと、`onBrowserLog` を付けない素の `<img>` の 404 は検出できない。
- 禁止事項: 大部分を ESLint（標準ルールと独自ルール 2 つ）と stylelint で検出できた。ubuntu-latest でも同じ結果だった。実行時に組み立てた文字列や別名経由は検出できない。
- Studio・チェックツール・Vite の 3 か所で、同じシーンのコードと共通部品が変更なしで動いた。
- 合成の約 3 分のレッスンは、ubuntu-latest で 1 本約 22〜28 秒だった。

## 残った事項

確かめられなかったこと。

- root で実行するとき（Docker など）の Chrome のサンドボックスの設定。GitHub Actions ではユーザー権限で動くので、要らなかった。
- WKWebView での Player の再生（別の Issue）。
- `CodeBlock` の制約違反の検出と、`LessonTitle` の個数の検査の実装。同じ仕組みでできると思われるが、作っていない。
- 実際の約 3 分の教材での所要時間。合成した 8 シーン・40 文のレッスンで代用した。実際の教材のほうが描画が重い可能性がある。

ユーザーの判断が要るとした点は、検証のあとに決まった（「検証のあとの判断」）。決める前の推奨と、決まったことは次のとおり。

- Remotion のライセンス: この検証では判断できない、とした。その後、ライセンスの確認は、一般公開をする場合に決めることになった（[ADR 0090](../decisions/0090-support-only-authors-mac-until-completion.md)、[非機能要件の未決事項](../spec/non-functional/open-questions.md)）。
- lint の範囲: `eval`・`new Function`・変数を使う動的 import・型アサーションされた `window` を禁止に加えることを推奨した。検査をすり抜ける手段だからである。ADR 0094 で加えた。
- CI の描画チェックの範囲: 毎回すべてのレッスンを推奨した。1 レッスン約 20 秒なので全レッスンでも許容できそうだが、Linux の runner での実測が要る、とした。ADR 0095 で、すべてのレッスンにした。runner では 1 本約 22〜28 秒だった。
- 仕様の変更案（教材チェックツールの描画の限界、色の名前の検出の範囲、「`window.__TAURI__` など」の範囲、CSS の `@import`、stylelint の併用、`staticFile`）: ADR 0094・0095・0096 で仕様に書いた。

検証中に見つけた別の問題。

- macOS で Chrome Headless Shell の実行ファイルを直接起動すると（`--screenshot`）、`MachPortRendezvousServer` のエラーで失敗し、スクリーンショットが取れなかった。`openBrowser()` 経由なら動く。直接起動する設計にしない。
- `openBrowser()` が返すブラウザ（Remotion 内蔵の puppeteer 派生）は、`page.screenshot` が公開されていない。画面を画像で比べたいときは `renderStill` を使う。
