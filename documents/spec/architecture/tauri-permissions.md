---
type: Specification
title: "Tauri の権限"
description: "フロントエンドに許可する Tauri の権限（capabilities）の範囲、アセットプロトコルで読める範囲、外部サイトをアプリ内で表示しないことを定める。"
---

- フロントエンドに許可する Tauri の権限（capabilities）は、本アプリのコマンドの呼び出しと、外部 URL を OS の既定ブラウザで開くことに限る。ファイルシステム・シェルのプラグインはフロントエンドに公開しない。
- Tauri のアセットプロトコル（`asset://`）を有効にする。読める範囲（スコープ）は、アプリ同梱データの動画の音声（`{app_resources}/content/lessons/{lesson_id}/video/voice/` の `.m4a`）に限る。使い方は [フロントエンドとバックエンドの役割分担](responsibilities.md) に定める。
- 外部サイトのページをアプリ内で表示しない。
