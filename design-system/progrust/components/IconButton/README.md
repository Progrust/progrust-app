# IconButton

アイコンだけの丸いボタン。必ず `aria-label` で操作の名前を付ける。

- `.pg-icon-btn`（44px、`field` の塗り）: 基本の形。
- `.pg-icon-btn--ink`（墨色）: 動画の再生／一時停止（44px）、質問の送信（`--sm` と合わせて 32px）。
- `.pg-icon-btn--md`（36px、塗りなし、`muted`）: 閉じるボタン、動画のミュート（VideoControls の `.pg-volume` の中。アイコンは `ink`）。
- `.pg-icon-btn--sm`（32px）: エディタの「ファイルを追加」。
- 切り替えのボタン（ミュート）は `aria-pressed` を付ける。`aria-pressed="true"` のときは `ink` の塗りと `on-ink` のアイコンになる。
- アイコンは Lucide。16px 以上は線幅 2.2、14px 以下は 2.4。再生と一時停止は塗りつぶす。
