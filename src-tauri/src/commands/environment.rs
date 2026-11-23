//! 環境のコマンド（spec/architecture/commands/environment.md）。

use tauri::State;

use super::error::{CommandError, ErrorCode};
use crate::environment::{Environment, EnvironmentStatus};

/// 環境の状態を返す。アプリ起動時の検出が終わっていないときは、終わるのを待ってから返す。
/// 待つ間に画面を止めないよう、async のコマンドにして、別のスレッドで待つ。
#[tauri::command(rename_all = "snake_case")]
pub async fn get_environment_status(
    environment: State<'_, Environment>,
) -> Result<EnvironmentStatus, CommandError> {
    let environment = environment.inner().clone();
    tauri::async_runtime::spawn_blocking(move || environment.status())
        .await
        .map_err(|error| CommandError {
            code: ErrorCode::Internal,
            message: "環境の状態を読めませんでした。".into(),
            detail: Some(error.to_string()),
        })
}
