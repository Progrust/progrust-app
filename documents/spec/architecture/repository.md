---
type: Specification
title: "リポジトリ構成"
description: "リポジトリ直下のディレクトリ（`documents/`、`content/`、`prompts/`、`runner/`、`tools/`、`src/`、`src-tauri/`）の役割を定める。"
---

```
progrust-app/
├── documents/           # ドキュメント（仕様、設計判断記録、運用ルール）
├── content/             # 教材の原本（動画のコードと音声を含む。形式は spec/content-format/index.md）
├── prompts/             # AI への指示文の原本
├── runner/              # 許可クレートの定義
├── tools/               # 教材チェックツール、音声を作るツールなどの開発用ツール（spec/content-guidelines/check-tool.md、spec/content-format/video/voice.md）
├── src/                 # フロントエンド
│   └── video/           # 動画の共通部品（プレーヤー・タイムライン・教材用の部品。spec/content-format/video/scenes.md）
└── src-tauri/           # バックエンド
```
