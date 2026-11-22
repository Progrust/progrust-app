# GeneratedList

生成問題一覧の行。問題名・状態・概要・使う要素・難易度・操作を出す。（画面デザインで追加した部品。D2 の面・角丸・文字の決まりで作った）

- 一覧は `.pg-gen-list`（`bg` の上のカードなので `line` の枠）、行は `.pg-gen`、行の区切りは `line-soft`。
- 状態の印は LessonList と同じ `.pg-status` に、生成中（`.pg-status--wait` と回転する `loader-circle`）と失敗（`.pg-status--ng` と `circle-alert`）を足した。生成問題は章の外なので、完了の塗りは墨色になる。
- 使う要素は `.pg-elchip`（その要素の章のバッジだけに `data-chapter`）、難易度は DifficultyMeter。
- 失敗した行は `.pg-gen-error`（`danger-soft`）に理由を書き、「再生成」を出す。生成中の行は問題名を `ink-3` にし（`.pg-gen--busy`）、開けないようにする。
- 操作は「開く」「同じ設定で生成」と、削除のアイコンボタン。
