//! 進捗のコマンド `get_progress`（spec/architecture/commands/progress.md）を、モックの実行環境と、
//! 一時ディレクトリに作った教材・学習者データで確かめる。

mod common;

use std::fs;
use std::path::Path;

use common::{LOCAL_URL, app, invoke};
use progrust_lib::attempts::{BuildOutput, SessionKind, SubmitRecord, SubmitStatus};
use progrust_lib::content::ContentDir;
use progrust_lib::runner::runs::Runs;
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
    app.manage(Runs::default());
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

/// 教材: v（動画のみ）、l（動画と演習）、r（章末復習。動画なし）、p（準備中）。
fn make_kinds_content() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    write(
        root,
        "chapters.json",
        &json!([
            { "id": "ch01", "title": "第1章", "description": "", "lessons": ["v", "l", "r", "p"] },
        ])
        .to_string(),
    );
    for (id, kind) in [
        ("v", "video_only"),
        ("l", "lesson"),
        ("r", "chapter_review"),
    ] {
        write(
            root,
            &format!("lessons/{id}/lesson.json"),
            &json!({ "id": id, "kind": kind, "title": id }).to_string(),
        );
    }
    dir
}

fn open_lesson(window: &WebviewWindow<MockRuntime>, lesson_id: &str) -> Result<Value, Value> {
    invoke(
        window,
        LOCAL_URL,
        "open_lesson",
        json!({ "lesson_id": lesson_id }),
    )
}

fn save_video_position(
    window: &WebviewWindow<MockRuntime>,
    lesson_id: &str,
    sentence_id: &str,
    watched: bool,
) -> Result<Value, Value> {
    invoke(
        window,
        LOCAL_URL,
        "save_video_position",
        json!({ "lesson_id": lesson_id, "last_sentence_id": sentence_id, "video_watched": watched }),
    )
}

fn saved_progress(data: &Path) -> Value {
    serde_json::from_str(&fs::read_to_string(data.join("progress.json")).unwrap()).unwrap()
}

#[test]
fn open_lesson_records_the_lesson_as_in_progress_and_returns_it() {
    let content = make_kinds_content();
    let data = tempfile::tempdir().unwrap();
    let (_app, window) = app_with(content.path(), data.path());
    let response = open_lesson(&window, "l").unwrap();
    assert!(
        response["session_id"].as_str().unwrap().ends_with("_l"),
        "{response}"
    );
    assert_eq!(response["generated"], Value::Null);
    assert_eq!(response["draft"], Value::Null);
    let progress = &response["progress"];
    assert_eq!(progress["status"], "in_progress");
    assert_eq!(progress["completion_count"], 0);
    assert_eq!(progress["video_watched"], false);
    assert!(progress["last_opened_at"].as_str().unwrap().contains('T'));
    assert_eq!(saved_progress(data.path())["lessons"]["l"], *progress);
    // 章末復習レッスンも開ける。
    assert_eq!(
        open_lesson(&window, "r").unwrap()["progress"]["status"],
        "in_progress"
    );
}

#[test]
fn reopening_a_completed_lesson_keeps_it_completed() {
    let content = make_kinds_content();
    let data = tempfile::tempdir().unwrap();
    write_progress(
        data.path(),
        json!({ "v": entry("completed", "2026-10-31T10:00:00+09:00") }),
    );
    let (_app, window) = app_with(content.path(), data.path());
    let progress = open_lesson(&window, "v").unwrap()["progress"].clone();
    assert_eq!(progress["status"], "completed");
    assert_eq!(progress["first_completed_at"], "2026-10-31T10:00:00+09:00");
    assert_eq!(progress["completion_count"], 1);
    assert_ne!(progress["last_opened_at"], "2026-10-31T10:00:00+09:00");
}

#[test]
fn watching_a_video_only_lesson_to_the_end_completes_it_once() {
    let content = make_kinds_content();
    let data = tempfile::tempdir().unwrap();
    let (_app, window) = app_with(content.path(), data.path());
    open_lesson(&window, "v").unwrap();

    let midway = save_video_position(&window, "v", "m1", false).unwrap();
    assert_eq!(midway["status"], "in_progress");
    assert_eq!(midway["last_sentence_id"], "m1");
    assert_eq!(midway["completion_count"], 0);

    let done = save_video_position(&window, "v", "m9", true).unwrap();
    assert_eq!(done["status"], "completed");
    assert_eq!(done["video_watched"], true);
    assert_eq!(done["completion_count"], 1);
    assert_eq!(done["first_completed_at"], done["last_completed_at"]);
    assert!(done["first_completed_at"].is_string());
    assert_eq!(done["last_passed_session_id"], Value::Null);
    assert_eq!(saved_progress(data.path())["lessons"]["v"], done);

    // 再生し直しても、完了の項目は変わらない。`video_watched` に false を渡しても戻らない。
    let keys = [
        "status",
        "first_completed_at",
        "last_completed_at",
        "completion_count",
    ];
    let again = save_video_position(&window, "v", "m2", false).unwrap();
    assert_eq!(again["last_sentence_id"], "m2");
    assert_eq!(again["video_watched"], true);
    for key in keys {
        assert_eq!(again[key], done[key], "{key}");
    }
    let replayed = save_video_position(&window, "v", "m9", true).unwrap();
    for key in keys {
        assert_eq!(replayed[key], done[key], "{key}");
    }
}

#[test]
fn watching_a_lesson_with_an_exercise_does_not_complete_it() {
    let content = make_kinds_content();
    let data = tempfile::tempdir().unwrap();
    let (_app, window) = app_with(content.path(), data.path());
    let progress = save_video_position(&window, "l", "m9", true).unwrap();
    assert_eq!(progress["video_watched"], true);
    assert_eq!(progress["status"], "in_progress");
    assert_eq!(progress["completion_count"], 0);
    assert_eq!(progress["first_completed_at"], Value::Null);
}

#[test]
fn video_watched_does_not_go_back_to_false() {
    let content = make_kinds_content();
    let data = tempfile::tempdir().unwrap();
    let (_app, window) = app_with(content.path(), data.path());
    save_video_position(&window, "l", "m9", true).unwrap();
    let progress = save_video_position(&window, "l", "m1", false).unwrap();
    assert_eq!(progress["video_watched"], true);
}

#[test]
fn saving_a_video_position_for_a_lesson_without_video_is_invalid() {
    let content = make_kinds_content();
    let data = tempfile::tempdir().unwrap();
    let (_app, window) = app_with(content.path(), data.path());
    let error = save_video_position(&window, "r", "m1", true).unwrap_err();
    assert_eq!(error["code"], "invalid_argument");
    // 失敗したときは、進捗を書かない。
    assert!(!data.path().join("progress.json").exists());
}

#[test]
fn commands_for_a_missing_lesson_return_not_found() {
    let content = make_kinds_content();
    let data = tempfile::tempdir().unwrap();
    let (_app, window) = app_with(content.path(), data.path());
    // none は chapters.json に無い。p は準備中（ディレクトリが無い）。
    for lesson_id in ["none", "p", "gen-20261031T130000"] {
        assert_eq!(
            open_lesson(&window, lesson_id).unwrap_err()["code"],
            "not_found"
        );
        assert_eq!(
            save_video_position(&window, lesson_id, "m1", true).unwrap_err()["code"],
            "not_found"
        );
    }
    assert!(!data.path().join("progress.json").exists());
}

// ---- 試行セッション（spec/learning-record-and-analysis/attempt-log.md）----

/// `attempts/` の中のファイルの名前の一覧（並べたもの）。`attempts/` が無いときは空。
fn attempt_names(data: &Path) -> Vec<String> {
    let Ok(entries) = fs::read_dir(data.join("attempts")) else {
        return Vec::new();
    };
    let mut names: Vec<_> = entries
        .map(|entry| entry.unwrap().file_name().into_string().unwrap())
        .collect();
    names.sort();
    names
}

fn saved_attempt(data: &Path, session_id: &str) -> Value {
    serde_json::from_str(
        &fs::read_to_string(data.join("attempts").join(format!("{session_id}.json"))).unwrap(),
    )
    .unwrap()
}

fn write_attempt(data: &Path, session_id: &str, lesson_id: &str, ended_at: Value) {
    write(
        data,
        &format!("attempts/{session_id}.json"),
        &json!({
            "schema_version": 1,
            "session_id": session_id,
            "lesson_id": lesson_id,
            "kind": "lesson",
            "difficulty": null,
            "started_at": "2026-10-30T09:00:00+09:00",
            "passed_at": if ended_at.is_null() { Value::Null } else { ended_at.clone() },
            "ended_at": ended_at,
            "events": [{
                "type": "run", "at": "2026-10-30T09:05:00+09:00", "mode": "main",
                "files": { "src/main.rs": "fn main() {}\n" },
                "result": { "status": "success", "error_codes": [], "stdout": "", "stderr": "",
                            "exit_code": 0, "stdin": "" },
            }],
        })
        .to_string(),
    );
}

#[test]
fn opening_a_lesson_starts_an_attempt_session() {
    let content = make_kinds_content();
    let data = tempfile::tempdir().unwrap();
    let (_app, window) = app_with(content.path(), data.path());
    for (lesson_id, kind) in [("l", "lesson"), ("r", "chapter_review")] {
        let session_id = open_lesson(&window, lesson_id).unwrap()["session_id"]
            .as_str()
            .unwrap()
            .to_string();
        let saved = saved_attempt(data.path(), &session_id);
        assert_eq!(saved["schema_version"], 1);
        assert_eq!(saved["session_id"], session_id);
        assert_eq!(saved["lesson_id"], lesson_id);
        assert_eq!(saved["kind"], kind);
        assert_eq!(saved["difficulty"], Value::Null);
        assert_eq!(saved["passed_at"], Value::Null);
        assert_eq!(saved["ended_at"], Value::Null);
        assert_eq!(saved["events"], json!([]));
        // ID は `{開始日時 YYYYMMDDTHHMMSS}_{lesson_id}`。
        let started_at =
            chrono::DateTime::parse_from_rfc3339(saved["started_at"].as_str().unwrap()).unwrap();
        assert_eq!(
            session_id,
            format!("{}_{lesson_id}", started_at.format("%Y%m%dT%H%M%S"))
        );
    }
}

#[test]
fn a_video_only_lesson_opened_and_completed_leaves_no_attempt_log() {
    let content = make_kinds_content();
    let data = tempfile::tempdir().unwrap();
    let (_app, window) = app_with(content.path(), data.path());
    assert_eq!(
        open_lesson(&window, "v").unwrap()["session_id"],
        Value::Null
    );
    let done = save_video_position(&window, "v", "m9", true).unwrap();
    assert_eq!(done["status"], "completed");
    assert_eq!(
        open_lesson(&window, "v").unwrap()["session_id"],
        Value::Null
    );
    assert!(!data.path().join("attempts").exists());
    assert!(!data.path().join("records").exists());
}

#[test]
fn reopening_a_lesson_with_an_unfinished_session_continues_it() {
    let content = make_kinds_content();
    let data = tempfile::tempdir().unwrap();
    // 前に開いて実行した、終わっていないセッション。
    write_attempt(data.path(), "20261030T090000_l", "l", Value::Null);
    let before = saved_attempt(data.path(), "20261030T090000_l");

    let (_app, window) = app_with(content.path(), data.path());
    assert_eq!(
        open_lesson(&window, "l").unwrap()["session_id"],
        "20261030T090000_l"
    );
    // アプリを起動し直しても続ける。
    let (_app, window) = app_with(content.path(), data.path());
    assert_eq!(
        open_lesson(&window, "l").unwrap()["session_id"],
        "20261030T090000_l"
    );
    assert_eq!(saved_attempt(data.path(), "20261030T090000_l"), before);
    assert_eq!(attempt_names(data.path()), ["20261030T090000_l.json"]);
}

#[test]
fn reopening_a_lesson_whose_sessions_all_ended_starts_a_new_one() {
    let content = make_kinds_content();
    let data = tempfile::tempdir().unwrap();
    write_attempt(
        data.path(),
        "20261030T090000_l",
        "l",
        json!("2026-10-30T09:30:00+09:00"),
    );
    let before = saved_attempt(data.path(), "20261030T090000_l");
    let (_app, window) = app_with(content.path(), data.path());
    let session_id = open_lesson(&window, "l").unwrap()["session_id"]
        .as_str()
        .unwrap()
        .to_string();
    assert_ne!(session_id, "20261030T090000_l");
    assert_eq!(saved_attempt(data.path(), &session_id)["events"], json!([]));
    // 終わったセッションの試行ログは、そのまま残す。
    assert_eq!(saved_attempt(data.path(), "20261030T090000_l"), before);
    assert_eq!(attempt_names(data.path()).len(), 2);
    // 次に開いたときは、新しいほうを続ける。
    assert_eq!(open_lesson(&window, "l").unwrap()["session_id"], session_id);
}

#[test]
fn a_broken_attempt_log_does_not_fail_open_lesson_and_a_new_session_starts() {
    let content = make_kinds_content();
    for broken in [
        "{ 壊れた".to_string(),
        json!({ "schema_version": 2, "session_id": "20261030T090000_l" }).to_string(),
        json!({ "session_id": "20261030T090000_l", "lesson_id": "l" }).to_string(),
    ] {
        let data = tempfile::tempdir().unwrap();
        write(data.path(), "attempts/20261030T090000_l.json", &broken);
        let (_app, window) = app_with(content.path(), data.path());
        let session_id = open_lesson(&window, "l").unwrap()["session_id"]
            .as_str()
            .unwrap()
            .to_string();
        assert_ne!(session_id, "20261030T090000_l", "{broken}");
        let names = attempt_names(data.path());
        assert_eq!(names.len(), 2, "{broken}: {names:?}");
        assert!(
            names[0].starts_with("20261030T090000_l.json.broken-"),
            "{names:?}"
        );
        assert_eq!(names[1], format!("{session_id}.json"));
        // 退避したファイルの中身は、そのまま残る。
        assert_eq!(
            fs::read_to_string(data.path().join("attempts").join(&names[0])).unwrap(),
            broken
        );
    }
}

// ---- 下書き（spec/lesson-flow/drafts.md）----

/// 教材: e（動画と演習。初期コードは 2 ファイル）、v（動画のみ）。
fn make_exercise_content() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    write(
        root,
        "chapters.json",
        &json!([
            { "id": "ch01", "title": "第1章", "description": "", "lessons": ["e", "v"] },
        ])
        .to_string(),
    );
    for (id, kind) in [("e", "lesson"), ("v", "video_only")] {
        write(
            root,
            &format!("lessons/{id}/lesson.json"),
            &json!({ "id": id, "kind": kind, "title": id }).to_string(),
        );
    }
    write(root, "lessons/e/exercise/starter/src/main.rs", STARTER_MAIN);
    write(
        root,
        "lessons/e/exercise/starter/src/shapes/circle.rs",
        STARTER_CIRCLE,
    );
    dir
}

const STARTER_MAIN: &str = "fn main() {\n    // ここに書く\n}\n";
const STARTER_CIRCLE: &str = "pub struct Circle;\n";

fn starter_files() -> Value {
    json!({ "src/main.rs": STARTER_MAIN, "src/shapes/circle.rs": STARTER_CIRCLE })
}

fn save_draft(
    window: &WebviewWindow<MockRuntime>,
    lesson_id: &str,
    files: Value,
) -> Result<Value, Value> {
    invoke(
        window,
        LOCAL_URL,
        "save_draft",
        json!({ "lesson_id": lesson_id, "files": files }),
    )
}

fn reset_draft(window: &WebviewWindow<MockRuntime>, lesson_id: &str) -> Result<Value, Value> {
    invoke(
        window,
        LOCAL_URL,
        "reset_draft",
        json!({ "lesson_id": lesson_id }),
    )
}

fn draft_path(data: &Path, lesson_id: &str) -> std::path::PathBuf {
    data.join("drafts").join(format!("{lesson_id}.json"))
}

/// `drafts/` の中のファイルの名前の一覧（並べたもの）。`drafts/` が無いときは空。
fn draft_dir_names(data: &Path) -> Vec<String> {
    let Ok(entries) = fs::read_dir(data.join("drafts")) else {
        return Vec::new();
    };
    let mut names: Vec<_> = entries
        .map(|entry| entry.unwrap().file_name().into_string().unwrap())
        .collect();
    names.sort();
    names
}

#[test]
fn without_a_draft_open_lesson_returns_null() {
    let content = make_exercise_content();
    let data = tempfile::tempdir().unwrap();
    let (_app, window) = app_with(content.path(), data.path());
    assert_eq!(open_lesson(&window, "e").unwrap()["draft"], Value::Null);
    // 読むだけで、下書きは作らない。
    assert!(!data.path().join("drafts").exists());
}

#[test]
fn a_saved_draft_is_returned_by_open_lesson() {
    let content = make_exercise_content();
    let data = tempfile::tempdir().unwrap();
    let (_app, window) = app_with(content.path(), data.path());
    let files = json!({
        "src/main.rs": "fn main() {\n    println!(\"途中\");\n}\n",
        "src/shapes/circle.rs": "pub struct Circle { r: f64 }\n",
        "tests/added.rs": "#[test]\nfn t() {}\n",
    });
    assert_eq!(
        save_draft(&window, "e", files.clone()).unwrap(),
        Value::Null
    );

    let draft = open_lesson(&window, "e").unwrap()["draft"].clone();
    assert_eq!(draft["schema_version"], 1);
    assert_eq!(draft["lesson_id"], "e");
    assert_eq!(draft["files"], files);
    // `updated_at` はバックエンドが付ける（時差を付けた日時）。
    let updated_at = draft["updated_at"].as_str().unwrap();
    assert!(
        chrono::DateTime::parse_from_rfc3339(updated_at).is_ok(),
        "{updated_at}"
    );
    // ファイルにも、同じ形で保存している。
    let saved: Value =
        serde_json::from_str(&fs::read_to_string(draft_path(data.path(), "e")).unwrap()).unwrap();
    assert_eq!(saved, draft);

    // もう一度保存すると、全ファイルを置き換える（消したファイルは残らない）。
    let fewer = json!({ "src/main.rs": "fn main() {}\n" });
    save_draft(&window, "e", fewer.clone()).unwrap();
    assert_eq!(open_lesson(&window, "e").unwrap()["draft"]["files"], fewer);
}

#[test]
fn reset_draft_overwrites_the_draft_with_the_starter_code() {
    let content = make_exercise_content();
    let data = tempfile::tempdir().unwrap();
    let (_app, window) = app_with(content.path(), data.path());
    save_draft(
        &window,
        "e",
        json!({ "src/main.rs": "// 書きかけ", "src/extra.rs": "// 足したファイル" }),
    )
    .unwrap();

    let reset = reset_draft(&window, "e").unwrap();
    assert_eq!(reset["schema_version"], 1);
    assert_eq!(reset["lesson_id"], "e");
    assert_eq!(reset["files"], starter_files());
    assert!(reset["updated_at"].is_string());

    let saved: Value =
        serde_json::from_str(&fs::read_to_string(draft_path(data.path(), "e")).unwrap()).unwrap();
    assert_eq!(saved, reset);
    assert_eq!(open_lesson(&window, "e").unwrap()["draft"], reset);
}

#[test]
fn reset_draft_without_a_draft_creates_one_from_the_starter_code() {
    let content = make_exercise_content();
    let data = tempfile::tempdir().unwrap();
    let (_app, window) = app_with(content.path(), data.path());
    let reset = reset_draft(&window, "e").unwrap();
    assert_eq!(reset["files"], starter_files());
    assert_eq!(open_lesson(&window, "e").unwrap()["draft"], reset);
}

#[test]
fn an_unreadable_draft_is_moved_aside_and_open_lesson_returns_null() {
    let content = make_exercise_content();
    let valid = json!({
        "schema_version": 1,
        "lesson_id": "e",
        "updated_at": "2026-10-31T13:10:00+09:00",
        "files": { "src/main.rs": "fn main() {}\n" },
    });
    let mut without_version = valid.clone();
    without_version
        .as_object_mut()
        .unwrap()
        .remove("schema_version");
    let mut newer_version = valid.clone();
    newer_version["schema_version"] = json!(2);
    let mut wrong_shape = valid.clone();
    wrong_shape["files"] = json!(["src/main.rs"]);

    for broken in [
        b"{ \xe5\xa3\x8a".to_vec(),
        // UTF-8 でない。
        b"{\"schema_version\": 1, \"\xff\xfe\": 1}".to_vec(),
        without_version.to_string().into_bytes(),
        newer_version.to_string().into_bytes(),
        wrong_shape.to_string().into_bytes(),
    ] {
        let label = String::from_utf8_lossy(&broken).into_owned();
        let data = tempfile::tempdir().unwrap();
        fs::create_dir_all(data.path().join("drafts")).unwrap();
        fs::write(draft_path(data.path(), "e"), &broken).unwrap();
        let (_app, window) = app_with(content.path(), data.path());

        assert_eq!(
            open_lesson(&window, "e").unwrap()["draft"],
            Value::Null,
            "{label}"
        );
        // 退避し、作り直さない。
        assert!(!draft_path(data.path(), "e").exists(), "{label}");
        let names = draft_dir_names(data.path());
        assert_eq!(names.len(), 1, "{label}: {names:?}");
        assert!(names[0].starts_with("e.json.broken-"), "{names:?}");
        assert_eq!(
            fs::read(data.path().join("drafts").join(&names[0])).unwrap(),
            broken
        );

        // 次に開いても、退避したファイルはそのまま残る。
        assert_eq!(open_lesson(&window, "e").unwrap()["draft"], Value::Null);
        assert_eq!(draft_dir_names(data.path()), names);
    }
}

#[test]
fn save_draft_with_a_parent_path_is_invalid_and_writes_nothing() {
    let content = make_exercise_content();
    let data = tempfile::tempdir().unwrap();
    let (_app, window) = app_with(content.path(), data.path());

    // 下書きが無いとき: 何も作らない。
    for path in ["../x.rs", "src/../../x.rs", "tests/../../../x.rs"] {
        let error = save_draft(
            &window,
            "e",
            json!({ "src/main.rs": "fn main() {}\n", path: "// 外" }),
        )
        .unwrap_err();
        assert_eq!(error["code"], "invalid_argument", "{path}");
    }
    assert!(!data.path().join("drafts").exists());
    assert!(!data.path().join("x.rs").exists());

    // 下書きがあるとき: 前の下書きを変えない。
    save_draft(&window, "e", json!({ "src/main.rs": "// 前の下書き" })).unwrap();
    let before = fs::read(draft_path(data.path(), "e")).unwrap();
    let error = save_draft(
        &window,
        "e",
        json!({ "src/main.rs": "// 新しい", "src/../../x.rs": "// 外" }),
    )
    .unwrap_err();
    assert_eq!(error["code"], "invalid_argument");
    assert_eq!(fs::read(draft_path(data.path(), "e")).unwrap(), before);
    assert_eq!(draft_dir_names(data.path()), ["e.json"]);
}

#[test]
fn drafts_of_a_lesson_without_an_exercise() {
    let content = make_exercise_content();
    let data = tempfile::tempdir().unwrap();
    let (_app, window) = app_with(content.path(), data.path());
    // 演習の無いレッスンでは、下書きのファイルがあっても読まない。
    write(
        data.path(),
        "drafts/v.json",
        &json!({
            "schema_version": 1,
            "lesson_id": "v",
            "updated_at": "2026-10-31T13:10:00+09:00",
            "files": { "src/main.rs": "fn main() {}\n" },
        })
        .to_string(),
    );
    assert_eq!(open_lesson(&window, "v").unwrap()["draft"], Value::Null);

    let files = json!({ "src/main.rs": "fn main() {}\n" });
    assert_eq!(
        save_draft(&window, "v", files).unwrap_err()["code"],
        "invalid_argument"
    );
    assert_eq!(
        reset_draft(&window, "v").unwrap_err()["code"],
        "invalid_argument"
    );
    assert_eq!(draft_dir_names(data.path()), ["v.json"]);
}

#[test]
fn draft_commands_for_a_missing_lesson_return_not_found() {
    let content = make_exercise_content();
    let data = tempfile::tempdir().unwrap();
    let (_app, window) = app_with(content.path(), data.path());
    for lesson_id in ["none", "gen-20261031T130000", "../e"] {
        let files = json!({ "src/main.rs": "fn main() {}\n" });
        assert_eq!(
            save_draft(&window, lesson_id, files).unwrap_err()["code"],
            "not_found",
            "{lesson_id}"
        );
        assert_eq!(
            reset_draft(&window, lesson_id).unwrap_err()["code"],
            "not_found",
            "{lesson_id}"
        );
    }
    assert!(!data.path().join("drafts").exists());
}

// ---- 正解と試行セッションの終了、前回の正解コード（spec/lesson-flow/completion.md、drafts.md）----

fn end_session(window: &WebviewWindow<MockRuntime>, lesson_id: &str) -> Result<Value, Value> {
    invoke(
        window,
        LOCAL_URL,
        "end_session",
        json!({ "lesson_id": lesson_id }),
    )
}

fn get_passed_code(window: &WebviewWindow<MockRuntime>, lesson_id: &str) -> Result<Value, Value> {
    invoke(
        window,
        LOCAL_URL,
        "get_passed_code",
        json!({ "lesson_id": lesson_id }),
    )
}

fn one_file(content: &str) -> std::collections::BTreeMap<String, String> {
    std::collections::BTreeMap::from([("src/main.rs".to_string(), content.to_string())])
}

/// 提出の判定が終わったときに提出のコマンドがすることを、部品で行う: `submit` イベントを追記し、正解なら正解を記録する。
/// 追記したセッションの ID と、正解のときは更新した後の進捗を返す。
fn submit(data: &Path, lesson_id: &str, code: &str, at: &str, passed: bool) -> (String, Value) {
    let dir = AppDataDir::new(data);
    let session_id = dir
        .append_submit(
            lesson_id,
            SessionKind::Lesson,
            SubmitRecord {
                at: at.into(),
                files: one_file(code),
                status: if passed {
                    SubmitStatus::Passed
                } else {
                    SubmitStatus::Wrong
                },
                error_codes: Vec::new(),
                passed: if passed { 3 } else { 1 },
                total: 3,
                failed_cases: Vec::new(),
                build: BuildOutput::default(),
                program_stderr: String::new(),
            },
        )
        .unwrap();
    if !passed {
        return (session_id, Value::Null);
    }
    let progress = dir
        .record_pass(lesson_id, &session_id, at)
        .unwrap()
        .expect("追記した試行ログがある");
    (session_id, serde_json::to_value(progress).unwrap())
}

/// 正解したが終わっていないセッション（`passed_at` があり `ended_at` が無い）の試行ログを書く。
fn write_passed_attempt(data: &Path, session_id: &str, lesson_id: &str, code: &str) {
    write(
        data,
        &format!("attempts/{session_id}.json"),
        &json!({
            "schema_version": 1,
            "session_id": session_id,
            "lesson_id": lesson_id,
            "kind": "lesson",
            "difficulty": null,
            "started_at": "2026-10-30T09:00:00+09:00",
            "passed_at": "2026-10-30T09:20:00+09:00",
            "ended_at": null,
            "events": [{
                "type": "submit", "at": "2026-10-30T09:20:00+09:00",
                "files": { "src/main.rs": code },
                "result": { "status": "passed", "error_codes": [], "passed": 3, "total": 3,
                            "failed_cases": [], "stderr": "" },
            }],
        })
        .to_string(),
    );
}

#[test]
fn after_passing_and_ending_the_session_the_lesson_starts_from_the_starter_code() {
    let content = make_exercise_content();
    let data = tempfile::tempdir().unwrap();
    let (_app, window) = app_with(content.path(), data.path());
    let opened = open_lesson(&window, "e").unwrap();
    let first_session = opened["session_id"].as_str().unwrap().to_string();

    const WRONG: &str = "fn main() { /* 途中 */ }\n";
    const RIGHT: &str = "fn main() { println!(\"正解\"); }\n";
    save_draft(&window, "e", json!({ "src/main.rs": WRONG })).unwrap();
    submit(data.path(), "e", WRONG, "2026-10-31T13:10:00+09:00", false);
    save_draft(&window, "e", json!({ "src/main.rs": RIGHT })).unwrap();
    let (session_id, progress) = submit(data.path(), "e", RIGHT, "2026-10-31T13:25:00+09:00", true);
    assert_eq!(session_id, first_session);

    // 正解を記録すると、試行ログに `passed_at` が入り、レッスンが完了になる。
    let attempt = saved_attempt(data.path(), &session_id);
    assert_eq!(attempt["passed_at"], "2026-10-31T13:25:00+09:00");
    assert_eq!(attempt["ended_at"], Value::Null);
    assert_eq!(progress["status"], "completed");
    assert_eq!(progress["first_completed_at"], "2026-10-31T13:25:00+09:00");
    assert_eq!(progress["last_completed_at"], "2026-10-31T13:25:00+09:00");
    assert_eq!(progress["completion_count"], 1);
    assert_eq!(progress["last_passed_session_id"], json!(session_id));
    assert_eq!(saved_progress(data.path())["lessons"]["e"], progress);
    // 終えるまでは、下書きは残っている。
    assert!(draft_path(data.path(), "e").exists());

    assert_eq!(end_session(&window, "e").unwrap(), Value::Null);
    let attempt = saved_attempt(data.path(), &session_id);
    assert!(
        chrono::DateTime::parse_from_rfc3339(attempt["ended_at"].as_str().unwrap()).is_ok(),
        "{attempt}"
    );
    assert_eq!(attempt["passed_at"], "2026-10-31T13:25:00+09:00");
    assert!(!draft_path(data.path(), "e").exists());

    // 開き直すと、新しいセッションで、下書きは無い（初期コードから始まる）。
    let reopened = open_lesson(&window, "e").unwrap();
    assert_ne!(reopened["session_id"], json!(session_id));
    assert_eq!(reopened["draft"], Value::Null);
    assert_eq!(reopened["progress"]["status"], "completed");

    // 「前回の正解コードを見る」では、正解したときのコードが返る。
    assert_eq!(
        get_passed_code(&window, "e").unwrap(),
        json!({
            "session_id": session_id,
            "submitted_at": "2026-10-31T13:25:00+09:00",
            "files": { "src/main.rs": RIGHT },
        })
    );

    // もう一度呼んでも、終えたセッションは変わらない。
    let ended = saved_attempt(data.path(), &session_id);
    assert_eq!(end_session(&window, "e").unwrap(), Value::Null);
    assert_eq!(saved_attempt(data.path(), &session_id), ended);
}

#[test]
fn end_session_does_nothing_for_a_session_without_passed_at() {
    let content = make_exercise_content();
    let data = tempfile::tempdir().unwrap();
    let (_app, window) = app_with(content.path(), data.path());
    let session_id = open_lesson(&window, "e").unwrap()["session_id"]
        .as_str()
        .unwrap()
        .to_string();
    save_draft(&window, "e", json!({ "src/main.rs": "// 途中" })).unwrap();
    submit(
        data.path(),
        "e",
        "// 途中",
        "2026-10-31T13:10:00+09:00",
        false,
    );
    let attempt_path = data
        .path()
        .join("attempts")
        .join(format!("{session_id}.json"));
    let attempt_before = fs::read(&attempt_path).unwrap();
    let draft_before = fs::read(draft_path(data.path(), "e")).unwrap();
    let progress_before = fs::read(data.path().join("progress.json")).unwrap();

    assert_eq!(end_session(&window, "e").unwrap(), Value::Null);
    assert_eq!(fs::read(&attempt_path).unwrap(), attempt_before);
    assert_eq!(
        fs::read(draft_path(data.path(), "e")).unwrap(),
        draft_before
    );
    assert_eq!(
        fs::read(data.path().join("progress.json")).unwrap(),
        progress_before
    );
    // 次に開くと、同じセッションを続ける。
    assert_eq!(open_lesson(&window, "e").unwrap()["session_id"], session_id);

    // セッションが無いレッスン（動画のみのレッスン）でも、何もしない。
    assert_eq!(end_session(&window, "v").unwrap(), Value::Null);
    assert_eq!(attempt_names(data.path()), [format!("{session_id}.json")]);
}

#[test]
fn restarting_the_app_ends_a_passed_session_that_was_not_ended() {
    let content = make_exercise_content();
    let data = tempfile::tempdir().unwrap();
    // 正解した後に、end_session を呼ばずにアプリが止まった。
    write_passed_attempt(data.path(), "20261030T090000_e", "e", "fn main() {}\n");
    save_draft_file(data.path(), "e");
    // 正解していない、ほかのレッスンのセッションと下書き。
    write_attempt(data.path(), "20261030T100000_v", "v", Value::Null);
    save_draft_file(data.path(), "v");
    let unpassed_before = fs::read(data.path().join("attempts/20261030T100000_v.json")).unwrap();

    // アプリの起動のときにすること。
    let ended = AppDataDir::new(data.path()).end_all_passed_sessions();
    assert!(ended.errors.is_empty(), "{:?}", ended.errors);
    assert_eq!(ended.ended, ["20261030T090000_e"]);

    let attempt = saved_attempt(data.path(), "20261030T090000_e");
    assert!(
        chrono::DateTime::parse_from_rfc3339(attempt["ended_at"].as_str().unwrap()).is_ok(),
        "{attempt}"
    );
    assert_eq!(attempt["passed_at"], "2026-10-30T09:20:00+09:00");
    assert!(!draft_path(data.path(), "e").exists());
    // 正解していないセッションと、その下書きは、そのまま残る。
    assert_eq!(
        fs::read(data.path().join("attempts/20261030T100000_v.json")).unwrap(),
        unpassed_before
    );
    assert!(draft_path(data.path(), "v").exists());

    // 次に開くと、新しいセッションが始まり、初期コードから始まる。
    let (_app, window) = app_with(content.path(), data.path());
    let reopened = open_lesson(&window, "e").unwrap();
    assert_ne!(reopened["session_id"], "20261030T090000_e");
    assert_eq!(reopened["draft"], Value::Null);

    // もう一度起動しても、終えたセッションは変わらない。
    let again = AppDataDir::new(data.path()).end_all_passed_sessions();
    assert!(again.ended.is_empty() && again.errors.is_empty());
    assert_eq!(saved_attempt(data.path(), "20261030T090000_e"), attempt);
}

fn save_draft_file(data: &Path, lesson_id: &str) {
    write(
        data,
        &format!("drafts/{lesson_id}.json"),
        &json!({
            "schema_version": 1,
            "lesson_id": lesson_id,
            "updated_at": "2026-10-30T09:10:00+09:00",
            "files": { "src/main.rs": "// 下書き" },
        })
        .to_string(),
    );
}

#[test]
fn passing_a_completed_lesson_again_counts_up_and_keeps_first_completed_at() {
    let content = make_exercise_content();
    let data = tempfile::tempdir().unwrap();
    let (_app, window) = app_with(content.path(), data.path());
    open_lesson(&window, "e").unwrap();
    let (first, done) = submit(
        data.path(),
        "e",
        "// 1 回目",
        "2026-10-31T13:25:00+09:00",
        true,
    );
    end_session(&window, "e").unwrap();

    // やり直す（新しいセッション）。途中でやめても、完了のまま。
    let redo = open_lesson(&window, "e").unwrap();
    assert_ne!(redo["session_id"], json!(first));
    assert_eq!(redo["progress"]["status"], "completed");
    assert_eq!(redo["progress"]["completion_count"], 1);

    let (second, again) = submit(
        data.path(),
        "e",
        "// 2 回目",
        "2026-11-01T09:30:00+09:00",
        true,
    );
    assert_ne!(second, first);
    assert_eq!(again["status"], "completed");
    assert_eq!(again["completion_count"], 2);
    assert_eq!(again["first_completed_at"], done["first_completed_at"]);
    assert_eq!(again["last_completed_at"], "2026-11-01T09:30:00+09:00");
    assert_eq!(again["last_passed_session_id"], json!(second));
    end_session(&window, "e").unwrap();

    // 前回の正解コードは、最後に正解したセッションのもの。
    let passed = get_passed_code(&window, "e").unwrap();
    assert_eq!(passed["session_id"], json!(second));
    assert_eq!(passed["files"], json!({ "src/main.rs": "// 2 回目" }));
}

#[test]
fn passing_twice_in_one_session_keeps_the_first_passed_at_and_code() {
    let content = make_exercise_content();
    let data = tempfile::tempdir().unwrap();
    let (_app, window) = app_with(content.path(), data.path());
    open_lesson(&window, "e").unwrap();
    let (session_id, _) = submit(
        data.path(),
        "e",
        "// 最初",
        "2026-10-31T13:25:00+09:00",
        true,
    );
    // 解説表示の状態では再び提出できないので、ふつうは起きない。起きたときも、正解した回数として数える。
    let (same, progress) = submit(data.path(), "e", "// 後", "2026-10-31T13:28:00+09:00", true);
    assert_eq!(same, session_id);
    assert_eq!(progress["completion_count"], 2);
    assert_eq!(progress["first_completed_at"], "2026-10-31T13:25:00+09:00");
    assert_eq!(progress["last_completed_at"], "2026-10-31T13:28:00+09:00");
    assert_eq!(progress["last_passed_session_id"], json!(session_id));
    assert_eq!(
        saved_attempt(data.path(), &session_id)["passed_at"],
        "2026-10-31T13:25:00+09:00"
    );
    let passed = get_passed_code(&window, "e").unwrap();
    assert_eq!(passed["submitted_at"], "2026-10-31T13:25:00+09:00");
    assert_eq!(passed["files"], json!({ "src/main.rs": "// 最初" }));
}

#[test]
fn recording_a_pass_without_the_attempt_log_writes_nothing() {
    let data = tempfile::tempdir().unwrap();
    let dir = AppDataDir::new(data.path());
    assert_eq!(
        dir.record_pass("e", "20261031T130000_e", "2026-10-31T13:25:00+09:00")
            .unwrap(),
        None
    );
    assert!(!data.path().join("progress.json").exists());
    assert!(!data.path().join("attempts").exists());
}

#[test]
fn get_passed_code_without_a_passed_submit_is_not_found() {
    let content = make_exercise_content();
    let data = tempfile::tempdir().unwrap();
    let (_app, window) = app_with(content.path(), data.path());

    // 進捗が無い。
    assert_eq!(
        get_passed_code(&window, "e").unwrap_err()["code"],
        "not_found"
    );
    // 開いただけで、正解していない（`last_passed_session_id` が null）。
    open_lesson(&window, "e").unwrap();
    assert_eq!(
        get_passed_code(&window, "e").unwrap_err()["code"],
        "not_found"
    );

    let mut progress = saved_progress(data.path());
    let mut entry = progress["lessons"]["e"].clone();
    entry["status"] = json!("completed");
    entry["last_passed_session_id"] = json!("20261030T090000_e");
    progress["lessons"]["e"] = entry;
    write(data.path(), "progress.json", &progress.to_string());
    // `last_passed_session_id` の試行ログが無い。
    assert_eq!(
        get_passed_code(&window, "e").unwrap_err()["code"],
        "not_found"
    );
    // 試行ログに、正解した `submit` イベントが無い。
    write_attempt(
        data.path(),
        "20261030T090000_e",
        "e",
        json!("2026-10-30T09:30:00+09:00"),
    );
    assert_eq!(
        get_passed_code(&window, "e").unwrap_err()["code"],
        "not_found"
    );
}

#[test]
fn completion_commands_for_a_missing_lesson_return_not_found() {
    let content = make_exercise_content();
    let data = tempfile::tempdir().unwrap();
    let (_app, window) = app_with(content.path(), data.path());
    for lesson_id in ["none", "gen-20261031T130000", "../e"] {
        assert_eq!(
            end_session(&window, lesson_id).unwrap_err()["code"],
            "not_found",
            "{lesson_id}"
        );
        assert_eq!(
            get_passed_code(&window, lesson_id).unwrap_err()["code"],
            "not_found",
            "{lesson_id}"
        );
    }
}

// ---- C の演習の下書き（spec/lesson-flow/drafts.md「C の演習」） ----

const C_STARTER: &str = "int main(void) {\n    // ここに書く\n    return 0;\n}\n";

/// C の章 c01 に、C の演習 ce がある教材。
fn make_c_exercise_content() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    write(
        root,
        "chapters.json",
        &json!([
            { "id": "c01", "title": "C の第1章", "description": "", "language": "c", "lessons": ["ce"] },
        ])
        .to_string(),
    );
    write(
        root,
        "lessons/ce/lesson.json",
        &json!({ "id": "ce", "kind": "lesson", "title": "ce" }).to_string(),
    );
    write(root, "lessons/ce/exercise/starter/main.c", C_STARTER);
    dir
}

/// 強制終了しても、保存した `main.c` の下書きを、開き直したときに読み込める。
#[test]
fn a_c_draft_of_main_c_is_saved_and_returned_by_open_lesson() {
    let content = make_c_exercise_content();
    let data = tempfile::tempdir().unwrap();
    let files = json!({ "main.c": "int main(void) {\n    return 1;\n}\n" });
    {
        let (_app, window) = app_with(content.path(), data.path());
        open_lesson(&window, "ce").unwrap();
        assert_eq!(
            save_draft(&window, "ce", files.clone()).unwrap(),
            Value::Null
        );
    }
    // アプリを作り直す（開き直す）と、保存した下書きが返る。
    let (_app, window) = app_with(content.path(), data.path());
    let draft = open_lesson(&window, "ce").unwrap()["draft"].clone();
    assert_eq!(draft["schema_version"], 1);
    assert_eq!(draft["lesson_id"], "ce");
    assert_eq!(draft["files"], files);

    // 「初期コードに戻す」は、初期コードの main.c で上書きする。
    assert_eq!(
        reset_draft(&window, "ce").unwrap()["files"],
        json!({ "main.c": C_STARTER })
    );
}

#[test]
fn save_draft_of_a_c_exercise_rejects_paths_other_than_main_c_and_writes_nothing() {
    let content = make_c_exercise_content();
    let data = tempfile::tempdir().unwrap();
    let (_app, window) = app_with(content.path(), data.path());
    for files in [
        json!({ "main.c": "", "../x.c": "// 外" }),
        json!({ "main.c": "", "/tmp/progrust-draft-test.c": "// 外" }),
        json!({ "main.c": "", "main.h": "" }),
        json!({ "src/main.rs": "fn main() {}\n" }),
        json!({}),
    ] {
        assert_eq!(
            save_draft(&window, "ce", files.clone()).unwrap_err()["code"],
            "invalid_argument",
            "{files}"
        );
    }
    assert!(!data.path().join("drafts").exists());
    assert!(!data.path().join("x.c").exists());
    assert!(!Path::new("/tmp/progrust-draft-test.c").exists());

    // Rust の演習は、これまでどおり main.c を受け付けない。
    let rust = make_exercise_content();
    let (_app, window) = app_with(rust.path(), data.path());
    assert_eq!(
        save_draft(&window, "e", json!({ "main.c": "" })).unwrap_err()["code"],
        "invalid_argument"
    );
    assert!(!data.path().join("drafts").exists());
}

/// 正解した後に開くと初期コードから始まり、「前回の正解コードを見る」で正解時の main.c が返る。
#[test]
fn after_passing_a_c_exercise_the_passed_main_c_is_returned() {
    let content = make_c_exercise_content();
    let data = tempfile::tempdir().unwrap();
    let (_app, window) = app_with(content.path(), data.path());
    open_lesson(&window, "ce").unwrap();
    const RIGHT: &str =
        "#include <stdio.h>\nint main(void) {\n    puts(\"正解\");\n    return 0;\n}\n";
    save_draft(&window, "ce", json!({ "main.c": RIGHT })).unwrap();
    let dir = AppDataDir::new(data.path());
    let files = std::collections::BTreeMap::from([("main.c".to_string(), RIGHT.to_string())]);
    let session_id = dir
        .append_submit(
            "ce",
            SessionKind::Lesson,
            SubmitRecord {
                at: "2026-11-28T10:00:00+09:00".into(),
                files,
                status: SubmitStatus::Passed,
                error_codes: Vec::new(),
                passed: 2,
                total: 2,
                failed_cases: Vec::new(),
                build: BuildOutput::default(),
                program_stderr: String::new(),
            },
        )
        .unwrap();
    dir.record_pass("ce", &session_id, "2026-11-28T10:00:00+09:00")
        .unwrap();
    end_session(&window, "ce").unwrap();

    assert_eq!(open_lesson(&window, "ce").unwrap()["draft"], Value::Null);
    assert_eq!(
        get_passed_code(&window, "ce").unwrap(),
        json!({
            "session_id": session_id,
            "submitted_at": "2026-11-28T10:00:00+09:00",
            "files": { "main.c": RIGHT },
        })
    );
}
