//! 学習者データの読み書きの失敗。

use std::fmt;
use std::io;
use std::path::PathBuf;

use crate::commands::error::{CommandError, ErrorCode};

/// 学習者データの読み書きの失敗。コマンドでは `CommandError` に変えて返す（`?` で変わる）。
#[derive(Debug)]
pub enum StorageError {
    /// ファイルの読み書きに失敗した。コマンドのエラー `io_error` になる。
    Io { path: PathBuf, source: io::Error },
    /// 書き込む値を JSON にできなかった。プログラムの誤りなので、`internal` になる。
    Serialize {
        path: PathBuf,
        source: serde_json::Error,
    },
    /// 初期状態の値が、期待する `schema_version` を持っていない。プログラムの誤りなので、`internal` になる。
    InitialStateWithoutSchemaVersion { path: PathBuf, expected: u64 },
    /// `{app_data_dir}` からの相対パスとして使えないパスを渡した。プログラムの誤りなので、`internal` になる。
    InvalidPath { path: PathBuf },
}

impl StorageError {
    pub(super) fn io(path: impl Into<PathBuf>, source: io::Error) -> Self {
        Self::Io {
            path: path.into(),
            source,
        }
    }
}

impl fmt::Display for StorageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io { path, source } => write!(f, "{}: {source}", path.display()),
            Self::Serialize { path, source } => {
                write!(f, "{}: JSON にできない: {source}", path.display())
            }
            Self::InitialStateWithoutSchemaVersion { path, expected } => write!(
                f,
                "{}: 初期状態に schema_version {expected} が無い",
                path.display()
            ),
            Self::InvalidPath { path } => {
                write!(
                    f,
                    "{}: 学習者データの相対パスとして使えない",
                    path.display()
                )
            }
        }
    }
}

impl std::error::Error for StorageError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            Self::Serialize { source, .. } => Some(source),
            Self::InitialStateWithoutSchemaVersion { .. } | Self::InvalidPath { .. } => None,
        }
    }
}

impl From<StorageError> for CommandError {
    fn from(error: StorageError) -> Self {
        let (code, message) = match error {
            StorageError::Io { .. } => (ErrorCode::IoError, "ファイルの読み書きに失敗しました。"),
            StorageError::Serialize { .. }
            | StorageError::InitialStateWithoutSchemaVersion { .. }
            | StorageError::InvalidPath { .. } => {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn io_failures_become_io_error_with_the_os_message_as_detail() {
        let error = StorageError::io(
            "/data/progress.json",
            io::Error::new(io::ErrorKind::PermissionDenied, "Permission denied"),
        );
        assert_eq!(
            serde_json::to_value(CommandError::from(error)).unwrap(),
            serde_json::json!({
                "code": "io_error",
                "message": "ファイルの読み書きに失敗しました。",
                "detail": "/data/progress.json: Permission denied",
            })
        );
    }

    #[test]
    fn program_errors_become_internal() {
        let errors = [
            StorageError::InvalidPath {
                path: "../x.json".into(),
            },
            StorageError::InitialStateWithoutSchemaVersion {
                path: "/data/progress.json".into(),
                expected: 1,
            },
        ];
        for error in errors {
            assert_eq!(CommandError::from(error).code, ErrorCode::Internal);
        }
    }
}
