---
type: Verification Report
title: "再生中のテーマと章カラーの切り替えの技術検証"
description: "アプリのテーマと章カラーを Player の中の動画まで届け、再生を止めずに切り替えられるかを Issue #6 で検証した結果（CSS 変数の継承・inputProps・毎回新しい inputProps の 3 通りの渡し方の比較表、フレーム・音・色の変化、Shadow DOM では章カラーが祖先の属性で変わらないこと、人の目での確認が未実施であること）を記す。"
---

- 日付: 2026-11-21
- Issue: [#6](https://github.com/Progrust/progrust-app/issues/6)
- 確かめた OS: macOS 27.2.1（Apple M1）
- 関連仕様: [外観（テーマ）](../spec/screens/appearance.md)、[動画の色](../spec/content-format/video/colors.md)、[ADR 0061](../decisions/0061-render-videos-with-remotion-player.md)

## 環境

- macOS 27.2.1、Apple M1。WebKit.framework 22625.2.8.11.9（Safari 27.2 相当）。rAF 58.8Hz。
- rustc 1.99.1、node v25.9.0、Tauri CLI 2.12.6、tauri 2.12.7、Vite 8.3.8、React 19.3.1、TypeScript 6.0。
- remotion・@remotion/player・@remotion/media 4.0.545。
- 計測は release ビルドの .app を起動して行った（ページが自分で計測して JSON に書く）。ウィンドウは前面に出してフォーカスした。全回で `visibilityState=visible`、`hasFocus=true`、rAF の間隔の中央値は 17ms で、間引かれた回はなかった。

## 方法

[Issue #4 の検証結果](issue-4-lazy-load-lesson-video.md)の試作（ch04 のレッスン）を使った。試作の Stage は `data-theme="light"` を固定していたので、テーマと章を props で受け取り、渡されなければ属性を付けない（祖先から継承する）ように直した。入れ子は `data-theme > data-chapter > data-lesson（レッスンの根） > .slide`。

ch04 の文 h1（6.3 秒）の途中から 5 秒再生した。1・2・3・4 秒目に、テーマ light→dark、章 4→2、テーマ dark→light、章 2→4 の順に切り替えた。rAF ごとに、フレーム・色・AnalyserNode の RMS・`<audio>` の currentTime を記録した。

### 渡し方

| 名前 | 方法 | 回数 |
|---|---|---|
| attr | Player の外のラッパー（`<div class="video-wrap" data-theme data-chapter>`）の属性を変える。動画には CSS 変数の継承で届く。Stage は属性を付けない | 5 |
| attr（html） | `<html>` の属性を変える | 3 |
| props | `inputProps={{ theme, chapter }}`（useMemo で、切り替えたときだけ新しいオブジェクト）。Stage が `data-theme`・`data-chapter` を付ける | 5 |
| props-fresh | 親が 100ms ごとに描き直し、そのたびに新しい inputProps のオブジェクトを渡す（中身は切り替えのときだけ変わる） | 5 |
| control | 切り替えない（比較用） | 5 |
| Html5Audio で attr・props・props-fresh・control | 音声を `<audio>` にしたもの | 各 3 |
| Shadow DOM で attr・props | [Issue #5 の検証結果](issue-5-scope-lesson-css.md)の候補 4 と組み合わせたもの | 各 3 |

## 結果

### アプリのテーマと章カラーを Player の中まで届けられるか

light DOM なら、どの渡し方でも届いた。Player は iframe ではなく同じ文書の DOM なので、祖先の `data-theme`・`data-chapter` で決まる CSS 変数がそのまま継承される。

- テーマ: `tokens.css` の `[data-theme="dark"]` は、`--slide-*` の派生トークンも定義し直している。そのため、`<html>` ではなく途中のラッパーに付けても、ダークの値になった。
- 章: `slides.css` の `[data-chapter="N"] .slide` は、祖先にあればよい。

### 再生中に切り替えても途切れず色が変わるか

変わった。各項目は、その渡し方の全回で、4 つの切り替え（テーマ 2 回・章 2 回）を通した範囲か最大。

| | attr（5 回） | attr html（3） | props（5） | props-fresh（5） | control（5） | Html5Audio attr / props / fresh（各 3） | Html5Audio control（3） |
|---|---|---|---|---|---|---|---|
| 色が変わるまで（.slide の背景・シーンの .slide・c-primary） | 次の rAF（≤19ms） | ≤18ms | ≤21ms | ≤18ms | — | ≤17 / ≤18 / ≤19ms | — |
| React の commit（状態の更新から） | 2〜8ms | 1〜6ms | 4〜7ms | 2〜10ms | — | 2〜8ms | — |
| ±500ms のフレームの進み（期待 30） | 29〜30 | 29〜31 | 29〜31 | 29〜31 | — | 29〜30 | — |
| ±500ms で同じフレームが続いた最長 | 36ms | 36ms | 37ms | 36ms | — | 38ms | — |
| ±500ms の rAF の最大間隔 | 20ms | 21ms | 21ms | 23ms | — | 24ms | — |
| frameupdate の最大間隔（通し 5 秒） | 57ms | 55ms | 56ms | 56ms | 55ms | 47〜57ms | 55ms |
| 通し 5 秒: 同じフレームが続いた最長 | 58ms | 56ms | 61ms | 68ms | 56ms | 44〜64ms | 69ms |
| 通し 5 秒のフレームの進み / 逆戻り / 最大の飛び | 147〜148 / 0 / 1 | 同じ | 同じ | 同じ | 同じ | 同じ | 同じ |
| 音（@remotion/media）: ±500ms で RMS > 0.01 の割合 | 0.78〜0.97 | 同じ | 同じ | 同じ | 同じ位置で同じ値 | — | — |
| 音（Html5Audio）: ±500ms の `<audio>` の進み | — | — | — | — | — | 0.97〜1.00 秒 | — |
| 再生中（5 秒）に tauri:// で配信した音声・音声の fetch | 0・0 | 0・0 | 0・0 | 0・0 | 0・0 | 3・0（全回） | 3・0（全回） |
| Player のイベント（再生中） | play → pause だけ | 同じ | 同じ | 同じ | 同じ | 同じ | 同じ |
| シーンのマウント・アンマウント（再生中） | 0・0 | 0・0 | 0・0 | 0・0 | 0・0 | 0・0 | 0・0 |
| 切り替えの前のシーンの要素が、後も同じ要素か | 全回 true | true | true | true | — | true | — |
| Stage（React.memo）の描画（再生中） | 0 | 0 | 4（切り替えの回数） | 4 | 0 | 0 / 4 / 4 | 0 |
| コンポジションの描画（再生中） | 148〜153 | 149〜151 | 150〜152 | 191〜199 | 148〜149 | 148〜153 / 149〜153 / 190〜199 | 148〜149 |

- どの渡し方でも、色は切り替えの次の rAF（1 フレーム以内）に変わった。フレームは止まらず、飛ばずに進んだ（±500ms で 29〜31 フレーム、同じフレームの最長 38ms で、control と同じ水準）。
- 音も control と同じだった。
  - `@remotion/media`: 同じ位置の RMS の割合が全回で同じ。再生中に音声の配信も fetch もなかった。
  - Html5Audio: `<audio>` は 1 秒で 0.97〜1.00 秒進んだ。再生中の音声の配信は、control を含む全回で 3 回ずつで、切り替えで増えてはいない。
- 再生中の AudioBufferSourceNode.start を、play から 500ms ごとに数えた。
  - 0.5〜4.5 秒の区間（切り替えはすべてこの中）では、どの渡し方・どの回も 23・24・23・24・23・24・23 とほぼ同じだった。
  - 回によって違うのは、play の直後の 500ms の数だけ（21〜116）。この数は control でも違った（21・30）。play の直後に予定を組むときの揺れで、切り替えとは関係がない。
- 色の変化は、`getComputedStyle` の値と、フレーム・音の進みで確かめた。人の目では見ていない。
- この WKWebView は `performance.getEntriesByType('resource')` に tauri:// を記録しない。そのため音声の読み込み直しは、バックエンドの配信の記録（`on_web_resource_request`）とページの fetch の記録で確かめた。
- `@remotion/media` の回では、計測を始める前のシークで `waiting`→`resume` が出た（control を含む全回で同じ。再生中ではない）。Html5Audio の回では出なかった。

### inputProps を変えたときの違い

- props では、切り替えのたびにコンポジションと Stage が描き直された（Stage が 4 回）。ただし、マウントし直しはなく（シーンのマウント 0、要素はそのまま）、音声の読み込み直しもなかった。
- props-fresh（毎回新しいオブジェクト）では、コンポジションの描画が約 45 回増えた（100ms ごとの描き直しの分）。それでも、React.memo の Stage は切り替えの 4 回だけ描かれた。フレーム・音・色の結果は attr と同じだった。
- Player はもともと毎フレーム、コンポジションの根を描き直している（control でも 5 秒で 148〜149 回）。そのため、inputProps の変化による増分は小さい。
- Stage を memo にしない場合は、新しい inputProps のたびにシーンの木が描き直される。この検証では memo にしたので、その場合は測っていない。

### Shadow DOM と組み合わせた場合

- attr（祖先の属性）では、テーマは変わった（カスタムプロパティは継承で届く）が、**章カラーは変わらなかった**。章 4→2 の後も c-primary は第4章の `rgb(166, 30, 85)` のまま（3 回とも）。`[data-chapter="N"] .slide` が Shadow の境界を越えず、Shadow の中の `.slide` の既定（第4章）が使われるため。
- props（Stage が Shadow の中に `data-chapter` を付ける）では、テーマも章も変わった（3 回とも ≤19ms）。

### 1 回目の計測

1 回目の計測（音声の配信を数えていない版）も、色・フレーム・音について同じ結果だった。

## 判定

基準「再生中にテーマを切り替えても、再生が途切れず、動画の色が切り替わる」は、macOS で満たした。

- light DOM なら、3 通りの渡し方のどれでも満たした。
- Shadow DOM を使うなら、章は inputProps（または Shadow の中への属性）で渡す必要がある。
- 人の目で見て途切れが目立たないかは未確認（人が見る必要がある）。代わりの指標として、フレームの進み・rAF・音の連続を測った。

## 残った事項

- 人の目での確認（途切れが目立たないか、色の切り替わりが自然か）は未確認。
- ユーザーの判断が要る点: テーマ・章の渡し方。CSS 変数の継承（Player の外の属性）と inputProps のどちらにするか。どちらも基準を満たした。
  - [Issue #5 の検証結果](issue-5-scope-lesson-css.md)で Shadow DOM を選ぶ場合は、章カラーを inputProps で渡す必要がある。
  - 継承の方法は React の描き直しがなく最も軽い。ただし、動画画面とレッスン検索のモーダルなどで、章の違う 2 本を同じ祖先の下に置くときは、Player ごとのラッパーが要る。
- 仕様の変更案: [ADR 0061](../decisions/0061-render-videos-with-remotion-player.md) の影響欄と[外観（テーマ）](../spec/screens/appearance.md)の「再生を止めずに色が切り替わる」は、macOS では成り立った。見直しは要らない。
- Stage を memo にしない場合の inputProps の変化の影響は、測っていない。
