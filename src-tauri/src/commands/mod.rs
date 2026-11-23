//! フロントエンドが `invoke` で呼ぶコマンド（spec/architecture/commands/）。
//!
//! コマンドは分類ごとのモジュールに置き、`lib.rs` の `app_builder` でハンドラに載せる。
//! どのコマンドにも `#[tauri::command(rename_all = "snake_case")]` を付け、`Result` を返す。

pub mod content;
pub mod environment;
pub mod error;
pub mod learning_record;
pub mod progress;
pub mod settings;
