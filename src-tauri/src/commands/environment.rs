//! 環境のコマンド（spec/architecture/commands/environment.md）。

use tauri::{AppHandle, Runtime, State};

use super::error::{CommandError, ErrorCode};
use crate::environment::{Environment, EnvironmentStatus, preparation};

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

/// 初回準備に失敗したときに、初回準備をやり直す。始めたら `null` を返し、経過と結果は `environment_updated` で
/// 知らせる。初回準備の状態が `failed` でないときは `invalid_state`、`cargo` が見つからないときは `cargo_unavailable`。
/// 起動時の検出が終わるのを待つので、async のコマンドにして、別のスレッドで待つ。
#[tauri::command(rename_all = "snake_case")]
pub async fn retry_crate_preparation<R: Runtime>(app: AppHandle<R>) -> Result<(), CommandError> {
    tauri::async_runtime::spawn_blocking(move || preparation::retry_crate_preparation(&app))
        .await
        .map_err(|error| CommandError {
            code: ErrorCode::Internal,
            message: "外部クレートの準備をやり直せませんでした。".into(),
            detail: Some(error.to_string()),
        })?
}
