//! コード実行・判定（spec/exercise-runner/）。
//!
//! - アプリ同梱の許可クレートの定義（`{app_resources}/runner/`。spec/exercise-runner/crates-and-workspace.md）は
//!   [`RunnerDir`]。ディレクトリは [`RunnerDir::new`] に渡す。アプリでは [`RunnerDir::from_app`] で
//!   Tauri の `resource_dir()` の下の `runner/` にし、テストでは一時ディレクトリを渡す。
//! - エディタから受け取るファイルパスの検証は [`file_path`]、作業領域のパスは [`workspace`]、
//!   実行・提出の `project/` の組み立てとビルドの条件は [`project`]。
//! - 学習者のプログラムなどを新しいプロセスグループで動かし、出力・標準入力・停止・制限を扱う部品は [`process`]。
//! - `project/` を cargo でビルドし、ビルドの出力・エラーコード・実行ファイルを得る部品は [`build`]。
//! - ビルド済みの実行ファイルを `stdout` 方式で判定する部品は [`judge`]、`test` 方式の隠しテストを組み込んで
//!   判定し、`min_learner_tests` のある演習では学習者が書いたテストも判定する部品は [`judge_test`]。
//! - 実行・提出を同時に 1 つだけ行う状態（止める操作と標準入力を含む）は [`runs`]、実行・テスト実行のタスクと
//!   イベント `run_output`・`run_finished` の内容は [`run`]、提出のタスクとイベント `submit_progress` と
//!   `run_finished` の `submit` の内容は [`submit`]。

pub mod build;
pub mod file_path;
pub mod judge;
pub mod judge_test;
pub mod process;
pub mod project;
pub mod run;
pub mod runs;
pub mod submit;
pub mod workspace;

use std::path::{Path, PathBuf};

use tauri::{AppHandle, Manager, Runtime};

/// 許可クレートの一覧のファイル名。
pub const ALLOWED_CRATES_FILE: &str = "allowed-crates.toml";
/// 同梱の `Cargo.lock` のファイル名。
pub const CARGO_LOCK_FILE: &str = "Cargo.lock";

/// 同梱の `runner/` のディレクトリ。
#[derive(Debug, Clone)]
pub struct RunnerDir {
    root: PathBuf,
}

impl RunnerDir {
    /// `root` を `runner/` のディレクトリとして使う。
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    /// Tauri の `resource_dir()` の下の `runner/` を使う。
    pub fn from_app<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<Self> {
        Ok(Self::new(app.path().resource_dir()?.join("runner")))
    }

    /// `runner/` のディレクトリのパス。
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// `allowed-crates.toml` のパス。
    pub fn allowed_crates(&self) -> PathBuf {
        self.root.join(ALLOWED_CRATES_FILE)
    }

    /// `Cargo.lock` のパス。
    pub fn cargo_lock(&self) -> PathBuf {
        self.root.join(CARGO_LOCK_FILE)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paths_are_under_the_given_directory() {
        let dir = RunnerDir::new("/tmp/runner");
        assert_eq!(dir.root(), Path::new("/tmp/runner"));
        assert_eq!(
            dir.allowed_crates(),
            Path::new("/tmp/runner/allowed-crates.toml")
        );
        assert_eq!(dir.cargo_lock(), Path::new("/tmp/runner/Cargo.lock"));
    }
}
