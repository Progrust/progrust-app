//! テスト用の偽の `cargo`・`rustc`（spec/non-functional/testing.md「偽の claude と偽の cargo」）を
//! 置くための手助け。
//!
//! 偽の `claude` の手助け（`mod.rs`）を使わないテストからも、このファイルだけを
//! `#[path = "fake_tools/rust.rs"] mod fake_rust;` で読み込めるよう、ほかのファイルに頼らずに書く。

use std::{
    fs,
    os::unix::fs::symlink,
    path::{Path, PathBuf},
};

use tempfile::TempDir;

/// 並べて置いた偽の `cargo` と `rustc`。
pub struct FakeRust {
    _dir: Option<TempDir>,
    cargo: PathBuf,
    rustc: PathBuf,
}

impl FakeRust {
    /// 新しい一時ディレクトリに置いた、`--version` で `version`（例: `1.99.0`）を返す偽の
    /// `cargo` と `rustc`。
    pub fn new(version: &str) -> Self {
        let dir = TempDir::new().expect("一時ディレクトリを作れない");
        let mut fake = Self::place(dir.path(), version);
        fake._dir = Some(dir);
        fake
    }

    /// ディレクトリ `dir`（無ければ作る）に置いた、`--version` で `version` を返す偽の `cargo` と
    /// `rustc`。`dir` は呼び出し側が消す。
    pub fn place(dir: &Path, version: &str) -> Self {
        fs::create_dir_all(dir).unwrap_or_else(|e| panic!("{} を作れない: {e}", dir.display()));
        // 原本を指すシンボリックリンクにする（写すと、Linux で起動に失敗することがある。mod.rs）。
        let originals = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fake_tools");
        let link = |name: &str| {
            let path = dir.join(name);
            symlink(originals.join(name), &path)
                .unwrap_or_else(|e| panic!("{} を置けない: {e}", path.display()));
            path
        };
        let cargo = link("cargo");
        let rustc = link("rustc");
        let version_file = dir.join("rust-version");
        fs::write(&version_file, format!("{version}\n"))
            .unwrap_or_else(|e| panic!("{} に書けない: {e}", version_file.display()));
        Self {
            _dir: None,
            cargo,
            rustc,
        }
    }

    pub fn cargo(&self) -> &Path {
        &self.cargo
    }

    pub fn rustc(&self) -> &Path {
        &self.rustc
    }
}
