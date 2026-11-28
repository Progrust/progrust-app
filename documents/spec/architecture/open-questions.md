---
type: Open Questions
title: "アーキテクチャの未決事項"
description: "アーキテクチャの仕様でまだ決まっていない事項（読み込めないファイルを退避したときの通知の出し方、Command Line Tools が入っていない Mac で `clang` の有無を確かめる方法）を記す。"
---

- 読み込めないファイル（`settings.json`・`progress.json`・`jobs.json`）を退避して初期状態で作り直したときと、読み込めない下書き・試行ログを退避したとき（[データの保存場所](data-locations.md)「書き込みの規則」）の、ユーザーへの通知の出し方と文言（下書き・試行ログを退避したときに通知するかどうかを含む）。通知を運ぶコマンド・イベントも無い（[コマンドとイベント](commands/index.md)）。
- Command Line Tools が入っていない Mac で、`clang` の有無を確かめる方法（[C のツール（clang）への依存](c-toolchain.md)「検出」）。Command Line Tools が無い Mac の `/usr/bin/clang` は、起動すると Command Line Tools の導入を促すダイアログを出すので、それを起動せずに確かめる方法を、技術検証で決める。
