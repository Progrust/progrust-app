//! テスト用の偽の claude・cargo・rustc・clang（spec/non-functional/testing.md「偽の claude と偽の cargo」）を、
//! テストごとの一時ディレクトリに置いて使うための手助け。
//!
//! 偽のツールの原本（このディレクトリの `claude`・`cargo`・`rustc`・`clang`）を指すシンボリックリンクを
//! 一時ディレクトリに置き、返す出力をその隣のファイルに書く。実行ファイルを書き出してすぐに
//! 起動すると、テストを並べて実行したときに Linux で起動に失敗する（Text file busy）ことが
//! あるので、写さずにリンクにする。

use std::path::{Path, PathBuf};

pub mod cargo_behavior;
pub mod clang;
pub mod claude;
pub mod claude_print;
pub mod claude_version;
pub mod rust;
pub use clang::FakeClang;
pub use claude::FakeClaude;
pub use claude_print::stream_json;
pub use rust::FakeRust;

/// 偽のツールの原本を置くディレクトリ。
pub fn originals_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fake_tools")
}
