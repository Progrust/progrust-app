//! エディタから受け取るファイルパスの検証（spec/exercise-runner/project-assembly.md）。
//!
//! 実行・提出の `project/` の組み立てと、下書きの保存の両方で使う。

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

#[cfg(test)]
mod tests {
    use super::*;

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
