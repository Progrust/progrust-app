---
type: Open Questions
title: "コード実行・判定の未決事項"
description: "コード実行・判定の仕様でまだ決まっていない事項（C の演習の提出で、`clang` の版が変わったときの検査の報告と警告の変化への対応）を記す。"
---

## C の演習の提出

[C のコードの提出（submit）](c-submit.md) について、次のことが決まっていない。

- Command Line Tools の更新で `clang` の版が変わったときの、検査の報告の書式の変化と、警告（`-Wall -Wextra`）の出方の変化への対応。Intel の Mac と、ほかの版の macOS は、対応環境の外である（[対応環境](../non-functional/environments.md)）。
