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
    assert_eq!(response["session_id"], Value::Null);
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
