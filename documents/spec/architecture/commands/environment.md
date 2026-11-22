---
type: Specification
title: "環境のコマンドとイベント"
description: "`cargo`・`claude` の検出結果と初回準備の状態を表す環境の状態の形と、`get_environment_status`・`redetect_tools`・`retry_crate_preparation` の引数と戻り値、`environment_updated` の内容を定める。"
---

動作は [外部ツール（cargo・claude）への依存](../external-tools.md) と [許可クレートと作業領域](../../exercise-runner/crates-and-workspace.md)「初回準備」に従う。共通の決まりは [コマンドとイベントの共通の決まり](conventions.md) に従う。

## 環境の状態

`get_environment_status`・`redetect_tools` の戻り値と、`environment_updated` の内容は、次の形の「環境の状態」とする。

```json
{
  "cargo": {
    "status": "found",
    "cargo_path": "/Users/me/.cargo/bin/cargo",
    "rustc_path": "/Users/me/.cargo/bin/rustc",
    "rustc_version": "1.100.0",
    "required_rustc_version": "1.100.0"
  },
  "claude": {
    "status": "found",
    "path": "/Users/me/.local/bin/claude",
    "version": "2.1.329"
  },
  "crates": { "status": "ready", "error": null }
}
```

**`cargo`**

| 項目 | 型 | 内容 |
|---|---|---|
| `status` | `"found" \| "not_found" \| "version_too_old"` | `found` は必要条件を満たす `cargo`・`rustc` が見つかった。`version_too_old` は見つかったが `rustc` のバージョンが必要条件に満たない（[外部ツール（cargo・claude）への依存](../external-tools.md)「バージョンが足りない場合」）。`not_found` は見つからない |
| `cargo_path` / `rustc_path` | `string \| null` | 見つかった `cargo`・`rustc` の絶対パス。見つからないときは `null` |
| `rustc_version` | `string \| null` | `rustc --version` から取り出したバージョン（例: `1.100.0`）。見つからないときは `null` |
| `required_rustc_version` | `string` | 必要条件のバージョン（[外部ツール（cargo・claude）への依存](../external-tools.md)「検出」の表） |

**`claude`**

| 項目 | 型 | 内容 |
|---|---|---|
| `status` | `"found" \| "not_found"` | `claude --version` が成功したら `found` |
| `path` | `string \| null` | 見つかった `claude` の絶対パス。見つからないときは `null` |
| `version` | `string \| null` | `claude --version` から取り出したバージョン。見つからないときは `null` |

**`crates`**（初回準備）

| 項目 | 型 | 内容 |
|---|---|---|
| `status` | `"not_started" \| "preparing" \| "ready" \| "failed"` | 初回準備の状態（[許可クレートと作業領域](../../exercise-runner/crates-and-workspace.md)「初回準備の状態」） |
| `error` | `string \| null` | `failed` のときのエラーの内容（`prepared.json` に書いたもの）。ほかの状態では `null` |

## コマンド

**`get_environment_status`**

- 引数: なし。
- 戻り値: 環境の状態。
- アプリ起動時の検出が終わっていないときは、終わるのを待ってから返す。

**`redetect_tools`**

設定画面の「再検出」で呼ぶ。

- 引数: なし。
- 戻り値: 検出し直した後の環境の状態。
- 検出の方法は [外部ツール（cargo・claude）への依存](../external-tools.md)「検出」に従い、設定の `tool_paths`（[設定画面と settings.json](../../screens/settings-screen.md)「settings.json」）を使う。検出し直したら、`environment_updated` も送る。

**`retry_crate_preparation`**

初回準備に失敗したときの再試行のボタン（[許可クレートと作業領域](../../exercise-runner/crates-and-workspace.md)「準備中・失敗時の動作」、[設定画面と settings.json](../../screens/settings-screen.md)「設定項目」のクレートの準備）で呼ぶ。

- 引数: なし。
- 戻り値: `null`。初回準備を始めたら返す。経過と結果は `environment_updated` で知らせる。
- エラー: 初回準備の状態が `failed` でないときは `invalid_state`。`cargo` が見つからないときは `cargo_unavailable`。

## イベント

**`environment_updated`**

- 内容: 変わった後の環境の状態（全体）。
- ツールの検出結果か、初回準備の状態が変わるたびに送る。
