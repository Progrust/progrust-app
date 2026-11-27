//! テスト用の偽の `claude`（spec/non-functional/testing.md「偽の claude と偽の cargo」）を置くための
//! 手助けのうち、置くことと、受け取ったもの（引数・環境変数・標準入力）を読むこと。
//!
//! `--version` の出力を決めるものは `claude_version.rs` に、`-p` の呼び出しの出力を決めるものと、
//! `-p` の呼び出しで書き残したプロセスと作業ディレクトリの記録を読むものは `claude_print.rs` にある。
//! テストのクレートは、このファイルを `claude` の名前で読み込み、その隣に、使うほうのファイルを
//! 読み込む（例: `#[path = "fake_tools/claude.rs"] mod claude;` と
//! `#[path = "fake_tools/claude_version.rs"] mod claude_version;`）。使わない手助けを読み込むと、
//! 未使用の警告になるので、ファイルを分けている。このファイルは、ほかのファイルに頼らずに書く。

use std::{
    collections::BTreeMap,
    fs,
    os::unix::fs::symlink,
    path::{Path, PathBuf},
};

use tempfile::TempDir;

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

    /// 実行ファイルと同じディレクトリに、出力を決めるファイル `name` を書く。
    pub fn write_beside(&self, name: &str, content: impl AsRef<[u8]>) {
        let path = self.path.with_file_name(name);
        fs::write(&path, content).unwrap_or_else(|e| panic!("{} に書けない: {e}", path.display()));
    }

    /// `number` 回目（1 から数える）の呼び出しの記録を置くディレクトリ。
    pub fn call_dir(&self, number: usize) -> PathBuf {
        self.path.with_file_name("calls").join(number.to_string())
    }

    /// これまでの呼び出しで受け取ったもの。呼び出した順に並ぶ。
    pub fn calls(&self) -> Vec<RecordedCall> {
        let mut calls = Vec::new();
        for number in 1.. {
            let dir = self.call_dir(number);
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
