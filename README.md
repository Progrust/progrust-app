# Progrust

Progrust は、プログラミング言語 Rust を学ぶためのデスクトップアプリです。コードエディタと、Claude Code との連携機能を備えたアプリを目指しています。

作者が自分の Rust の学習のために作っている、個人開発のプロジェクトです。まだ開発の途中で、アプリは配布していません。

## 目指しているアプリ

- Rust の要素（所有権、構造体、エラー処理など）ごとに、細かく区切って学びます。
- 1 つの要素ごとに、まずナレーションつきの動画で知識を得て、次にアプリの中のコードエディタで演習問題を解きます。書いたコードは、その場でビルドして、正しく動くかを判定します。
- AI（学習者の手元の Claude Code）が、演習のたびに学習者の特徴を分析して記録します。記録をもとに、その学習者に合った演習問題を作ります。
- 動画の内容や演習問題について、AI に質問できます。

## いまできていること

いまは、レッスンの一覧から動画を見て、エディタで演習問題を解き、判定するところまでを作っています。AI を使う機能（質問、学習者の分析と記録、演習問題の生成）は、これから作ります。

## 技術

- デスクトップアプリ: Tauri 2（バックエンドは Rust）、React と TypeScript
- 動画: Remotion
- AI の機能: 学習者の環境にある Claude Code を呼び出します

仕様、設計判断の記録（ADR）、開発の決まりは、すべて [`documents/`](documents/index.md) にあります。開発環境と使うコマンドは [開発環境と依存の版](documents/spec/architecture/development-environment.md) にあります。

## 動画の音声について

教材の動画の音声（`content/` の中の `.m4a` のファイル）は、VOICEPEAK で作った合成音声です。この音声を、ほかの用途に使うことは認めていません。

## ライセンス

このリポジトリには、ライセンス（LICENSE のファイル）を置いていません。コード、教材、ドキュメントは誰でも見られますが、コピー、改変、再配布などの再利用は認めていません。ただし、GitHub の利用規約が公開リポジトリについて認めている範囲（GitHub の中での閲覧と fork）は除きます。

## Issue と pull request

このリポジトリでは、外部の方からの Issue と pull request を受け付けていません。

## In English

Progrust is a desktop app for learning Rust that its author is building as a personal project for their own study. It is under development and is not distributed. The goal is an app where you learn each Rust topic through a narrated video and then solve exercises in a built-in code editor, with AI features powered by the learner's own Claude Code (answering questions, analyzing the learner, and generating exercises). Today, the lesson list, video playback, and solving and judging exercises in the editor are being built; the AI features are not built yet.

There is no license file: you may view the code and content, but reuse (copying, modifying, or redistributing) is not permitted, except for what GitHub's Terms of Service allow for public repositories (viewing and forking on GitHub). The narration audio in `content/` (`.m4a` files) was made with VOICEPEAK and may not be used for any other purpose. Issues and pull requests from outside contributors are not accepted.
