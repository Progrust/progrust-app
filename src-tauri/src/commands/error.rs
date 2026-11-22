//! コマンドが失敗したときに返すエラー（spec/architecture/commands/conventions.md「エラー」）。

use serde::Serialize;

/// エラーの種類。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCode {
    InvalidArgument,
    NotFound,
    InvalidState,
    Busy,
    CargoUnavailable,
    ClaudeUnavailable,
    CratesNotReady,
    IoError,
    Internal,
}

/// コマンドの `Err` の値。フロントエンドでは `invoke` の Promise の拒否の値になる。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CommandError {
    pub code: ErrorCode,
    /// 学習者にそのまま見せてよい日本語の説明。
    pub message: String,
    /// 原因の詳しい内容。無いときは `None`（JSON では `null`）。
    pub detail: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serializes_in_the_spec_shape() {
        let error = CommandError {
            code: ErrorCode::Busy,
            message: "ほかの実行・提出が終わるまで待ってください。".into(),
            detail: None,
        };
        assert_eq!(
            serde_json::to_value(&error).unwrap(),
            serde_json::json!({
                "code": "busy",
                "message": "ほかの実行・提出が終わるまで待ってください。",
                "detail": null,
            })
        );
    }

    #[test]
    fn error_codes_are_snake_case() {
        let codes = [
            (ErrorCode::InvalidArgument, "invalid_argument"),
            (ErrorCode::NotFound, "not_found"),
            (ErrorCode::InvalidState, "invalid_state"),
            (ErrorCode::Busy, "busy"),
            (ErrorCode::CargoUnavailable, "cargo_unavailable"),
            (ErrorCode::ClaudeUnavailable, "claude_unavailable"),
            (ErrorCode::CratesNotReady, "crates_not_ready"),
            (ErrorCode::IoError, "io_error"),
            (ErrorCode::Internal, "internal"),
        ];
        for (code, expected) in codes {
            assert_eq!(serde_json::to_value(code).unwrap(), expected);
        }
    }
}
