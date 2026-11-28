//! 教材のコマンド（spec/architecture/commands/content.md）を、モックの実行環境と、
//! 一時ディレクトリに作った教材で確かめる。

mod common;

use std::fs;
use std::path::Path;

use common::{LOCAL_URL, app, invoke};
use progrust_lib::content::{ContentDir, ContentError, Language};
use serde_json::{Value, json};
use tauri::test::MockRuntime;
use tauri::{App, Manager, WebviewWindow};

fn write(root: &Path, relative: &str, content: &str) {
    let path = root.join(relative);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, content).unwrap();
}

fn lesson_json(id: &str, kind: &str) -> String {
    json!({
        "id": id,
        "chapter": "ch01",
        "kind": kind,
        "title": format!("{id} のタイトル"),
        "summary": format!("{id} の説明"),
        "estimated_minutes": 15,
    })
    .to_string()
}

fn add_video(root: &Path, id: &str) {
    write(
        root,
        &format!("lessons/{id}/video/script.json"),
        r#"{"scenes":[{"id":"title","name":"表紙"}],"sentences":[]}"#,
    );
    // 音声や動画のコードは、返さない。
    write(root, &format!("lessons/{id}/video/voice/t1.m4a"), "audio");
}

fn add_exercise(root: &Path, id: &str) {
    let base = format!("lessons/{id}/exercise");
    write(root, &format!("{base}/problem.md"), "## 問題\nP\n");
    write(root, &format!("{base}/explanation.md"), "## 考え方\nE\n");
    write(
        root,
        &format!("{base}/starter/src/main.rs"),
        "fn main() {}\n",
    );
    write(
        root,
        &format!("{base}/starter/src/util/mod.rs"),
        "// util\n",
    );
    write(
        root,
        &format!("{base}/solution/src/main.rs"),
        "SOLUTION_CODE",
    );
    write(root, &format!("{base}/cases.json"), "CASES_JSON");
    write(root, &format!("{base}/judge_tests.rs"), "JUDGE_TESTS_RS");
}

/// ch01: 通常のレッスン（ch01-01）、動画のみ（ch01-02）、準備中（ch01-03）、章末復習（ch01-review）。
fn make_content() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    write(
        root,
        "chapters.json",
        &json!([{
            "id": "ch01",
            "title": "第1章",
            "description": "第1章の説明",
            "lessons": ["ch01-01", "ch01-02", "ch01-03", "ch01-review"],
        }])
        .to_string(),
    );
    write(
        root,
        "lessons/ch01-01/lesson.json",
        &lesson_json("ch01-01", "lesson"),
    );
    add_video(root, "ch01-01");
    add_exercise(root, "ch01-01");
    write(
        root,
        "lessons/ch01-02/lesson.json",
        &lesson_json("ch01-02", "video_only"),
    );
    add_video(root, "ch01-02");
    write(
        root,
        "lessons/ch01-review/lesson.json",
        &lesson_json("ch01-review", "chapter_review"),
    );
    add_exercise(root, "ch01-review");
    // 生成問題と同じ名前のディレクトリがあっても、開けない。
    write(
        root,
        "lessons/gen-abc/lesson.json",
        &lesson_json("gen-abc", "generated"),
    );
    dir
}

fn app_with_content(root: &Path) -> (App<MockRuntime>, WebviewWindow<MockRuntime>) {
    let (app, window) = app();
    // 教材のディレクトリは、アプリでは `resource_dir()` の下になる。テストでは一時ディレクトリに差し替える。
    app.manage(ContentDir::new(root));
    (app, window)
}

fn get_lesson(window: &WebviewWindow<MockRuntime>, lesson_id: &str) -> Result<Value, Value> {
    invoke(
        window,
        LOCAL_URL,
        "get_lesson",
        json!({ "lesson_id": lesson_id }),
    )
}

#[test]
fn list_chapters_returns_chapters_and_lessons_in_order() {
    let content = make_content();
    let (_app, window) = app_with_content(content.path());
    let response = invoke(&window, LOCAL_URL, "list_chapters", json!({})).unwrap();
    assert_eq!(response["chapters"].as_array().unwrap().len(), 1);
    let chapter = &response["chapters"][0];
    assert_eq!(chapter["id"], "ch01");
    assert_eq!(chapter["title"], "第1章");
    assert_eq!(chapter["description"], "第1章の説明");
    let ids: Vec<&str> = chapter["lessons"]
        .as_array()
        .unwrap()
        .iter()
        .map(|lesson| lesson["id"].as_str().unwrap())
        .collect();
    assert_eq!(ids, ["ch01-01", "ch01-02", "ch01-03", "ch01-review"]);
    assert_eq!(
        chapter["lessons"][0],
        json!({
            "id": "ch01-01",
            "kind": "lesson",
            "title": "ch01-01 のタイトル",
            "summary": "ch01-01 の説明",
            "estimated_minutes": 15,
            "in_preparation": false,
        })
    );
}

#[test]
fn unprepared_lessons_are_in_preparation_with_null_fields() {
    let content = make_content();
    let (_app, window) = app_with_content(content.path());
    let response = invoke(&window, LOCAL_URL, "list_chapters", json!({})).unwrap();
    assert_eq!(
        response["chapters"][0]["lessons"][2],
        json!({
            "id": "ch01-03",
            "kind": null,
            "title": null,
            "summary": null,
            "estimated_minutes": null,
            "in_preparation": true,
        })
    );
}

#[test]
fn get_lesson_of_an_unprepared_or_missing_lesson_is_not_found() {
    let content = make_content();
    let (_app, window) = app_with_content(content.path());
    for lesson_id in [
        "ch01-03",
        "ch99-01",
        "gen-abc",
        "gen-missing",
        "../chapters.json",
        "",
    ] {
        let error = get_lesson(&window, lesson_id).unwrap_err();
        assert_eq!(error["code"], "not_found", "{lesson_id:?}");
    }
}

#[test]
fn get_lesson_returns_lesson_script_voice_dir_and_exercise() {
    let content = make_content();
    let (_app, window) = app_with_content(content.path());
    let response = get_lesson(&window, "ch01-01").unwrap();
    assert_eq!(response["lesson"]["id"], "ch01-01");
    assert_eq!(response["lesson"]["kind"], "lesson");
    assert_eq!(response["script"]["scenes"][0]["id"], "title");
    assert_eq!(
        response["voice_dir"],
        json!(
            content
                .path()
                .join("lessons/ch01-01/video/voice")
                .to_str()
                .unwrap()
        )
    );
    assert_eq!(
        response["exercise"],
        json!({
            "problem_md": "## 問題\nP\n",
            "explanation_md": "## 考え方\nE\n",
            "starter_files": {
                "src/main.rs": "fn main() {}\n",
                "src/util/mod.rs": "// util\n",
            },
        })
    );
}

#[test]
fn video_only_lesson_has_no_exercise() {
    let content = make_content();
    let (_app, window) = app_with_content(content.path());
    let response = get_lesson(&window, "ch01-02").unwrap();
    assert_eq!(response["exercise"], Value::Null);
    assert_ne!(response["script"], Value::Null);
    assert_ne!(response["voice_dir"], Value::Null);
}

#[test]
fn chapter_review_lesson_has_no_script_or_voice_dir() {
    let content = make_content();
    let (_app, window) = app_with_content(content.path());
    let response = get_lesson(&window, "ch01-review").unwrap();
    assert_eq!(response["script"], Value::Null);
    assert_eq!(response["voice_dir"], Value::Null);
    assert_ne!(response["exercise"], Value::Null);
}

#[test]
fn get_lesson_does_not_return_judge_files_or_solution() {
    let content = make_content();
    let (_app, window) = app_with_content(content.path());
    for lesson_id in ["ch01-01", "ch01-review"] {
        let text = get_lesson(&window, lesson_id).unwrap().to_string();
        for secret in ["SOLUTION_CODE", "CASES_JSON", "JUDGE_TESTS_RS"] {
            assert!(!text.contains(secret), "{lesson_id} が {secret} を返した");
        }
    }
}

#[test]
fn a_lesson_directory_without_lesson_json_is_an_io_error_not_in_preparation() {
    let content = make_content();
    fs::create_dir_all(content.path().join("lessons/ch01-03")).unwrap();
    let (_app, window) = app_with_content(content.path());
    let error = invoke(&window, LOCAL_URL, "list_chapters", json!({})).unwrap_err();
    assert_eq!(error["code"], "io_error");
}

/// C の章（c01・c02）、`language` を省略した Rust の章、`language: "rust"` の Rust の章を並べた教材。
fn make_mixed_content() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    write(
        root,
        "chapters.json",
        &json!([
            { "id": "c01", "title": "C1", "description": "", "language": "c", "lessons": ["c01-01"] },
            { "id": "c02", "title": "C2", "description": "", "language": "c", "lessons": ["c02-01"] },
            { "id": "ch01", "title": "R1", "description": "", "lessons": ["ch01-01"] },
            { "id": "ch02", "title": "R2", "description": "", "language": "rust", "lessons": ["ch02-01"] },
        ])
        .to_string(),
    );
    for id in ["c01-01", "c02-01", "ch01-01", "ch02-01"] {
        write(
            root,
            &format!("lessons/{id}/lesson.json"),
            &lesson_json(id, "video_only"),
        );
        add_video(root, id);
    }
    dir
}

#[test]
fn list_chapters_returns_the_language_and_the_number_within_the_language() {
    let content = make_mixed_content();
    let (_app, window) = app_with_content(content.path());
    let response = invoke(&window, LOCAL_URL, "list_chapters", json!({})).unwrap();
    let actual: Vec<(&str, &str, u64)> = response["chapters"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| {
            (
                c["id"].as_str().unwrap(),
                c["language"].as_str().unwrap(),
                c["number"].as_u64().unwrap(),
            )
        })
        .collect();
    assert_eq!(
        actual,
        [
            ("c01", "c", 1),
            ("c02", "c", 2),
            ("ch01", "rust", 1),
            ("ch02", "rust", 2),
        ]
    );
}

#[test]
fn get_lesson_returns_the_language_and_the_chapter_number() {
    let content = make_mixed_content();
    let (_app, window) = app_with_content(content.path());
    for (lesson_id, language, number) in [
        ("c01-01", "c", 1),
        ("c02-01", "c", 2),
        ("ch01-01", "rust", 1),
        ("ch02-01", "rust", 2),
    ] {
        let response = get_lesson(&window, lesson_id).unwrap();
        assert_eq!(response["language"], language, "{lesson_id}");
        assert_eq!(response["chapter_number"], number, "{lesson_id}");
    }
}

#[test]
fn lesson_language_is_the_language_of_the_chapter() {
    let content = make_mixed_content();
    let dir = ContentDir::new(content.path());
    for (lesson_id, language) in [
        ("c01-01", Language::C),
        ("c02-01", Language::C),
        ("ch01-01", Language::Rust),
        ("ch02-01", Language::Rust),
    ] {
        assert_eq!(
            dir.lesson_language(lesson_id).unwrap(),
            language,
            "{lesson_id}"
        );
    }
    // レッスンが無いときの扱いは get_lesson と同じ。
    for lesson_id in ["none", "gen-20261031T130000", "../c01-01"] {
        assert!(
            matches!(
                dir.lesson_language(lesson_id),
                Err(ContentError::LessonNotFound { .. })
            ),
            "{lesson_id}"
        );
    }
}

#[test]
fn a_chapter_with_an_unknown_language_is_an_unreadable_content() {
    let content = make_mixed_content();
    write(
        content.path(),
        "chapters.json",
        &json!([{ "id": "x01", "title": "X", "description": "", "language": "go", "lessons": ["c01-01"] }])
            .to_string(),
    );
    let (_app, window) = app_with_content(content.path());
    let error = invoke(&window, LOCAL_URL, "list_chapters", json!({})).unwrap_err();
    assert_eq!(error["code"], "internal");
    let error = get_lesson(&window, "c01-01").unwrap_err();
    assert_eq!(error["code"], "internal");
}
