//! 学習記録・分析のコマンド（spec/architecture/commands/learning-record.md）。

use tauri::State;

use super::error::CommandError;
use crate::content::ContentDir;
use crate::dashboard::Dashboard;
use crate::storage::AppDataDir;

/// トップページの、次にやるべきレッスンと、完了数と、章ごとの完了率を返す。
/// 分析に関する項目は M5 で作る。それまでは、学習記録がない状態（`none`）を返す。
#[tauri::command(rename_all = "snake_case")]
pub fn get_dashboard(
    data_dir: State<'_, AppDataDir>,
    content: State<'_, ContentDir>,
) -> Result<Dashboard, CommandError> {
    let chapters = content.list_chapters()?;
    let progress = data_dir.read_progress()?;
    Ok(Dashboard::new(&chapters, &progress))
}
