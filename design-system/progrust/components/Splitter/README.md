# Splitter

演習画面で、問題文とエディタ、エディタとコンソールの境界をドラッグで動かす取っ手。（画面デザインで追加した部品。D2 の面・角丸・文字の決まりで作った）

- 縦の境界は `.pg-split-v`（幅 14px）、横の境界は `.pg-split-h`（高さ 12px）。中央に `chart-bar` の短い棒を置き、マウスを重ねたときとフォーカスしたときは `ink-3` に濃くする。
- `role="separator"`・`aria-orientation`・`aria-label`・`tabindex="0"` を付け、キーボードでは矢印キーで動かす。
