//! 進捗のコマンド `get_progress`（spec/architecture/commands/progress.md）を、モックの実行環境と、
//! 一時ディレクトリに作った教材・学習者データで確かめる。

mod common;

use std::fs;
use std::path::Path;

use common::{LOCAL_URL, app, invoke};
use progrust_lib::content::ContentDir;
use progrust_lib::storage::AppDataDir;
use serde_json::{Value, json};
use tauri::test::MockRuntime;
use tauri::{App, Manager, WebviewWindow};

fn write(root: &Path, relative: &str, content: &str) {
    let path = root.join(relative);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, content).unwrap();
}

/// 教材: ch01 は a, b（準備中）, c。ch02 は d（準備中）。ディレクトリがあるレッスンが、準備中でないレッスン。
fn make_content() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    write(
        root,
        "chapters.json",
        &json!([
            { "id": "ch01", "title": "第1章", "description": "", "lessons": ["a", "b", "c"] },
            { "id": "ch02", "title": "第2章", "description": "", "lessons": ["d"] },
        ])
        .to_string(),
    );
    for id in ["a", "c"] {
        write(
            root,
            &format!("lessons/{id}/lesson.json"),
            &json!({ "id": id, "kind": "video_only", "title": format!("{id} のタイトル") })
                .to_string(),
        );
    }
    dir
}

fn app_with(content: &Path, data: &Path) -> (App<MockRuntime>, WebviewWindow<MockRuntime>) {
    let (app, window) = app();
    app.manage(ContentDir::new(content));
    app.manage(AppDataDir::new(data));
    (app, window)
}

fn entry(status: &str, opened_at: &str) -> Value {
    let completed = status == "completed";
    json!({
        "status": status,
        "first_completed_at": if completed { json!(opened_at) } else { Value::Null },
        "last_completed_at": if completed { json!(opened_at) } else { Value::Null },
        "completion_count": if completed { 1 } else { 0 },
        "last_opened_at": opened_at,
        "last_sentence_id": null,
        "video_watched": completed,
        "last_passed_session_id": null,
    })
}

fn write_progress(data: &Path, lessons: Value) {
    write(
        data,
        "progress.json",
        &json!({ "schema_version": 1, "lessons": lessons }).to_string(),
    );
}

fn get_progress(window: &WebviewWindow<MockRuntime>) -> Value {
    invoke(window, LOCAL_URL, "get_progress", json!({})).unwrap()
}

#[test]
fn without_a_file_nothing_is_started_and_the_first_prepared_lesson_is_recommended() {
    let content = make_content();
    let data = tempfile::tempdir().unwrap();
    let (_app, window) = app_with(content.path(), data.path());
    assert_eq!(
        get_progress(&window),
        json!({ "lessons": {}, "recommended_lesson_id": "a" })
    );
    // 読むだけで、ファイルは作らない。
    assert!(!data.path().join("progress.json").exists());
}

#[test]
fn the_most_recently_opened_unfinished_lesson_is_recommended() {
    let content = make_content();
    let data = tempfile::tempdir().unwrap();
    write_progress(
        data.path(),
        json!({
            "a": entry("in_progress", "2026-10-31T10:00:00+09:00"),
            "c": entry("in_progress", "2026-10-31T12:00:00+09:00"),
        }),
    );
    let (_app, window) = app_with(content.path(), data.path());
    let response = get_progress(&window);
    assert_eq!(response["recommended_lesson_id"], "c");
    // 保存した形のまま返す（どのフィールドも省かない）。
    assert_eq!(
        response["lessons"]["a"],
        entry("in_progress", "2026-10-31T10:00:00+09:00")
    );
}

#[test]
fn unprepared_lessons_are_not_recommended() {
    let content = make_content();
    let data = tempfile::tempdir().unwrap();
    // a を完了した。次の候補は、準備中の b ではなく c。
    write_progress(
        data.path(),
        json!({ "a": entry("completed", "2026-10-31T10:00:00+09:00") }),
    );
    let (_app, window) = app_with(content.path(), data.path());
    assert_eq!(get_progress(&window)["recommended_lesson_id"], "c");
}

#[test]
fn completing_all_prepared_lessons_recommends_nothing() {
    let content = make_content();
    let data = tempfile::tempdir().unwrap();
    write_progress(
        data.path(),
        json!({
            "a": entry("completed", "2026-10-31T10:00:00+09:00"),
            "c": entry("completed", "2026-10-31T11:00:00+09:00"),
        }),
    );
    let (_app, window) = app_with(content.path(), data.path());
    // 準備中の b と d が残っていても、推奨レッスンはなし。
    assert_eq!(get_progress(&window)["recommended_lesson_id"], Value::Null);
}

#[test]
fn progress_of_lessons_not_in_chapters_json_is_not_returned() {
    let content = make_content();
    let data = tempfile::tempdir().unwrap();
    write_progress(
        data.path(),
        json!({
            "removed": entry("in_progress", "2026-10-31T23:00:00+09:00"),
            "a": entry("completed", "2026-10-31T10:00:00+09:00"),
        }),
    );
    let (_app, window) = app_with(content.path(), data.path());
    let response = get_progress(&window);
    let keys: Vec<&String> = response["lessons"].as_object().unwrap().keys().collect();
    assert_eq!(keys, ["a"]);
    // 無いレッスンの進捗は、推奨にも影響しない。
    assert_eq!(response["recommended_lesson_id"], "c");
    // ファイルからは消さない（教材が戻ったときのために残す）。
    let saved: Value =
        serde_json::from_str(&fs::read_to_string(data.path().join("progress.json")).unwrap())
            .unwrap();
    assert!(saved["lessons"].get("removed").is_some());
}

#[test]
fn an_unreadable_progress_file_is_moved_aside_and_started_over() {
    let content = make_content();
    let data = tempfile::tempdir().unwrap();
    write(data.path(), "progress.json", "{ 壊れた");
    let (_app, window) = app_with(content.path(), data.path());
    assert_eq!(
        get_progress(&window),
        json!({ "lessons": {}, "recommended_lesson_id": "a" })
    );
    let backups = fs::read_dir(data.path())
        .unwrap()
        .filter_map(Result::ok)
        .filter(|entry| {
            entry
                .file_name()
                .to_string_lossy()
                .starts_with("progress.json.broken-")
        })
        .count();
    assert_eq!(backups, 1);
}
