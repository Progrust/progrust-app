# EnvCheck

初回起動画面の、外部ツールと準備の状態の一覧。（画面デザインで追加した部品。D2 の面・角丸・文字の決まりで作った）

- 一覧は `.pg-env`（`bg` の上のカードなので `line` の枠）。行ごとに状態のアイコン、名前（`label-strong`）、詳細（`caption`・`muted`、パスは `code`）、右端に結果か操作。
- 使える: `circle-check`（`.is-ok`、墨色）と「使えます」（`.pg-ok-line`）。見つからない・古い: `triangle-alert`（`.is-ng`、`warning`）と導入・更新の手順へのボタン。準備中: 回転する `loader-circle`（`.is-busy`）。失敗: `circle-alert`（`.is-error`、`danger-ink`）と「再試行」。
- 見つからなくても先に進めることを、一覧の下か詳細に書く。
