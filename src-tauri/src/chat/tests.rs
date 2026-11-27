//! 質疑応答の入力の組み立て（spec/qa-chat/ai-input.md）を、一時ディレクトリに作った教材と、
//! リポジトリの指示文（`prompts/`）で確かめる。

use std::fs;
use std::path::Path;

use serde_json::{Value, json};

use super::*;

/// 教材を作る。
///
/// - ch01: ch01-01（`a1` を導入）、ch01-02（準備中。`a2` を導入）、ch01-review
/// - ch02: ch02-01（`b1` を導入）、ch02-02（準備中。`b3` を導入）、ch02-03（動画のみ）、
///   ch02-04（`b4` を導入。副要素は `b2`・`a1`）、ch02-05（`b5` を導入）、ch02-review
/// - ch03: ch03-01（準備中。`c1` を導入）
///
/// `b2` は、一覧では ch02-02 より前の、別の準備中のレッスンが導入する想定で、ch02-04 の副要素にだけ出る。
fn write_content(root: &Path) {
    write_json(
        &root.join("chapters.json"),
        json!([
            { "id": "ch01", "title": "一", "description": "一の章",
              "lessons": ["ch01-01", "ch01-02", "ch01-review"] },
            { "id": "ch02", "title": "二", "description": "二の章",
              "lessons": ["ch02-01", "ch02-02", "ch02-03", "ch02-04", "ch02-05", "ch02-review"] },
            { "id": "ch03", "title": "三", "description": "三の章", "lessons": ["ch03-01"] }
        ]),
    );
    let element = |id: &str, chapter: &str| {
        json!({
            "id": id, "chapter": chapter, "name": format!("要素{id}"),
            "description": format!("要素{id}の説明"), "related_errors": []
        })
    };
    let mut deprecated = element("z", "ch01");
    deprecated["deprecated"] = json!(true);
    write_json(
        &root.join("elements.json"),
        json!([
            element("a1", "ch01"),
            element("a2", "ch01"),
            element("b1", "ch02"),
            element("b2", "ch02"),
            element("b3", "ch02"),
            element("b4", "ch02"),
            element("b5", "ch02"),
            element("c1", "ch03"),
            deprecated,
        ]),
    );

    write_lesson(root, "ch01-01", "lesson", &["a1"], &[]);
    write_lesson(root, "ch01-review", "chapter_review", &["a1", "a2"], &[]);
    write_lesson(root, "ch02-01", "lesson", &["b1"], &["a1"]);
    write_lesson(root, "ch02-03", "video_only", &[], &[]);
    write_lesson(root, "ch02-04", "lesson", &["b4"], &["b2", "a1"]);
    write_lesson(root, "ch02-05", "lesson", &["b5"], &[]);
    write_lesson(root, "ch02-review", "chapter_review", &["b1", "b4"], &[]);

    for lesson_id in ["ch01-01", "ch02-01", "ch02-03", "ch02-04", "ch02-05"] {
        write_json(
            &root.join(format!("lessons/{lesson_id}/video/script.json")),
            script(),
        );
    }
    for lesson_id in [
        "ch01-01",
        "ch01-review",
        "ch02-01",
        "ch02-04",
        "ch02-05",
        "ch02-review",
    ] {
        write_exercise(root, lesson_id);
    }
}

fn write_json(path: &Path, value: Value) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, serde_json::to_string_pretty(&value).unwrap()).unwrap();
}

fn write_text(path: &Path, text: &str) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, text).unwrap();
}

fn write_lesson(root: &Path, id: &str, kind: &str, primary: &[&str], secondary: &[&str]) {
    write_json(
        &root.join(format!("lessons/{id}/lesson.json")),
        json!({
            "id": id, "chapter": &id[..4], "kind": kind, "title": format!("レッスン{id}"),
            "summary": "概要", "primary_elements": primary, "secondary_elements": secondary,
            "estimated_minutes": 10
        }),
    );
}

fn write_exercise(root: &Path, id: &str) {
    let dir = root.join(format!("lessons/{id}/exercise"));
    write_text(
        &dir.join("problem.md"),
        "# 問題\n\nPROBLEM_MARKER を出力する。\n",
    );
    write_text(
        &dir.join("explanation.md"),
        "# 解説\n\nEXPLANATION_MARKER の説明。\n",
    );
    write_text(
        &dir.join("solution/src/main.rs"),
        "fn main() {\n    println!(\"SOLUTION_MARKER\");\n}\n",
    );
    write_text(
        &dir.join("starter/src/main.rs"),
        "fn main() {\n    todo!()\n}\n",
    );
    write_text(
        &dir.join("cases.json"),
        r#"[{ "name": "CASE_MARKER", "stdin": "", "expected_stdout": "x\n" }]"#,
    );
}

fn script() -> Value {
    json!({
        "ai_note": "AI_NOTE_MARKER の補足",
        "code_blocks": [
            { "id": "main", "label": "src/main.rs", "language": "rust",
              "code": "fn main() {\n    let s1 = String::from(\"hi\");\n}" },
            { "id": "middle", "label": "src/lib.rs", "language": "rust", "start_line": 9,
              "code": "fn a() {}\nfn b() {}\nfn c() {}" },
            { "id": "change", "label": "src/main.rs", "language": "rust", "start_line": 5, "diff": true,
              "code": " let a = 1;\n-let b = 2;\n+let b = 3;\n\n let c = 4;" },
            { "id": "input", "language": "plaintext", "code": "1 2\n3 4" },
            { "id": "error", "language": "text",
              "code": "error[E0382]: borrow of moved value: `s1`\n --> src/main.rs:4:16" }
        ],
        "scenes": [
            { "id": "title", "name": "表紙" },
            { "id": "move", "name": "ムーブの図解", "lead": 1.7 }
        ],
        "sentences": [
            { "id": "t1", "scene": "title", "text": "所有権のムーブを学びます。", "say": "よみ" },
            { "id": "m1", "scene": "move", "text": "2 行目で、s1 に値を入れました。", "say": "よみ" },
            { "id": "m2", "scene": "move", "text": "s1 が所有者です。", "say": "よみ" }
        ]
    })
}

struct Fixture {
    _temp: tempfile::TempDir,
    content: ContentDir,
    prompts: PromptDir,
}

fn fixture() -> Fixture {
    let temp = tempfile::tempdir().unwrap();
    write_content(temp.path());
    Fixture {
        content: ContentDir::new(temp.path()),
        prompts: PromptDir::new(Path::new(env!("CARGO_MANIFEST_DIR")).join("../prompts")),
        _temp: temp,
    }
}

fn turns(count: usize) -> Vec<ChatTurn> {
    (1..=count)
        .map(|n| ChatTurn {
            question: format!("Q{n:02} の質問"),
            answer: format!("A{n:02} の回答"),
        })
        .collect()
}

fn video_input(fixture: &Fixture, lesson_id: &str, sentence_id: &str) -> String {
    build_video_input(
        &fixture.content,
        &fixture.prompts,
        lesson_id,
        sentence_id,
        &turns(2),
        "ムーブとは？",
    )
    .unwrap()
}

/// `input` の中の、見出し `# {heading}` の節の本文（次の `# ` の見出しまで）。コードブロックの中の行は、
/// 見出しとして扱わない。
fn section(input: &str, heading: &str) -> String {
    let mut fence: Option<usize> = None;
    let mut body: Option<Vec<&str>> = None;
    for line in input.lines() {
        let ticks = line.len() - line.trim_start_matches('`').len();
        let is_heading = fence.is_none() && line.starts_with("# ");
        match fence {
            None if ticks >= 3 => fence = Some(ticks),
            Some(open) if ticks >= open && line.trim_start_matches('`').is_empty() => fence = None,
            _ => {}
        }
        if is_heading {
            if body.is_some() {
                break;
            }
            if line == format!("# {heading}") {
                body = Some(Vec::new());
                continue;
            }
        }
        if let Some(lines) = body.as_mut() {
            lines.push(line);
        }
    }
    body.unwrap_or_else(|| panic!("見出し {heading} が無い:\n{input}"))
        .join("\n")
        .trim()
        .to_string()
}

#[test]
fn video_input_has_no_exercise_content() {
    let fixture = fixture();
    let input = video_input(&fixture, "ch02-04", "m1");
    for marker in [
        "PROBLEM_MARKER",
        "EXPLANATION_MARKER",
        "SOLUTION_MARKER",
        "CASE_MARKER",
    ] {
        assert!(!input.contains(marker), "{marker} が入っている:\n{input}");
    }
}

#[test]
fn video_input_has_lesson_script_and_note() {
    let fixture = fixture();
    let input = video_input(&fixture, "ch02-04", "m1");
    assert!(input.starts_with("あなたは Rust 学習アプリ「Progrust」の質問回答係です。"));
    assert_eq!(
        section(&input, "レッスン"),
        "タイトル: レッスンch02-04\n\n## 主要素\n\n- 要素b4: 要素b4の説明"
    );
    assert!(section(&input, "動画の台本").ends_with(
        "## 表紙\n\n- [t1] 所有権のムーブを学びます。\n\n## ムーブの図解\n\n\
         - [m1] 2 行目で、s1 に値を入れました。\n- [m2] s1 が所有者です。"
    ));
    assert_eq!(
        section(&input, "補足（学習者には見えない）"),
        "AI_NOTE_MARKER の補足"
    );
    assert_eq!(section(&input, "今回の質問"), "ムーブとは？");
}

#[test]
fn video_input_has_current_sentence_id_and_text() {
    let fixture = fixture();
    let input = video_input(&fixture, "ch02-04", "m2");
    assert_eq!(
        section(&input, "質問を送信したときに再生中だった文"),
        "[m2] s1 が所有者です。"
    );
}

#[test]
fn code_blocks_have_line_numbers_from_start_line() {
    let fixture = fixture();
    let input = video_input(&fixture, "ch02-04", "m1");
    let code = section(&input, "画面に出すコード");

    assert!(
        code.contains(
            "```rust\n1 | fn main() {\n2 |     let s1 = String::from(\"hi\");\n3 | }\n```"
        ),
        "{code}"
    );
    // `start_line` から数え、桁をそろえる。
    assert!(
        code.contains("```rust\n 9 | fn a() {}\n10 | fn b() {}\n11 | fn c() {}\n```"),
        "{code}"
    );
    // `text` 以外（`plaintext`）には行番号を付ける。
    assert!(
        code.contains("```plaintext\n1 | 1 2\n2 | 3 4\n```"),
        "{code}"
    );
    assert!(code.contains("## src/main.rs（言語: rust）"), "{code}");
}

/// Diff は、変更後のファイルの行番号を付け、削除行には付けず、`+`・`-` を残す。
#[test]
fn diff_code_blocks_number_lines_of_changed_file() {
    let fixture = fixture();
    let input = video_input(&fixture, "ch02-04", "m1");
    let code = section(&input, "画面に出すコード");
    assert!(
        code.contains(
            "```rust\n5 |  let a = 1;\n  | -let b = 2;\n6 | +let b = 3;\n7 | \n8 |  let c = 4;\n```"
        ),
        "{code}"
    );
    assert!(code.contains("Diff として表示する。"), "{code}");
}

/// `text` のコードブロックは、ターミナルの出力として、行番号を付けずに渡す。
#[test]
fn text_code_blocks_have_no_line_numbers() {
    let fixture = fixture();
    let input = video_input(&fixture, "ch02-04", "m1");
    let code = section(&input, "画面に出すコード");
    assert!(
        code.contains(
            "## ターミナルの出力\n\n```text\nerror[E0382]: borrow of moved value: `s1`\n --> src/main.rs:4:16\n```"
        ),
        "{code}"
    );
}

/// 既習の要素は、導入レッスンがそのレッスンより前にある要素で、後で導入される要素と、
/// そのレッスン自身が導入する要素を含まない。
#[test]
fn learned_elements_follow_learning_order() {
    let fixture = fixture();
    let input = video_input(&fixture, "ch02-04", "m1");
    assert_eq!(
        section(&input, "既習の要素"),
        [
            // 前の章の要素。導入レッスン（ch01-02）が準備中でも、要素マスタの `chapter` で分かる。
            "- 要素a1: 要素a1の説明",
            "- 要素a2: 要素a2の説明",
            // 同じ章の、前にある準備中でないレッスンの主要素。
            "- 要素b1: 要素b1の説明",
            // そのレッスンの副要素。
            "- 要素b2: 要素b2の説明",
        ]
        .join("\n")
    );
    // 同じ章の、前にある準備中のレッスン（ch02-02）だけが導入する `b3` は、求められないので入らない。
    // そのレッスン自身の `b4`、後のレッスンの `b5`・`c1`、廃止した `z` は入らない。
    let learned = section(&input, "既習の要素");
    for id in ["b3", "b4", "b5", "c1", "z"] {
        assert!(!learned.contains(&format!("要素{id}")), "{learned}");
    }
}

#[test]
fn first_lesson_has_no_learned_elements() {
    let fixture = fixture();
    let input = video_input(&fixture, "ch01-01", "t1");
    assert_eq!(section(&input, "既習の要素"), "なし");
}

/// 動画のみのレッスンには主要素が無いので、タイトルだけを渡す。
#[test]
fn video_only_lesson_has_title_only() {
    let fixture = fixture();
    let input = video_input(&fixture, "ch02-03", "t1");
    assert_eq!(section(&input, "レッスン"), "タイトル: レッスンch02-03");
    assert_eq!(
        section(&input, "既習の要素"),
        "- 要素a1: 要素a1の説明\n- 要素a2: 要素a2の説明\n- 要素b1: 要素b1の説明"
    );
}

#[test]
fn script_without_note_and_code_blocks_is_written_as_none() {
    let fixture = fixture();
    let path = fixture
        .content
        .root()
        .join("lessons/ch02-05/video/script.json");
    let mut value = script();
    value.as_object_mut().unwrap().remove("ai_note");
    value.as_object_mut().unwrap().remove("code_blocks");
    write_json(&path, value);

    let input = build_video_input(
        &fixture.content,
        &fixture.prompts,
        "ch02-05",
        "t1",
        &[],
        "質問",
    )
    .unwrap();
    assert_eq!(section(&input, "補足（学習者には見えない）"), "なし");
    assert_eq!(section(&input, "画面に出すコード"), "なし");
    assert_eq!(section(&input, "これまでの会話"), "なし");
}

/// 会話履歴は、直近 10 往復までを古い順に入れる。11 往復渡すと、最も古い 1 往復が入らない。
#[test]
fn history_keeps_last_ten_turns_in_order() {
    let fixture = fixture();
    let input = build_video_input(
        &fixture.content,
        &fixture.prompts,
        "ch02-04",
        "m1",
        &turns(11),
        "次の質問",
    )
    .unwrap();
    let history = section(&input, "これまでの会話");
    assert!(!history.contains("Q01"), "{history}");
    assert!(!history.contains("A01"), "{history}");
    let mut last = 0;
    for n in 2..=11 {
        let question = history
            .find(&format!("Q{n:02} の質問"))
            .unwrap_or_else(|| panic!("Q{n:02} が無い:\n{history}"));
        let answer = history.find(&format!("A{n:02} の回答")).unwrap();
        assert!(last < question && question < answer, "{history}");
        last = answer;
    }
    assert!(history.starts_with("## 学習者の質問 1\n\nQ02 の質問\n\n## 回答 1\n\nA02 の回答"));
}

/// 指示文の `{{name}}` はすべて置き換わり、値の中の `{{…}}` はそのまま残る。
#[test]
fn placeholders_in_values_are_kept() {
    let fixture = fixture();
    let question = "{{history}} と format!(\"{{}}\") の違いは？";
    let history = [ChatTurn {
        question: "{{question}} とは？".into(),
        answer: "{{lesson}} です".into(),
    }];
    let input = build_video_input(
        &fixture.content,
        &fixture.prompts,
        "ch02-04",
        "m1",
        &history,
        question,
    )
    .unwrap();
    assert_eq!(section(&input, "今回の質問"), question);
    assert!(section(&input, "これまでの会話").contains("{{question}} とは？"));
    assert!(section(&input, "これまでの会話").contains("{{lesson}} です"));
    // 値から来たもの以外に `{{` は残らない。
    assert_eq!(input.matches("{{").count(), 4, "{input}");
}

#[test]
fn video_input_fails_for_unknown_sentence_and_chapter_review() {
    let fixture = fixture();
    let unknown = build_video_input(
        &fixture.content,
        &fixture.prompts,
        "ch02-04",
        "x9",
        &[],
        "質問",
    );
    assert!(
        matches!(unknown, Err(ChatInputError::UnknownSentence { ref sentence_id, .. }) if sentence_id == "x9"),
        "{unknown:?}"
    );
    let review = build_video_input(
        &fixture.content,
        &fixture.prompts,
        "ch02-review",
        "t1",
        &[],
        "質問",
    );
    assert!(
        matches!(review, Err(ChatInputError::NoVideo { .. })),
        "{review:?}"
    );
    let missing = build_video_input(
        &fixture.content,
        &fixture.prompts,
        "ch02-02",
        "t1",
        &[],
        "質問",
    );
    assert!(
        matches!(
            missing,
            Err(ChatInputError::Content(ContentError::LessonNotFound { .. }))
        ),
        "{missing:?}"
    );
}

/// 試行ログ。誤った提出の後に、2 ファイルで正解した提出がある。
fn attempt_log(lesson_id: &str, passed: bool) -> AttemptLog {
    let submit = |status: &str, main: &str| {
        json!({
            "type": "submit", "at": "2026-11-27T10:05:00+09:00",
            "files": { "src/main.rs": main, "src/lib.rs": "pub fn helper() -> u32 {\n    LEARNER_LIB\n}\n" },
            "result": { "status": status, "error_codes": [], "passed": 1, "total": 1,
                        "failed_cases": [], "stderr": "" }
        })
    };
    let mut events = vec![submit("wrong", "fn main() { WRONG_MARKER }\n")];
    if passed {
        events.push(submit("passed", "fn main() {\n    LEARNER_MAIN\n}\n"));
    }
    serde_json::from_value(json!({
        "schema_version": 1,
        "session_id": format!("20261127T100000_{lesson_id}"),
        "lesson_id": lesson_id,
        "kind": "lesson",
        "difficulty": null,
        "started_at": "2026-11-27T10:00:00+09:00",
        "passed_at": if passed { json!("2026-11-27T10:05:00+09:00") } else { Value::Null },
        "ended_at": null,
        "events": events,
    }))
    .unwrap()
}

#[test]
fn explanation_input_has_problem_explanation_solution_and_learner_code() {
    let fixture = fixture();
    let input = build_explanation_input(
        &fixture.content,
        &fixture.prompts,
        "ch02-04",
        &attempt_log("ch02-04", true),
        &turns(1),
        "もっと良い書き方は？",
    )
    .unwrap();

    assert!(
        input.starts_with(
            "あなたは Rust 学習アプリ「Progrust」の質問回答係です。学習者は演習に正解し"
        )
    );
    assert_eq!(
        section(&input, "レッスン"),
        "タイトル: レッスンch02-04\n\n## 主要素\n\n- 要素b4: 要素b4の説明\n\n\
         ## 副要素\n\n- 要素b2: 要素b2の説明\n- 要素a1: 要素a1の説明"
    );
    assert_eq!(
        section(&input, "問題文"),
        "```markdown\n# 問題\n\nPROBLEM_MARKER を出力する。\n```"
    );
    assert_eq!(
        section(&input, "解説"),
        "```markdown\n# 解説\n\nEXPLANATION_MARKER の説明。\n```"
    );
    assert_eq!(
        section(&input, "模範解答"),
        "## src/main.rs\n\n```rust\nfn main() {\n    println!(\"SOLUTION_MARKER\");\n}\n```"
    );
    // 複数のファイルは、ファイルパスを添えて並べる。正解した提出のコードを使う。
    assert_eq!(
        section(&input, "学習者が正解したときのコード"),
        "## src/lib.rs\n\n```rust\npub fn helper() -> u32 {\n    LEARNER_LIB\n}\n```\n\n\
         ## src/main.rs\n\n```rust\nfn main() {\n    LEARNER_MAIN\n}\n```"
    );
    assert!(!input.contains("WRONG_MARKER"));
    assert!(section(&input, "これまでの会話").contains("Q01 の質問"));
    assert_eq!(section(&input, "今回の質問"), "もっと良い書き方は？");
    // 台本は入れない。
    assert!(!input.contains("AI_NOTE_MARKER"));
}

/// 章末復習レッスンは、動画が無くても、解説表示中の入力を作れる。
#[test]
fn explanation_input_for_chapter_review() {
    let fixture = fixture();
    let input = build_explanation_input(
        &fixture.content,
        &fixture.prompts,
        "ch02-review",
        &attempt_log("ch02-review", true),
        &[],
        "質問",
    )
    .unwrap();
    assert_eq!(
        section(&input, "レッスン"),
        "タイトル: レッスンch02-review\n\n## 主要素\n\n- 要素b1: 要素b1の説明\n- 要素b4: 要素b4の説明\n\n\
         ## 副要素\n\nなし"
    );
}

#[test]
fn explanation_input_fails_without_passed_submit_or_exercise() {
    let fixture = fixture();
    let not_passed = build_explanation_input(
        &fixture.content,
        &fixture.prompts,
        "ch02-04",
        &attempt_log("ch02-04", false),
        &[],
        "質問",
    );
    assert!(
        matches!(not_passed, Err(ChatInputError::NotPassed { .. })),
        "{not_passed:?}"
    );

    let other = build_explanation_input(
        &fixture.content,
        &fixture.prompts,
        "ch02-04",
        &attempt_log("ch02-05", true),
        &[],
        "質問",
    );
    assert!(
        matches!(other, Err(ChatInputError::OtherLesson { .. })),
        "{other:?}"
    );

    let video_only = build_explanation_input(
        &fixture.content,
        &fixture.prompts,
        "ch02-03",
        &attempt_log("ch02-03", true),
        &[],
        "質問",
    );
    assert!(
        matches!(video_only, Err(ChatInputError::NoExercise { .. })),
        "{video_only:?}"
    );
}

#[test]
fn fences_are_longer_than_backticks_in_text() {
    assert_eq!(
        fenced("markdown", "a\n```rust\nb\n```\n"),
        "````markdown\na\n```rust\nb\n```\n````"
    );
    assert_eq!(fenced("", "x"), "```\nx\n```");
}

#[test]
fn missing_prompt_file_is_an_error() {
    let fixture = fixture();
    let temp = tempfile::tempdir().unwrap();
    let result = build_video_input(
        &fixture.content,
        &PromptDir::new(temp.path()),
        "ch02-04",
        "m1",
        &[],
        "質問",
    );
    assert!(
        matches!(result, Err(ChatInputError::Prompt { .. })),
        "{result:?}"
    );
}
