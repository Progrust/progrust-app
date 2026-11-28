---
type: Specification
title: "コード実行・判定の概要"
description: "Rust の演習を `cargo` で、C の演習を `clang` でビルド・実行する前提と、それぞれの手順の参照先、Rust の演習の edition 2024 と dev プロファイル、C の演習と Rust の演習を合わせて実行・提出を同時に 1 つだけ行う制約、サンドボックスを使わないことを定める。"
---

- Rust の演習（レッスンの演習のうち Rust のレッスンのものと、生成問題）は、学習者の環境にある `cargo` を使ってビルド・実行する（検出方法は [外部ツール（cargo・claude）への依存](../architecture/external-tools.md)）。手順は [実行・提出時のプロジェクトの組み立て](project-assembly.md)、[コードの実行（run）](run.md)、[コードの提出（submit）](submit.md) に従う。
- C の演習（C のレッスンの演習）は、学習者の環境にある `clang` でビルドする（検出方法は [C のツール（clang）への依存](../architecture/c-toolchain.md)）。手順は [C の演習の作業領域とビルド](c-project-assembly.md)、[C のコードの実行（run）](c-run.md)、[C のコードの提出（submit）](c-submit.md) に従う。
- Rust の演習は、すべて edition 2024、dev プロファイルでビルドする。
- 学習者による実行・提出は同時に 1 つだけ行う。C の演習と Rust の演習を合わせて 1 つである。テスト実行（[コードの実行（run）](run.md)「テスト実行」）も実行に含める。処理中は実行ボタン・テストボタン・提出ボタンを無効にする。生成問題の検証は別のプロジェクト（`validate/`）と別のビルド先で行い、学習者の実行と並行して行える（[許可クレートと作業領域](crates-and-workspace.md)「作業領域」）。
- 学習者のコードはサンドボックスに入れず、学習者のユーザー権限で実行する（[セキュリティとプライバシー](../non-functional/security-privacy.md)）。
