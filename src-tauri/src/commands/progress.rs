//! 進捗のコマンド（spec/architecture/commands/progress.md）。

use tauri::State;

use super::error::CommandError;
use crate::content::ContentDir;
use crate::progress::ProgressSummary;
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
