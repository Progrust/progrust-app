//! 設定のコマンド（spec/architecture/commands/settings.md）。

use std::path::PathBuf;
use std::time::Duration;

use serde::Serialize;
use serde_json::Value;
use tauri::{AppHandle, PackageInfo, Runtime, State};

use super::error::{CommandError, ErrorCode};
use crate::claude::{ClaudeCall, ClaudeEnv, ClaudeError, ClaudeOutcome, ClaudeRequest};
use crate::environment::{ClaudeTool, Environment};
use crate::settings::Settings;
use crate::storage::AppDataDir;

/// 接続テストの制限時間。質疑応答と同じ（spec/claude-integration/invocation.md「制限時間」）。
pub const CONNECTION_TEST_TIME_LIMIT: Duration = Duration::from_secs(120);

/// 接続テストで `--system-prompt` に渡す文。応答が返るかだけを確かめる。
pub const CONNECTION_TEST_SYSTEM_PROMPT: &str =
    "あなたは接続テストに答えるアシスタントです。指示に従い、できるだけ短く答えてください。";

/// 接続テストで標準入力に渡す指示文。
pub const CONNECTION_TEST_PROMPT: &str = "接続テストです。「OK」とだけ答えてください。";

/// `test_claude_connection` の戻り値。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ConnectionTestResult {
    /// 応答が返ったら `true`。
    pub ok: bool,
    /// 使ったモデル（質疑応答のモデル）。
    pub model: String,
    /// `ok` が `false` のときのエラー。
    pub error: Option<ClaudeError>,
}

/// 短い指示文を `model` で `claude` に実行させ、応答が返るかを確かめる。呼び出しが終わるまで待つ。
/// `claude` が見つからない（`claude` が `None`）ときは `claude_unavailable`。
pub fn run_connection_test(
    claude: Option<ClaudeTool>,
    model: String,
    env: ClaudeEnv,
    work_dir_parent: PathBuf,
    time_limit: Duration,
) -> Result<ConnectionTestResult, CommandError> {
    let Some(claude) = claude else {
        return Err(CommandError {
            code: ErrorCode::ClaudeUnavailable,
            message: "Claude Code が見つかりません。".into(),
            detail: None,
        });
    };
    let call = ClaudeCall::new(ClaudeRequest {
        claude: claude.path,
        env,
        model: model.clone(),
        system_prompt: CONNECTION_TEST_SYSTEM_PROMPT.into(),
        prompt: CONNECTION_TEST_PROMPT.into(),
        time_limit,
        work_dir_parent,
    });
    match call.run(None) {
        ClaudeOutcome::Completed { .. } => Ok(ConnectionTestResult {
            ok: true,
            model,
            error: None,
        }),
        ClaudeOutcome::Failed(error) => Ok(ConnectionTestResult {
            ok: false,
            model,
            error: Some(error),
        }),
        // 中断する手段を渡していないので、ここには来ない。
        ClaudeOutcome::Cancelled => Err(CommandError {
            code: ErrorCode::Internal,
            message: "接続テストが中断されました。".into(),
            detail: None,
        }),
    }
}

/// 設定の質疑応答のモデルで、短い指示文を実際の `claude` に実行させ、応答が返るかを確かめる。
/// 終わるまで最大 120 秒かかるので、async のコマンドにして、別のスレッドで呼ぶ。
#[tauri::command(rename_all = "snake_case")]
pub async fn test_claude_connection(
    data_dir: State<'_, AppDataDir>,
    environment: State<'_, Environment>,
) -> Result<ConnectionTestResult, CommandError> {
    let model = data_dir.read_settings()?.models.chat;
    let environment = environment.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        run_connection_test(
            environment.claude(),
            model,
            ClaudeEnv::from_process(),
            std::env::temp_dir(),
            CONNECTION_TEST_TIME_LIMIT,
        )
    })
    .await
    .map_err(|error| CommandError {
        code: ErrorCode::Internal,
        message: "接続テストを実行できませんでした。".into(),
        detail: Some(error.to_string()),
    })?
}

/// `get_app_version` の戻り値。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AppVersion {
    pub version: String,
}

impl AppVersion {
    /// アプリのパッケージの情報から作る。Tauri の実行時に頼らずに呼べる。
    pub fn from_package_info(package_info: &PackageInfo) -> Self {
        Self {
            version: package_info.version.to_string(),
        }
    }
}

/// アプリのバージョンを返す。フロントエンドには Tauri のアプリ情報の API を許可しないので、
/// 設定画面はこのコマンドでバージョンを受け取る。
#[tauri::command(rename_all = "snake_case")]
pub fn get_app_version<R: Runtime>(app: AppHandle<R>) -> Result<AppVersion, CommandError> {
    Ok(AppVersion::from_package_info(app.package_info()))
}

/// 設定を返す。`settings.json` が無いときは、初期値の設定を返す。
#[tauri::command(rename_all = "snake_case")]
pub fn get_settings(data_dir: State<'_, AppDataDir>) -> Result<Settings, CommandError> {
    Ok(data_dir.read_settings()?)
}

/// 設定の全体を検査してから保存し、保存した設定を返す。`schema_version` はバックエンドが付ける。
/// 値が条件を満たさないときは `invalid_argument` を返し、ファイルは変えない。
#[tauri::command(rename_all = "snake_case")]
pub fn update_settings(
    data_dir: State<'_, AppDataDir>,
    settings: Value,
) -> Result<Settings, CommandError> {
    let settings = Settings::from_argument(settings)?;
    data_dir.write_settings(&settings)?;
    Ok(settings)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn package_info(version: &str) -> PackageInfo {
        PackageInfo {
            name: "Progrust".into(),
            version: version.parse().unwrap(),
            authors: "",
            description: "",
            crate_name: "progrust",
        }
    }

    #[test]
    fn app_version_is_the_package_version() {
        assert_eq!(
            AppVersion::from_package_info(&package_info("9.8.7")),
            AppVersion {
                version: "9.8.7".into()
            }
        );
    }

    #[test]
    fn app_version_serializes_as_an_object() {
        assert_eq!(
            serde_json::to_value(AppVersion::from_package_info(&package_info("1.2.3-beta.1")))
                .unwrap(),
            serde_json::json!({ "version": "1.2.3-beta.1" })
        );
    }
}
