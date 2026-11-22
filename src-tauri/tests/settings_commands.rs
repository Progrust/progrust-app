//! 設定のコマンド（spec/architecture/commands/settings.md）を、モックの実行環境で確かめる。

mod common;

use common::{LOCAL_URL, TEST_APP_VERSION, app, invoke};
use serde_json::json;

#[test]
fn get_app_version_returns_the_app_version() {
    let (_app, window) = app();
    let response = invoke(&window, LOCAL_URL, "get_app_version", json!({}));
    assert_eq!(response, Ok(json!({ "version": TEST_APP_VERSION })));
}
