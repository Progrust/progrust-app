# Tabs

`panel` の上で表示を切り替えるタブ（問題文 / 解説、コンソール / 判定結果）。

- 選択中のタブは `raised` の塗り・`ink` の文字・`shadow-tab` の輪郭。ほかは塗りなしの `muted`。
- `role="tablist"` / `role="tab"` / `aria-selected` を付ける。
- 必ず `panel` の上に置く（`bg` の上では選択中のタブが沈む）。`bg` の上で選ばせるときは SegmentedControl を使う。
