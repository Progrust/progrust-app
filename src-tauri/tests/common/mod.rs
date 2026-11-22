//! テストで共通に使う、モックの実行環境のアプリとコマンドの呼び出し。

use progrust_lib::{app_builder, app_context};
use serde_json::Value;
use tauri::{
    App, WebviewWindow, WebviewWindowBuilder,
    ipc::{CallbackFn, InvokeBody},
    test::{INVOKE_KEY, MockRuntime, get_ipc_response, mock_builder},
    webview::InvokeRequest,
};

/// テストのアプリのバージョン。`tauri.conf.json` の版や `tauri::test` の既定（`0.1.0`）と
/// 違う値にして、コマンドがコンテキストの版を返していることを確かめられるようにする。
pub const TEST_APP_VERSION: &str = "9.8.7";

/// アプリのページの URL（macOS）。
pub const LOCAL_URL: &str = "tauri://localhost";

/// 本アプリの `tauri.conf.json` と `capabilities/` を使い、モックの実行環境で作ったアプリと、
/// そのウィンドウ（`main`）。
pub fn app() -> (App<MockRuntime>, WebviewWindow<MockRuntime>) {
    let mut context = app_context::<MockRuntime>();
    context.package_info_mut().version = TEST_APP_VERSION.parse().unwrap();
    let app = app_builder(mock_builder())
        .build(context)
        .expect("モックのアプリを作れない");
    let window = WebviewWindowBuilder::new(&app, "main", Default::default())
        .build()
        .expect("ウィンドウを作れない");
    (app, window)
}

/// `url` のページから `invoke(cmd, args)` を呼んだときの結果。
pub fn invoke(
    window: &WebviewWindow<MockRuntime>,
    url: &str,
    cmd: &str,
    args: Value,
) -> Result<Value, Value> {
    get_ipc_response(
        window,
        InvokeRequest {
            cmd: cmd.into(),
            callback: CallbackFn(0),
            error: CallbackFn(1),
            url: url.parse().unwrap(),
            body: InvokeBody::Json(args),
            headers: Default::default(),
            invoke_key: INVOKE_KEY.to_string(),
        },
    )
    .map(|body| body.deserialize::<Value>().unwrap())
}
