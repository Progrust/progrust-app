# ChapterBadge

章の番号の印。`[data-chapter="N"]` の中に置くと第N章の色になる。

- `.pg-badge`（24px・角 `radius-badge`）: タイル、気をつけたい要素の行。`.pg-badge--sm`（20px・角 `radius-cell`）: 相互参照、検索結果。
- `.pg-chip`: 章の番号と名前を並べたチップ（`accent-soft` の塗りに `accent-ink` の文字）。画面上部の文脈に置く。
- 番号は章の番号（同じ言語の章の中での順番。1 から）。13 章目からは色が 1 章目に戻る（README の「章カラー」）。
- C の章は、印の中の番号の前に「C」を付ける（`C1`）。印の大きさは変えない。チップの文字は「C 第1章 C の基本」の形にする（仕様の `content-format/chapters-json.md`「章とレッスンの番号」）。色は章の番号で決まり、C の第1章は Rust の第1章と同じ色になる。
