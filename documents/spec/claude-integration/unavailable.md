---
type: Specification
title: "claude が使えない場合の動作"
description: "`claude` が見つからない間の質疑応答・問題生成・ジョブの動作と、設定画面の「接続テスト」ボタンの動作を定める。"
---

- `claude` の検出方法は [外部ツール（cargo・claude）への依存](../architecture/external-tools.md) に従う。
- `claude` が見つからない間は、次のように動作する。
  - 質疑応答の入力欄を無効にし、「Claude Code が見つかりません」と導入手順へのリンクを表示する。
  - 問題生成の「生成」ボタンを無効にする。
  - ジョブは登録するが実行しない（`pending` のまま残す）。`claude` が見つかったら実行を始める。
- 設定画面の「接続テスト」ボタン（`test_claude_connection`）で、短い指示文を質疑応答用のモデルで実行し、応答が返るかを確認できる。
