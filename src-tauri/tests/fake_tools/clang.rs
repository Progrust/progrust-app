//! テスト用の偽の `clang`（spec/non-functional/testing.md「偽の claude と偽の cargo」）を置き、`--version` の
//! 出力を決め、受け取ったもの（引数・環境変数）を読むための手助け。
//!
//! テストのクレートは、`mod fake_tools;` の下で使うか、このファイルを直接読み込む
//! （`#[path = "fake_tools/clang.rs"] mod clang;`）。このファイルは、ほかのファイルに頼らずに書く。

use std::{
    collections::BTreeMap,
    fs,
    os::unix::fs::symlink,
    path::{Path, PathBuf},
};

use tempfile::TempDir;

/// 一時ディレクトリに置いた偽の `clang`。
pub struct FakeClang {
    _dir: Option<TempDir>,
    path: PathBuf,
}

impl FakeClang {
    /// 新しい一時ディレクトリに置いた、何も用意していない偽の `clang`。`--version` は、
    /// `Apple clang version 21.0.0 (clang-2100.4.1.1)` から始まる出力で成功する。
    pub fn new() -> Self {
        let dir = TempDir::new().expect("一時ディレクトリを作れない");
        let mut fake = Self::place(dir.path());
        fake._dir = Some(dir);
        fake
    }

    /// ディレクトリ `dir`（無ければ作る）に置いた、何も用意していない偽の `clang`。
    /// `dir` は呼び出し側が消す。
    pub fn place(dir: &Path) -> Self {
        fs::create_dir_all(dir).unwrap_or_else(|e| panic!("{} を作れない: {e}", dir.display()));
        // 原本を指すシンボリックリンクにする（写すと、Linux で起動に失敗することがある。mod.rs）。
        let original = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fake_tools/clang");
        let path = dir.join("clang");
        symlink(original, &path).unwrap_or_else(|e| panic!("{} を置けない: {e}", path.display()));
        Self { _dir: None, path }
    }

    /// 実行ファイルのパス。
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// 実行ファイルと同じディレクトリに、出力を決めるファイル `name` を書く。
    fn write_beside(&self, name: &str, content: impl AsRef<[u8]>) {
        let path = self.path.with_file_name(name);
        fs::write(&path, content).unwrap_or_else(|e| panic!("{} に書けない: {e}", path.display()));
    }

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

    /// これまでの呼び出しで受け取ったもの。呼び出した順に並ぶ。
    pub fn calls(&self) -> Vec<ClangCall> {
        let calls_dir = self.path.with_file_name("calls");
        let mut calls = Vec::new();
        for number in 1.. {
            let dir = calls_dir.join(number.to_string());
            if !dir.is_dir() {
                break;
            }
            calls.push(ClangCall::read(&dir));
        }
        calls
    }
}

/// 偽の `clang` が 1 回の呼び出しで受け取ったもの。
#[derive(Debug)]
pub struct ClangCall {
    pub args: Vec<String>,
    /// 環境変数。値に改行を含む変数は、正しく読めない（2 行目からは別の行として読む）。
    pub env: BTreeMap<String, String>,
}

impl ClangCall {
    fn read(dir: &Path) -> Self {
        let read = |name: &str| -> String {
            let path = dir.join(name);
            fs::read_to_string(&path)
                .unwrap_or_else(|e| panic!("{} を読めない: {e}", path.display()))
        };
        let args = read("args").lines().map(String::from).collect();
        let env = read("env")
            .lines()
            .filter_map(|line| line.split_once('='))
            .map(|(name, value)| (name.to_string(), value.to_string()))
            .collect();
        Self { args, env }
    }
}
