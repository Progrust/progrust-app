---
type: Open Questions
title: "教材作成ガイドラインの未決事項"
description: "教材作成ガイドラインでまだ決まっていない事項（検索ワード（`tags`）の選び方、CI での C の教材の判定）を記す。"
---

- 検索ワード（`lesson.json` の `tags`）に書く語の選び方（語の数、英語の表記や記号・エラーコードを含めるか など）。
- CI での C の教材の判定。教材チェックツール（[教材チェックツール](check-tool.md)）は CI でも実行するが、C のレッスンの「模範解答」「初期コード」の判定には、macOS と `clang` が要る（[C のツール（clang）への依存](../architecture/c-toolchain.md)）。メモリリークの検査（[C のコードの提出（submit）](../exercise-runner/c-submit.md)「メモリリークの検査」）は、macOS でだけ行える。CI の環境（[テストの方針](../non-functional/testing.md)「CI」）で、C のレッスンのこの 2 つの検査をどう動かすか（macOS で動かす、メモリリークの検査だけを省く、CI では検査しない など）。
