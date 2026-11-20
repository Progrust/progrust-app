---
type: Specification
title: "コード実行・判定の概要"
description: "`cargo` を使ったビルドと実行の前提、edition 2024 と dev プロファイル、実行・提出を同時に 1 つだけ行う制約、サンドボックスを使わないことを定める。"
---

- 学習者の環境にある `cargo` を使ってビルド・実行する（検出方法は [外部ツール（cargo・claude）への依存](../architecture/external-tools.md)）。
- すべての演習は edition 2024、dev プロファイルでビルドする。
- 学習者による実行・提出は同時に 1 つだけ行う。処理中は実行ボタンと提出ボタンを無効にする。生成問題の検証は別のプロジェクト（`validate/`）で行い、学習者の実行と並行して行える。
- 学習者のコードはサンドボックスに入れず、学習者のユーザー権限で実行する（[セキュリティとプライバシー](../non-functional/security-privacy.md)）。
