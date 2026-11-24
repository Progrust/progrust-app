//! フロントエンドに許可する Tauri の権限（spec/architecture/tauri-permissions.md）と、
//! フロントエンドがファイルシステム・子プロセスを直接扱えないこと
//! （spec/architecture/responsibilities.md）と、フロントエンドから開ける外部の URL が
//! 導入手順の URL だけであること（spec/architecture/external-tools.md「見つからない場合」）を確かめる。

mod common;

use std::{fs, path::Path};

use common::{LOCAL_URL, app, invoke};
use serde_json::{Value, json};

/// フロントエンドに許可してよい、スコープの無い権限。本アプリのコマンドは、権限を書かなくても呼べる。
const ALLOWED_PERMISSIONS: [&str; 2] = [
    // バックエンドが送るイベントの受け取り（`listen` と、受け取りをやめる `unlisten`）。
    "core:event:allow-listen",
    "core:event:allow-unlisten",
];

/// 外部 URL を OS の既定ブラウザで開く権限。開ける URL を、スコープで絞る。
const OPEN_URL_PERMISSION: &str = "opener:allow-open-url";

/// フロントエンドから開ける外部の URL。導入手順の URL だけ
/// （spec/architecture/external-tools.md「見つからない場合」、ADR 0240）。
const OPENABLE_URLS: [&str; 2] = ["https://rustup.rs", "https://code.claude.com/docs/ja/setup"];

/// 外部 URL を開くコマンド。
const OPEN_URL_COMMAND: &str = "plugin:opener|open_url";

/// フロントエンドに公開しないプラグイン。
const FORBIDDEN_PLUGINS: [&str; 2] = ["fs", "shell"];

fn manifest_dir() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

fn read_json(path: &Path) -> Value {
    let text =
        fs::read_to_string(path).unwrap_or_else(|e| panic!("{} を読めない: {e}", path.display()));
    serde_json::from_str(&text)
        .unwrap_or_else(|e| panic!("{} を JSON として読めない: {e}", path.display()))
}

#[test]
fn capabilities_allow_only_listed_permissions() {
    let dir = manifest_dir().join("capabilities");
    // Tauri が権限の定義として読む拡張子のファイル（.DS_Store などは数えない）。
    let mut files: Vec<_> = fs::read_dir(&dir)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| {
            path.extension().is_some_and(|extension| {
                ["json", "json5", "toml"].contains(&extension.to_str().unwrap_or(""))
            })
        })
        .collect();
    files.sort();
    assert_eq!(
        files,
        [dir.join("default.json")],
        "capabilities/ のファイルは default.json だけにする"
    );

    let capability = read_json(&files[0]);
    assert_eq!(capability["windows"], json!(["main"]));
    assert_eq!(
        capability.get("remote"),
        None,
        "外部のページに権限を許可しない"
    );
    let permissions = capability["permissions"]
        .as_array()
        .expect("permissions は配列");
    let mut scoped = Vec::new();
    for permission in permissions {
        match permission.as_str() {
            Some(identifier) => assert!(
                ALLOWED_PERMISSIONS.contains(&identifier),
                "許可していない権限がある: {identifier}"
            ),
            None => scoped.push(permission),
        }
    }
    // スコープ付きの権限は、外部 URL を開く権限の 1 つだけ。開ける URL は導入手順の 2 つだけで、
    // 開くアプリ（`app`）は指定させない（既定ブラウザだけ）。ファイルやパスを開く権限は無い。
    let allow: Vec<Value> = OPENABLE_URLS
        .iter()
        .map(|url| json!({ "url": url }))
        .collect();
    assert_eq!(
        scoped,
        [&json!({ "identifier": OPEN_URL_PERMISSION, "allow": allow })],
        "スコープ付きの権限は、導入手順の URL を開く権限だけにする"
    );
}

#[test]
fn tauri_conf_does_not_define_capabilities() {
    let config = read_json(&manifest_dir().join("tauri.conf.json"));
    assert_eq!(
        config["app"]["security"].get("capabilities"),
        None,
        "権限は capabilities/ にだけ書く"
    );
}

#[test]
fn forbidden_plugins_are_not_dependencies() {
    let cargo_lock = fs::read_to_string(manifest_dir().join("Cargo.lock")).unwrap();
    let package_lock = read_json(&manifest_dir().join("../package-lock.json"));
    let npm_packages = package_lock["packages"].as_object().unwrap();
    for plugin in FORBIDDEN_PLUGINS {
        let crate_name = format!("name = \"tauri-plugin-{plugin}\"");
        assert!(
            !cargo_lock.lines().any(|line| line == crate_name),
            "Cargo.lock に tauri-plugin-{plugin} がある"
        );
        let npm_name = format!("node_modules/@tauri-apps/plugin-{plugin}");
        assert!(
            !npm_packages.contains_key(&npm_name),
            "package-lock.json に @tauri-apps/plugin-{plugin} がある"
        );
    }
}

/// Tauri の組み込みの API（`core:` の権限で許可するもの）のコマンドの一覧。
fn core_commands() -> Vec<String> {
    plugin_commands(|key| key.strip_prefix("core:"))
}

/// 権限の定義のうち、`plugin` が名前を返すもののコマンドの一覧（`plugin:{名前}|{コマンド}` の形）。
/// `tauri-build` がビルドのときに書き出す `gen/schemas/acl-manifests.json` から読む。
fn plugin_commands(plugin: impl Fn(&str) -> Option<&str>) -> Vec<String> {
    let manifests = read_json(&manifest_dir().join("gen/schemas/acl-manifests.json"));
    let mut commands = Vec::new();
    for (key, manifest) in manifests.as_object().unwrap() {
        let Some(plugin) = plugin(key) else {
            continue;
        };
        for permission in manifest["permissions"].as_object().unwrap().values() {
            for command in permission["commands"]["allow"].as_array().unwrap() {
                let command = format!("plugin:{plugin}|{}", command.as_str().unwrap());
                if !commands.contains(&command) {
                    commands.push(command);
                }
            }
        }
    }
    commands.sort();
    commands
}

fn is_acl_rejection(error: &Value) -> bool {
    error
        .as_str()
        .is_some_and(|message| message.contains("not allowed"))
}

#[test]
fn frontend_can_call_app_commands() {
    let (_app, window) = app();
    assert!(invoke(&window, LOCAL_URL, "get_app_version", json!({})).is_ok());
}

#[test]
fn remote_pages_cannot_call_app_commands() {
    let (_app, window) = app();
    let response = invoke(&window, "https://example.com", "get_app_version", json!({}));
    assert!(
        response.as_ref().is_err_and(is_acl_rejection),
        "外部のページから呼べてしまう: {response:?}"
    );
}

#[test]
fn frontend_can_listen_to_events() {
    let (_app, window) = app();
    let listen = invoke(
        &window,
        LOCAL_URL,
        "plugin:event|listen",
        json!({ "event": "job_updated", "target": { "kind": "Any" }, "handler": 0 }),
    );
    let event_id = listen.expect("listen を呼べない");
    let unlisten = invoke(
        &window,
        LOCAL_URL,
        "plugin:event|unlisten",
        json!({ "event": "job_updated", "eventId": event_id }),
    );
    assert!(unlisten.is_ok(), "unlisten を呼べない: {unlisten:?}");
}

/// 組み込みの API は、イベントの受け取り（`listen`・`unlisten`）のほかは、どれも呼べない。
/// ファイルのパス、アプリ情報、ウィンドウの操作、イベントの送信などを含む。
#[test]
fn frontend_cannot_call_other_core_commands() {
    let commands = core_commands();
    assert!(
        commands.len() > 50,
        "組み込みの API のコマンドの一覧を読めていない: {commands:?}"
    );
    let (_app, window) = app();
    for command in commands {
        if command == "plugin:event|listen" || command == "plugin:event|unlisten" {
            continue;
        }
        let response = invoke(&window, LOCAL_URL, &command, json!({}));
        assert!(
            response.as_ref().is_err_and(is_acl_rejection),
            "{command} を呼べてしまう: {response:?}"
        );
    }
}

fn is_forbidden_url(error: &Value) -> bool {
    error
        .as_str()
        .is_some_and(|message| message.starts_with("Not allowed to open url"))
}

/// 導入手順の 2 つのほかの URL は、開けない。スコープの照合は URL の文字列をそのまま比べるので、
/// 末尾の `/`、`http`、別のパス、別のホストも、開けない。
///
/// 開ける URL は、ここでは呼ばない（モックの実行環境でも、本当に既定ブラウザで開く）。開ける URL は、
/// 権限の設定（`capabilities_allow_only_listed_permissions`）で確かめる。
#[test]
fn frontend_can_open_only_install_guide_urls() {
    let (_app, window) = app();
    for url in [
        "https://example.com",
        "https://rustup.rs/",
        "http://rustup.rs",
        "https://rustup.rs.example.com",
        "https://rustup.rs/../example",
        "https://code.claude.com/docs/ja/setup/",
        "https://code.claude.com/docs/en/setup",
        "https://code.claude.com/docs/ja/setup?x=1",
        "file:///etc/passwd",
        "/etc/passwd",
    ] {
        let response = invoke(&window, LOCAL_URL, OPEN_URL_COMMAND, json!({ "url": url }));
        assert!(
            response.as_ref().is_err_and(is_forbidden_url),
            "{url} を開けてしまう: {response:?}"
        );
    }
}

/// 導入手順の URL でも、既定ブラウザのほかのアプリでは開けない。
#[test]
fn frontend_cannot_choose_app_to_open_urls() {
    let (_app, window) = app();
    for url in OPENABLE_URLS {
        let response = invoke(
            &window,
            LOCAL_URL,
            OPEN_URL_COMMAND,
            // 誤って通っても何も起動しないように、無いアプリの名前を渡す。
            json!({ "url": url, "with": "progrust-no-such-app" }),
        );
        assert!(
            response.as_ref().is_err_and(is_forbidden_url),
            "{url} をほかのアプリで開けてしまう: {response:?}"
        );
    }
}

/// 外部のページからは、URL を開くコマンドを呼べない。スコープで拒否される URL を渡し、権限で
/// 拒否されること（スコープの照合まで進まないこと）を確かめる。
#[test]
fn remote_pages_cannot_open_urls() {
    let (_app, window) = app();
    let response = invoke(
        &window,
        "https://example.com",
        OPEN_URL_COMMAND,
        json!({ "url": "https://example.com" }),
    );
    assert!(
        response.as_ref().is_err_and(is_acl_rejection),
        "外部のページから呼べてしまう: {response:?}"
    );
}

/// opener のプラグインのコマンドは、URL を開くほかは、どれも呼べない（ファイルやパスを開く、
/// フォルダに表示するなど）。
#[test]
fn frontend_cannot_call_other_opener_commands() {
    let commands = plugin_commands(|key| (key == "opener").then_some("opener"));
    assert!(
        commands.contains(&"plugin:opener|open_path".to_string()),
        "opener のコマンドの一覧を読めていない: {commands:?}"
    );
    let (_app, window) = app();
    for command in commands {
        if command == OPEN_URL_COMMAND {
            continue;
        }
        let response = invoke(&window, LOCAL_URL, &command, json!({}));
        assert!(
            response.as_ref().is_err_and(is_acl_rejection),
            "{command} を呼べてしまう: {response:?}"
        );
    }
}
