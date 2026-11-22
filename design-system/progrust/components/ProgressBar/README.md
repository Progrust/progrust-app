# ProgressBar

章の完了率の棒（`.pg-progress`）。

- 高さ 6px・角 `radius-bar`。地は `accent-soft`（学習中のタイルの中では `accent-line`）、埋まった部分は `accent`。中の `<span>` の幅で割合を示す。
- 完了数は「1/5」のように文字でも示す。
- 動画の再生位置には使わない。再生位置は Slider（`.pg-slider--accent`）で示す。
