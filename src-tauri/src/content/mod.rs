//! アプリ同梱の教材（`{app_resources}/content/`）の読み込み（spec/architecture/commands/content.md）。
//!
//! 教材のディレクトリは [`ContentDir::new`] に渡す。アプリでは [`ContentDir::from_app`] で
//! Tauri の `resource_dir()` の下の `content/` にし、テストでは一時ディレクトリに作った教材を渡す。
//! 教材の内容の検査は、教材チェックツールが行う。ここでは、読めないファイルを失敗として返すだけにする。

use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use tauri::{AppHandle, Manager, Runtime};

use crate::commands::error::{CommandError, ErrorCode};

/// 生成問題のレッスン ID の接頭辞。生成問題を開くのは M6 で作るので、いまは無いレッスンと同じに扱う。
const GENERATED_PREFIX: &str = "gen-";

/// 教材のディレクトリ。
#[derive(Debug, Clone)]
pub struct ContentDir {
    root: PathBuf,
}

/// `list_chapters` の戻り値。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ChapterList {
    pub chapters: Vec<Chapter>,
}

/// 章。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Chapter {
    pub id: String,
    pub title: String,
    pub description: String,
    pub lessons: Vec<LessonSummary>,
}

/// 章の中のレッスン 1 件。準備中のレッスンは `lesson.json` が無いので、`in_preparation` 以外の項目が `None` になる。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LessonSummary {
    pub id: String,
    pub kind: Option<String>,
    pub title: Option<String>,
    pub summary: Option<String>,
    pub estimated_minutes: Option<u64>,
    pub in_preparation: bool,
}

/// `get_lesson` の戻り値。
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct LessonDetail {
    /// `lesson.json` の内容。
    pub lesson: Value,
    /// `script.json` の内容。動画の無いレッスンでは `None`。
    pub script: Option<Value>,
    /// 音声のディレクトリの絶対パス。動画の無いレッスンでは `None`。
    pub voice_dir: Option<String>,
    /// 演習の内容。演習の無いレッスンでは `None`。
    pub exercise: Option<Exercise>,
}

/// 演習の内容。判定に使うファイル（`cases.json`・`judge_tests.rs`）と `solution/` は含めない。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Exercise {
    pub problem_md: String,
    pub explanation_md: String,
    /// 初期コード。キーは cargo パッケージのルートからの相対パス（`/` 区切り）。
    pub starter_files: std::collections::BTreeMap<String, String>,
}

/// 教材を読めなかった理由。コマンドでは `CommandError` に変えて返す（`?` で変わる）。
#[derive(Debug)]
pub enum ContentError {
    /// レッスンが無い、または準備中である。
    LessonNotFound { lesson_id: String },
    /// ファイルやディレクトリを読めなかった。
    Io { path: PathBuf, source: io::Error },
    /// JSON として読めない、または期待する形でない。教材の誤りなので、`internal` になる。
    Invalid { path: PathBuf, reason: String },
}

impl fmt::Display for ContentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::LessonNotFound { lesson_id } => write!(f, "レッスン {lesson_id} が無い"),
            Self::Io { path, source } => write!(f, "{}: {source}", path.display()),
            Self::Invalid { path, reason } => write!(f, "{}: {reason}", path.display()),
        }
    }
}

impl std::error::Error for ContentError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            Self::LessonNotFound { .. } | Self::Invalid { .. } => None,
        }
    }
}

impl From<ContentError> for CommandError {
    fn from(error: ContentError) -> Self {
        let (code, message) = match &error {
            ContentError::LessonNotFound { .. } => {
                (ErrorCode::NotFound, "レッスンが見つかりません。")
            }
            ContentError::Io { .. } => (ErrorCode::IoError, "教材の読み込みに失敗しました。"),
            ContentError::Invalid { .. } => (ErrorCode::Internal, "想定外のエラーが起きました。"),
        };
        CommandError {
            code,
            message: message.into(),
            detail: Some(error.to_string()),
        }
    }
}

/// `chapters.json` の 1 章。
#[derive(Deserialize)]
struct RawChapter {
    id: String,
    title: String,
    description: String,
    lessons: Vec<String>,
}

impl ContentDir {
    /// `root` を教材のディレクトリ（`content/`）として使う。
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    /// Tauri の `resource_dir()` の下の `content/` を教材のディレクトリとして使う。
    pub fn from_app<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<Self> {
        Ok(Self::new(app.path().resource_dir()?.join("content")))
    }

    /// 教材のディレクトリのパス。
    pub fn root(&self) -> &Path {
        &self.root
    }

    fn lesson_dir(&self, lesson_id: &str) -> PathBuf {
        self.root.join("lessons").join(lesson_id)
    }

    /// 章とレッスンの並びを読む。
    pub fn list_chapters(&self) -> Result<ChapterList, ContentError> {
        let chapters = self
            .read_chapters_json()?
            .into_iter()
            .map(|raw| {
                let lessons = raw
                    .lessons
                    .iter()
                    .map(|lesson_id| self.lesson_summary(lesson_id))
                    .collect::<Result<_, _>>()?;
                Ok(Chapter {
                    id: raw.id,
                    title: raw.title,
                    description: raw.description,
                    lessons,
                })
            })
            .collect::<Result<_, ContentError>>()?;
        Ok(ChapterList { chapters })
    }

    /// レッスンの `kind`（`lesson`・`video_only`・`chapter_review` など）を返す。
    /// レッスンが無いときの扱いは `get_lesson` と同じ。
    pub fn lesson_kind(&self, lesson_id: &str) -> Result<String, ContentError> {
        let not_found = || ContentError::LessonNotFound {
            lesson_id: lesson_id.to_string(),
        };
        if lesson_id.starts_with(GENERATED_PREFIX) || !self.is_listed(lesson_id)? {
            return Err(not_found());
        }
        let dir = self.lesson_dir(lesson_id);
        if !dir.is_dir() {
            return Err(not_found());
        }
        let lesson = read_json(&dir.join("lesson.json"))?;
        Ok(lesson
            .get("kind")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string())
    }

    /// レッスンの定義・台本・音声のディレクトリ・演習の内容を読む。
    /// 準備中のレッスン、`chapters.json` に無いレッスン、生成問題（`gen-` で始まる ID）は `LessonNotFound` にする。
    pub fn get_lesson(&self, lesson_id: &str) -> Result<LessonDetail, ContentError> {
        let not_found = || ContentError::LessonNotFound {
            lesson_id: lesson_id.to_string(),
        };
        // ID を `chapters.json` の中から探すので、パスの区切りや `..` を含む ID でディレクトリの外を読むことは無い。
        if lesson_id.starts_with(GENERATED_PREFIX) || !self.is_listed(lesson_id)? {
            return Err(not_found());
        }
        let dir = self.lesson_dir(lesson_id);
        if !dir.is_dir() {
            return Err(not_found());
        }

        let lesson = read_json(&dir.join("lesson.json"))?;
        let kind = lesson.get("kind").and_then(Value::as_str).unwrap_or("");

        let has_video = kind != "chapter_review";
        let has_exercise = kind != "video_only";

        let script = if has_video {
            Some(read_json(&dir.join("video").join("script.json"))?)
        } else {
            None
        };
        let voice_dir = has_video.then(|| {
            dir.join("video")
                .join("voice")
                .to_string_lossy()
                .into_owned()
        });
        let exercise = if has_exercise {
            Some(read_exercise(&dir.join("exercise"))?)
        } else {
            None
        };

        Ok(LessonDetail {
            lesson,
            script,
            voice_dir,
            exercise,
        })
    }

    fn read_chapters_json(&self) -> Result<Vec<RawChapter>, ContentError> {
        let path = self.root.join("chapters.json");
        let value = read_json(&path)?;
        serde_json::from_value(value).map_err(|error| ContentError::Invalid {
            path,
            reason: error.to_string(),
        })
    }

    fn is_listed(&self, lesson_id: &str) -> Result<bool, ContentError> {
        Ok(self
            .read_chapters_json()?
            .iter()
            .any(|chapter| chapter.lessons.iter().any(|id| id == lesson_id)))
    }

    fn lesson_summary(&self, lesson_id: &str) -> Result<LessonSummary, ContentError> {
        let dir = self.lesson_dir(lesson_id);
        if !dir.is_dir() {
            return Ok(LessonSummary {
                id: lesson_id.to_string(),
                kind: None,
                title: None,
                summary: None,
                estimated_minutes: None,
                in_preparation: true,
            });
        }
        let path = dir.join("lesson.json");
        let lesson = read_json(&path)?;
        let text = |key: &str| lesson.get(key).and_then(Value::as_str).map(str::to_string);
        let field = |name: &str, value: Option<String>| {
            value.ok_or_else(|| ContentError::Invalid {
                path: path.clone(),
                reason: format!("{name} が文字列でない"),
            })
        };
        Ok(LessonSummary {
            id: lesson_id.to_string(),
            kind: Some(field("kind", text("kind"))?),
            title: Some(field("title", text("title"))?),
            summary: text("summary"),
            estimated_minutes: lesson.get("estimated_minutes").and_then(Value::as_u64),
            in_preparation: false,
        })
    }
}

fn read_text(path: &Path) -> Result<String, ContentError> {
    fs::read_to_string(path).map_err(|source| ContentError::Io {
        path: path.to_path_buf(),
        source,
    })
}

fn read_json(path: &Path) -> Result<Value, ContentError> {
    serde_json::from_str(&read_text(path)?).map_err(|error| ContentError::Invalid {
        path: path.to_path_buf(),
        reason: format!("JSON として読めない: {error}"),
    })
}

/// 演習のディレクトリから、問題文・解説・初期コードだけを読む。
fn read_exercise(dir: &Path) -> Result<Exercise, ContentError> {
    let mut starter_files = std::collections::BTreeMap::new();
    let starter = dir.join("starter");
    collect_files(&starter, &starter, &mut starter_files)?;
    Ok(Exercise {
        problem_md: read_text(&dir.join("problem.md"))?,
        explanation_md: read_text(&dir.join("explanation.md"))?,
        starter_files,
    })
}

/// `dir` の下のファイルを、`base` からの相対パス（`/` 区切り）をキーにして集める。
fn collect_files(
    base: &Path,
    dir: &Path,
    files: &mut std::collections::BTreeMap<String, String>,
) -> Result<(), ContentError> {
    let entries = fs::read_dir(dir).map_err(|source| ContentError::Io {
        path: dir.to_path_buf(),
        source,
    })?;
    for entry in entries {
        let entry = entry.map_err(|source| ContentError::Io {
            path: dir.to_path_buf(),
            source,
        })?;
        let path = entry.path();
        if path.is_dir() {
            collect_files(base, &path, files)?;
        } else {
            let relative = path
                .strip_prefix(base)
                .expect("base の下のパス")
                .components()
                .map(|component| component.as_os_str().to_string_lossy())
                .collect::<Vec<_>>()
                .join("/");
            files.insert(relative, read_text(&path)?);
        }
    }
    Ok(())
}
