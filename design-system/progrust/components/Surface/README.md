# Surface

面の重ね方の見本。案 D から受け継いだ「白と `panel` の面の分け方」を部品の土台にする。

- `bg`（白）: 画面の地。`panel`: 地の上に置く面（`.pg-panel`、角 `radius-section`、内側 20/22px）。`raised`: `panel` の上に置く面。
- 面の境目は塗りの差で示し、枠線を付けない。枠線を付けるのは `bg` の上に置くカード（`.pg-card`、`line` の枠）だけ。
- 影は浮かぶ層（`shadow-pop`）と動画の台紙（`shadow-video`）だけに使う。
- 面の見出しは `subheading`、本文は `body-sm` の `ink-2`。
