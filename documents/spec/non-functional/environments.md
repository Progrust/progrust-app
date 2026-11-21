---
type: Specification
title: "対応環境"
description: "対応環境を作者の Mac（macOS、Apple Silicon）だけとすること、macOS の他のバージョンと他の OS を開発で考慮しないこと、学習者の環境に必要なものと rustc の最低バージョンの上げ方を定める。"
---

| OS | 対応バージョン | アーキテクチャ |
|---|---|---|
| macOS | 作者の Mac に入っているバージョン（現在は 27.2.1） | Apple Silicon |

- 対応環境は、アプリの作者が使っている Mac（作者の Mac）だけとする。アプリは、この Mac で動けばよい。
- macOS の他のバージョン、Intel の Mac、Windows などの他の OS は、開発で考慮しない。
- 学習者の環境に必要なもの: Rust ツールチェーン（rustc 1.100.0 以上）、Claude Code（AI 機能を使う場合）。検出方法は [外部ツール（cargo・claude）への依存](../architecture/external-tools.md)。
- rustc の最低バージョンは、アプリのリリースに合わせては上げない。上げるときは作者が決める。
