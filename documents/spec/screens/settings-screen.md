---
type: Specification
title: "設定画面と settings.json"
description: "設定画面の設定項目（表示、動画、AI、環境、学習データ、アプリ）とリセット操作の確認ダイアログ、`settings.json` の形式を定める。"
---

## 設定項目

| 区分 | 項目 | 内容 |
|---|---|---|
| 表示 | テーマ | ライト / ダーク / システムに合わせる |
| 表示 | エディタの文字サイズ | 12〜24px |
| 動画 | 音量とミュート | 動画の音量と、ミュートするかどうか（[動画画面](video-screen.md)） |
| 動画 | 再生速度 | [動画画面](video-screen.md) の再生速度から選ぶ |
| 動画 | 字幕 | 表示する / 表示しない（[動画画面](video-screen.md)） |
| AI | 質疑応答のモデル、記録・分析・問題生成のモデル | [AI モデルの選択](../claude-integration/model-selection.md) |
| AI | 接続テスト | [claude が使えない場合の動作](../claude-integration/unavailable.md) |
| 環境 | Rust ツールチェーン | 検出したパスとバージョン、パスの手動指定、再検出 |
| 環境 | Claude Code | 検出したパスとバージョン、パスの手動指定、再検出 |
| 環境 | クレートの準備 | 初回準備の状態と、失敗時の再試行（[許可クレートと作業領域](../exercise-runner/crates-and-workspace.md)「初回準備」） |
| 学習データ | 学習記録をエディタで開く | `records/` ディレクトリを OS の既定アプリで開く |
| 学習データ | 分析ファイルをエディタで開く | `analysis/analysis.md` を OS の既定アプリで開く |
| 学習データ | 分析を再構築 | rebuild ジョブを登録する（[学習記録・分析の AI ジョブ](../learning-record-and-analysis/ai-jobs.md)「分析の再構築（rebuild）」） |
| 学習データ | 分析のみリセット | `analysis.md` をテンプレートに戻す（記録は保持） |
| 学習データ | 学習履歴をすべてリセット | `attempts/`・`records/`・`analysis/`・`jobs.json`・`progress.json`・`drafts/` を削除する。生成問題は残し、`meta.json` の完了に関する項目を初期値に戻す |
| 学習データ | 失敗したジョブ | `failed` のジョブの一覧（種類・対象・エラー内容）と再実行ボタン |
| 学習データ | 読み込めない学習記録 | frontmatter を解析できず集計から除外された記録の一覧（[学習記録・分析ファイルの手動編集への対応](../learning-record-and-analysis/manual-edit.md)） |
| アプリ | バージョン情報 | 現在のバージョンと、更新の確認ボタン（[配布と更新](../non-functional/distribution.md)） |

- リセット操作は確認ダイアログを表示してから実行する。「学習履歴をすべてリセット」は、確認ダイアログでテキスト「リセット」の入力を求める。

## settings.json

```json
{
  "schema_version": 1,
  "theme": "system",
  "editor_font_size": 14,
  "video": { "volume": 1.0, "muted": false, "playback_rate": 1.0, "captions": false },
  "models": { "chat": "sonnet", "background": "opus" },
  "tool_paths": { "cargo": null, "claude": null },
  "first_launch_completed": true
}
```

- `video.volume` は 0.0〜1.0、`video.playback_rate` は [動画画面](video-screen.md) の再生速度のどれかとする。
- `tool_paths` が `null` の場合は自動検出する。
