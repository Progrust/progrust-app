---
type: Specification
title: "セキュリティとプライバシー"
description: "学習者のコードの実行、アプリ自体の制限、データの送信（Anthropic への送信の説明を含む）に関する方針を定める。"
---

## 学習者のコードの実行

- 学習者のコードはサンドボックスに入れず、学習者のユーザー権限で実行する。C の演習のコードも、実行・提出のどちらでも同じである（[C のコードの実行（run）](../exercise-runner/c-run.md)、[C のコードの提出（submit）](../exercise-runner/c-submit.md)）。
- 生成問題の検証では、AI が生成した模範解答と初期コードを自動で実行する（[演習問題生成の出力と検証](../exercise-generation/output-and-validation.md)「検証」）。問題生成の指示文で、ファイル・ネットワーク・プロセスを操作するコードを生成しないよう指示する。実行には [実行・判定の制限値](../exercise-runner/limits.md) の制限時間をかける。

## アプリ自体

- 動画のコードはアプリのビルドに含める教材で、実行時に外部から読み込まない（[フロントエンドとバックエンドの役割分担](../architecture/responsibilities.md)）。動画のコードから Tauri の API・ネットワーク・ファイルにアクセスすることを、教材チェックツールで禁じる（[動画のコードの禁止事項](../content-format/video/prohibited-code.md)）。
- フロントエンドに許可する Tauri の権限は最小限にする（[Tauri の権限](../architecture/tauri-permissions.md)）。
- アプリのページに CSP（Content Security Policy）を設定する。スクリプト・Worker・接続の読み込み先は、アプリ自身（`'self'`）と Tauri の IPC に限る。`style-src` には、Monaco Editor が実行時にスタイルを作るため `'unsafe-inline'` を許す。フォントと画像は `data:` も許す。音声（`media-src`）は、アプリ自身と、Tauri のアセットプロトコル（[Tauri の権限](../architecture/tauri-permissions.md)）と、`data:` に限る。
- デスクトップの開発時の起動（`tauri dev`）では、CSP が掛からない。CSP の下で動くかどうかは、本番ビルド（`tauri build`）で確かめる。
- AI にはツールを使わせない（[claude -p の呼び出しの共通ルール](../claude-integration/invocation.md)）。

## データの送信

- 学習データはすべて学習者の PC 内に保存する。アプリ独自のサーバーへの送信や利用状況の収集（テレメトリ）は行わない。
- AI 機能を使うと、学習者のコード・質問・学習記録・分析ファイルの内容が Claude Code を通じて Anthropic に送信される。このことを初回起動画面で説明する（[初回起動画面](../screens/first-launch.md)）。
- 外部への通信は、Claude Code の呼び出しと、クレートのダウンロード（初回準備）に限る。
