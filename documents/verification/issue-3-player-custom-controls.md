---
type: Verification Report
title: "独自のプレーヤー操作を Player の API で作れるかの技術検証"
description: "Remotion Player の外に動画画面の操作（再生位置の取得と変更、文ごとの移動、5 秒のスキップ、シークバーのシーンの印、枠の外の字幕、文の単位での位置の保存、Space・←→・Shift+←→ のキー操作）を作れるかを Issue #3 で検証した結果（自動確認 21 項目、キー操作の衝突を避ける controls={false} と spaceKeyToPlayOrPause={false}、合成イベントでの確認の限界）を記す。"
---

- 日付: 2026-11-21
- Issue: [#3](https://github.com/Progrust/progrust-app/issues/3)
- 確かめた OS: macOS 27.2.1（Apple M1）だけ。Windows（WebView2）は確かめていない
- 関連仕様: [動画画面](../spec/screens/video-screen.md)、[ADR 0061](../decisions/0061-render-videos-with-remotion-player.md)、[ADR 0067](../decisions/0067-captions-as-app-feature.md)、[ADR 0070](../decisions/0070-video-controls-sentence-jump-and-5s-skip.md)、[ADR 0071](../decisions/0071-save-playback-position-by-sentence.md)

## 環境

- macOS 27.2.1、Apple M1、メモリ 16GB。WebKit.framework 22625.2.8.11.9（Safari 27.2 と同じ）。
- rustc 1.99.1、node v25.9.0、npm 11.12.1。
- Tauri CLI 2.12.6、tauri 2.12.7、wry 0.57.3、@tauri-apps/api 2.12.5。React 19.3.1、Vite 8.3.8、TypeScript 6.0。
- remotion・@remotion/player・@remotion/media 4.0.545。
- 計測時のウィンドウの状態は、Issue #1 と同じ（[Issue #1 の検証結果](issue-1-remotion-player-webview.md)「環境」）。controls の確認は 2 回行った。

## 方法

### 作ったもの

Issue #1 と同じ試作（Tauri の最小アプリに Remotion の `Player` を置き、第 4 章のレッスン 3175 フレーム・台本 21 文を再生する）に、動画画面の試作を作った。Player は次の props で標準の操作を切り、操作は Player の外に作った。

```tsx
<Player controls={false} clickToPlay={false} spaceKeyToPlayOrPause={false}
        moveToBeginningWhenEnded={false} playbackRate={rate} preservePitch />
```

作った操作は、再生／一時停止、時間、5 秒戻る／進む、前の文へ／次の文へ、速度、音量・ミュート、字幕、演習へ、シークバー（ドラッグで移せる。シーンの印つき）、Space・←→・Shift+←→ のキー操作（`window` の `keydown`。文字の入力欄では効かない）、文の単位での位置の保存（Tauri のコマンドで JSON に書く）。独自のボタンは、`mousedown` で `preventDefault` してフォーカスを取らないようにした（Space でフォーカスのあるボタンも押されるのを防ぐため）。

### 確認の仕組み

- release ビルドの .app を環境変数（`PROGRUST_BENCH=controls`）付きで起動する。ページ（`tauri://localhost`）が自分で操作と確認を行い、結果を Tauri のコマンドで JSON に書き出して自分で終了する。
- 確認は 21 項目。`@remotion/media` の Audio と Html5Audio のそれぞれで 1 回ずつ実行した。
- キー操作は、`window` に合成した `KeyboardEvent`（`new KeyboardEvent('keydown', { key: ' ' })` など）を dispatch して確かめた。本物のキー入力は使っていない（人の操作か、アクセシビリティの許可が要るため）。

## 結果

自動確認 21 項目は、`@remotion/media` と Html5Audio のどちらでも、すべて通った。

### Player の外からの操作

- 再生位置の取得・変更（`getCurrentFrame`・`seekTo`）、再生・一時停止（`play`・`pause`・`toggle`・`isPlaying`）、音量（`setVolume`・`getVolume`）、ミュート（`mute`・`unmute`・`isMuted`）が使えた。
- 使えたイベントは `frameupdate`・`seeked`・`play`・`pause`・`ended`・`ratechange`・`volumechange`。
- 再生速度は ref のメソッドがなく、Player の prop `playbackRate` で変える。1.5 倍にして実測 1.517 倍、`ratechange` の値は 1.5 だった。

### 仕様の操作の試作

| 操作 | 結果 |
|---|---|
| 前の文へ・次の文へ | 最初の文の前・最初の文・途中・最後の文で確かめた。最初の文で前の文へ → 最初の文の頭。最後の文で次の文へ → 動かない（ボタンは無効） |
| 5 秒戻る・進む | 先頭・末尾を越えるときは、先頭・末尾で止まる |
| 再生位置を移したときの状態 | 再生中は再生中のまま、一時停止中は一時停止のまま |
| シークバーのシーンの印 | 4 つ。各シーンの開始位置 |
| 動画の枠の外（下）の字幕 | 再生中の文の台本の `text` を出す。字幕の場所は常に確保してあり、切り替えても動画の大きさ（1120×630）は変わらない |
| 文の単位での位置の保存 | 4 文目（m4）の途中で保存 → 開き直すと m4 の頭で一時停止していた |
| 音量・速度の設定の保存 | 保存して復元できた |
| 末尾への到達 | シークで末尾に着くと「演習へ」が出る |

### キー操作と Player 自身の操作の衝突

- 合成した `KeyboardEvent` では、Space・←→・Shift+←→ が 1 回押すと 1 回だけ効いた。文字の入力欄では効かなかった。
- `controls={false}` のとき、Player の操作の部品（Controls）は描画されない（ソースで確認）。Player がキーを扱うのは、この Controls の中だけである。
  - 再生ボタンにフォーカスを移して Space をボタンの既定の動作で受けること。
  - 速度のメニューが開いているときの ↑↓・Enter・Esc。
- Player 自身のキー操作は、props の `controls={false}` か `spaceKeyToPlayOrPause={false}` で切れる。
- 再生・シーク・一時停止のあとも、フォーカスは Player の中に入らなかった（`document.activeElement` は `BODY`）。
- 参考として、標準の操作を出す（`controls` と `spaceKeyToPlayOrPause` を true）と、再生・一時停止のたびに Player の再生ボタン（aria-label `"Pause video"`）にフォーカスが移った。この状態で本物の Space を押すと、アプリのショートカットとボタンの両方が効いて、二重に切り替わるおそれがある。合成したイベントではボタンの既定の動作が起きないので、この二重の切り替えは確かめられなかった。

## 判定

| 基準 | 結果 |
|---|---|
| 動画画面の仕様のプレーヤーの操作とキーボードショートカットを、すべて実装できる見込みが立つ | 満たした。macOS で、すべて実装できた。キーは合成イベントでの確認 |

Issue は閉じる。Windows は確かめていない（当面 macOS に集中する方針による）。

## 残った事項

実装の決まりにするとよい点（仕様の変更案。documents/ は変更していない）

- Player の標準の操作を使わない。`controls={false}` と `spaceKeyToPlayOrPause={false}` にする。動画画面のキーボードショートカットの仕様に、実装の決まりとして書くとよい。
- 速度は ref のメソッドがなく、`playbackRate` の prop で変える。

確かめられなかった項目

- 本物のキー入力での確認（合成したイベントだけで確かめた）。標準の操作を出したときの二重の切り替えも、本物のキーでは確かめていない。
- Windows の WebView2 での動作。

別に確かめた結果（関連する事実）

- 絵と音のずれ・再生開始・シークの時間は [Issue #1 の検証結果](issue-1-remotion-player-webview.md)と [Issue #2 の検証結果](issue-2-m4a-audio-sync.md)に書いた。
- `@remotion/media` では、Player が文ごとに `waiting` イベントを出す。`waiting` を読み込み中の表示に使うと、文ごとにちらつくおそれがある（[Issue #1 の検証結果](issue-1-remotion-player-webview.md)「残った事項」）。
- 試作の `Stage` は `data-theme="light"` を固定で書いていた。テーマの切り替えでは変える必要がある。
