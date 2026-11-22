# PassModal

演習に正解したときに開くモーダル。解説、解説への質問（ChatPanel）、次へ進むボタンをまとめて出す。（画面デザインで追加した部品。D2 の面・角丸・文字の決まりで作った）

- `.pg-scrim` の中に `.pg-pass`（`bg`、`radius-section`、`shadow-pop`）を置く。最大 1160×720px。
- 見出し（`.pg-pass-head`）は `accent-soft` の面に、`accent` の円の `check`（28px、線幅 3）と「正解」（`accent-ink`）、通ったテストの件数と完了したレッスン。右に章のチップ（生成問題は「生成問題」の Tag）と閉じるボタン。生成問題では墨色になる。
- 左（`.pg-pass-main`）は Tabs で「解説」「問題文」「提出したコード」を切り替え、本文は `.pg-prose.pg-prose--on-bg`。右に幅 360px の ChatPanel（解説の質問は空の状態から始まる）。
- 足元（`.pg-pass-foot`）は左に「閉じてコードを見る」（テキストボタン）、右に「レッスン一覧へ」と主ボタンの「次のレッスンへ」（`pg-btn--accent pg-btn--xl`、次のレッスン名を `.pg-btn-sub` で添える）。生成問題は「生成問題一覧へ」だけ。
- 閉じたあとは SolvedBar から開き直せる。
