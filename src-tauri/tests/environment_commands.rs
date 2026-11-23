//! `cargo`・`rustc` の検出（spec/architecture/external-tools.md「検出」）と、環境のコマンドとイベント
//! （spec/architecture/commands/environment.md）を、偽の `cargo`・`rustc` とモックの実行環境で確かめる。

mod common;
#[path = "fake_tools/rust.rs"]
mod fake_rust;

use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    sync::mpsc,
    thread,
    time::Duration,
};

use common::{LOCAL_URL, app, invoke};
use fake_rust::FakeRust;
use progrust_lib::commands::error::ErrorCode;
use progrust_lib::environment::{
    CargoSearch, ENVIRONMENT_UPDATED, Environment, REQUIRED_RUSTC_VERSION, RustTools,
    configured_cargo, start_detection,
};
use progrust_lib::settings::Settings;
use progrust_lib::storage::AppDataDir;
use serde_json::{Value, json};
use tauri::{Listener, Manager};
use tempfile::TempDir;

fn dir_of(fake: &FakeRust) -> &Path {
    fake.cargo().parent().unwrap()
}

/// 偽のツールのほかに、何も無いディレクトリ（`PATH` やホームディレクトリに使う）。
fn empty_dir() -> TempDir {
    TempDir::new().unwrap()
}

fn search(configured: Option<&Path>, path: &[&Path], home: &Path) -> CargoSearch {
    CargoSearch {
        configured_cargo: configured.map(Path::to_path_buf),
        path_var: Some(std::env::join_paths(path).unwrap()),
        home: Some(home.to_path_buf()),
    }
}

/// 検出の結果を、`get_environment_status` の `cargo` の形にする。
fn cargo_status(search: &CargoSearch) -> Value {
    let status = Environment::detected(search.detect()).status();
    serde_json::to_value(status.cargo).unwrap()
}

fn found_json(status: &str, fake: &FakeRust, version: &str) -> Value {
    json!({
        "status": status,
        "cargo_path": fake.cargo().to_str().unwrap(),
        "rustc_path": fake.rustc().to_str().unwrap(),
        "rustc_version": version,
        "required_rustc_version": "1.100.0",
    })
}

fn not_found_json() -> Value {
    json!({
        "status": "not_found",
        "cargo_path": null,
        "rustc_path": null,
        "rustc_version": null,
        "required_rustc_version": "1.100.0",
    })
}

// ---- found・version_too_old・not_found ----

#[test]
fn rustc_at_the_required_version_is_found() {
    let fake = FakeRust::new("1.100.0");
    let home = empty_dir();
    assert_eq!(
        cargo_status(&search(None, &[dir_of(&fake)], home.path())),
        found_json("found", &fake, "1.100.0")
    );
}

#[test]
fn newer_rustc_is_found() {
    for version in ["1.100.1", "1.101.0", "1.102.0-nightly", "2.0.0"] {
        let fake = FakeRust::new(version);
        let home = empty_dir();
        assert_eq!(
            cargo_status(&search(None, &[dir_of(&fake)], home.path())),
            found_json("found", &fake, version)
        );
    }
}

#[test]
fn older_rustc_is_version_too_old() {
    for version in ["1.99.1", "1.85.0", "1.9.0"] {
        let fake = FakeRust::new(version);
        let home = empty_dir();
        assert_eq!(
            cargo_status(&search(None, &[dir_of(&fake)], home.path())),
            found_json("version_too_old", &fake, version)
        );
    }
}

#[test]
fn nothing_found_is_not_found() {
    let path = empty_dir();
    let home = empty_dir();
    assert_eq!(
        cargo_status(&search(None, &[path.path()], home.path())),
        not_found_json()
    );
    // PATH もホームディレクトリも分からないとき。
    let nothing = CargoSearch {
        configured_cargo: None,
        path_var: None,
        home: None,
    };
    assert_eq!(cargo_status(&nothing), not_found_json());
}

// ---- 探す順 ----

#[test]
fn the_configured_cargo_comes_before_path() {
    let configured = FakeRust::new("1.100.0");
    let in_path = FakeRust::new("1.101.0");
    let home = empty_dir();
    let search = search(Some(configured.cargo()), &[dir_of(&in_path)], home.path());
    assert_eq!(
        cargo_status(&search),
        found_json("found", &configured, "1.100.0")
    );
}

/// 設定ファイル（settings.json）の `tool_paths.cargo` を、探す場所の最初に使う。
#[test]
fn tool_paths_cargo_in_settings_json_is_used() {
    let data = empty_dir();
    let data_dir = AppDataDir::new(data.path());
    let configured = FakeRust::new("1.100.0");
    let mut settings = Settings::default();
    settings.tool_paths.cargo = Some(configured.cargo().to_str().unwrap().to_string());
    data_dir.write_settings(&settings).unwrap();

    let in_path = FakeRust::new("1.101.0");
    let home = empty_dir();
    let mut search = search(None, &[dir_of(&in_path)], home.path());
    search.configured_cargo = configured_cargo(&data_dir);
    assert_eq!(search.configured_cargo.as_deref(), Some(configured.cargo()));
    assert_eq!(
        cargo_status(&search),
        found_json("found", &configured, "1.100.0")
    );
}

/// `tool_paths.cargo` が `null` のときと、settings.json が無いときは、設定のパスを使わない。
#[test]
fn without_tool_paths_cargo_the_settings_give_no_path() {
    let data = empty_dir();
    let data_dir = AppDataDir::new(data.path());
    assert_eq!(configured_cargo(&data_dir), None);
    data_dir.write_settings(&Settings::default()).unwrap();
    assert_eq!(configured_cargo(&data_dir), None);
}

#[test]
fn the_configured_cargo_is_used_even_if_its_rustc_is_too_old() {
    let configured = FakeRust::new("1.99.0");
    let in_path = FakeRust::new("1.100.0");
    let home = empty_dir();
    let search = search(Some(configured.cargo()), &[dir_of(&in_path)], home.path());
    assert_eq!(
        cargo_status(&search),
        found_json("version_too_old", &configured, "1.99.0")
    );
}

#[test]
fn a_missing_configured_cargo_falls_back_to_path() {
    let in_path = FakeRust::new("1.100.0");
    let home = empty_dir();
    let missing = home.path().join("no-such-dir/cargo");
    let search = search(Some(&missing), &[dir_of(&in_path)], home.path());
    assert_eq!(
        cargo_status(&search),
        found_json("found", &in_path, "1.100.0")
    );
}

#[test]
fn the_configured_cargo_uses_rustc_in_the_same_directory() {
    // 設定のパスの `cargo` の隣に `rustc` が無いときは、PATH の `rustc` と組み合わせずに、次の場所を探す。
    let configured = FakeRust::new("1.100.0");
    fs::remove_file(configured.rustc()).unwrap();
    let in_path = FakeRust::new("1.101.0");
    let home = empty_dir();
    let search = search(Some(configured.cargo()), &[dir_of(&in_path)], home.path());
    assert_eq!(
        cargo_status(&search),
        found_json("found", &in_path, "1.101.0")
    );
}

#[test]
fn cargo_bin_in_home_is_searched_when_path_has_no_cargo() {
    let path = empty_dir();
    let home = empty_dir();
    let in_home = FakeRust::place(&home.path().join(".cargo/bin"), "1.100.0");
    assert_eq!(
        cargo_status(&search(None, &[path.path()], home.path())),
        found_json("found", &in_home, "1.100.0")
    );
}

#[test]
fn cargo_bin_in_home_is_searched_after_a_configured_cargo_is_missing() {
    let path = empty_dir();
    let home = empty_dir();
    let in_home = FakeRust::place(&home.path().join(".cargo/bin"), "1.100.0");
    let missing = path.path().join("cargo");
    assert_eq!(
        cargo_status(&search(Some(&missing), &[path.path()], home.path())),
        found_json("found", &in_home, "1.100.0")
    );
}

#[test]
fn path_comes_before_cargo_bin_in_home() {
    let in_path = FakeRust::new("1.99.0");
    let home = empty_dir();
    FakeRust::place(&home.path().join(".cargo/bin"), "1.100.0");
    assert_eq!(
        cargo_status(&search(None, &[dir_of(&in_path)], home.path())),
        found_json("version_too_old", &in_path, "1.99.0")
    );
}

#[test]
fn path_directories_are_searched_in_order() {
    let empty = empty_dir();
    let first = FakeRust::new("1.100.0");
    let second = FakeRust::new("1.101.0");
    let home = empty_dir();
    assert_eq!(
        cargo_status(&search(
            None,
            &[empty.path(), dir_of(&first), dir_of(&second)],
            home.path()
        )),
        found_json("found", &first, "1.100.0")
    );
}

// ---- 使えない場所は飛ばす ----

#[test]
fn a_place_without_rustc_is_skipped() {
    let without_rustc = FakeRust::new("1.100.0");
    fs::remove_file(without_rustc.rustc()).unwrap();
    let home = empty_dir();
    assert_eq!(
        cargo_status(&search(None, &[dir_of(&without_rustc)], home.path())),
        not_found_json()
    );
    let in_home = FakeRust::place(&home.path().join(".cargo/bin"), "1.100.0");
    assert_eq!(
        cargo_status(&search(None, &[dir_of(&without_rustc)], home.path())),
        found_json("found", &in_home, "1.100.0")
    );
}

#[test]
fn a_place_without_cargo_is_skipped() {
    let without_cargo = FakeRust::new("1.100.0");
    fs::remove_file(without_cargo.cargo()).unwrap();
    let home = empty_dir();
    assert_eq!(
        cargo_status(&search(None, &[dir_of(&without_cargo)], home.path())),
        not_found_json()
    );
}

#[test]
fn a_rustc_whose_version_fails_is_skipped() {
    // 偽の rustc は、rust-version が無いと `--version` に失敗する（終了コード 1）。
    let failing = FakeRust::new("1.100.0");
    fs::remove_file(dir_of(&failing).join("rust-version")).unwrap();
    let next = FakeRust::new("1.101.0");
    let home = empty_dir();
    assert_eq!(
        cargo_status(&search(None, &[dir_of(&failing)], home.path())),
        not_found_json()
    );
    assert_eq!(
        cargo_status(&search(
            None,
            &[dir_of(&failing), dir_of(&next)],
            home.path()
        )),
        found_json("found", &next, "1.101.0")
    );
}

#[test]
fn a_rustc_with_unreadable_version_output_is_skipped() {
    let odd = FakeRust::new("unknown");
    let home = empty_dir();
    assert_eq!(
        cargo_status(&search(None, &[dir_of(&odd)], home.path())),
        not_found_json()
    );
}

#[test]
fn files_that_are_not_executable_are_skipped() {
    let dir = empty_dir();
    for name in ["cargo", "rustc"] {
        let path = dir.path().join(name);
        fs::write(&path, "#!/bin/sh\necho rustc 1.100.0\n").unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).unwrap();
    }
    let home = empty_dir();
    assert_eq!(
        cargo_status(&search(None, &[dir.path()], home.path())),
        not_found_json()
    );
}

// ---- 実行・提出のコマンドが使う取り出し口 ----

fn tools(fake: &FakeRust, version: &str) -> RustTools {
    RustTools {
        cargo: fake.cargo().to_path_buf(),
        rustc: fake.rustc().to_path_buf(),
        rustc_version: version.parse().unwrap(),
    }
}

#[test]
fn rust_tools_returns_paths_when_found_or_too_old() {
    for version in ["1.100.0", "1.99.0"] {
        let fake = FakeRust::new(version);
        let environment = Environment::detected(Some(tools(&fake, version)));
        let found = environment.rust_tools().unwrap();
        assert_eq!(found.cargo, fake.cargo());
        assert_eq!(found.rustc, fake.rustc());
        assert_eq!(found.rustc_version.as_str(), version);
    }
}

#[test]
fn rust_tools_is_cargo_unavailable_when_not_found() {
    let error = Environment::detected(None).rust_tools().unwrap_err();
    assert_eq!(error.code, ErrorCode::CargoUnavailable);
    assert!(!error.message.is_empty());
}

#[test]
fn rust_tools_waits_for_detection() {
    let fake = FakeRust::new("1.100.0");
    let environment = Environment::pending();
    let (sender, receiver) = mpsc::channel();
    let reader = environment.clone();
    let handle = thread::spawn(move || sender.send(reader.rust_tools()).unwrap());
    // 検出が終わるまでは返らない。
    assert!(receiver.recv_timeout(Duration::from_millis(300)).is_err());
    environment.finish_detection(Some(tools(&fake, "1.100.0")));
    let found = receiver.recv_timeout(Duration::from_secs(10)).unwrap();
    assert_eq!(found.unwrap().cargo, fake.cargo());
    handle.join().unwrap();
}

// ---- get_environment_status と environment_updated ----

fn status_json(cargo: Value) -> Value {
    json!({
        "cargo": cargo,
        // claude は M4 で検出するまで not_found、初回準備は M3 で作るまで not_started。
        "claude": { "status": "not_found", "path": null, "version": null },
        "crates": { "status": "not_started", "error": null },
    })
}

#[test]
fn get_environment_status_without_cargo_returns_not_found() {
    let (app, window) = app();
    let path = empty_dir();
    let home = empty_dir();
    start_detection(app.handle(), search(None, &[path.path()], home.path()));
    let response = invoke(&window, LOCAL_URL, "get_environment_status", json!({}));
    assert_eq!(response, Ok(status_json(not_found_json())));
}

#[test]
fn get_environment_status_returns_the_detected_tools() {
    let (app, window) = app();
    let fake = FakeRust::new("1.100.0");
    let home = empty_dir();
    start_detection(app.handle(), search(None, &[dir_of(&fake)], home.path()));
    let response = invoke(&window, LOCAL_URL, "get_environment_status", json!({}));
    assert_eq!(
        response,
        Ok(status_json(found_json("found", &fake, "1.100.0")))
    );
}

#[test]
fn get_environment_status_waits_for_detection() {
    let (app, window) = app();
    let environment = Environment::pending();
    app.manage(environment.clone());
    let fake = FakeRust::new("1.99.0");
    let finished = tools(&fake, "1.99.0");
    let finisher = thread::spawn(move || {
        thread::sleep(Duration::from_millis(300));
        environment.finish_detection(Some(finished));
    });
    let response = invoke(&window, LOCAL_URL, "get_environment_status", json!({}));
    assert_eq!(
        response,
        Ok(status_json(found_json("version_too_old", &fake, "1.99.0")))
    );
    finisher.join().unwrap();
}

#[test]
fn environment_updated_is_sent_when_detection_finishes() {
    let (app, window) = app();
    let (sender, receiver) = mpsc::channel();
    app.listen(ENVIRONMENT_UPDATED, move |event| {
        let payload: Value = serde_json::from_str(event.payload()).unwrap();
        let _ = sender.send(payload);
    });
    let fake = FakeRust::new("1.101.0");
    let home = empty_dir();
    start_detection(app.handle(), search(None, &[dir_of(&fake)], home.path()));
    let payload = receiver.recv_timeout(Duration::from_secs(10)).unwrap();
    let expected = status_json(found_json("found", &fake, "1.101.0"));
    assert_eq!(payload, expected);
    // 送った後に読んでも、同じ状態が返る。
    let response = invoke(&window, LOCAL_URL, "get_environment_status", json!({}));
    assert_eq!(response, Ok(expected));
}

#[test]
fn required_rustc_version_is_reported_from_the_constant() {
    let status = Environment::detected(None).status();
    assert_eq!(status.cargo.required_rustc_version, REQUIRED_RUSTC_VERSION);
}

/// 検出は、実行ファイルのパスを書き換えない（シンボリックリンクの先にしない）。rustup の
/// `~/.cargo/bin/cargo` は、起動した名前でどのツールかを決めるので、リンクの先では動かない。
#[test]
fn found_paths_are_not_resolved_through_symlinks() {
    let fake = FakeRust::new("1.100.0");
    let home = empty_dir();
    let found: PathBuf = search(None, &[dir_of(&fake)], home.path())
        .detect()
        .unwrap()
        .cargo;
    assert_eq!(found, fake.cargo());
    assert!(
        fs::symlink_metadata(&found)
            .unwrap()
            .file_type()
            .is_symlink()
    );
}
