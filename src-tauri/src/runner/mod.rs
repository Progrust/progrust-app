//! コード実行・判定（spec/exercise-runner/）。
//!
//! - アプリ同梱の許可クレートの定義（`{app_resources}/runner/`。spec/exercise-runner/crates-and-workspace.md）は
//!   [`RunnerDir`]。ディレクトリは [`RunnerDir::new`] に渡す。アプリでは [`RunnerDir::from_app`] で
//!   Tauri の `resource_dir()` の下の `runner/` にし、テストでは一時ディレクトリを渡す。
//! - エディタから受け取るファイルパスの検証は [`file_path`]、作業領域のパスは [`workspace`]、
//!   実行・提出の `project/` の組み立てとビルドの条件は [`project`]。

pub mod file_path;
pub mod project;
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
