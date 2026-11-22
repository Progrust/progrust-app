---
type: Specification
title: "設定のコマンド"
description: "設定のコマンド `get_settings`・`update_settings`（`settings.json` の全体）、`open_in_os`（開く対象 `records`・`analysis`・`logs`）、`test_claude_connection`（接続テストの結果）、`get_app_version` の引数と戻り値を定める。"
---

設定の項目と動作は [設定画面と settings.json](../../screens/settings-screen.md) と [Claude Code 連携](../../claude-integration/index.md) に従う。共通の決まりは [コマンドとイベントの共通の決まり](conventions.md) に従う。

## get_settings

- 引数: なし。
- 戻り値: 設定（[設定画面と settings.json](../../screens/settings-screen.md)「settings.json」の形）。

## update_settings

設定画面で設定を変えたとき、動画画面で音量・ミュート・再生速度・字幕を変えたとき（[動画画面](../../screens/video-screen.md)）、初回起動画面で「始める」を押したとき（`first_launch_completed`。[初回起動画面](../../screens/first-launch.md)）に呼ぶ。

- 引数:

| 項目 | 型 | 内容 |
|---|---|---|
| `settings` | [設定画面と settings.json](../../screens/settings-screen.md)「settings.json」の形 | 変えた後の設定の全体。`schema_version` はバックエンドが付けるので、渡した値は使わない |

- 戻り値: 保存した後の設定（`get_settings` と同じ形）。
- `tool_paths` を変えても、ツールの検出はし直さない。検出し直すときは `redetect_tools`（[環境のコマンドとイベント](environment.md)）を呼ぶ。
- エラー: 値が [設定画面と settings.json](../../screens/settings-screen.md) の条件（`theme` の値、`editor_font_size` の範囲、`video.volume`・`video.playback_rate` の値など）を満たさないときは `invalid_argument`。

## open_in_os

ファイルやディレクトリを OS の既定アプリで開く。開けるのは、次の対象だけとする。任意のパスは受け付けない。

- 引数:

| 項目 | 型 | 内容 |
|---|---|---|
| `target` | `"records" \| "analysis" \| "logs"` | 開く対象 |

| `target` | 開くもの | 使う所 |
|---|---|---|
| `records` | `records/` ディレクトリ | 設定画面の「学習記録をエディタで開く」 |
| `analysis` | `analysis/analysis.md` | 設定画面の「分析ファイルをエディタで開く」 |
| `logs` | `logs/` ディレクトリ | ログのフォルダを開く（[アプリのログ](../../non-functional/logging.md)） |

- パスは [データの保存場所](../data-locations.md)「学習者データ（`{app_data_dir}`）」による。
- 戻り値: `null`。
- エラー: 開く対象が無いときは `not_found`。OS が開けなかったときは `io_error`。
- 外部の URL（導入手順など）は、このコマンドではなく、フロントエンドから OS の既定ブラウザで開く（[Tauri の権限](../tauri-permissions.md)）。

## test_claude_connection

設定画面の「接続テスト」で呼ぶ（[claude が使えない場合の動作](../../claude-integration/unavailable.md)）。

- 引数: なし。
- 戻り値:

| 項目 | 型 | 内容 |
|---|---|---|
| `ok` | `boolean` | 応答が返ったら `true` |
| `model` | `string` | 使ったモデル（質疑応答のモデル。[AI モデルの選択](../../claude-integration/model-selection.md)） |
| `error` | [コマンドとイベントの共通の決まり](conventions.md)「AI 呼び出しのエラー」の形 `\| null` | `ok` が `false` のときのエラー。`true` のときは `null` |

- 呼び出しが終わってから返す。
- エラー: `claude` が見つからないときは `claude_unavailable`。

## get_app_version

設定画面の「バージョン情報」で呼ぶ。フロントエンドには Tauri のアプリ情報の API を許可しないので（[Tauri の権限](../tauri-permissions.md)）、バックエンドから受け取る。

- 引数: なし。
- 戻り値: `{ version: string }`。アプリのバージョン（例: `0.1.0`）。
