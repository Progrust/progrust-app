# Slider

棒の上のつまみで値を示す・変える部品で、動画の再生位置（シークバー）と音量に使う。（動画への切り替えで追加した部品。D2 の面・角丸・文字の決まりで作った）

- `.pg-slider` の中に、棒（`.pg-slider-track`）・塗り（`.pg-slider-fill`）・つまみ（`.pg-slider-thumb`）を置く。値は 0〜1 の数を `--pg-slider-value` に渡す。
- 棒は高さ 6px・角 `radius-bar`（ProgressBar と同じ）。つまみは 14px の円。押せる高さは 20px。
- 既定は墨色（塗りとつまみが `ink`、地が `chart-bar`）。章の進み具合を示すとき（シークバー）だけ `.pg-slider--accent` を付け、塗りとつまみを `accent`、地を `accent-line` にする。
- 印（`.pg-slider-mark`）は、棒の上下に 4px ずつはみ出す幅 2px の線（`muted`）で、`left` に位置を % で渡す。シークバーのシーンの区切り（各シーンの開始位置。仕様の `content-format/video/timeline-and-cues.md`）に使う。
- `role="slider"`・`tabindex="0"`・`aria-label`・`aria-valuemin`・`aria-valuemax`・`aria-valuenow` を付ける。再生位置のように数だけでは伝わらない値は、`aria-valuetext`（「0 分 42 秒」）も付ける。
- 使えないときは `aria-disabled="true"`（地は `disabled`、塗りとつまみは `disabled-ink`）。
- 注意: 地（`accent-line`・`chart-bar`）は、置く面に対して 3:1 に届かない。値は塗りとつまみの位置で読ませる。
