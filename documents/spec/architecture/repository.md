---
type: Specification
title: "リポジトリ構成"
description: "リポジトリ直下のディレクトリ（`.github/workflows/`、`documents/`、`design-system/`、`content/`、`prompts/`、`runner/`、`tools/`、`src/`、`src-tauri/`）とファイル（`package.json`、`package-lock.json`）の役割を定める。"
---

```
progrust-app/
├── .github/workflows/   # CI の定義（spec/non-functional/testing.md）
├── documents/           # ドキュメント（仕様、設計判断記録、運用ルール）
├── design-system/       # デザインシステムと画面デザインのファイル（spec/screens/design-system-files.md、spec/screens/ui-design-files.md）
│   ├── progrust/        # 画面のデザインシステム「Progrust」
│   ├── progrust-video/  # 動画のデザインシステム「Progrust Video」
│   └── ui-design/       # 画面デザイン「Progrust UIデザイン」
├── content/             # 教材の原本（動画のコードと音声を含む。形式は spec/content-format/index.md）
├── prompts/             # AI への指示文の原本
├── runner/              # 許可クレートの定義
├── tools/               # 教材チェックツール、音声を作るツール、デザインシステムの検査の道具、実装のループのスクリプトなどの開発用ツール（spec/content-guidelines/check-tool.md、spec/content-format/video/voice.md、spec/screens/design-system-files.md、conventions/implementation-loop.md）
├── src/                 # フロントエンド
│   └── video/           # 動画の共通部品（プレーヤー・タイムライン・教材用の部品。spec/content-format/video/scenes.md）
├── src-tauri/           # バックエンド
│   ├── build.rs         # ビルドスクリプト。同梱する教材を選んで写す（spec/architecture/data-locations.md「アプリ同梱データ（`{app_resources}`）」）
│   ├── build/           # ビルドスクリプトの部品。テストからも使う
│   ├── app-icon.svg     # アプリのアイコンの元の絵（spec/screens/app-icon.md）
│   ├── icons/           # アプリに入れるアイコンのファイル。元の絵から作る（spec/screens/app-icon.md「変える手順」）
│   └── tests/           # バックエンドのテストと、テスト用の偽のツール（spec/non-functional/testing.md）
├── package.json         # JavaScript の依存と npm のコマンド（spec/architecture/development-environment.md）
└── package-lock.json    # JavaScript の依存のロックファイル
```
