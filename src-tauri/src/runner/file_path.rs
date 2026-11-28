//! エディタから受け取るファイルパスの検証（spec/exercise-runner/project-assembly.md、
//! spec/exercise-runner/c-project-assembly.md「作業領域」）。
//!
//! - Rust の演習は [`validate_file_path`]。実行・提出の `project/` の組み立てと、下書きの保存の両方で使う。
//! - C の演習は [`validate_c_file_paths`]。`main.c` の 1 つだけを受け付ける。

use std::fmt;

use crate::commands::error::{CommandError, ErrorCode};

/// 受け付けるパスの、最初の要素。
const ROOTS: [&str; 2] = ["src", "tests"];
/// 受け付けるファイルの拡張子。
const EXTENSION: &str = ".rs";

/// ファイルパスが条件を満たさない理由。コマンドでは `invalid_argument` になる。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvalidFilePath {
    /// 受け取ったパス。
    pub path: String,
    /// 満たさなかった条件。
    pub reason: &'static str,
}

impl fmt::Display for InvalidFilePath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "ファイルパス {:?} は使えない: {}",
            self.path, self.reason
        )
    }
}

impl std::error::Error for InvalidFilePath {}

impl From<InvalidFilePath> for CommandError {
    fn from(error: InvalidFilePath) -> Self {
        CommandError {
            code: ErrorCode::InvalidArgument,
            message:
                "使えないファイル名があります。src/ か tests/ の下の .rs ファイルにしてください。"
                    .into(),
            detail: Some(error.to_string()),
        }
    }
}

/// エディタから受け取ったファイルパス（cargo パッケージのルートからの、`/` 区切りの相対パス）を確かめる。
///
/// `src/` 以下または `tests/` 以下で、拡張子が `.rs` の相対パスだけを受け付ける。
/// `/` で区切った要素のうち、空のもの（`/` で始まる絶対パス、`//`、末尾の `/`）、`.`、`..` を含むパスは拒否する。
/// `Path::components` は `src//a.rs` や `src/./a.rs` を黙って正規化するので使わず、文字列のまま区切る。
pub fn validate_file_path(path: &str) -> Result<(), InvalidFilePath> {
    let invalid = |reason| {
        Err(InvalidFilePath {
            path: path.to_string(),
            reason,
        })
    };
    if path.contains('\0') {
        return invalid("NUL 文字を含む");
    }
    let segments: Vec<&str> = path.split('/').collect();
    if segments.iter().any(|segment| segment.is_empty()) {
        return invalid("空の要素を含む（絶対パス、`//`、末尾の `/` など）");
    }
    if segments
        .iter()
        .any(|segment| *segment == "." || *segment == "..")
    {
        return invalid("`.` か `..` を含む");
    }
    let [first, .., last] = segments.as_slice() else {
        return invalid("`src/` 以下か `tests/` 以下のファイルでない");
    };
    if !ROOTS.contains(first) {
        return invalid("`src/` 以下か `tests/` 以下のファイルでない");
    }
    match last.strip_suffix(EXTENSION) {
        Some(stem) if !stem.is_empty() => Ok(()),
        _ => invalid("拡張子が `.rs` のファイルでない"),
    }
}

/// C の演習で、エディタから受け取る唯一のファイル。
pub const C_MAIN_FILE: &str = "main.c";

/// C の演習のファイルパスが条件を満たさない理由。コマンドでは `invalid_argument` になる。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InvalidCFilePaths {
    /// `main.c` のほかのパスがある（`..` を含むパスや絶対パスを含む）。
    Unexpected { path: String },
    /// `main.c` が無い。
    MissingMain,
}

impl fmt::Display for InvalidCFilePaths {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unexpected { path } => {
                write!(
                    f,
                    "ファイルパス {path:?} は使えない: `{C_MAIN_FILE}` でない"
                )
            }
            Self::MissingMain => write!(f, "`{C_MAIN_FILE}` が無い"),
        }
    }
}

impl std::error::Error for InvalidCFilePaths {}

impl From<InvalidCFilePaths> for CommandError {
    fn from(error: InvalidCFilePaths) -> Self {
        CommandError {
            code: ErrorCode::InvalidArgument,
            message:
                "使えないファイル名があります。C の演習のファイルは main.c だけにしてください。"
                    .into(),
            detail: Some(error.to_string()),
        }
    }
}

/// C の演習で、エディタから受け取ったファイルパスの全部を確かめる（c-project-assembly.md「作業領域」）。
///
/// パスがちょうど `main.c` の 1 つだけのときに受け付ける。`main.c` のほかのパスがあるときは、最初に見つけたものを
/// 理由にして拒否する。`main.c` が無いときも拒否する。Rust の演習の [`validate_file_path`] とは別の条件である。
pub fn validate_c_file_paths<'a>(
    paths: impl IntoIterator<Item = &'a str>,
) -> Result<(), InvalidCFilePaths> {
    let mut has_main = false;
    for path in paths {
        if path == C_MAIN_FILE {
            has_main = true;
        } else {
            return Err(InvalidCFilePaths::Unexpected {
                path: path.to_string(),
            });
        }
    }
    if has_main {
        Ok(())
    } else {
        Err(InvalidCFilePaths::MissingMain)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn c_accepts_only_main_c() {
        assert_eq!(validate_c_file_paths(["main.c"]), Ok(()));
    }

    #[test]
    fn c_rejects_paths_other_than_main_c() {
        for path in [
            "../main.c",
            "../../etc/passwd",
            "/main.c",
            "/tmp/main.c",
            "./main.c",
            "src/main.c",
            "main.h",
            "Main.c",
            "main.c/",
            "main.c\0",
            "",
            "..",
            "src/main.rs",
        ] {
            assert_eq!(
                validate_c_file_paths(["main.c", path]),
                Err(InvalidCFilePaths::Unexpected {
                    path: path.to_string()
                }),
                "{path:?} は拒否する"
            );
            assert!(
                validate_c_file_paths([path]).is_err(),
                "{path:?} だけでも拒否する"
            );
        }
    }

    #[test]
    fn c_rejects_files_without_main_c() {
        assert_eq!(
            validate_c_file_paths([]),
            Err(InvalidCFilePaths::MissingMain)
        );
    }

    #[test]
    fn c_invalid_paths_are_invalid_argument() {
        let error = CommandError::from(InvalidCFilePaths::Unexpected {
            path: "../x.c".into(),
        });
        assert_eq!(error.code, ErrorCode::InvalidArgument);
        assert!(error.detail.unwrap().contains("../x.c"));
        let error = CommandError::from(InvalidCFilePaths::MissingMain);
        assert_eq!(error.code, ErrorCode::InvalidArgument);
    }

    #[test]
    fn accepts_rs_files_under_src_and_tests() {
        for path in [
            "src/main.rs",
            "src/lib.rs",
            "src/shapes/circle.rs",
            "src/bin/tool.rs",
            "tests/integration.rs",
            "tests/common/mod.rs",
        ] {
            assert_eq!(validate_file_path(path), Ok(()), "{path:?} は受け付ける");
        }
    }

    #[test]
    fn rejects_paths_that_can_leave_the_project() {
        for path in [
            "../main.rs",
            "src/../../main.rs",
            "src/../main.rs",
            "tests/..",
            "/src/main.rs",
            "/etc/passwd.rs",
            "..",
        ] {
            assert!(validate_file_path(path).is_err(), "{path:?} は拒否する");
        }
    }

    #[test]
    fn rejects_empty_and_dot_segments() {
        for path in [
            "",
            "src//main.rs",
            "src/./main.rs",
            "./src/main.rs",
            "src/main.rs/",
            "src/",
        ] {
            assert!(validate_file_path(path).is_err(), "{path:?} は拒否する");
        }
    }

    #[test]
    fn rejects_files_outside_src_and_tests() {
        for path in [
            "main.rs",
            "Cargo.toml",
            "build.rs",
            "examples/demo.rs",
            "benches/bench.rs",
            "target/main.rs",
            "Src/main.rs",
            "src",
            "tests",
        ] {
            assert!(validate_file_path(path).is_err(), "{path:?} は拒否する");
        }
    }

    #[test]
    fn rejects_files_without_the_rs_extension() {
        for path in [
            "src/main",
            "src/main.txt",
            "src/main.rs.bak",
            "src/main.RS",
            "src/.rs",
            "tests/data.json",
            "src/main.rs\0",
        ] {
            assert!(validate_file_path(path).is_err(), "{path:?} は拒否する");
        }
    }
}
