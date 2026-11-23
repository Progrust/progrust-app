//! 進捗のコマンド（spec/architecture/commands/progress.md）。

use serde::Serialize;
use serde_json::Value;
use tauri::State;

use super::error::{CommandError, ErrorCode};
use crate::content::ContentDir;
use crate::progress::{LessonProgress, ProgressSummary, now_string};
use crate::storage::AppDataDir;

/// 進捗と、推奨レッスンを返す。`progress.json` が無いときは、何も進んでいない状態として扱う。
/// `chapters.json` に無いレッスンの進捗は含めない。
#[tauri::command(rename_all = "snake_case")]
pub fn get_progress(
    data_dir: State<'_, AppDataDir>,
    content: State<'_, ContentDir>,
) -> Result<ProgressSummary, CommandError> {
    let chapters = content.list_chapters()?;
    let progress = data_dir.read_progress()?;
    Ok(ProgressSummary::new(&chapters, progress))
}

/// `open_lesson` の戻り値。試行セッションと下書きは M2、生成問題は M6 で作るので、いまは `null` を返す。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct OpenedLesson {
    pub session_id: Option<String>,
    pub progress: Option<LessonProgress>,
    pub generated: Option<Value>,
    pub draft: Option<Value>,
}

/// レッスンを開いたことを進捗に記録し、更新した後の進捗を返す。
#[tauri::command(rename_all = "snake_case")]
pub fn open_lesson(
    data_dir: State<'_, AppDataDir>,
    content: State<'_, ContentDir>,
    lesson_id: String,
) -> Result<OpenedLesson, CommandError> {
    content.lesson_kind(&lesson_id)?;
    let now = now_string();
    let progress =
        data_dir.update_progress(|progress| progress.open_lesson(&lesson_id, &now).clone())?;
    Ok(OpenedLesson {
        session_id: None,
        progress: Some(progress),
        generated: None,
        draft: None,
    })
}

/// 動画の再生位置と、最後まで再生したかを保存する。動画のみのレッスンで初めて最後まで再生したときは、完了にする。
/// 動画の無いレッスン（章末復習レッスン）では `invalid_argument` を返す。
#[tauri::command(rename_all = "snake_case")]
pub fn save_video_position(
    data_dir: State<'_, AppDataDir>,
    content: State<'_, ContentDir>,
    lesson_id: String,
    last_sentence_id: String,
    video_watched: bool,
) -> Result<LessonProgress, CommandError> {
    let kind = content.lesson_kind(&lesson_id)?;
    if kind == "chapter_review" {
        return Err(CommandError {
            code: ErrorCode::InvalidArgument,
            message: "このレッスンには動画がありません。".into(),
            detail: Some(format!("レッスン {lesson_id} は動画の無いレッスン")),
        });
    }
    let now = now_string();
    Ok(data_dir.update_progress(|progress| {
        progress
            .save_video_position(
                &lesson_id,
                &last_sentence_id,
                video_watched,
                kind == "video_only",
                &now,
            )
            .clone()
    })?)
}
