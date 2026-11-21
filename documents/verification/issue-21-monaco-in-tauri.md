---
type: Verification Report
title: "Monaco Editor を Tauri の中で使えるかの技術検証"
description: "Monaco Editor を Tauri の WKWebView で、CDN なしの同梱、Web Worker、CSP と両立して使えるかと、Cmd+Enter・Cmd+Shift+Enter・Cmd+. の既定の割り当てを上書きできるかを Issue #21 で検証した結果（style-src に 'unsafe-inline' が要ること、人が操作して確かめた日本語入力と実キーでのショートカット、CSP を仕様に定めた判断）を記す。"
---

- 日付: 2026-11-21
- Issue: [#21](https://github.com/Progrust/progrust-app/issues/21)
- 確かめた OS: macOS 27.2.1（Apple Silicon、WKWebView）。日本語入力（IME）と実キーでのショートカットは、検証のあとに人が試作のアプリを操作して確かめた
- 関連仕様: [演習画面](../spec/screens/exercise-screen.md)、[技術スタック](../spec/architecture/tech-stack.md)、[セキュリティとプライバシー](../spec/non-functional/security-privacy.md)、[ADR 0004](../decisions/0004-monaco-without-completion.md)

## 環境

| 項目 | 値 |
|---|---|
| OS | macOS 27.2.1、Apple Silicon。WKWebView（UA は AppleWebKit/605.1.15、`navigator.platform` は MacIntel） |
| Rust | rustc 1.99.1、cargo 1.99.1 |
| Node | v25.9.0 |
| Tauri | CLI 2.12.6、`tauri` 2.12.7、wry 0.57.3 |
| Monaco | monaco-editor 0.57.0、@monaco-editor/react 4.7.0（loader 1.7.0） |
| フロントエンド | React 19.3.1、Vite 8.3.8（bundler は Rolldown） |

試したのは release ビルドの .app（`tauri build --bundles app`）。

## 方法

最小の Tauri v2 + React + Vite のアプリを作った。ページ自身が全項目を実行し、結果を Tauri コマンドでファイルに書いて終了する。メニューは Rust 側から `NSApp.mainMenu` を再帰的にたどって取った。

CDN を止め、同梱の monaco-editor を使う書き方（これが決め手）:

```ts
import Editor, { loader } from "@monaco-editor/react";
import * as monaco from "./monaco-lean";   // 下記の自前の入口
import EditorWorker from "monaco-editor/editor/editor.worker?worker";

loader.config({ monaco });                 // CDN（jsdelivr）を読まなくなる
self.MonacoEnvironment = { getWorker: () => new EditorWorker() };
```

自前の入口（`monaco-lean.ts`）は、`editor.main.js` から「全言語の定義」「css・html・json・ts の言語機能」「lsp」を除き、contrib（編集機能）はすべて import し、Rust だけ次で登録する。

```ts
import "monaco-editor/languages/definitions/rust/register";
```

editor.main を丸ごと使うより軽い（dist 4.6MB、editor.worker 304KB、rust の定義 3.7KB、codicon.ttf 153KB、.app 11MB）。

monaco-editor 0.57 の落とし穴。package.json の exports が `"./*.js": "./esm/vs/*.js"`、`"./*": "./esm/vs/*.js"` なので、import の指定子に `esm/vs/` を付けない。

- 正: `monaco-editor/editor/editor.worker?worker`、`monaco-editor/languages/definitions/rust/register`、`monaco-editor/contrib/...`
- 誤: `monaco-editor/esm/vs/...`（Rolldown が解決できない）
- `.css` は exports に当たらないので、相対パス（`../node_modules/monaco-editor/esm/vs/base/.../codicon.css`）で読む。
- 自前の入口を作るなら、0.57 の editor.main の import 一覧を写して保守する必要がある（版を上げるたびに確認する）。
- `tsc` は Monaco の deep import の型を解決できないので、入口に `// @ts-nocheck` と宣言が要る。

CSP は、仕様に定めが無いので試作で決めた厳しめのものを使った（`tauri.conf.json` の `app.security.csp`。Tauri は `csp: null` だと CSP を何も付けない）。全文:

```
default-src 'self'; script-src 'self'; worker-src 'self'; style-src 'self' 'unsafe-inline'; font-src 'self' data:; img-src 'self' data:; connect-src 'self' ipc: http://ipc.localhost
```

`connect-src 'self'` は、CSP ヘッダーを fetch で読むために入れた。外への接続は許していない。

## 結果

### オフライン・Worker・CSP（macOS の release ビルド）: 確かめた

- Worker: `new Worker("/assets/editor.worker-….js")` が動いた。Worker から message を 24 回受信し、error は 0。差分エディタ（計算は editor worker）で行の差分が返った（`diffMs` は 26〜29）。`worker-src 'self'` だけで足り、`blob:` は不要だった。
- CDN: loader に `monaco` を渡したので CDN を読まない。ページの外部読み込みは 0。
  - **WKWebView の `tauri://` のリソースは `performance.getEntriesByType('resource')` に載らない**（結果は空配列）。ネットワークの確認には使えないので、次で確認した。
    - fetch・XHR・WebSocket をフックした記録では、アプリが行ったのは `ipc://localhost/...` だけ（対照の外部 fetch を除く）。
    - DOM の script・link の URL は `tauri://localhost/assets/...` だけ。CSS の `url()` は `/assets/codicon-….ttf` と `data:` だけ。
    - CSP 違反イベントは 0 件（対照の外部 fetch の 1 件を除く）。対照として `https://cdn.jsdelivr.net/...` を fetch すると、`connect-src` で拒否されて違反が 1 件出た。CSP が実際に効いていることの確認になる。
  - 実際にネットワークを切った状態での実行はしていない（システムの設定を変えないため）。
- 実際に効いた CSP ヘッダー（Tauri が `script-src` にインラインのハッシュを 3 個足す）:
  `worker-src 'self'; style-src 'self' 'unsafe-inline'; ... default-src 'self'; script-src 'self' 'sha256-…'; connect-src 'self' ipc: http://ipc.localhost`
- **`style-src` から `'unsafe-inline'` を外すと違反が出る。** Monaco が `<style>` 要素と `style` 属性を実行時に作るため、`style-src-elem` と `style-src-attr` の違反が計 96 件出た。このときも Worker と Rust の強調と編集の動作自体は動いた。見た目が崩れるかは確かめていない。よって Monaco を使う限り `style-src 'unsafe-inline'` が要る。`script-src` は `'self'` のままでよく、`unsafe-eval` も要らなかった。
- Rust の構文強調: `monaco.editor.colorize` が 5 種類の色クラス（mtk1・6・7・8・20）を返し、`tokenize` が `keyword.rust` などを返した。0.57 の新しいパスで登録できる。
- 非同期の `Canceled` の unhandled rejection が 1 件、Monaco のログに `NotAllowedError`（clipboard の `write` 由来）が 1 件出た。合成のキーイベント（`isTrusted=false`）を送ったことによるものと思われるが、切り分けていない。

### 日本語入力（IME）: 人が操作して確かめた

最初の検証では、人の入力が要るので試さず、既知の問題を調べた（下記）。そのあと、人が試作のアプリ（release ビルドの .app、CSP は上の `style-src 'unsafe-inline'` ありのもの）を操作して確かめた。試作には、押したキーと変換の開始・確定（`compositionstart`・`compositionend`）を画面に出す欄を足した。確かめた項目は次のとおりで、すべて問題なかった（人の報告は「全部大丈夫だった」）。

1. コメント行の後ろで「こんにちは」を変換して確定する。
2. 変換中に矢印キーと Enter を押す。
3. 変換候補がカーソルの近くに出るかを見る。
4. 変換中に Cmd+Enter を押し、文字が消えたり二重になったりしないかを見る。

調べた既知の問題:

- Monaco は隠した `<textarea>` で入力を受け、composition イベントを処理する。VS Code の Wiki に IME のテスト手順がある（https://github.com/Microsoft/vscode/wiki/IME-Test）。microsoft/monaco-editor の Issue で「Safari・WKWebView の日本語入力が壊れる」という具体的な報告は、検索では見つからなかった（網羅はしていない）。
- WebKit Bug 165004（https://bugs.webkit.org/show_bug.cgi?id=165004）: macOS の WebKit は、日本語 IME のとき `compositionstart`・`compositionupdate`・`input` を `keydown` より先に発火し、変換を確定する Enter の keydown は `compositionend` の後に `isComposing === false` で届く（Chrome は keydown が先）。このため「Enter で送信」のような keydown の処理は、変換の確定で誤作動する。回避は `keyCode === 229` の併用。この Bug は 2026 年 4 月に別の Bug（311717）の修正で解消されたと報告されている。**この Mac の WKWebView に修正が入っているかは未確認**。macOS 13 の WKWebView は修正前の可能性が高い。
- 演習画面のショートカットは Cmd+Enter など修飾キー付きなので、変換の確定の Enter（修飾キー無し）とは別物。アプリ側で自前の keydown ハンドラーを作る場合は、変換中（`isComposing` または `keyCode === 229`）は処理しない判定が要る。Monaco 自身のキー処理が WebKit の順序の問題をどう扱うかは未確認。
- 古い関連 Bug: WebKit Bug 49523（compositionupdate を聞く textarea の style を変えると、変換の途中の文字が混入する）。Monaco は textarea 方式なので同じ型の問題に注意する。

### ショートカット: Monaco 側と、人が押した実キーで確かめた

Monaco の既定の割り当て（`editor._standaloneKeybindingService.getKeybindings()` の 380 件。mac では CtrlCmd = Cmd）:

| キー | Monaco の既定 | 条件 |
|---|---|---|
| Cmd+Enter | `editor.action.insertLineAfter`（下に行を挿入） | `editorTextFocus && !editorReadonly` |
| Cmd+Enter | `editor.action.replaceAll`、`acceptRenameInputWithPreview`、`quickInput.accept`、`previewSelectedCodeAction` | 検索の置換欄、リネームの入力、クイック入力、コードアクションのメニューの表示中 |
| Cmd+Shift+Enter | `editor.action.insertLineBefore`（上に行を挿入） | （`shift+cmd+enter`） |
| Cmd+. | `editor.action.quickFix` | `editorHasCodeActionsProvider && textInputFocus && !editorReadonly` |
| Cmd+. | `editor.changeDropType`、`editor.changePasteType`、`acceptSelectedCodeAction` | ドロップ・貼り付けのウィジェット、コードアクションのメニューの表示中 |

textarea への合成の `keydown`（`isTrusted=false`、`editorTextFocus=true`）での動作:

- 既定のまま: Cmd+Enter は行を挿入し（`line1\nline2\n` が 4 行になる）、`defaultPrevented=true` で、window のバブリングのリスナーには届かない。Cmd+Shift+Enter も同様に上へ挿入する。Cmd+. は、Rust に codeActionsProvider が無いので Monaco は処理せず、`defaultPrevented=false` で window に届いた。
- **`addCommand` で上書きできる。** 3 つのキーとも fn が 1 回呼ばれ、テキストは変わらず（既定の行の挿入は起きない）、`defaultPrevented=true` で、window には届かない。読み取り専用（`readOnly: true`）でも fn は呼ばれる。使ったコード:

```ts
const K = monaco.KeyCode, M = monaco.KeyMod;
editor.addCommand(M.CtrlCmd | K.Enter, () => { /* 実行 */ });
editor.addCommand(M.CtrlCmd | M.Shift | K.Enter, () => { /* 提出 */ });
editor.addCommand(M.CtrlCmd | K.Period, () => { /* ヒント */ });
```

- `editor.addAction({ id, label, keybindings: [...] })` も動いた（コマンドパレット用の名前も付けられる）。
- アプリ側の `window.addEventListener("keydown", fn, true)`（キャプチャ）で Cmd+Enter を取って `stopPropagation` すると、Monaco の既定も `addCommand` も走らない。つまりアプリが先に取れる。ただし Monaco が処理するキーはバブリングでは window に届かない（Monaco が止める）ので、アプリ全体のショートカットは、キャプチャか、Monaco 側の `addCommand` に登録する必要がある。
- フォーカスがエディタの外のときは window に届き、Monaco は反応しない。

Tauri の macOS の既定メニュー（`NSApp.mainMenu` の実測、46 項目）: アクセラレーターがあるのは Hide（Cmd+H）、Hide Others（Cmd+Opt+H）、Quit（Cmd+Q）、Close Window・Close All（Cmd+W・Cmd+Opt+W）、Undo・Redo・Cut・Copy・Paste・Select All、Emoji & Symbols（Cmd+Ctrl+Space）、Toggle Full Screen（Cmd+Ctrl+F）、Minimize（Cmd+M）、Start Dictation など。**Cmd+Enter・Cmd+Shift+Enter・Cmd+. はメニューに無く、衝突しない。** 将来メニューを自前で作るときは避ける。

確かめられなかったこと: WKWebView・AppKit が、本物の Cmd+Enter・Cmd+Shift+Enter・Cmd+. を、ページの keydown に渡す前に取るか。macOS では Cmd+. が標準の「中止」の意味を持ち、AppKit では Esc と同じく `cancelOperation:` に結び付くため、ページに keydown が届かない可能性がある。Cmd+Enter・Cmd+Shift+Enter には標準の割り当てが無い。

検証のあと、人が試作のアプリで、エディタにフォーカスして実キーの Cmd+Enter・Cmd+Shift+Enter・Cmd+. を押した。試作は、上の `addCommand` で 3 つのキーを登録し、呼ばれたら画面の欄に表示する。3 つとも表示され、問題なかった。Cmd+. も、WKWebView・AppKit に取られずにページに届いた。

### 検証のあとの判断

検証のあと、ユーザーは、残った判断を Claude の推奨に任せた。Claude は次のように決め、仕様に書いた（[ADR 0111](../decisions/0111-bundle-monaco-and-set-csp.md)）。

- CSP を仕様に定める。読み込み先はアプリ自身と Tauri の IPC に限り、`style-src` に `'unsafe-inline'` を許す。フォントと画像は `data:` も許す。
- Monaco は CDN を使わず、Web Worker を含めて同梱する。
- 演習画面のショートカットは、Monaco の `addCommand` で既定を上書きし、エディタの外でも効くよう window の keydown のキャプチャでも受ける。日本語の変換中はショートカットとして扱わない。
- 不要な言語・機能を外した入口を使うかは、仕様に書かず、実装で決める。

## 判定

基準「オフラインで動き、日本語を入力でき、演習画面のショートカットが効く」は、macOS で満たした。

- オフライン: 満たした。Worker 込みで、厳しめの CSP と両立し、外部読み込みは無かった。
- 日本語入力: 満たした（人が操作して確かめた）。
- ショートカット: 満たした。Monaco の既定との衝突は上書きでき、Tauri の既定メニューとも衝突しない。実キーの 3 つ（Cmd+. を含む）がページに届くことを、人が操作して確かめた。

## 残った事項

- 人が確かめる項目は、検証のあとに確かめた（「日本語入力（IME）」「ショートカット」）。エディタの外にフォーカスがあるときの実キーと、window の keydown のキャプチャで受ける場合の変換中の判定（`isComposing` または `keyCode === 229`）は、人の操作では確かめていない。実装で確かめる。
- ユーザーの判断が要るとした点（CSP を仕様に定めるか）は、検証のあとに決まった（「検証のあとの判断」）。
- 仕様の変更案（下の 2 つは ADR 0111 で仕様に書いた。3 つ目は実装で決める）:
  - 技術スタックかセキュリティとプライバシーに「Monaco は CDN を使わず同梱する（loader にローカルの monaco-editor を渡す）。Worker は同梱し `worker-src 'self'`。CSP は `style-src 'unsafe-inline'` が必要」と書く。
  - 演習画面のショートカットは Monaco の `addCommand` で登録する。エディタの外でも効かせるなら、window の keydown のキャプチャを併用する。補完を使わない（ADR 0004）ので、Rust では Cmd+.（quickFix）に何も割り当てられておらず衝突しない。
  - 補完・LSP を使わない前提で、不要な言語・機能を外した入口を使うと軽い。保守の手間（版を上げるたびの import 一覧の確認）が増える。
- 別の問題: `performance.getEntriesByType('resource')` が WKWebView の `tauri://` で空になるので、「ネットワークを使っていないことの確認」には別の方法が要る（上の「結果」の方法）。
