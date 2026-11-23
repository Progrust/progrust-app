//! 教材のコマンド（spec/architecture/commands/content.md）。

use tauri::State;

use super::error::CommandError;
use crate::content::{ChapterList, ContentDir, LessonDetail};

/// 章とレッスンの並びを返す。
#[tauri::command(rename_all = "snake_case")]
pub fn list_chapters(content: State<'_, ContentDir>) -> Result<ChapterList, CommandError> {
    Ok(content.list_chapters()?)
}

/// レッスンを表示するのに要るものを返す。
#[tauri::command(rename_all = "snake_case")]
pub fn get_lesson(
    content: State<'_, ContentDir>,
    lesson_id: String,
) -> Result<LessonDetail, CommandError> {
    Ok(content.get_lesson(&lesson_id)?)
}
