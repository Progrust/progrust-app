//! 設定のコマンド（spec/architecture/commands/settings.md）を、モックの実行環境で確かめる。

mod common;

use std::fs;
use std::path::Path;

use common::{LOCAL_URL, TEST_APP_VERSION, app, invoke};
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
