//! C の演習の実行・提出のたびに組み立てる `c-project/`（spec/exercise-runner/c-project-assembly.md「作業領域」）。
//!
//! - [`assemble_c_project`] で、`c-project/` を空にしてから `main.c` を置く。受け取ったファイルパスは、先に
//!   [`validate_c_file_paths`] で確かめ、満たさないときは `c-project/` に手を付けない。
//! - 提出のメモリリークの検査のライブラリのソースは、[`write_leakcheck_source`] で `leakcheck/leakcheck.c` に置く。
//!   ソースそのものは呼び出し側が渡す。
//! - ビルドは [`super::c_build`] が行う。
//!
//! Tauri の実行時に頼らない（教材チェックツールの判定からも使う。ADR 0273）。

use std::collections::BTreeMap;
use std::fmt;
use std::io;
use std::path::{Path, PathBuf};

use super::file_path::{C_MAIN_FILE, InvalidCFilePaths, validate_c_file_paths};
use super::project::make_empty_dir;
use super::workspace::WorkspaceDir;
use crate::commands::error::{CommandError, ErrorCode};

/// メモリリークの検査のライブラリのソースとビルドしたライブラリを置くディレクトリ（`c-project/` からの相対パス）。
pub const LEAKCHECK_DIR: &str = "leakcheck";
/// メモリリークの検査のライブラリのソースのファイル名。
pub const LEAKCHECK_SOURCE_FILE: &str = "leakcheck.c";
/// ビルドしたメモリリークの検査のライブラリのファイル名。
pub const LEAKCHECK_LIBRARY_FILE: &str = "libprogrust_leakcheck.dylib";

/// `c-project/` を組み立てられなかった理由。
#[derive(Debug)]
pub enum CAssemblyError {
    /// エディタから受け取ったファイルパスが条件を満たさない。`invalid_argument` になる。
    InvalidPaths(InvalidCFilePaths),
    /// ファイルやディレクトリを書き込めなかった。`io_error` になる。
    Io { path: PathBuf, source: io::Error },
}

impl fmt::Display for CAssemblyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidPaths(error) => error.fmt(f),
            Self::Io { path, source } => write!(f, "{}: {source}", path.display()),
        }
    }
}

impl std::error::Error for CAssemblyError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::InvalidPaths(error) => Some(error),
            Self::Io { source, .. } => Some(source),
        }
    }
}

impl From<InvalidCFilePaths> for CAssemblyError {
    fn from(error: InvalidCFilePaths) -> Self {
        Self::InvalidPaths(error)
    }
}

impl From<CAssemblyError> for CommandError {
    fn from(error: CAssemblyError) -> Self {
        match error {
            CAssemblyError::InvalidPaths(invalid) => invalid.into(),
            CAssemblyError::Io { .. } => CommandError {
                code: ErrorCode::IoError,
                message: "実行するプロジェクトのファイルを書き込めませんでした。".into(),
                detail: Some(error.to_string()),
            },
        }
    }
}

/// `c-project/` の `main.c` のパス。
pub fn main_c(workspace: &WorkspaceDir) -> PathBuf {
    workspace.c_project().join(C_MAIN_FILE)
}

/// `c-project/` の、メモリリークの検査のライブラリのソースのパス。
pub fn leakcheck_source(workspace: &WorkspaceDir) -> PathBuf {
    workspace
        .c_project()
        .join(LEAKCHECK_DIR)
        .join(LEAKCHECK_SOURCE_FILE)
}

/// `c-project/` の、ビルドしたメモリリークの検査のライブラリのパス。
pub fn leakcheck_library(workspace: &WorkspaceDir) -> PathBuf {
    workspace
        .c_project()
        .join(LEAKCHECK_DIR)
        .join(LEAKCHECK_LIBRARY_FILE)
}

/// `c-project/` を空にしてから、エディタの `main.c` を書く。
///
/// `files` はエディタ上の全ファイル（キーはファイルパス）。ファイルパスを確かめてから `c-project/` に手を付け、
/// 確かめられなかったときは、`c-project/` を変えずに失敗する。書く先は、受け取ったパスではなく、いつも
/// `c-project/main.c` である。
pub fn assemble_c_project(
    workspace: &WorkspaceDir,
    files: &BTreeMap<String, String>,
) -> Result<(), CAssemblyError> {
    validate_c_file_paths(files.keys().map(String::as_str))?;
    let source = files
        .get(C_MAIN_FILE)
        .ok_or(InvalidCFilePaths::MissingMain)?;
    let dir = workspace.c_project();
    make_empty_dir(&dir).map_err(|source| CAssemblyError::Io {
        path: dir.clone(),
        source,
    })?;
    write(&main_c(workspace), source)
}

/// `c-project/leakcheck/leakcheck.c` に、メモリリークの検査のライブラリのソースを書く。`leakcheck/` が無いときは
/// 作る。書いたファイルのパスを返す。[`assemble_c_project`] の後に使う。
pub fn write_leakcheck_source(
    workspace: &WorkspaceDir,
    source: &str,
) -> Result<PathBuf, CAssemblyError> {
    let path = leakcheck_source(workspace);
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|source| CAssemblyError::Io {
            path: dir.to_path_buf(),
            source,
        })?;
    }
    write(&path, source)?;
    Ok(path)
}

fn write(path: &Path, content: &str) -> Result<(), CAssemblyError> {
    std::fs::write(path, content).map_err(|source| CAssemblyError::Io {
        path: path.to_path_buf(),
        source,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::fs;

    fn files(entries: &[(&str, &str)]) -> BTreeMap<String, String> {
        entries
            .iter()
            .map(|(path, content)| (path.to_string(), content.to_string()))
            .collect()
    }

    /// `dir` の下のファイルとディレクトリを、`dir` からの相対パスで返す。シンボリックリンクはたどらない。
    fn list_entries(dir: &Path) -> Vec<String> {
        fn walk(base: &Path, dir: &Path, out: &mut Vec<String>) {
            for entry in fs::read_dir(dir).unwrap() {
                let path = entry.unwrap().path();
                let relative = path.strip_prefix(base).unwrap();
                out.push(relative.to_string_lossy().into_owned());
                if fs::symlink_metadata(&path).unwrap().is_dir() {
                    walk(base, &path, out);
                }
            }
        }
        let mut out = Vec::new();
        walk(dir, dir, &mut out);
        out.sort();
        out
    }

    const MAIN: &str = "int main(void) { return 0; }\n";

    #[test]
    fn writes_only_main_c_into_c_project() {
        let temp = tempfile::tempdir().unwrap();
        let workspace = WorkspaceDir::new(temp.path().join("workspace"));
        assemble_c_project(&workspace, &files(&[("main.c", MAIN)])).unwrap();

        assert_eq!(list_entries(&workspace.c_project()), ["main.c"]);
        assert_eq!(fs::read_to_string(main_c(&workspace)).unwrap(), MAIN);
        // `c-project/` のほかには、作業領域に何も作らない。
        assert_eq!(
            list_entries(temp.path()),
            [
                "workspace",
                "workspace/c-project",
                "workspace/c-project/main.c"
            ]
        );
    }

    #[test]
    fn empties_c_project_before_writing_main_c() {
        let temp = tempfile::tempdir().unwrap();
        let workspace = WorkspaceDir::new(temp.path().join("workspace"));
        let dir = workspace.c_project();
        fs::create_dir_all(dir.join("main.dSYM/Contents")).unwrap();
        fs::create_dir_all(dir.join("reports")).unwrap();
        fs::write(dir.join("main"), "前の実行ファイル").unwrap();
        fs::write(dir.join("reports/case-1.123"), "前の報告").unwrap();
        // Rust の演習の `project/` には触らない。
        fs::create_dir_all(workspace.project().join("src")).unwrap();
        fs::write(workspace.project().join("src/main.rs"), "fn main() {}\n").unwrap();

        assemble_c_project(&workspace, &files(&[("main.c", MAIN)])).unwrap();

        assert_eq!(list_entries(&dir), ["main.c"]);
        assert_eq!(
            fs::read_to_string(workspace.project().join("src/main.rs")).unwrap(),
            "fn main() {}\n"
        );
    }

    #[test]
    fn paths_other_than_main_c_write_nothing_outside_and_leave_c_project_alone() {
        let temp = tempfile::tempdir().unwrap();
        let workspace = WorkspaceDir::new(temp.path().join("data/workspace"));
        let dir = workspace.c_project();
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("main.c"), "前の main.c").unwrap();
        let outside = temp.path().join("outside.c");
        let before = list_entries(temp.path());

        for path in [
            "../outside.c".to_string(),
            "../../../outside.c".to_string(),
            outside.to_string_lossy().into_owned(),
            "/tmp/progrust-c-project-test.c".to_string(),
            "sub/main.c".to_string(),
            "main.h".to_string(),
        ] {
            let error = assemble_c_project(
                &workspace,
                &files(&[("main.c", MAIN), (&path, "// 外に書く")]),
            )
            .unwrap_err();
            assert!(
                matches!(
                    error,
                    CAssemblyError::InvalidPaths(InvalidCFilePaths::Unexpected { .. })
                ),
                "{path:?}: {error:?}"
            );
            assert_eq!(
                CommandError::from(error).code,
                ErrorCode::InvalidArgument,
                "{path:?}"
            );
            assert_eq!(list_entries(temp.path()), before, "{path:?}");
            assert_eq!(
                fs::read_to_string(dir.join("main.c")).unwrap(),
                "前の main.c"
            );
        }
        assert!(!Path::new("/tmp/progrust-c-project-test.c").exists());
    }

    #[test]
    fn missing_main_c_is_rejected_and_leaves_c_project_alone() {
        let temp = tempfile::tempdir().unwrap();
        let workspace = WorkspaceDir::new(temp.path().join("workspace"));
        let dir = workspace.c_project();
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("main"), "前の実行ファイル").unwrap();

        for editor in [files(&[]), files(&[("src/main.rs", "fn main() {}\n")])] {
            let error = assemble_c_project(&workspace, &editor).unwrap_err();
            assert!(
                matches!(error, CAssemblyError::InvalidPaths(_)),
                "{error:?}"
            );
            assert_eq!(list_entries(&dir), ["main"]);
        }
    }

    #[test]
    fn a_symlinked_c_project_is_replaced_without_touching_its_target() {
        let temp = tempfile::tempdir().unwrap();
        let workspace = WorkspaceDir::new(temp.path().join("workspace"));
        let target = temp.path().join("elsewhere");
        fs::create_dir_all(&target).unwrap();
        fs::write(target.join("keep.txt"), "残す").unwrap();
        fs::create_dir_all(workspace.root()).unwrap();
        std::os::unix::fs::symlink(&target, workspace.c_project()).unwrap();

        assemble_c_project(&workspace, &files(&[("main.c", MAIN)])).unwrap();

        assert!(
            !fs::symlink_metadata(workspace.c_project())
                .unwrap()
                .is_symlink()
        );
        assert_eq!(list_entries(&workspace.c_project()), ["main.c"]);
        assert_eq!(list_entries(&target), ["keep.txt"]);
    }

    #[test]
    fn the_leak_check_source_is_written_under_leakcheck() {
        let temp = tempfile::tempdir().unwrap();
        let workspace = WorkspaceDir::new(temp.path().join("workspace"));
        assemble_c_project(&workspace, &files(&[("main.c", MAIN)])).unwrap();

        let path = write_leakcheck_source(&workspace, "/* 数える */\n").unwrap();

        assert_eq!(path, workspace.c_project().join("leakcheck/leakcheck.c"));
        assert_eq!(fs::read_to_string(&path).unwrap(), "/* 数える */\n");
        assert_eq!(
            leakcheck_library(&workspace),
            workspace
                .c_project()
                .join("leakcheck/libprogrust_leakcheck.dylib")
        );
        assert_eq!(
            list_entries(&workspace.c_project()),
            ["leakcheck", "leakcheck/leakcheck.c", "main.c"]
        );
    }

    #[test]
    fn io_errors_are_io_error() {
        let temp = tempfile::tempdir().unwrap();
        // 作業領域のルートがファイルなので、`c-project/` を作れない。
        let root = temp.path().join("workspace");
        fs::write(&root, "ファイル").unwrap();
        let workspace = WorkspaceDir::new(&root);
        let error = assemble_c_project(&workspace, &files(&[("main.c", MAIN)])).unwrap_err();
        assert!(matches!(error, CAssemblyError::Io { .. }), "{error:?}");
        assert_eq!(CommandError::from(error).code, ErrorCode::IoError);
    }
}
