//! 演習コードの下書き（`drafts/{lesson_id}.json`。spec/lesson-flow/drafts.md）の型と読み書き。
//!
//! - 読み込めない下書きは `.broken-{日時}` を付けて退避し、作り直さずに、無いものとして扱う
//!   （spec/architecture/data-locations.md「書き込みの規則」）。
//! - 学習者が編集したファイルを保存するときは、ファイルパスを確かめてから書く（[`AppDataDir::save_draft`]）。

use std::collections::BTreeMap;
use std::fmt;

use serde::{Deserialize, Serialize};

use crate::commands::error::CommandError;
use crate::progress::now_string;
use crate::runner::file_path::{InvalidFilePath, validate_file_path};
use crate::storage::{AppDataDir, StorageError};

/// 下書きの `schema_version`。
pub const SCHEMA_VERSION: u64 = 1;

/// 下書き 1 件。キーが cargo パッケージのルートからの相対パス（`/` 区切り）で、値がファイルの内容。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Draft {
    pub schema_version: u64,
    pub lesson_id: String,
    /// 保存した日時。RFC 3339 の文字列（例: `2026-10-31T13:10:00+09:00`）。
    pub updated_at: String,
    pub files: BTreeMap<String, String>,
}

/// 下書きを保存できなかった理由。コマンドでは `CommandError` に変えて返す（`?` で変わる）。
#[derive(Debug)]
pub enum DraftError {
    /// ファイルパスが条件を満たさない。`invalid_argument` になる。
    InvalidPath(InvalidFilePath),
    /// 学習者データを読み書きできなかった。
    Storage(StorageError),
}

impl fmt::Display for DraftError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidPath(error) => error.fmt(f),
            Self::Storage(error) => error.fmt(f),
        }
    }
}

impl std::error::Error for DraftError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::InvalidPath(error) => Some(error),
            Self::Storage(error) => Some(error),
        }
    }
}

impl From<InvalidFilePath> for DraftError {
    fn from(error: InvalidFilePath) -> Self {
        Self::InvalidPath(error)
    }
}

impl From<StorageError> for DraftError {
    fn from(error: StorageError) -> Self {
        Self::Storage(error)
    }
}

impl From<DraftError> for CommandError {
    fn from(error: DraftError) -> Self {
        match error {
            DraftError::InvalidPath(error) => error.into(),
            DraftError::Storage(error) => error.into(),
        }
    }
}

/// 下書きの、`{app_data_dir}` からの相対パス。
fn draft_file(lesson_id: &str) -> String {
    format!("drafts/{lesson_id}.json")
}

impl AppDataDir {
    /// 下書きを読む。無いときと、読み込めない下書きを退避したときは `None`。
    pub fn read_draft(&self, lesson_id: &str) -> Result<Option<Draft>, StorageError> {
        Ok(self
            .read_optional_versioned_json(draft_file(lesson_id), SCHEMA_VERSION)?
            .value)
    }

    /// エディタ上の全ファイル `files` を下書きに保存し、保存した下書きを返す。`updated_at` はいまの日時にする。
    /// ファイルパスが条件（spec/exercise-runner/project-assembly.md）を満たさないものがあるときは、何も書かない。
    pub fn save_draft(
        &self,
        lesson_id: &str,
        files: BTreeMap<String, String>,
    ) -> Result<Draft, DraftError> {
        for path in files.keys() {
            validate_file_path(path)?;
        }
        Ok(self.write_draft(lesson_id, files)?)
    }

    /// 教材の初期コード `starter_files` で下書きを上書きし、保存した下書きを返す。
    pub fn reset_draft(
        &self,
        lesson_id: &str,
        starter_files: BTreeMap<String, String>,
    ) -> Result<Draft, StorageError> {
        self.write_draft(lesson_id, starter_files)
    }

    /// 下書きを消す。無いときは、何もしない。正解して試行セッションが終わったときに呼ぶ。
    pub fn delete_draft(&self, lesson_id: &str) -> Result<(), StorageError> {
        self.remove_file(draft_file(lesson_id))
    }

    fn write_draft(
        &self,
        lesson_id: &str,
        files: BTreeMap<String, String>,
    ) -> Result<Draft, StorageError> {
        let draft = Draft {
            schema_version: SCHEMA_VERSION,
            lesson_id: lesson_id.to_string(),
            updated_at: now_string(),
            files,
        };
        self.write_json(draft_file(lesson_id), &draft)?;
        Ok(draft)
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;
    use crate::commands::error::ErrorCode;

    fn files(entries: &[(&str, &str)]) -> BTreeMap<String, String> {
        entries
            .iter()
            .map(|(path, content)| (path.to_string(), content.to_string()))
            .collect()
    }

    #[test]
    fn a_saved_draft_is_read_back_in_the_spec_shape() {
        let temp = tempfile::tempdir().unwrap();
        let dir = AppDataDir::new(temp.path());
        let saved = dir
            .save_draft("ch04-04", files(&[("src/main.rs", "fn main() {}\n")]))
            .unwrap();
        assert_eq!(saved.schema_version, 1);
        assert_eq!(saved.lesson_id, "ch04-04");
        assert!(saved.updated_at.contains('T'), "{}", saved.updated_at);
        assert_eq!(dir.read_draft("ch04-04").unwrap(), Some(saved.clone()));

        let json: serde_json::Value = serde_json::from_str(
            &fs::read_to_string(temp.path().join("drafts/ch04-04.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(
            json,
            serde_json::json!({
                "schema_version": 1,
                "lesson_id": "ch04-04",
                "updated_at": saved.updated_at,
                "files": { "src/main.rs": "fn main() {}\n" },
            })
        );
    }

    #[test]
    fn a_missing_draft_is_none() {
        let temp = tempfile::tempdir().unwrap();
        let dir = AppDataDir::new(temp.path());
        assert_eq!(dir.read_draft("ch04-04").unwrap(), None);
        assert!(!temp.path().join("drafts").exists());
    }

    #[test]
    fn an_invalid_path_writes_nothing_and_is_invalid_argument() {
        let temp = tempfile::tempdir().unwrap();
        let dir = AppDataDir::new(temp.path());
        let error = dir
            .save_draft(
                "ch04-04",
                files(&[("src/main.rs", "fn main() {}\n"), ("../x.rs", "")]),
            )
            .unwrap_err();
        assert!(matches!(error, DraftError::InvalidPath(_)), "{error}");
        assert_eq!(CommandError::from(error).code, ErrorCode::InvalidArgument);
        assert!(!temp.path().join("drafts").exists());
    }

    #[test]
    fn delete_draft_removes_the_draft() {
        let temp = tempfile::tempdir().unwrap();
        let dir = AppDataDir::new(temp.path());
        dir.save_draft("ch04-04", files(&[("src/main.rs", "")]))
            .unwrap();
        dir.delete_draft("ch04-04").unwrap();
        assert_eq!(dir.read_draft("ch04-04").unwrap(), None);
        // 無い下書きを消しても失敗しない。
        dir.delete_draft("ch04-04").unwrap();
    }

    #[test]
    fn a_draft_with_unknown_fields_is_unreadable() {
        let temp = tempfile::tempdir().unwrap();
        let dir = AppDataDir::new(temp.path());
        dir.write_text(
            "drafts/ch04-04.json",
            r#"{"schema_version":1,"lesson_id":"ch04-04","updated_at":"2026-10-31T13:10:00+09:00","files":{},"extra":1}"#,
        )
        .unwrap();
        assert_eq!(dir.read_draft("ch04-04").unwrap(), None);
        assert!(!temp.path().join("drafts/ch04-04.json").exists());
    }
}
