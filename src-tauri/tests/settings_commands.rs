//! 設定のコマンド（spec/architecture/commands/settings.md）を、モックの実行環境で確かめる。

#[path = "fake_tools/claude.rs"]
mod claude;
mod common;

use std::fs;
use std::path::Path;

use std::time::Duration;

use claude::FakeClaude;
use common::{LOCAL_URL, TEST_APP_VERSION, app, invoke};
use progrust_lib::claude::{ClaudeEnv, ENV_VAR_NAMES};
use progrust_lib::commands::error::ErrorCode;
use progrust_lib::commands::settings::{
    CONNECTION_TEST_PROMPT, CONNECTION_TEST_SYSTEM_PROMPT, CONNECTION_TEST_TIME_LIMIT,
    run_connection_test,
};
use progrust_lib::environment::{ClaudeTool, DetectedTools, Environment};
use progrust_lib::storage::AppDataDir;
use serde_json::{Value, json};
use tauri::test::MockRuntime;
use tauri::{App, Manager, WebviewWindow};

#[test]
fn get_app_version_returns_the_app_version() {
    let (_app, window) = app();
    let response = invoke(&window, LOCAL_URL, "get_app_version", json!({}));
    assert_eq!(response, Ok(json!({ "version": TEST_APP_VERSION })));
}

fn app_with_data_dir(root: &Path) -> (App<MockRuntime>, WebviewWindow<MockRuntime>) {
    let (app, window) = app();
    app.manage(AppDataDir::new(root));
    (app, window)
}

fn settings_json() -> Value {
    json!({
        "schema_version": 1,
        "theme": "dark",
        "editor_font_size": 18,
        "video": { "volume": 0.25, "muted": true, "playback_rate": 1.25, "captions": true },
        "models": { "chat": "haiku", "background": "sonnet" },
        "tool_paths": { "cargo": "/opt/cargo", "claude": null },
        "first_launch_completed": true
    })
}

fn default_settings_json() -> Value {
    json!({
        "schema_version": 1,
        "theme": "system",
        "editor_font_size": 14,
        "video": { "volume": 1.0, "muted": false, "playback_rate": 1.0, "captions": false },
        "models": { "chat": "sonnet", "background": "opus" },
        "tool_paths": { "cargo": null, "claude": null },
        "first_launch_completed": false
    })
}

fn update(window: &WebviewWindow<MockRuntime>, settings: Value) -> Result<Value, Value> {
    invoke(
        window,
        LOCAL_URL,
        "update_settings",
        json!({ "settings": settings }),
    )
}

#[test]
fn get_settings_without_a_file_returns_the_defaults_and_creates_nothing() {
    let temp = tempfile::tempdir().unwrap();
    let (_app, window) = app_with_data_dir(temp.path());
    let response = invoke(&window, LOCAL_URL, "get_settings", json!({}));
    assert_eq!(response, Ok(default_settings_json()));
    assert!(!temp.path().join("settings.json").exists());
}

#[test]
fn update_settings_saves_and_a_fresh_read_returns_the_same_values() {
    let temp = tempfile::tempdir().unwrap();
    {
        let (_app, window) = app_with_data_dir(temp.path());
        assert_eq!(update(&window, settings_json()), Ok(settings_json()));
    }
    // アプリを作り直して（ファイルから読み込み直して）も、同じ値が返る。
    let (_app, window) = app_with_data_dir(temp.path());
    let response = invoke(&window, LOCAL_URL, "get_settings", json!({}));
    assert_eq!(response, Ok(settings_json()));
}

#[test]
fn update_settings_ignores_the_given_schema_version() {
    let temp = tempfile::tempdir().unwrap();
    let (_app, window) = app_with_data_dir(temp.path());
    let mut settings = settings_json();
    settings["schema_version"] = json!(99);
    assert_eq!(update(&window, settings), Ok(settings_json()));
    let saved: Value =
        serde_json::from_str(&fs::read_to_string(temp.path().join("settings.json")).unwrap())
            .unwrap();
    assert_eq!(saved["schema_version"], json!(1));
}

#[test]
fn invalid_settings_are_invalid_argument_and_leave_the_file_unchanged() {
    let temp = tempfile::tempdir().unwrap();
    let (_app, window) = app_with_data_dir(temp.path());
    update(&window, settings_json()).unwrap();
    let before = fs::read(temp.path().join("settings.json")).unwrap();

    let invalid = [
        ("theme", json!("pink")),
        ("editor_font_size", json!(11)),
        ("editor_font_size", json!(25)),
        (
            "video",
            json!({ "volume": 1.5, "muted": false, "playback_rate": 1.0, "captions": false }),
        ),
        (
            "video",
            json!({ "volume": 1.0, "muted": false, "playback_rate": 3.0, "captions": false }),
        ),
        ("models", json!({ "chat": "", "background": "opus" })),
        // `-` で始まる値と、空白を含む値（ADR 0258）。
        ("models", json!({ "chat": "--bare", "background": "opus" })),
        ("models", json!({ "chat": "sonnet", "background": "-p" })),
        (
            "models",
            json!({ "chat": "sonnet", "background": "claude opus" }),
        ),
    ];
    for (key, value) in invalid {
        let mut settings = settings_json();
        settings[key] = value.clone();
        let error = update(&window, settings).unwrap_err();
        assert_eq!(error["code"], json!("invalid_argument"), "{key} = {value}");
        assert_eq!(fs::read(temp.path().join("settings.json")).unwrap(), before);
    }
}

#[test]
fn invalid_settings_do_not_create_the_file() {
    let temp = tempfile::tempdir().unwrap();
    let (_app, window) = app_with_data_dir(temp.path());
    let mut settings = settings_json();
    settings["theme"] = json!("pink");
    let error = update(&window, settings).unwrap_err();
    assert_eq!(error["code"], json!("invalid_argument"));
    assert!(!temp.path().join("settings.json").exists());
}

// ---- test_claude_connection ----

fn tool(fake: &FakeClaude) -> ClaudeTool {
    ClaudeTool {
        path: fake.path().to_path_buf(),
        version: "2.1.329".into(),
    }
}

/// 成功で終わる stream-json の出力（最後の `result` の行だけで足りる）。
fn success_output() -> String {
    format!(
        "{}\n",
        json!({ "type": "result", "subtype": "success", "is_error": false, "result": "OK" })
    )
}

/// `assistant` の行の `error` が `error` の、エラーで終わる出力（errors.md「分類」）。
fn api_error(error: &str, message: &str, _status: Option<u16>) -> String {
    format!(
        "{}\n{}\n",
        json!({ "type": "assistant", "error": error, "message": { "content": [] } }),
        json!({ "type": "result", "subtype": "success", "is_error": true, "result": message })
    )
}

fn run_with(fake: &FakeClaude, model: &str, time_limit: Duration) -> Value {
    let work = tempfile::tempdir().unwrap();
    let result = run_connection_test(
        Some(tool(fake)),
        model.into(),
        ClaudeEnv::from_process(),
        work.path().to_path_buf(),
        time_limit,
    )
    .unwrap();
    serde_json::to_value(result).unwrap()
}

#[test]
fn a_connection_test_that_gets_an_answer_is_ok_with_the_model() {
    let fake = FakeClaude::new();
    fake.write_beside("stdout", success_output());
    let result = run_with(&fake, "haiku", Duration::from_secs(60));
    assert_eq!(
        result,
        json!({ "ok": true, "model": "haiku", "error": null })
    );
    // 渡したモデルと、置き換えたシステムプロンプトと、指示文が、`claude` に届く。
    let calls = fake.calls();
    assert_eq!(calls.len(), 1);
    let args = &calls[0].args;
    let after = |flag: &str| &args[args.iter().position(|a| a == flag).unwrap() + 1];
    assert_eq!(after("--model"), "haiku");
    assert_eq!(after("--system-prompt"), CONNECTION_TEST_SYSTEM_PROMPT);
    assert_eq!(calls[0].stdin.as_deref(), Some(CONNECTION_TEST_PROMPT));
    // 渡す環境変数は 6 つの名前だけ。
    assert!(
        calls[0]
            .env
            .keys()
            .all(|name| ENV_VAR_NAMES.contains(&name.as_str()))
    );
}

#[test]
fn a_connection_test_error_returns_ok_false_with_the_category() {
    let cases = [
        (
            api_error("authentication_failed", "Not logged in", None),
            json!({ "category": "not_logged_in", "detail": null }),
        ),
        (
            api_error("rate_limit", "limit", Some(429)),
            json!({ "category": "rate_limited", "detail": null }),
        ),
        (
            api_error("unknown", "There's an issue with the selected model", None),
            json!({ "category": "other", "detail": "There's an issue with the selected model" }),
        ),
    ];
    for (stdout, error) in cases {
        let fake = FakeClaude::new();
        fake.write_beside("stdout", &stdout);
        fake.write_beside("exit-code", "1");
        let result = run_with(&fake, "sonnet", Duration::from_secs(60));
        assert_eq!(
            result,
            json!({ "ok": false, "model": "sonnet", "error": error })
        );
    }
}

#[test]
fn a_connection_test_that_times_out_is_a_timeout_error() {
    let fake = FakeClaude::new();
    fake.write_beside("hang", "");
    let result = run_with(&fake, "sonnet", Duration::from_millis(300));
    assert_eq!(
        result,
        json!({ "ok": false, "model": "sonnet", "error": { "category": "timeout", "detail": null } })
    );
}

#[test]
fn the_connection_test_time_limit_is_120_seconds() {
    assert_eq!(CONNECTION_TEST_TIME_LIMIT, Duration::from_secs(120));
}

#[test]
fn a_connection_test_without_claude_is_claude_unavailable() {
    let work = tempfile::tempdir().unwrap();
    let error = run_connection_test(
        None,
        "sonnet".into(),
        ClaudeEnv::from_process(),
        work.path().to_path_buf(),
        Duration::from_secs(1),
    )
    .unwrap_err();
    assert_eq!(error.code, ErrorCode::ClaudeUnavailable);
}

/// コマンドは、設定の質疑応答のモデルを使い、見つかった `claude` を呼ぶ。
#[test]
fn the_command_uses_the_chat_model_from_settings() {
    let temp = tempfile::tempdir().unwrap();
    let (app, window) = app_with_data_dir(temp.path());
    let mut settings = settings_json();
    settings["models"] = json!({ "chat": "haiku", "background": "opus" });
    update(&window, settings).unwrap();

    let fake = FakeClaude::new();
    fake.write_beside("stdout", success_output());
    app.manage(Environment::detected(DetectedTools {
        cargo: None,
        claude: Some(tool(&fake)),
    }));
    let response = invoke(&window, LOCAL_URL, "test_claude_connection", json!({}));
    assert_eq!(
        response,
        Ok(json!({ "ok": true, "model": "haiku", "error": null }))
    );
    assert!(
        fake.calls()[0]
            .args
            .windows(2)
            .any(|w| w == ["--model", "haiku"])
    );
}

#[test]
fn the_command_without_claude_is_claude_unavailable() {
    let temp = tempfile::tempdir().unwrap();
    let (app, window) = app_with_data_dir(temp.path());
    app.manage(Environment::detected(DetectedTools::default()));
    let error = invoke(&window, LOCAL_URL, "test_claude_connection", json!({})).unwrap_err();
    assert_eq!(error["code"], json!("claude_unavailable"));
}
