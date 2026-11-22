# KeyHint

キーボードの操作を添える小さな案内（「Esc で閉じる」など）。（画面デザインで追加した部品。D2 の面・角丸・文字の決まりで作った）

- `.pg-kbd` の中に、キーを `kbd`（`field` の面に `code` の書体）で書き、続けて操作を `caption`・`muted` で書く。
- ボタンの `title` と `aria-keyshortcuts` にも同じショートカットを書く。
- 動画のプレーヤーのショートカット（仕様の `screens/video-screen.md`）は、各ボタンの `title` と `aria-keyshortcuts` に書く（VideoControls）。
