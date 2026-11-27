//! テスト用の偽の `claude`（spec/non-functional/testing.md「偽の claude と偽の cargo」）の、
//! `--version` の出力を決める手助け。
//!
//! `claude.rs` を `claude` の名前で読み込んだモジュールの隣に読み込む（`claude.rs` の先頭の説明）。

use super::claude::FakeClaude;

impl FakeClaude {
    /// `--version` で標準出力に書く内容。
    pub fn set_version_stdout(&self, stdout: &str) {
        self.write_beside("version-stdout", stdout);
    }

    /// `--version` の終了コード。
    pub fn set_version_exit_code(&self, code: i32) {
        self.write_beside("version-exit-code", code.to_string());
    }

    /// `--version` で、何も書かずに、終了せずに止まる（応答しない）。
    pub fn set_version_hang(&self) {
        self.write_beside("version-hang", "");
    }
}
