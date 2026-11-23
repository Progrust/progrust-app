//! 実行・提出のたびに組み立てる `project/` と、そのビルドの条件
//! （spec/exercise-runner/project-assembly.md）。

use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsString;
use std::fmt;
use std::io;
use std::path::{Path, PathBuf};

use super::RunnerDir;
use super::file_path::{InvalidFilePath, validate_file_path};
use super::workspace::WorkspaceDir;
use crate::commands::error::{CommandError, ErrorCode};

/// 演習のプロジェクトのパッケージ名。
pub const PACKAGE_NAME: &str = "progrust_exercise";
/// 全許可クレートに依存する空のライブラリ（`allcrates/`）のパッケージ名。
pub const ALLOWED_CRATES_PACKAGE: &str = "progrust_allowed_crates";

/// `project/` を組み立てられなかった理由。
#[derive(Debug)]
pub enum AssemblyError {
    /// エディタから受け取ったファイルパスが条件を満たさない。`invalid_argument` になる。
    InvalidPath(InvalidFilePath),
    /// 演習の `dependencies` に、許可クレートの一覧に無いクレートがある。教材の誤りなので、`internal` になる。
    UnknownCrate { name: String },
    /// 同梱の許可クレートの一覧を読めない、または期待する形でない。`internal` になる。
    InvalidAllowedCrates { path: PathBuf, reason: String },
    /// ファイルやディレクトリを読み書きできなかった。`io_error` になる。
    Io { path: PathBuf, source: io::Error },
}

impl fmt::Display for AssemblyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidPath(error) => error.fmt(f),
            Self::UnknownCrate { name } => write!(f, "許可クレートの一覧に {name:?} が無い"),
            Self::InvalidAllowedCrates { path, reason } => {
                write!(f, "{}: {reason}", path.display())
            }
            Self::Io { path, source } => write!(f, "{}: {source}", path.display()),
        }
    }
}

impl std::error::Error for AssemblyError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::InvalidPath(error) => Some(error),
            Self::Io { source, .. } => Some(source),
            Self::UnknownCrate { .. } | Self::InvalidAllowedCrates { .. } => None,
        }
    }
}

impl From<InvalidFilePath> for AssemblyError {
    fn from(error: InvalidFilePath) -> Self {
        Self::InvalidPath(error)
    }
}

impl From<AssemblyError> for CommandError {
    fn from(error: AssemblyError) -> Self {
        let (code, message) = match &error {
            AssemblyError::InvalidPath(_) => (
                ErrorCode::InvalidArgument,
                "使えないファイル名があります。src/ か tests/ の下の .rs ファイルにしてください。",
            ),
            AssemblyError::Io { .. } => (
                ErrorCode::IoError,
                "実行するプロジェクトのファイルを書き込めませんでした。",
            ),
            AssemblyError::UnknownCrate { .. } | AssemblyError::InvalidAllowedCrates { .. } => {
                (ErrorCode::Internal, "想定外のエラーが起きました。")
            }
        };
        CommandError {
            code,
            message: message.into(),
            detail: Some(error.to_string()),
        }
    }
}

/// `project/` を空にしてから、`Cargo.toml`・`Cargo.lock`・エディタ上の全ファイルを書く。
///
/// - `dependencies` は演習の `dependencies`（使う許可クレートの名前）。
/// - `files` はエディタ上の全ファイル。キーは cargo パッケージのルートからの相対パス（`/` 区切り）。
///
/// ファイルパスと許可クレートを確かめてから、`project/` に手を付ける。確かめられなかったときは、
/// `project/` を変えずに失敗する。
pub fn assemble_project(
    workspace: &WorkspaceDir,
    runner: &RunnerDir,
    dependencies: &[String],
    files: &BTreeMap<String, String>,
) -> Result<(), AssemblyError> {
    for path in files.keys() {
        validate_file_path(path)?;
    }
    let manifest = cargo_toml(runner, dependencies)?;

    let project = workspace.project();
    empty_dir(&project)?;
    write(&project.join("Cargo.toml"), &manifest)?;
    let lock = runner.cargo_lock();
    std::fs::copy(&lock, project.join("Cargo.lock")).map_err(|source| AssemblyError::Io {
        path: lock.clone(),
        source,
    })?;
    for (path, content) in files {
        let target = path
            .split('/')
            .fold(project.clone(), |dir, segment| dir.join(segment));
        if let Some(parent) = target.parent() {
            create_dir_all(parent)?;
        }
        write(&target, content)?;
    }
    Ok(())
}

/// `project/` のビルドで cargo に渡す環境変数。ビルド先（`CARGO_TARGET_DIR`）と `RUST_BACKTRACE=0`。
pub fn build_env(
    workspace: &WorkspaceDir,
    dependencies: &[String],
) -> Vec<(&'static str, OsString)> {
    vec![
        (
            "CARGO_TARGET_DIR",
            workspace
                .project_target(uses_allowed_crates(dependencies))
                .into_os_string(),
        ),
        ("RUST_BACKTRACE", OsString::from("0")),
    ]
}

/// 許可クレートを使う演習かどうか。
pub fn uses_allowed_crates(dependencies: &[String]) -> bool {
    !dependencies.is_empty()
}

/// 演習のプロジェクトの `Cargo.toml`。
///
/// 許可クレートを使う演習では、`[dependencies]` に、使うクレートを `allowed-crates.toml` の指定のまま書き、
/// 最後に `allcrates/` への path 依存を書く。続けて `[lints.cargo]` で cargo の「使っていない依存」の警告を止める。
/// 許可クレートを使わない演習では `[dependencies]` を空にする。
fn cargo_toml(runner: &RunnerDir, dependencies: &[String]) -> Result<String, AssemblyError> {
    let mut lines = vec![
        "[package]".to_string(),
        format!("name = \"{PACKAGE_NAME}\""),
        "version = \"0.1.0\"".to_string(),
        "edition = \"2024\"".to_string(),
        String::new(),
        "[dependencies]".to_string(),
    ];
    if uses_allowed_crates(dependencies) {
        let allowed = read_allowed_crates(runner)?;
        let names: BTreeSet<&str> = dependencies.iter().map(String::as_str).collect();
        for name in names {
            let entry = allowed
                .get(name)
                .ok_or_else(|| AssemblyError::UnknownCrate {
                    name: name.to_string(),
                })?;
            lines.push(format!("{name} = {entry}"));
        }
        lines.push(format!(
            "{ALLOWED_CRATES_PACKAGE} = {{ path = \"../allcrates\" }}"
        ));
        lines.push(String::new());
        lines.push("[lints.cargo]".to_string());
        lines.push("unused_dependencies = \"allow\"".to_string());
    }
    lines.push(String::new());
    Ok(lines.join("\n"))
}

/// `allowed-crates.toml` の `[dependencies]` を、クレートの名前から、指定（TOML のインラインの値）への対応にする。
fn read_allowed_crates(runner: &RunnerDir) -> Result<BTreeMap<String, String>, AssemblyError> {
    let path = runner.allowed_crates();
    let invalid = |reason: String| AssemblyError::InvalidAllowedCrates {
        path: path.clone(),
        reason,
    };
    let text = std::fs::read_to_string(&path).map_err(|source| AssemblyError::Io {
        path: path.clone(),
        source,
    })?;
    let mut table: toml::Table = text.parse().map_err(|error| invalid(format!("{error}")))?;
    let Some(toml::Value::Table(dependencies)) = table.remove("dependencies") else {
        return Err(invalid("[dependencies] が無い".into()));
    };
    dependencies
        .into_iter()
        .map(|(name, value)| {
            let is_bare_key = !name.is_empty()
                && name
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-');
            if !is_bare_key {
                return Err(invalid(format!("クレートの名前 {name:?} が使えない")));
            }
            Ok((name, value.to_string()))
        })
        .collect()
}

/// `dir` を空のディレクトリにする。無いときは作る。
fn empty_dir(dir: &Path) -> Result<(), AssemblyError> {
    let io_error = |source| AssemblyError::Io {
        path: dir.to_path_buf(),
        source,
    };
    match std::fs::symlink_metadata(dir) {
        Ok(metadata) if metadata.is_dir() => std::fs::remove_dir_all(dir).map_err(io_error)?,
        Ok(_) => std::fs::remove_file(dir).map_err(io_error)?,
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(error) => return Err(io_error(error)),
    }
    create_dir_all(dir)
}

fn create_dir_all(dir: &Path) -> Result<(), AssemblyError> {
    std::fs::create_dir_all(dir).map_err(|source| AssemblyError::Io {
        path: dir.to_path_buf(),
        source,
    })
}

fn write(path: &Path, content: &str) -> Result<(), AssemblyError> {
    std::fs::write(path, content).map_err(|source| AssemblyError::Io {
        path: path.to_path_buf(),
        source,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::fs;

    /// テストごとの一時ディレクトリに、作業領域と、同梱の `runner/` を置く。
    struct Fixture {
        temp: tempfile::TempDir,
        workspace: WorkspaceDir,
        runner: RunnerDir,
    }

    const BUNDLED_LOCK: &str = "# 同梱の Cargo.lock\nversion = 4\n";

    fn fixture(allowed_crates: &str) -> Fixture {
        let temp = tempfile::tempdir().unwrap();
        let runner_root = temp.path().join("resources/runner");
        fs::create_dir_all(&runner_root).unwrap();
        fs::write(runner_root.join("allowed-crates.toml"), allowed_crates).unwrap();
        fs::write(runner_root.join("Cargo.lock"), BUNDLED_LOCK).unwrap();
        let workspace = WorkspaceDir::new(temp.path().join("data/workspace"));
        Fixture {
            temp,
            workspace,
            runner: RunnerDir::new(runner_root),
        }
    }

    fn files(entries: &[(&str, &str)]) -> BTreeMap<String, String> {
        entries
            .iter()
            .map(|(path, content)| (path.to_string(), content.to_string()))
            .collect()
    }

    fn deps(names: &[&str]) -> Vec<String> {
        names.iter().map(|name| name.to_string()).collect()
    }

    /// `dir` の下のファイル（ディレクトリを除く）を、`dir` からの相対パスで返す。
    fn list_files(dir: &Path) -> Vec<String> {
        fn walk(base: &Path, dir: &Path, out: &mut Vec<String>) {
            for entry in fs::read_dir(dir).unwrap() {
                let path = entry.unwrap().path();
                if path.is_dir() {
                    walk(base, &path, out);
                } else {
                    let relative = path.strip_prefix(base).unwrap();
                    out.push(relative.to_string_lossy().replace('\\', "/"));
                }
            }
        }
        let mut out = Vec::new();
        walk(dir, dir, &mut out);
        out.sort();
        out
    }

    const ALLOWED: &str = "[dependencies]\nrand = \"=0.9.1\"\n";

    #[test]
    fn writes_the_manifest_lock_and_all_files() {
        let fx = fixture(ALLOWED);
        let editor = files(&[
            ("src/main.rs", "fn main() {}\n"),
            ("src/shapes/circle.rs", "pub struct Circle;\n"),
            ("tests/it.rs", "#[test]\nfn ok() {}\n"),
        ]);
        assemble_project(&fx.workspace, &fx.runner, &[], &editor).unwrap();

        let project = fx.workspace.project();
        assert_eq!(
            list_files(&project),
            [
                "Cargo.lock",
                "Cargo.toml",
                "src/main.rs",
                "src/shapes/circle.rs",
                "tests/it.rs",
            ]
        );
        for (path, content) in &editor {
            assert_eq!(&fs::read_to_string(project.join(path)).unwrap(), content);
        }
        assert_eq!(
            fs::read_to_string(project.join("Cargo.lock")).unwrap(),
            BUNDLED_LOCK
        );
    }

    #[test]
    fn manifest_without_allowed_crates_has_no_path_dependency_or_lints() {
        let fx = fixture(ALLOWED);
        assemble_project(
            &fx.workspace,
            &fx.runner,
            &[],
            &files(&[("src/main.rs", "fn main() {}\n")]),
        )
        .unwrap();
        let manifest = fs::read_to_string(fx.workspace.project().join("Cargo.toml")).unwrap();
        assert_eq!(
            manifest,
            "[package]\n\
             name = \"progrust_exercise\"\n\
             version = \"0.1.0\"\n\
             edition = \"2024\"\n\
             \n\
             [dependencies]\n"
        );
        assert!(!manifest.contains("allcrates"));
        assert!(!manifest.contains("[lints"));
    }

    #[test]
    fn manifest_with_allowed_crates_follows_the_spec() {
        let fx = fixture(ALLOWED);
        assemble_project(
            &fx.workspace,
            &fx.runner,
            &deps(&["rand"]),
            &files(&[("src/main.rs", "fn main() {}\n")]),
        )
        .unwrap();
        let manifest = fs::read_to_string(fx.workspace.project().join("Cargo.toml")).unwrap();
        assert_eq!(
            manifest,
            "[package]\n\
             name = \"progrust_exercise\"\n\
             version = \"0.1.0\"\n\
             edition = \"2024\"\n\
             \n\
             [dependencies]\n\
             rand = \"=0.9.1\"\n\
             progrust_allowed_crates = { path = \"../allcrates\" }\n\
             \n\
             [lints.cargo]\n\
             unused_dependencies = \"allow\"\n"
        );
    }

    #[test]
    fn manifest_copies_only_the_listed_crates_as_specified() {
        let fx = fixture(
            "[dependencies]\n\
             rand = \"=0.9.1\"\n\
             serde = { version = \"=1.0.228\", features = [\"derive\"] }\n\
             regex = \"=1.12.2\"\n",
        );
        assemble_project(
            &fx.workspace,
            &fx.runner,
            &deps(&["serde", "rand", "serde"]),
            &files(&[("src/main.rs", "fn main() {}\n")]),
        )
        .unwrap();
        let manifest = fs::read_to_string(fx.workspace.project().join("Cargo.toml")).unwrap();
        let dependencies: toml::Table = manifest.parse::<toml::Table>().unwrap()["dependencies"]
            .as_table()
            .unwrap()
            .clone();
        let expected: toml::Table = "rand = \"=0.9.1\"\n\
             serde = { version = \"=1.0.228\", features = [\"derive\"] }\n\
             progrust_allowed_crates = { path = \"../allcrates\" }\n"
            .parse()
            .unwrap();
        assert_eq!(dependencies, expected);
        // path 依存は、[dependencies] の最後の行に書く。
        let section: Vec<&str> = manifest
            .split("[dependencies]\n")
            .nth(1)
            .unwrap()
            .lines()
            .take_while(|line| !line.is_empty())
            .collect();
        assert_eq!(
            section.last(),
            Some(&"progrust_allowed_crates = { path = \"../allcrates\" }")
        );
        assert!(manifest.contains("[lints.cargo]\nunused_dependencies = \"allow\"\n"));
    }

    #[test]
    fn removes_files_that_were_in_project_before() {
        let fx = fixture(ALLOWED);
        let project = fx.workspace.project();
        fs::create_dir_all(project.join("src/old")).unwrap();
        fs::create_dir_all(project.join("tests")).unwrap();
        fs::write(project.join("src/old/stale.rs"), "// 前の演習").unwrap();
        fs::write(project.join("src/main.rs"), "// 前の演習").unwrap();
        fs::write(project.join("tests/stale.rs"), "// 前の演習").unwrap();
        fs::write(project.join("Cargo.lock"), "# cargo が書き換えた lock").unwrap();
        fs::write(project.join("notes.txt"), "前の演習").unwrap();

        assemble_project(
            &fx.workspace,
            &fx.runner,
            &[],
            &files(&[("src/main.rs", "fn main() {}\n")]),
        )
        .unwrap();

        assert_eq!(
            list_files(&project),
            ["Cargo.lock", "Cargo.toml", "src/main.rs"]
        );
        assert_eq!(
            fs::read_to_string(project.join("src/main.rs")).unwrap(),
            "fn main() {}\n"
        );
        assert_eq!(
            fs::read_to_string(project.join("Cargo.lock")).unwrap(),
            BUNDLED_LOCK
        );
    }

    #[test]
    fn paths_with_parent_segments_write_nothing_outside_project() {
        let fx = fixture(ALLOWED);
        let project = fx.workspace.project();
        fs::create_dir_all(project.join("src")).unwrap();
        fs::write(project.join("src/main.rs"), "// 前の実行").unwrap();
        let outside = fx.temp.path().join("outside.rs");
        let before = list_files(fx.temp.path());

        for path in [
            "../outside.rs".to_string(),
            "src/../../outside.rs".to_string(),
            "src/../../../../outside.rs".to_string(),
            "tests/../../../outside.rs".to_string(),
            outside.to_string_lossy().into_owned(),
        ] {
            let editor = files(&[("src/main.rs", "fn main() {}\n"), (&path, "// 外に書く")]);
            let error = assemble_project(&fx.workspace, &fx.runner, &[], &editor).unwrap_err();
            assert!(
                matches!(error, AssemblyError::InvalidPath(_)),
                "{path:?}: {error}"
            );
            assert_eq!(
                CommandError::from(error).code,
                ErrorCode::InvalidArgument,
                "{path:?}"
            );
        }

        // project/ の外にも中にも、何も書いていない（前の実行のファイルも残っている）。
        assert_eq!(list_files(fx.temp.path()), before);
        assert!(!outside.exists());
        assert_eq!(
            fs::read_to_string(project.join("src/main.rs")).unwrap(),
            "// 前の実行"
        );
    }

    #[test]
    fn unknown_crates_are_an_internal_error_and_leave_project_alone() {
        let fx = fixture(ALLOWED);
        let error = assemble_project(
            &fx.workspace,
            &fx.runner,
            &deps(&["rand", "tokio"]),
            &files(&[("src/main.rs", "fn main() {}\n")]),
        )
        .unwrap_err();
        assert!(
            matches!(&error, AssemblyError::UnknownCrate { name } if name == "tokio"),
            "{error}"
        );
        assert_eq!(CommandError::from(error).code, ErrorCode::Internal);
        assert!(!fx.workspace.project().exists());
    }

    #[test]
    fn allowed_crates_without_dependencies_table_is_an_internal_error() {
        let fx = fixture("rand = \"=0.9.1\"\n");
        let error = assemble_project(
            &fx.workspace,
            &fx.runner,
            &deps(&["rand"]),
            &files(&[("src/main.rs", "fn main() {}\n")]),
        )
        .unwrap_err();
        assert!(
            matches!(error, AssemblyError::InvalidAllowedCrates { .. }),
            "{error}"
        );
        assert_eq!(CommandError::from(error).code, ErrorCode::Internal);
    }

    #[test]
    fn missing_bundled_lock_is_an_io_error() {
        let fx = fixture(ALLOWED);
        fs::remove_file(fx.runner.cargo_lock()).unwrap();
        let error = assemble_project(
            &fx.workspace,
            &fx.runner,
            &[],
            &files(&[("src/main.rs", "fn main() {}\n")]),
        )
        .unwrap_err();
        assert!(matches!(error, AssemblyError::Io { .. }), "{error}");
        assert_eq!(CommandError::from(error).code, ErrorCode::IoError);
    }

    #[test]
    fn build_env_sets_target_dir_and_backtrace() {
        let workspace = WorkspaceDir::new("/w");
        assert_eq!(
            build_env(&workspace, &deps(&["rand"])),
            [
                ("CARGO_TARGET_DIR", OsString::from("/w/target")),
                ("RUST_BACKTRACE", OsString::from("0")),
            ]
        );
        assert_eq!(
            build_env(&workspace, &[]),
            [
                ("CARGO_TARGET_DIR", OsString::from("/w/target-std")),
                ("RUST_BACKTRACE", OsString::from("0")),
            ]
        );
    }

    /// 同梱の許可クレートの一覧（リポジトリの `runner/`）で、仕様のとおりに組み立てられる。
    #[test]
    fn assembles_with_the_bundled_allowed_crates() {
        let temp = tempfile::tempdir().unwrap();
        let workspace = WorkspaceDir::new(temp.path().join("workspace"));
        let runner = RunnerDir::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../runner"));
        assemble_project(
            &workspace,
            &runner,
            &deps(&["rand"]),
            &files(&[("src/main.rs", "fn main() {}\n")]),
        )
        .unwrap();
        let project = workspace.project();
        let manifest = fs::read_to_string(project.join("Cargo.toml")).unwrap();
        assert!(manifest.contains("\nrand = \"=0.9.1\"\n"), "{manifest}");
        assert_eq!(
            fs::read(project.join("Cargo.lock")).unwrap(),
            fs::read(runner.cargo_lock()).unwrap()
        );
    }
}
