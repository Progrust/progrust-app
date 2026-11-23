# Captions

動画の台紙の下に出す字幕で、再生中の文を 1 つ表示する。（動画への切り替えで追加した部品。D2 の面・角丸・文字の決まりで作った）

- `p.pg-captions` に、再生中の文の `text`（仕様の `content-format/video/script-json.md`）をそのまま書く。文が変わったら書き替える。表示する条件と既定（表示しない）は仕様の `screens/video-screen.md` に従う。
- 動画の枠の外（下）に置く。動画の中には置かない（動画の中に字幕の帯を置くと、文字と図が小さくなるため）。
- 文字は 15px・太さ 500・行間 1.7・`ink`、中央揃え。2 行になるときは、行の長さを揃え（`text-wrap: balance`）、文節の切れ目で折り返す（`word-break: auto-phrase`。対応していない WebView では文字の単位で折り返す）。面と枠は付けず、`bg` の上にそのまま書く。
- 高さは 2 行ぶんを確保する。文が 1 行でも 2 行でも、高さは変わらない。1 文の目安は 60 字（仕様の `content-guidelines/video.md`）で、3 行以上になる文では下に伸びる。
- 表示しないときも、場所を空けておく（仕様の `screens/video-screen.md`）。中身のない `.pg-captions` を置き、`aria-hidden="true"` を付ける。操作（VideoControls）の位置は、字幕を表示しているときと変わらない。
- 字幕の切り替えで、動画の大きさは変わらない（仕様の `screens/video-screen.md`）。
- 切り替えは VideoControls の「字幕」と、設定画面の「字幕」（Switch）で行う。
