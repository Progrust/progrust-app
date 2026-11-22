使わない部品。画面に表示しない補足は、台本（`script.json`）の `ai_note` に移った（仕様の `content-format/video/script-json.md`）。以前の `aside.ai-note`（画面に出さない要素）は、動画では書かない。

## 動画での書き方

- `ai_note` に何を書くかは仕様書に従う。動画だけでは伝えにくい背景知識を書くことは仕様の `content-guidelines/video.md`「構成と長さ」、マーカーや枠で指すときに、指している字句や範囲（行番号）を書くことは仕様の `content-guidelines/video.md`「コード」 に定めてある。質疑応答で AI に渡される（仕様の `qa-chat/ai-input.md`「動画の再生中」）。
- 画面にも字幕にも出ないので、見た目の規則はない。
- `bundle.css` の `.video-stage aside.ai-note { display: none }` は、古い書き方が残っていても画面に出さないための規則で、新しく使わない。
- この部品（README と見本）は、消してよいかの確認が済むまで残している。
