# VideoControls

動画の下に置くプレーヤーの操作で、シークバーの下に、再生・位置の移動・再生速度・音量・字幕のボタンを並べる。（動画への切り替えで追加した部品。D2 の面・角丸・文字の決まりで作った）

操作の種類・動作・キーの割り当ては仕様の `screens/video-screen.md` に従う。ここでは見た目と組み方だけを定める。

| 操作 | 部品 | 見た目 |
|---|---|---|
| シークバー | Slider（`.pg-slider--accent`） | 再生済みの範囲は `accent`、残りは `accent-line`。シーンの区切りに `.pg-slider-mark` |
| 再生／一時停止 | IconButton（`.pg-icon-btn--ink`、44px） | 塗りつぶした `play` / `pause`（18px） |
| 時間 | `.pg-video-time` | 「0:42 / 1:44」。現在の位置は `ink` の太字、全体の長さは `muted`。数字は等幅（`tabular-nums`） |
| 5 秒戻る／5 秒進む | Button（灰） | `rotate-ccw`＋「5 秒」、「5 秒」＋`rotate-cw` |
| 再生速度 | Select（`.pg-select--pill`） | 「0.75 倍」「1 倍」「1.25 倍」「1.5 倍」「2 倍」 |
| 音量とミュート | `.pg-volume` | `field` の丸い面に、ミュートの IconButton（`volume-2`。ミュート中は `volume-x` と `aria-pressed="true"`）と Slider（幅 72px、墨色） |
| 字幕 | Button（灰、`aria-pressed`） | `captions`＋「字幕」。表示中は `aria-pressed="true"`（`ink` の塗り） |
| 演習へ | Button（主、`--lg`） | 表示する条件は仕様の `lesson-flow/flow.md`。条件を満たすまでは置かない（無効のボタンとしても出さない）。動画のみのレッスンでは置かない |
| 次のレッスンへ | Button（主、`--lg`） | 動画のみのレッスンだけで、「演習へ」の位置に置く。「レッスン一覧へ」は置かない（画面の左上の「レッスン一覧」で移る）。表示する条件は仕様の `lesson-flow/flow.md`。条件を満たすまでは置かない（無効のボタンとしても出さない）。最後のレッスンでは置かない（仕様の `lesson-flow/order.md`） |

- `.pg-video-controls` の中に、Slider と操作の行（`.pg-video-bar`）を置く。行は 3 つのまとまりに分ける: 再生と位置の移動（`.pg-video-group`）、再生速度・音量・字幕（`.pg-video-group`）、右端の行動（`.pg-video-end`）。
- 行は折り返す。幅が足りないときは、右のまとまりから順に 2 行目に移る。まず右端の行動が 2 行目の右に移り、さらに狭いと、再生速度・音量・字幕が 2 行目の左に移る。
- 見本の文言で測った幅の目安: 動画画面（右端が「演習へ」）で 1 行に収まるのは約 720px から。動画のみのレッスン（右端が「次のレッスンへ」）では約 775px から。約 615px より狭いと、再生速度・音量・字幕も 2 行目に移る。最小のウィンドウで質問パネルを開いた幅（約 620px）では、「演習へ」だけが 2 行目に移る。VideoModal（840px）では「このレッスンを開く」まで 1 行に収まる（約 805px から）。
- 章の色を使うのは、シークバーと右端の主ボタンだけ。再生／一時停止は墨色（`ink`）、ほかのボタンは `field` の灰にする。
- 位置を移すボタンは、文字を必ず出す（アイコンだけにしない）。
- キーボードショートカットは、各ボタンの `title` と `aria-keyshortcuts` に書く（「再生（Space）」「5 秒戻る（←）」「5 秒進む（→）」）。
- ミュートしても、音量の Slider の位置は変えない（音量とミュートは別の設定。仕様の `screens/settings-screen.md`「settings.json」）。
- アイコンは Lucide の `play`・`pause`・`rotate-ccw`・`rotate-cw`・`volume-2`・`volume-x`・`captions`・`chevron-down`。
- レッスン検索のモーダル（VideoModal）でも同じ操作を使い、右端の行動だけを「このレッスンを開く」に替える。
