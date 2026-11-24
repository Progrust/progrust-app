//! ビルド・実行用の作業領域（spec/exercise-runner/crates-and-workspace.md「作業領域」）のパス。
//!
//! ルートのディレクトリは [`WorkspaceDir::new`] に渡す。アプリでは [`WorkspaceDir::in_app_data`] で
//! `{app_data_dir}/workspace/` にする。教材チェックツールの判定は、別のルートで使う。

use std::path::{Path, PathBuf};

use crate::storage::AppDataDir;

/// `{app_data_dir}` の下の、作業領域のディレクトリの名前。
pub const WORKSPACE_DIR: &str = "workspace";

/// 作業領域のディレクトリ。作るときにはディレクトリを作らない。
#[derive(Debug, Clone)]
pub struct WorkspaceDir {
    root: PathBuf,
}

impl WorkspaceDir {
    /// `root` を作業領域のルートとして使う。
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    /// 学習者データのディレクトリの下の `workspace/` を使う。
    pub fn in_app_data(data_dir: &AppDataDir) -> Self {
        Self::new(data_dir.root().join(WORKSPACE_DIR))
    }

    /// 作業領域のルートのパス。
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// 初回準備の状態の記録（`prepared.json`。[`super::prepared`]）。
    pub fn prepared_json(&self) -> PathBuf {
        self.root.join("prepared.json")
    }

    /// 実行・提出のたびに組み立てるプロジェクト（`project/`）。
    pub fn project(&self) -> PathBuf {
        self.root.join("project")
    }

    /// `project/` の、許可クレートを使う演習のビルド先（`target/`。初回準備と共有）。
    pub fn target(&self) -> PathBuf {
        self.root.join("target")
    }

    /// `project/` の、許可クレートを使わない演習のビルド先（`target-std/`）。
    pub fn target_std(&self) -> PathBuf {
        self.root.join("target-std")
    }

    /// `project/` のビルド先。許可クレートを使う演習は `target/`、使わない演習は `target-std/`。
    pub fn project_target(&self, uses_allowed_crates: bool) -> PathBuf {
        if uses_allowed_crates {
            self.target()
        } else {
            self.target_std()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paths_are_under_the_given_root() {
        let workspace = WorkspaceDir::new("/tmp/workspace");
        assert_eq!(workspace.root(), Path::new("/tmp/workspace"));
        assert_eq!(
            workspace.prepared_json(),
            Path::new("/tmp/workspace/prepared.json")
        );
        assert_eq!(workspace.project(), Path::new("/tmp/workspace/project"));
        assert_eq!(workspace.target(), Path::new("/tmp/workspace/target"));
        assert_eq!(
            workspace.target_std(),
            Path::new("/tmp/workspace/target-std")
        );
    }

    #[test]
    fn app_workspace_is_under_the_app_data_dir() {
        let workspace = WorkspaceDir::in_app_data(&AppDataDir::new("/data"));
        assert_eq!(workspace.root(), Path::new("/data/workspace"));
    }

    #[test]
    fn project_target_depends_on_allowed_crates() {
        let workspace = WorkspaceDir::new("/w");
        assert_eq!(workspace.project_target(true), Path::new("/w/target"));
        assert_eq!(workspace.project_target(false), Path::new("/w/target-std"));
    }
}
