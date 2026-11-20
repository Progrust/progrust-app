---
type: Specification
title: "対応環境"
description: "対応する OS（macOS、Windows）のバージョンとアーキテクチャ、学習者の環境に必要なものを定める。"
---

| OS | 対応バージョン | アーキテクチャ |
|---|---|---|
| macOS | 13 以降 | Apple Silicon・Intel（ユニバーサルバイナリ） |
| Windows | 10（22H2）以降、11 | x64 |

- Windows のインストーラーには WebView2 ランタイムの導入処理を含める。
- 学習者の環境に必要なもの: Rust ツールチェーン（rustc 1.85 以上）、Claude Code（AI 機能を使う場合）。検出方法は [外部ツール（cargo・claude）への依存](../architecture/external-tools.md)。
