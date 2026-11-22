# JudgeResult

提出の結果のカード。演習画面の左の列の、問題文の上に置く。コンソールの出力はそのまま残す。

- 不正解: 面を `danger-soft` にして、ひと目で不正解と分かるようにする。`danger` の円に `x`、「不正解」は `danger-ink`、下に通過数と提出した時刻（`.pg-judge-sub`）。
- 既定では見出しの行だけを出し、詳細は畳んでおく。右端の「詳細を見る」（`.pg-judge-toggle`、`aria-expanded`・`aria-controls`）で `.pg-judge-detail` を開閉する。次に提出したら最新の結果に置き換え、畳んだ状態に戻す。
- 詳細: テストの一覧（`.pg-cases`）は、通ったものに `check`（`accent-ink`）、失敗したものに `x` と太字（`danger-ink`）、時間切れに `clock`。色だけで区別しない。最初に失敗したケースの入力・期待する出力・実際の出力（または panic のメッセージ）を `.pg-case`（`raised` の欄）に出し、違う行を `.diff` にする。空白は `␣`（`.pg-ws`）で見えるようにし、そのことを `.pg-judge-time` で添える。
- ビルドに失敗: `.pg-judge--warn` で面を `warning-soft`、円を `warning` の塗りに `triangle-alert`（線は `raised`）。テストを実行していないこと、エラーはコンソールにあることを書き、隠しテストの側のエラーなら `.pg-judge-note` で確かめる点を添える。
- 正解は、このカードではなく PassModal で示す。
