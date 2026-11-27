use std::fs;

use chrono::TimeZone;
use serde_json::{Value, json};

use super::*;

fn at(hour: u32, minute: u32, second: u32) -> DateTime<Local> {
    Local
        .with_ymd_and_hms(2026, 10, 31, hour, minute, second)
        .single()
        .unwrap()
}

fn files(content: &str) -> BTreeMap<String, String> {
    BTreeMap::from([("src/main.rs".to_string(), content.to_string())])
}

fn run_record(status: RunStatus) -> RunRecord {
    RunRecord {
        at: "2026-10-31T13:05:00+09:00".into(),
        mode: RunMode::Main,
        files: files("fn main() {}\n"),
        status,
        error_codes: Vec::new(),
        build: BuildOutput::default(),
        stdout: String::new(),
        program_stderr: String::new(),
        exit_code: None,
        stdin: String::new(),
    }
}

fn saved_json(temp: &tempfile::TempDir, session_id: &str) -> Value {
    serde_json::from_str(
        &fs::read_to_string(temp.path().join(format!("attempts/{session_id}.json"))).unwrap(),
    )
    .unwrap()
}

fn attempt_names(temp: &tempfile::TempDir) -> Vec<String> {
    let mut names: Vec<_> = fs::read_dir(temp.path().join("attempts"))
        .unwrap()
        .map(|entry| entry.unwrap().file_name().into_string().unwrap())
        .collect();
    names.sort();
    names
}

#[test]
fn a_new_session_is_written_in_the_spec_shape() {
    let temp = tempfile::tempdir().unwrap();
    let dir = AppDataDir::new(temp.path());
    let now = at(13, 2, 0);
    let log = dir
        .start_or_continue_session_at("ch04-04", SessionKind::Lesson, now)
        .unwrap();
    assert_eq!(log.session_id, "20261031T130200_ch04-04");
    assert_eq!(
        saved_json(&temp, "20261031T130200_ch04-04"),
        json!({
            "schema_version": 1,
            "session_id": "20261031T130200_ch04-04",
            "lesson_id": "ch04-04",
            "kind": "lesson",
            "difficulty": null,
            "started_at": now.to_rfc3339_opts(SecondsFormat::Secs, false),
            "passed_at": null,
            "ended_at": null,
            "events": [],
        })
    );
}

#[test]
fn an_unfinished_session_is_continued_and_an_ended_one_is_not() {
    let temp = tempfile::tempdir().unwrap();
    let dir = AppDataDir::new(temp.path());
    let first = dir
        .start_or_continue_session_at("ch04-04", SessionKind::Lesson, at(13, 2, 0))
        .unwrap();
    // アプリを起動し直しても（別の `AppDataDir` でも）、同じセッションを続ける。
    let again = AppDataDir::new(temp.path())
        .start_or_continue_session_at("ch04-04", SessionKind::Lesson, at(14, 0, 0))
        .unwrap();
    assert_eq!(again, first);
    // 正解しても、終了するまでは同じセッション。
    dir.mark_session_passed(&first.session_id, "2026-10-31T13:25:00+09:00")
        .unwrap();
    let passed = dir
        .start_or_continue_session_at("ch04-04", SessionKind::Lesson, at(14, 1, 0))
        .unwrap();
    assert_eq!(passed.session_id, first.session_id);

    dir.end_attempt_session(&first.session_id, "2026-10-31T13:31:00+09:00")
        .unwrap();
    let next = dir
        .start_or_continue_session_at("ch04-04", SessionKind::Lesson, at(15, 0, 0))
        .unwrap();
    assert_eq!(next.session_id, "20261031T150000_ch04-04");
    assert_eq!(next.ended_at, None);
    assert_eq!(
        attempt_names(&temp),
        [
            "20261031T130200_ch04-04.json",
            "20261031T150000_ch04-04.json"
        ]
    );
}

#[test]
fn sessions_of_other_lessons_are_not_continued() {
    let temp = tempfile::tempdir().unwrap();
    let dir = AppDataDir::new(temp.path());
    dir.start_or_continue_session_at("ch04-04", SessionKind::Lesson, at(13, 0, 0))
        .unwrap();
    // `ch04-04` で終わる別のレッスン ID も、別のレッスン。
    let other = dir
        .start_or_continue_session_at("x_ch04-04", SessionKind::ChapterReview, at(13, 0, 0))
        .unwrap();
    assert_eq!(other.session_id, "20261031T130000_x_ch04-04");
    assert_eq!(other.kind, SessionKind::ChapterReview);
    assert_eq!(
        dir.unfinished_session("ch04-04")
            .unwrap()
            .unwrap()
            .session_id,
        "20261031T130000_ch04-04"
    );
}

/// 同じ秒に前のセッションが終わっていても、前の試行ログを上書きしない。
#[test]
fn a_new_session_in_the_same_second_does_not_overwrite_the_previous_log() {
    let temp = tempfile::tempdir().unwrap();
    let dir = AppDataDir::new(temp.path());
    let now = at(13, 2, 0);
    let first = dir
        .start_or_continue_session_at("ch04-04", SessionKind::Lesson, now)
        .unwrap();
    dir.end_attempt_session(&first.session_id, "2026-10-31T13:02:00+09:00")
        .unwrap();
    let second = dir
        .start_or_continue_session_at("ch04-04", SessionKind::Lesson, now)
        .unwrap();
    assert_eq!(second.session_id, "20261031T130201_ch04-04");
    assert_eq!(second.started_at, first.started_at);
    assert_eq!(
        dir.read_attempt_log(&first.session_id)
            .unwrap()
            .unwrap()
            .ended_at
            .as_deref(),
        Some("2026-10-31T13:02:00+09:00")
    );
}

#[test]
fn a_broken_log_is_moved_aside_and_a_new_session_starts() {
    let temp = tempfile::tempdir().unwrap();
    let dir = AppDataDir::new(temp.path());
    dir.write_text("attempts/20261030T090000_ch04-04.json", "{ 壊れた")
        .unwrap();
    let log = dir
        .start_or_continue_session_at("ch04-04", SessionKind::Lesson, at(13, 2, 0))
        .unwrap();
    assert_eq!(log.session_id, "20261031T130200_ch04-04");
    let names = attempt_names(&temp);
    assert_eq!(names.len(), 2, "{names:?}");
    assert!(
        names[0].starts_with("20261030T090000_ch04-04.json.broken-"),
        "{names:?}"
    );
    assert_eq!(names[1], "20261031T130200_ch04-04.json");
}

#[test]
fn a_log_with_another_schema_version_is_unreadable() {
    let temp = tempfile::tempdir().unwrap();
    let dir = AppDataDir::new(temp.path());
    let log = dir
        .start_or_continue_session_at("ch04-04", SessionKind::Lesson, at(13, 2, 0))
        .unwrap();
    let mut json = saved_json(&temp, &log.session_id);
    json["schema_version"] = json!(2);
    dir.write_json(attempt_file(&log.session_id), &json)
        .unwrap();
    assert_eq!(dir.read_attempt_log(&log.session_id).unwrap(), None);
    assert_eq!(dir.unfinished_session("ch04-04").unwrap(), None);
}

#[test]
fn a_run_event_is_appended_with_the_output_rules() {
    let temp = tempfile::tempdir().unwrap();
    let dir = AppDataDir::new(temp.path());
    let session = dir
        .start_or_continue_session_at("ch04-04", SessionKind::Lesson, at(13, 2, 0))
        .unwrap();
    let record = RunRecord {
        at: "2026-10-31T13:05:00+09:00".into(),
        mode: RunMode::Main,
        files: files("fn main() { let s = String::new(); }\n"),
        status: RunStatus::RuntimeError,
        error_codes: Vec::new(),
        build: BuildOutput {
            compiler_messages: "\u{1b}[1m\u{1b}[33mwarning\u{1b}[0m: unused variable: `s`\n".into(),
            cargo_stderr: "\u{1b}[33mwarning\u{1b}[0m: cargo の警告\n".into(),
        },
        stdout: "あ".repeat(OUTPUT_LIMIT_CHARS + 10),
        program_stderr: "thread 'main' panicked\n".into(),
        exit_code: Some(101),
        stdin: "3\n4\n".into(),
    };
    let session_id = dir
        .append_run("ch04-04", SessionKind::Lesson, record)
        .unwrap();
    assert_eq!(session_id, session.session_id);

    let saved = saved_json(&temp, &session_id);
    let events = saved["events"].as_array().unwrap();
    assert_eq!(events.len(), 1);
    assert_eq!(
        events[0],
        json!({
            "type": "run",
            "at": "2026-10-31T13:05:00+09:00",
            "mode": "main",
            "files": { "src/main.rs": "fn main() { let s = String::new(); }\n" },
            "result": {
                "status": "runtime_error",
                "error_codes": [],
                "stdout": "あ".repeat(OUTPUT_LIMIT_CHARS),
                "stderr": "warning: unused variable: `s`\nwarning: cargo の警告\nthread 'main' panicked\n",
                "exit_code": 101,
                "stdin": "3\n4\n",
            },
        })
    );
}

#[test]
fn a_compile_error_run_keeps_error_codes_in_order_without_duplicates() {
    let temp = tempfile::tempdir().unwrap();
    let dir = AppDataDir::new(temp.path());
    dir.start_or_continue_session_at("ch04-04", SessionKind::Lesson, at(13, 2, 0))
        .unwrap();
    let mut record = run_record(RunStatus::CompileError);
    record.mode = RunMode::Test;
    record.error_codes = ["E0382", "E0499", "E0382"].map(String::from).to_vec();
    record.build.cargo_stderr = "error: could not compile `exercise`\n".into();
    let session_id = dir
        .append_run("ch04-04", SessionKind::Lesson, record)
        .unwrap();
    let result = &saved_json(&temp, &session_id)["events"][0]["result"];
    assert_eq!(result["error_codes"], json!(["E0382", "E0499"]));
    assert_eq!(result["exit_code"], Value::Null);
    assert_eq!(result["stderr"], "error: could not compile `exercise`\n");
    assert_eq!(saved_json(&temp, &session_id)["events"][0]["mode"], "test");
}

#[test]
fn events_are_appended_in_order_to_the_same_session() {
    let temp = tempfile::tempdir().unwrap();
    let dir = AppDataDir::new(temp.path());
    let session = dir
        .start_or_continue_session_at("ch04-04", SessionKind::Lesson, at(13, 2, 0))
        .unwrap();
    let first = dir
        .append_run(
            "ch04-04",
            SessionKind::Lesson,
            run_record(RunStatus::Success),
        )
        .unwrap();
    // アプリを起動し直して開き直しても（別の `AppDataDir` でも）、同じセッションに追記する。
    let reopened = AppDataDir::new(temp.path());
    let continued = reopened
        .start_or_continue_session_at("ch04-04", SessionKind::Lesson, at(14, 0, 0))
        .unwrap();
    assert_eq!(continued.session_id, session.session_id);
    let second = reopened
        .append_submit(
            "ch04-04",
            SessionKind::Lesson,
            SubmitRecord {
                at: "2026-10-31T14:10:00+09:00".into(),
                files: files("fn main() {}\n"),
                status: SubmitStatus::Wrong,
                error_codes: Vec::new(),
                passed: 3,
                total: 5,
                failed_cases: vec!["空の入力".into(), "負の数".into()],
                build: BuildOutput {
                    compiler_messages: "\u{1b}[1mwarning\u{1b}[0m: 警告\n".into(),
                    cargo_stderr: String::new(),
                },
                program_stderr: String::new(),
            },
        )
        .unwrap();
    assert_eq!(first, session.session_id);
    assert_eq!(second, first);
    let saved = saved_json(&temp, &session.session_id);
    let events = saved["events"].as_array().unwrap();
    assert_eq!(events.len(), 2);
    assert_eq!(events[0]["type"], "run");
    assert_eq!(
        events[1],
        json!({
            "type": "submit",
            "at": "2026-10-31T14:10:00+09:00",
            "files": { "src/main.rs": "fn main() {}\n" },
            "result": {
                "status": "wrong",
                "error_codes": [],
                "passed": 3,
                "total": 5,
                "failed_cases": ["空の入力", "負の数"],
                "stderr": "warning: 警告\n",
            },
        })
    );
    assert_eq!(attempt_names(&temp).len(), 1);
}

/// 試行ログが壊れて退避された後などで、未完了のセッションが無いときも、学習者の試行を落とさない。
/// 終わったセッションには追記しない。
#[test]
fn without_an_unfinished_session_a_new_session_is_started_for_the_event() {
    let temp = tempfile::tempdir().unwrap();
    let dir = AppDataDir::new(temp.path());
    let first = dir
        .start_or_continue_session_at("ch04-04", SessionKind::Lesson, at(13, 2, 0))
        .unwrap();
    dir.end_attempt_session(&first.session_id, "2026-10-31T13:31:00+09:00")
        .unwrap();
    let before = saved_json(&temp, &first.session_id);
    let session_id = dir
        .append_run(
            "ch04-04",
            SessionKind::Lesson,
            run_record(RunStatus::Stopped),
        )
        .unwrap();
    assert_ne!(session_id, first.session_id);
    assert_eq!(saved_json(&temp, &first.session_id), before);
    let log = dir.read_attempt_log(&session_id).unwrap().unwrap();
    assert_eq!(log.ended_at, None);
    assert_eq!(log.events.len(), 1);

    // 試行ログが無いレッスンでも、新しいセッションに追記する。
    let other = dir
        .append_run(
            "ch05-01",
            SessionKind::ChapterReview,
            run_record(RunStatus::Success),
        )
        .unwrap();
    let log = dir.read_attempt_log(&other).unwrap().unwrap();
    assert_eq!(log.kind, SessionKind::ChapterReview);
    assert_eq!(log.events.len(), 1);
}

#[test]
fn passed_at_and_ended_at_are_written_once() {
    let temp = tempfile::tempdir().unwrap();
    let dir = AppDataDir::new(temp.path());
    let session = dir
        .start_or_continue_session_at("ch04-04", SessionKind::Lesson, at(13, 2, 0))
        .unwrap();
    let id = &session.session_id;
    let passed = dir
        .mark_session_passed(id, "2026-10-31T13:25:00+09:00")
        .unwrap()
        .unwrap();
    assert_eq!(
        passed.passed_at.as_deref(),
        Some("2026-10-31T13:25:00+09:00")
    );
    assert_eq!(passed.ended_at, None);
    // 2 回目の正解では、最初の日時を残す。
    let again = dir
        .mark_session_passed(id, "2026-10-31T13:28:00+09:00")
        .unwrap()
        .unwrap();
    assert_eq!(
        again.passed_at.as_deref(),
        Some("2026-10-31T13:25:00+09:00")
    );

    let ended = dir
        .end_attempt_session(id, "2026-10-31T13:31:00+09:00")
        .unwrap()
        .unwrap();
    assert_eq!(ended.ended_at.as_deref(), Some("2026-10-31T13:31:00+09:00"));
    let ended_again = dir
        .end_attempt_session(id, "2026-10-31T14:00:00+09:00")
        .unwrap()
        .unwrap();
    assert_eq!(
        ended_again.ended_at.as_deref(),
        Some("2026-10-31T13:31:00+09:00")
    );
    let saved = saved_json(&temp, id);
    assert_eq!(saved["passed_at"], "2026-10-31T13:25:00+09:00");
    assert_eq!(saved["ended_at"], "2026-10-31T13:31:00+09:00");

    // 無いセッションは、何もしない。
    assert_eq!(
        dir.mark_session_passed("20261031T000000_none", "x")
            .unwrap(),
        None
    );
    assert_eq!(
        dir.end_attempt_session("20261031T000000_none", "x")
            .unwrap(),
        None
    );
    assert_eq!(attempt_names(&temp).len(), 1);
}

#[test]
fn unfinished_sessions_include_passed_ones_and_exclude_ended_ones() {
    let temp = tempfile::tempdir().unwrap();
    let dir = AppDataDir::new(temp.path());
    assert_eq!(dir.unfinished_sessions().unwrap(), Vec::new());
    assert!(!temp.path().join("attempts").exists());

    let open = dir
        .start_or_continue_session_at("a", SessionKind::Lesson, at(10, 0, 0))
        .unwrap();
    let passed = dir
        .start_or_continue_session_at("b", SessionKind::Lesson, at(11, 0, 0))
        .unwrap();
    dir.mark_session_passed(&passed.session_id, "2026-10-31T11:10:00+09:00")
        .unwrap();
    let ended = dir
        .start_or_continue_session_at("c", SessionKind::ChapterReview, at(12, 0, 0))
        .unwrap();
    dir.mark_session_passed(&ended.session_id, "2026-10-31T12:10:00+09:00")
        .unwrap();
    dir.end_attempt_session(&ended.session_id, "2026-10-31T12:20:00+09:00")
        .unwrap();
    dir.write_text("attempts/20261031T130000_d.json", "壊れた")
        .unwrap();
    dir.write_text("attempts/memo.json", "{}").unwrap();

    let ids: Vec<String> = dir
        .unfinished_sessions()
        .unwrap()
        .into_iter()
        .map(|log| log.session_id)
        .collect();
    assert_eq!(ids, [open.session_id.clone(), passed.session_id.clone()]);
    assert_eq!(
        dir.unfinished_session("b")
            .unwrap()
            .unwrap()
            .passed_at
            .as_deref(),
        Some("2026-10-31T11:10:00+09:00")
    );
    assert_eq!(dir.unfinished_session("c").unwrap(), None);
    assert_eq!(dir.unfinished_session("d").unwrap(), None);
}

/// 質問のイベント（M4 で書く）を含む試行ログも、読み込めないものとして扱わない。
#[test]
fn a_log_with_chat_events_is_readable() {
    let temp = tempfile::tempdir().unwrap();
    let dir = AppDataDir::new(temp.path());
    let json = json!({
        "schema_version": 1,
        "session_id": "20261031T130200_ch04-04",
        "lesson_id": "ch04-04",
        "kind": "lesson",
        "difficulty": null,
        "started_at": "2026-10-31T13:02:00+09:00",
        "passed_at": null,
        "ended_at": null,
        "events": [
            { "type": "chat", "at": "2026-10-31T13:03:00+09:00", "context": "video:m5",
              "question": "ムーブ後に使えないのはなぜ？", "answer_summary": null },
        ],
    });
    dir.write_json("attempts/20261031T130200_ch04-04.json", &json)
        .unwrap();
    let log = dir
        .read_attempt_log("20261031T130200_ch04-04")
        .unwrap()
        .unwrap();
    assert_eq!(serde_json::to_value(&log).unwrap(), json);
    assert!(matches!(log.events[0], Event::Chat(_)));
}

#[test]
fn session_ids_are_parsed_strictly() {
    assert_eq!(lesson_id_of("20261031T130200_ch04-04"), Some("ch04-04"));
    assert_eq!(lesson_id_of("20261031T130200_gen-1_2"), Some("gen-1_2"));
    for id in [
        "20261031T130200_",
        "20261031T130200ch04",
        "20261031X130200_ch04",
        "2026103aT130200_ch04",
        "memo",
        "あいうえおかきくけこさしすせそ_x",
    ] {
        assert_eq!(lesson_id_of(id), None, "{id}");
    }
}

#[test]
fn session_kinds_follow_the_lesson_kind() {
    assert_eq!(SessionKind::for_lesson_kind("video_only"), None);
    assert_eq!(
        SessionKind::for_lesson_kind("lesson"),
        Some(SessionKind::Lesson)
    );
    assert_eq!(
        SessionKind::for_lesson_kind("chapter_review"),
        Some(SessionKind::ChapterReview)
    );
}

#[test]
fn a_passed_session_is_ended_only_after_before_end_succeeds() {
    let temp = tempfile::tempdir().unwrap();
    let dir = AppDataDir::new(temp.path());
    let log = dir
        .start_or_continue_session_at("ch04-04", SessionKind::Lesson, at(13, 2, 0))
        .unwrap();
    let id = log.session_id.clone();

    // 正解していないセッションは終えない。`before_end` も呼ばない。
    let ended = dir
        .end_passed_session_of_lesson("ch04-04", "2026-10-31T13:30:00+09:00", |_| {
            panic!("正解していないのに呼んだ")
        })
        .unwrap();
    assert_eq!(ended, None);

    dir.mark_session_passed(&id, "2026-10-31T13:25:00+09:00")
        .unwrap();
    // `before_end` が失敗したら、`ended_at` を書かない。
    let failed = dir.end_passed_session_by_id(&id, "2026-10-31T13:30:00+09:00", |_| {
        Err(StorageError::InvalidPath { path: "x".into() })
    });
    assert!(failed.is_err());
    assert_eq!(saved_json(&temp, &id)["ended_at"], Value::Null);

    let ended = dir
        .end_passed_session_of_lesson("ch04-04", "2026-10-31T13:30:00+09:00", |log| {
            assert_eq!(log.session_id, id);
            Ok(())
        })
        .unwrap()
        .unwrap();
    assert_eq!(ended.ended_at.as_deref(), Some("2026-10-31T13:30:00+09:00"));
    assert_eq!(
        saved_json(&temp, &id)["ended_at"],
        "2026-10-31T13:30:00+09:00"
    );

    // 終えたセッションは、もう一度は終えない。
    assert_eq!(
        dir.end_passed_session_by_id(&id, "2026-10-31T13:40:00+09:00", |_| Ok(()))
            .unwrap(),
        None
    );
    assert_eq!(
        dir.end_passed_session_of_lesson("ch04-04", "2026-10-31T13:40:00+09:00", |_| Ok(()))
            .unwrap(),
        None
    );
}

fn chat_event(context: &str, question: &str) -> ChatEvent {
    ChatEvent {
        at: "2026-10-31T13:10:00+09:00".into(),
        context: context.into(),
        question: question.into(),
        answer_summary: None,
    }
}

#[test]
fn a_chat_event_is_appended_to_the_unfinished_session_in_the_spec_shape() {
    let temp = tempfile::tempdir().unwrap();
    let dir = AppDataDir::new(temp.path());
    let log = dir
        .start_or_continue_session_at("ch04-04", SessionKind::Lesson, at(13, 2, 0))
        .unwrap();
    let appended = dir
        .append_chat(
            "ch04-04",
            chat_event("video:m5", "ムーブ後に使えないのはなぜ？"),
        )
        .unwrap();
    assert_eq!(appended.as_deref(), Some(log.session_id.as_str()));
    dir.append_chat("ch04-04", chat_event("explanation", "次の質問"))
        .unwrap();
    assert_eq!(
        saved_json(&temp, &log.session_id)["events"],
        json!([
            { "type": "chat", "at": "2026-10-31T13:10:00+09:00", "context": "video:m5",
              "question": "ムーブ後に使えないのはなぜ？", "answer_summary": null },
            { "type": "chat", "at": "2026-10-31T13:10:00+09:00", "context": "explanation",
              "question": "次の質問", "answer_summary": null },
        ])
    );
}

/// 未完了のセッションが無いとき（動画のみのレッスン、終えたセッションの後）は、セッションを始めず、何も書かない。
#[test]
fn a_chat_event_without_an_unfinished_session_writes_nothing() {
    let temp = tempfile::tempdir().unwrap();
    let dir = AppDataDir::new(temp.path());
    assert_eq!(
        dir.append_chat("ch01-03", chat_event("video:t1", "質問"))
            .unwrap(),
        None
    );
    assert!(!temp.path().join("attempts").exists());

    let log = dir
        .start_or_continue_session_at("ch04-04", SessionKind::Lesson, at(13, 2, 0))
        .unwrap();
    dir.end_attempt_session(&log.session_id, "2026-10-31T13:30:00+09:00")
        .unwrap();
    let before = saved_json(&temp, &log.session_id);
    assert_eq!(
        dir.append_chat("ch04-04", chat_event("explanation", "質問"))
            .unwrap(),
        None
    );
    assert_eq!(saved_json(&temp, &log.session_id), before);
    assert_eq!(attempt_names(&temp), [format!("{}.json", log.session_id)]);
}
