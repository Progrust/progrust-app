# SettingsRow

設定画面の 1 項目の行。左に名前と説明、右に操作を置く。（新規の案。D2 のモックアップにない部品を、D2 の面・角丸・文字の決まりで作った）

- 区分ごとに `.pg-settings`（`panel` の面）でまとめ、区分の見出しは `heading` で面の上に置く。
- 行は `line-soft` で区切る。説明は `caption` の `muted`。検出したパスなどは `<code>` で等幅にする。
- 右側には Button・SegmentedControl・Select・Switch・音量（VideoControls の `.pg-volume`）のどれか 1 つを置く。リセットの操作は灰のボタンで出し、確定は Dialog の中の危険ボタンで行う。
- 「動画」の区分（仕様の `screens/settings-screen.md`）: 音量とミュートは `.pg-volume`、再生速度は SegmentedControl、字幕は Switch。動画のプレーヤーで変えた値と同じ設定を表示する。
