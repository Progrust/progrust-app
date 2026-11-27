---
type: Specification
title: "設定画面と settings.json"
description: "設定画面の設定項目（表示、動画、AI、環境、学習データ、アプリ）と、環境の項目のパスの手動指定と再検出、リセット操作の確認ダイアログ、`settings.json` の形式と初期値を定める。"
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
| 環境 | Rust ツールチェーン | 検出したパスとバージョン、パスの手動指定、再検出（下の「環境」） |
| 環境 | Claude Code | 検出したパスとバージョン、パスの手動指定、再検出（下の「環境」） |
| 環境 | 外部クレートの準備 | 初回準備の状態（準備できたときは、その日時）と、失敗時の再試行（[許可クレートと作業領域](../exercise-runner/crates-and-workspace.md)「初回準備」） |
| 学習データ | 学習記録をエディタで開く | `records/` ディレクトリを OS の既定アプリで開く |
| 学習データ | 分析ファイルをエディタで開く | `analysis/analysis.md` を OS の既定アプリで開く |
| 学習データ | 分析を再構築 | rebuild ジョブを登録する（[学習記録・分析の AI ジョブ](../learning-record-and-analysis/ai-jobs.md)「分析の再構築（rebuild）」） |
| 学習データ | 分析のみリセット | `analysis.md` をテンプレートに戻す（記録は保持） |
| 学習データ | 学習履歴をすべてリセット | `attempts/`・`records/`・`analysis/`・`jobs.json`・`progress.json`・`drafts/` を削除する。生成問題は残し、`meta.json` の完了に関する項目を初期値に戻す |
| 学習データ | 失敗したジョブ | `failed` のジョブの一覧（種類・対象・エラー内容）と再実行ボタン |
| 学習データ | 読み込めない学習記録 | frontmatter を解析できず集計から除外された記録の一覧（[学習記録・分析ファイルの手動編集への対応](../learning-record-and-analysis/manual-edit.md)） |
| アプリ | バージョン情報 | 現在のバージョン |

## 環境

- 「Rust ツールチェーン」「Claude Code」には、検出したパスとバージョン（[環境のコマンドとイベント](../architecture/commands/environment.md)「環境の状態」）と、そのパスが自動で検出したものか、設定で指定したものかを表示する。見つからないときは、見つからないことと導入手順へのリンク（[外部ツール（cargo・claude）への依存](../architecture/external-tools.md)「見つからない場合」）を表示する。
- パスの手動指定は、文字の入力欄で行う。学習者は、実行ファイルの絶対パス（例: ターミナルの `which cargo` の出力）を入力して保存する。入力欄を空にして保存すると、`tool_paths` のその項目を `null` にし、自動検出に戻る。
- パスを保存したら、`update_settings` で `tool_paths` を保存してから、`redetect_tools` を呼んで検出し直す。
- 指定したパスが使えず（[外部ツール（cargo・claude）への依存](../architecture/external-tools.md)「検出」で飛ばす場所に当たる）、ほかの場所で見つかったか、見つからなかったときは、指定したパスが使えなかったことを表示する。
- 「再検出」は `redetect_tools` を呼ぶ。

## リセット操作

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
- `models.chat`・`models.background` は、[AI モデルの選択](../claude-integration/model-selection.md) のモデルのエイリアスかモデル ID とする。空でなく、空白と制御文字を含まず、`-` で始まらない文字列とする。
- `tool_paths` が `null` の場合は自動検出する。
- `settings.json` が無いときは、次の初期値の設定として扱う。`theme` は `system`、`editor_font_size` は `14`、`video` は `{ "volume": 1.0, "muted": false, "playback_rate": 1.0, "captions": false }`、`models` は [AI モデルの選択](../claude-integration/model-selection.md) の既定値、`tool_paths` はどちらも `null`、`first_launch_completed` は `false` とする。
