# Switch

オン/オフをすぐ切り替える設定のスイッチ（生成設定画面の「苦手分野を中心にする」、設定画面の「字幕」など）。（画面デザインで追加した部品。D2 の面・角丸・文字の決まりで作った）

- `label.pg-switch` の中に `input[type=checkbox][role=switch]` と文言を置く。文言は `label-strong`。
- オンの塗りは `accent`、つまみは `on-accent`。章外の画面では墨色になる。オフは `field` に `shadow-control` の輪郭、つまみは `muted`。
- 無効にするときは `disabled` を付け、切り替えられない理由を `.pg-field-help` で添える。
- 押した結果がすぐ反映される設定にだけ使う。「保存」などの確定が必要な項目には Checkbox を使う。
