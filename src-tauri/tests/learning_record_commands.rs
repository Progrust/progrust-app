//! コマンド `get_dashboard`（spec/architecture/commands/learning-record.md）を、モックの実行環境と、
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

/// 教材: `chapter_count` 章。各章は、準備中でない c（章の番号）-1〜3 と、準備中の c（章の番号）-p。
fn make_content(chapter_count: usize) -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let mut chapters = Vec::new();
    for c in 1..=chapter_count {
        let ids: Vec<String> = ["1", "2", "3", "p"]
            .iter()
            .map(|n| format!("c{c:02}-{n}"))
            .collect();
        for id in &ids[..3] {
            write(
                root,
                &format!("lessons/{id}/lesson.json"),
                &json!({
                    "id": id, "kind": "video_only",
                    "title": format!("{id} のタイトル"), "summary": format!("{id} の概要"),
                })
                .to_string(),
            );
        }
        chapters.push(json!({
            "id": format!("ch{c:02}"), "title": format!("第{c}章"), "description": "", "lessons": ids,
        }));
    }
    write(root, "chapters.json", &Value::Array(chapters).to_string());
    dir
}

fn app_with(content: &Path, data: &Path) -> (App<MockRuntime>, WebviewWindow<MockRuntime>) {
    let (app, window) = app();
    app.manage(ContentDir::new(content));
    app.manage(AppDataDir::new(data));
    (app, window)
}

fn completed_entry(id: &str) -> (String, Value) {
    let at = "2026-10-31T10:00:00+09:00";
    (
        id.to_string(),
        json!({
            "status": "completed", "first_completed_at": at, "last_completed_at": at,
            "completion_count": 1, "last_opened_at": at, "last_sentence_id": null,
            "video_watched": true, "last_passed_session_id": null,
        }),
    )
}

fn write_completed(data: &Path, ids: &[String]) {
    let lessons: serde_json::Map<String, Value> =
        ids.iter().map(|id| completed_entry(id)).collect();
    write(
        data,
        "progress.json",
        &json!({ "schema_version": 1, "lessons": lessons }).to_string(),
    );
}

fn get_dashboard(window: &WebviewWindow<MockRuntime>) -> Value {
    invoke(window, LOCAL_URL, "get_dashboard", json!({})).unwrap()
}

fn chapter_ids(dashboard: &Value) -> Vec<String> {
    dashboard["chapters"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["id"].as_str().unwrap().to_string())
        .collect()
}

fn ids(from: usize, to: usize) -> Vec<String> {
    (from..=to).map(|i| format!("ch{i:02}")).collect()
}

#[test]
fn without_any_learning_record_the_dashboard_does_not_fail() {
    let content = make_content(3);
    let data = tempfile::tempdir().unwrap();
    let (_app, window) = app_with(content.path(), data.path());
    let dashboard = get_dashboard(&window);
    assert_eq!(
        dashboard["next_lesson"],
        json!({
            "lesson_id": "c01-1", "chapter_id": "ch01", "chapter_title": "第1章",
            "title": "c01-1 のタイトル", "summary": "c01-1 の概要", "started": false,
        })
    );
    assert_eq!(
        dashboard["lesson_counts"],
        json!({ "completed": 0, "total": 9 })
    );
    assert_eq!(chapter_ids(&dashboard), ids(1, 3));
    assert_eq!(
        dashboard["analysis"],
        json!({
            "state": "none", "weak_elements": [], "review_due_elements": [],
            "regressed_elements": [], "last_4_weeks": [], "summary_md": null,
        })
    );
    // 読むだけで、ファイルは作らない。
    assert!(!data.path().join("progress.json").exists());
}

#[test]
fn when_the_next_lesson_is_in_chapter_4_chapters_2_to_10_are_shown() {
    let content = make_content(23);
    let data = tempfile::tempdir().unwrap();
    let done: Vec<String> = (1..=3)
        .flat_map(|c| (1..=3).map(move |n| format!("c{c:02}-{n}")))
        .collect();
    write_completed(data.path(), &done);
    let (_app, window) = app_with(content.path(), data.path());
    let dashboard = get_dashboard(&window);
    assert_eq!(dashboard["next_lesson"]["chapter_id"], "ch04");
    assert_eq!(chapter_ids(&dashboard), ids(2, 10));
    assert_eq!(
        dashboard["chapters"][0],
        json!({ "id": "ch02", "title": "第2章", "completed": 3, "total": 3 })
    );
}

#[test]
fn lessons_in_preparation_are_not_counted_in_the_totals() {
    // 各章 4 レッスンのうち 1 レッスンが準備中。第1章の 3 レッスンのうち 2 つを完了した。
    let content = make_content(2);
    let data = tempfile::tempdir().unwrap();
    write_completed(data.path(), &["c01-1".to_string(), "c01-2".to_string()]);
    let (_app, window) = app_with(content.path(), data.path());
    let dashboard = get_dashboard(&window);
    assert_eq!(
        dashboard["lesson_counts"],
        json!({ "completed": 2, "total": 6 })
    );
    assert_eq!(
        dashboard["chapters"][0],
        json!({ "id": "ch01", "title": "第1章", "completed": 2, "total": 3 })
    );
    assert_eq!(dashboard["next_lesson"]["lesson_id"], "c01-3");
}

#[test]
fn when_everything_is_completed_there_is_no_next_lesson_and_the_last_chapters_are_shown() {
    let content = make_content(12);
    let data = tempfile::tempdir().unwrap();
    let done: Vec<String> = (1..=12)
        .flat_map(|c| (1..=3).map(move |n| format!("c{c:02}-{n}")))
        .collect();
    write_completed(data.path(), &done);
    let (_app, window) = app_with(content.path(), data.path());
    let dashboard = get_dashboard(&window);
    assert_eq!(dashboard["next_lesson"], Value::Null);
    assert_eq!(
        dashboard["lesson_counts"],
        json!({ "completed": 36, "total": 36 })
    );
    assert_eq!(chapter_ids(&dashboard), ids(4, 12));
}
