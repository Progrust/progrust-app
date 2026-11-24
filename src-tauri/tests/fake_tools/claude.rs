//! テスト用の偽の `claude`（spec/non-functional/testing.md「偽の claude と偽の cargo」）を置くための
//! 手助けのうち、置くことと、`--version` の出力を決めることと、受け取ったものを読むこと。
//!
//! `-p` の呼び出しの出力を決めるものは `mod.rs` にある。`claude` の検出のテストのように、`-p` を
//! 使わないテストからも、このファイルだけを `#[path = "fake_tools/claude.rs"] mod fake_claude;` で
//! 読み込めるよう、ほかのファイルに頼らずに書く。

use std::{
    collections::BTreeMap,
    fs,
    os::unix::fs::symlink,
    path::{Path, PathBuf},
};

use tempfile::TempDir;

fn write(path: &Path, content: impl AsRef<[u8]>) {
    fs::write(path, content).unwrap_or_else(|e| panic!("{} に書けない: {e}", path.display()));
}

/// 一時ディレクトリに置いた偽の `claude`。
pub struct FakeClaude {
    _dir: Option<TempDir>,
    path: PathBuf,
}

impl FakeClaude {
    /// 新しい一時ディレクトリに置いた、何も用意していない偽の `claude`。`--version` は成功し、
    /// `-p` の呼び出しは、何も出力せずに終了コード 0 で終わる。
    pub fn new() -> Self {
        let dir = TempDir::new().expect("一時ディレクトリを作れない");
        let mut fake = Self::place(dir.path());
        fake._dir = Some(dir);
        fake
    }

    /// ディレクトリ `dir`（無ければ作る）に置いた、何も用意していない偽の `claude`。
    /// `dir` は呼び出し側が消す。
    pub fn place(dir: &Path) -> Self {
        fs::create_dir_all(dir).unwrap_or_else(|e| panic!("{} を作れない: {e}", dir.display()));
        // 原本を指すシンボリックリンクにする（写すと、Linux で起動に失敗することがある。mod.rs）。
        let original = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fake_tools/claude");
        let path = dir.join("claude");
        symlink(original, &path).unwrap_or_else(|e| panic!("{} を置けない: {e}", path.display()));
        Self { _dir: None, path }
    }

    /// 実行ファイルのパス。
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// `--version` で標準出力に書く内容。
    pub fn set_version_stdout(&self, stdout: &str) {
        write(&self.path.with_file_name("version-stdout"), stdout);
    }

    /// `--version` の終了コード。
    pub fn set_version_exit_code(&self, code: i32) {
        write(
            &self.path.with_file_name("version-exit-code"),
            code.to_string(),
        );
    }

    /// `--version` で、何も書かずに、終了せずに止まる（応答しない）。
    pub fn set_version_hang(&self) {
        write(&self.path.with_file_name("version-hang"), "");
    }

    /// これまでの呼び出しで受け取ったもの。呼び出した順に並ぶ。
    pub fn calls(&self) -> Vec<RecordedCall> {
        let calls_dir = self.path.with_file_name("calls");
        let mut calls = Vec::new();
        for number in 1.. {
            let dir = calls_dir.join(number.to_string());
            if !dir.is_dir() {
                break;
            }
            calls.push(RecordedCall::read(&dir));
        }
        calls
    }
}

/// 偽の `claude` が 1 回の呼び出しで受け取ったもの。
#[derive(Debug)]
pub struct RecordedCall {
    pub args: Vec<String>,
    pub env: BTreeMap<String, String>,
    /// 標準入力。`--version` では読まないので `None`。`-p` でも、読み終える前は `None`。
    pub stdin: Option<String>,
}

impl RecordedCall {
    fn read(dir: &Path) -> Self {
        let read = |name: &str| -> Option<String> {
            let path = dir.join(name);
            let bytes = fs::read(&path).ok()?;
            Some(String::from_utf8(bytes).unwrap_or_else(|e| {
                panic!("{} が UTF-8 でない: {e}", path.display());
            }))
        };
        let split = |text: String| -> Vec<String> {
            let mut items: Vec<String> = text.split('\0').map(String::from).collect();
            assert_eq!(items.pop().as_deref(), Some(""), "記録が NUL で終わらない");
            items
        };
        let args = split(read("args").expect("引数の記録が無い"));
        let env = split(read("env").expect("環境変数の記録が無い"))
            .into_iter()
            .map(|item| {
                let (name, value) = item.split_once('=').expect("環境変数の記録の形が違う");
                (name.to_string(), value.to_string())
            })
            .collect();
        Self {
            args,
            env,
            stdin: read("stdin"),
        }
    }
}
