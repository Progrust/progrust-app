//! テスト用の偽の claude・cargo・rustc（spec/non-functional/testing.md「偽の claude と偽の cargo」）が、
//! 決めたとおりに動くことと、アプリに同梱されないことを確かめる。

mod fake_tools;

use std::{
    collections::BTreeMap,
    fs,
    io::{BufRead, BufReader, Read, Write},
    path::{Component, Path, PathBuf},
    process::{Child, Command, Output, Stdio},
    thread,
    time::{Duration, Instant},
};

use fake_tools::{FakeClaude, FakeRust, originals_dir, stream_json};
use serde_json::Value;

/// アプリが `claude` に渡す 6 つの環境変数（spec/claude-integration/invocation.md「環境変数」）。
/// テストの実行環境に左右されないように、決めた値を渡す。
const SIX_ENV_VARS: [(&str, &str); 6] = [
    ("HOME", "/Users/learner"),
    ("USER", "learner"),
    ("LOGNAME", "learner"),
    ("PATH", "/usr/bin:/bin:/usr/sbin:/sbin"),
    ("SHELL", "/bin/zsh"),
    ("TMPDIR", "/var/folders/xx/progrust-test/T/"),
];

const SYSTEM_PROMPT: &str = "あなたは Rust の先生です。\n完成したコードは返さないでください。";

/// `-p` の呼び出しの引数（invocation.md「起動のフラグ」「出力形式」の質疑応答の形）。
/// 空の文字列と、改行を含む文字列が、そのまま届くことも確かめる。
fn chat_args() -> Vec<&'static str> {
    vec![
        "-p",
        "--model",
        "haiku",
        "--safe-mode",
        "--setting-sources",
        "",
        "--strict-mcp-config",
        "--tools",
        "",
        "--disable-slash-commands",
        "--no-session-persistence",
        "--system-prompt",
        SYSTEM_PROMPT,
        "--output-format",
        "stream-json",
        "--verbose",
        "--include-partial-messages",
    ]
}

/// 日本語と改行を含む、大きめの指示文。
fn prompt() -> String {
    "## 質問\n所有権とは何ですか？\n".repeat(2_000)
}

fn six_env_vars() -> BTreeMap<String, String> {
    SIX_ENV_VARS
        .iter()
        .map(|(name, value)| (name.to_string(), value.to_string()))
        .collect()
}

/// 6 つの環境変数だけを渡して、偽の `claude` を起動する。
fn spawn_claude(fake: &FakeClaude, args: &[&str], stdin: Stdio) -> Child {
    Command::new(fake.path())
        .args(args)
        .env_clear()
        .envs(SIX_ENV_VARS)
        .stdin(stdin)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("偽の claude を起動できない")
}

fn run_version(fake: &FakeClaude) -> Output {
    spawn_claude(fake, &["--version"], Stdio::null())
        .wait_with_output()
        .unwrap()
}

/// `-p` の呼び出しを、標準入力に `stdin` を渡して実行する。
fn run_chat(fake: &FakeClaude, stdin: &str) -> Output {
    let mut child = spawn_claude(fake, &chat_args(), Stdio::piped());
    child
        .stdin
        .take()
        .unwrap()
        .write_all(stdin.as_bytes())
        .unwrap();
    child.wait_with_output().unwrap()
}

fn stdout_text(output: &Output) -> &str {
    std::str::from_utf8(&output.stdout).unwrap()
}

fn json_lines(text: &str) -> Vec<Value> {
    text.lines()
        .map(|line| serde_json::from_str(line).expect("JSON の行でない"))
        .collect()
}

/// `condition` が成り立つまで待つ。
fn wait_until(what: &str, condition: impl Fn() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(10);
    while !condition() {
        assert!(Instant::now() < deadline, "{what}まで待てなかった");
        thread::sleep(Duration::from_millis(20));
    }
}

#[test]
fn claude_version_succeeds() {
    let fake = FakeClaude::new();
    let output = run_version(&fake);
    assert!(output.status.success(), "{output:?}");
    assert_eq!(stdout_text(&output), "2.1.329 (Claude Code)\n");
}

#[test]
fn claude_returns_prepared_stream_json() {
    let fake = FakeClaude::new();
    let prepared =
        stream_json::answer(&["所有権は、", "値の持ち主が", "1 つだけという決まりです。"]);
    fake.set_stdout(&prepared);

    let output = run_chat(&fake, &prompt());
    assert!(output.status.success(), "{output:?}");
    assert_eq!(stdout_text(&output), prepared);
    assert!(output.stderr.is_empty());

    let lines = json_lines(stdout_text(&output));
    let fragments: Vec<&str> = lines
        .iter()
        .filter(|line| line["type"] == "stream_event")
        .filter_map(|line| line["event"]["delta"]["text"].as_str())
        .collect();
    assert_eq!(
        fragments,
        ["所有権は、", "値の持ち主が", "1 つだけという決まりです。"]
    );
    let result = lines.last().unwrap();
    assert_eq!(result["type"], "result");
    assert_eq!(result["is_error"], false);
    assert_eq!(
        result["result"],
        "所有権は、値の持ち主が1 つだけという決まりです。"
    );
}

#[test]
fn claude_returns_prepared_error() {
    let fake = FakeClaude::new();
    let prepared = stream_json::api_error(
        "authentication_failed",
        "Not logged in · Please run /login",
        None,
    );
    let warning = "Warning: this sign-in ends between 2026-12-01 and 2026-12-08.\n";
    fake.set_stdout(&prepared);
    fake.set_stderr(warning);
    fake.set_exit_code(1);

    let output = run_chat(&fake, &prompt());
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(stdout_text(&output), prepared);
    assert_eq!(std::str::from_utf8(&output.stderr).unwrap(), warning);

    let lines = json_lines(stdout_text(&output));
    let assistant = lines
        .iter()
        .find(|line| line["type"] == "assistant")
        .unwrap();
    assert_eq!(assistant["error"], "authentication_failed");
    let result = lines.last().unwrap();
    assert_eq!(result["type"], "result");
    assert_eq!(result["subtype"], "success");
    assert_eq!(result["is_error"], true);
    assert_eq!(result["result"], "Not logged in · Please run /login");
}

#[test]
fn claude_returns_prepared_exit_code_without_output() {
    let fake = FakeClaude::new();
    fake.set_stderr("Error: something went wrong\n");
    fake.set_exit_code(3);

    let output = run_chat(&fake, &prompt());
    assert_eq!(output.status.code(), Some(3));
    assert!(output.stdout.is_empty());
    assert_eq!(
        std::str::from_utf8(&output.stderr).unwrap(),
        "Error: something went wrong\n"
    );
}

/// 応答しない偽の `claude` を起動し、標準入力を読み終えたところまで待つ。
fn spawn_hanging_claude(fake: &FakeClaude) -> Child {
    fake.set_hang();
    let mut child = spawn_claude(fake, &chat_args(), Stdio::piped());
    child
        .stdin
        .take()
        .unwrap()
        .write_all(prompt().as_bytes())
        .unwrap();
    wait_until("標準入力を読み終える", || {
        fake.calls()
            .first()
            .is_some_and(|call| call.stdin.is_some())
    });
    child
}

/// 止まったまま終了していないことを確かめてから、終了させる。
fn assert_still_running_then_kill(child: &mut Child) {
    thread::sleep(Duration::from_millis(300));
    assert!(
        child.try_wait().unwrap().is_none(),
        "応答しないはずの偽の claude が終了した"
    );
    child.kill().unwrap();
    child.wait().unwrap();
}

#[test]
fn claude_can_stop_responding() {
    let fake = FakeClaude::new();
    let mut child = spawn_hanging_claude(&fake);
    assert_still_running_then_kill(&mut child);

    let mut stdout = Vec::new();
    child
        .stdout
        .take()
        .unwrap()
        .read_to_end(&mut stdout)
        .unwrap();
    assert!(stdout.is_empty(), "何も出力しないはず");
}

#[test]
fn claude_can_stop_responding_after_partial_output() {
    let fake = FakeClaude::new();
    let prepared = stream_json::answer(&["所有権は、"]);
    let first_line = prepared.lines().next().unwrap();
    fake.set_stdout(&format!("{first_line}\n"));
    let mut child = spawn_hanging_claude(&fake);

    let mut stdout = BufReader::new(child.stdout.take().unwrap());
    let mut line = String::new();
    stdout.read_line(&mut line).unwrap();
    assert_eq!(line, format!("{first_line}\n"));
    assert_still_running_then_kill(&mut child);
}

#[test]
fn claude_records_args_env_and_stdin() {
    let fake = FakeClaude::new();
    fake.set_stdout(&stream_json::answer(&["はい。"]));
    assert!(run_version(&fake).status.success());
    let prompt = prompt();
    assert!(run_chat(&fake, &prompt).status.success());

    let calls = fake.calls();
    assert_eq!(calls.len(), 2, "{calls:?}");
    assert_eq!(calls[0].args, ["--version"]);
    assert_eq!(calls[0].env, six_env_vars());
    assert_eq!(calls[0].stdin, None, "--version では標準入力を読まない");
    assert_eq!(calls[1].args, chat_args());
    assert_eq!(calls[1].env, six_env_vars());
    assert_eq!(calls[1].stdin.as_deref(), Some(prompt.as_str()));
}

/// 同じ 6 つの環境変数を渡しても、偽の `claude` ごとに用意した出力が返る。
/// 返す出力を環境変数で決めていないことを確かめる。
#[test]
fn claude_output_does_not_depend_on_env_vars() {
    let first = FakeClaude::new();
    first.set_stdout(&stream_json::answer(&["1 つ目の回答"]));
    let second = FakeClaude::new();
    second.set_stdout(&stream_json::api_error(
        "model_not_found",
        "There's an issue with the selected model (claude-nonexistent-model-9).",
        Some(404),
    ));
    second.set_exit_code(1);

    for (fake, expected_code) in [(&first, 0), (&second, 1)] {
        let output = run_chat(fake, &prompt());
        assert_eq!(output.status.code(), Some(expected_code));
        assert_eq!(
            stdout_text(&output),
            fs::read_to_string(fake.path().with_file_name("stdout")).unwrap()
        );
        assert_eq!(fake.calls()[0].env, six_env_vars());
    }
}

#[test]
fn rustc_and_cargo_return_prepared_version() {
    let fake = FakeRust::new("1.99.0");
    for (tool, path) in [("rustc", fake.rustc()), ("cargo", fake.cargo())] {
        let output = Command::new(path).arg("--version").output().unwrap();
        assert!(output.status.success(), "{output:?}");
        assert_eq!(
            stdout_text(&output),
            format!("{tool} 1.99.0 (0123456789 2026-01-01)\n")
        );
    }
    assert_eq!(fake.rustc().parent(), fake.cargo().parent());
}

// ---- 偽のツールがアプリに同梱されないこと ----

fn manifest_dir() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

/// `.` と `..` を、ファイルシステムを見ずに取り除く。
fn normalize(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                normalized.pop();
            }
            other => normalized.push(other),
        }
    }
    normalized
}

/// 同梱するファイルの指定（`src-tauri/` からの相対パス。glob を含んでよい）が、
/// 偽のツールのディレクトリを含みうるか。指定がその中を指すときと、その親を指すとき
/// （ディレクトリごと、または glob で含むとき）に含みうるとする。
fn may_bundle_fake_tools(entry: &str) -> bool {
    let base = entry.split(['*', '?', '[', '{']).next().unwrap();
    let base = normalize(&manifest_dir().join(base));
    let fakes = normalize(&originals_dir());
    base.starts_with(&fakes) || fakes.starts_with(&base)
}

/// `tauri.conf.json` の `bundle` の、アプリに同梱するファイルの指定。
fn bundled_entries(bundle: &Value) -> Vec<String> {
    let strings = |value: &Value| -> Vec<String> {
        match value {
            Value::Null => Vec::new(),
            Value::String(entry) => vec![entry.clone()],
            Value::Array(entries) => entries
                .iter()
                .map(|entry| entry.as_str().expect("文字列の指定").to_string())
                .collect(),
            other => panic!("想定していない指定: {other}"),
        }
    };
    let mut entries = Vec::new();
    // `resources` は、パスの配列か、同梱元から同梱先への対応。
    match &bundle["resources"] {
        Value::Object(map) => entries.extend(map.keys().cloned()),
        other => entries.extend(strings(other)),
    }
    entries.extend(strings(&bundle["externalBin"]));
    entries.extend(strings(&bundle["macOS"]["frameworks"]));
    // `macOS.files` は、同梱先から同梱元への対応。
    if let Value::Object(map) = &bundle["macOS"]["files"] {
        entries.extend(
            map.values()
                .map(|source| source.as_str().expect("文字列の指定").to_string()),
        );
    }
    entries
}

#[test]
fn may_bundle_fake_tools_detects_entries_that_include_them() {
    assert!(may_bundle_fake_tools("tests/fake_tools/claude"));
    assert!(may_bundle_fake_tools("./tests/"));
    assert!(may_bundle_fake_tools("tests/**/*"));
    assert!(may_bundle_fake_tools("../**/*"));
    assert!(may_bundle_fake_tools("../src-tauri"));
    assert!(!may_bundle_fake_tools("../content/"));
    assert!(!may_bundle_fake_tools("../prompts/**/*"));
    assert!(!may_bundle_fake_tools("icons/icon.icns"));
}

#[test]
fn fake_tools_are_not_bundled() {
    let config: Value =
        serde_json::from_str(&fs::read_to_string(manifest_dir().join("tauri.conf.json")).unwrap())
            .unwrap();
    for entry in bundled_entries(&config["bundle"]) {
        assert!(
            !may_bundle_fake_tools(&entry),
            "tauri.conf.json の bundle の {entry} が偽のツールを同梱しうる"
        );
    }

    // バイナリに埋め込んだり、Cargo のビルドの対象にしたりしていない。
    let mut sources = vec![
        manifest_dir().join("Cargo.toml"),
        manifest_dir().join("build.rs"),
    ];
    let mut dirs = vec![manifest_dir().join("src")];
    while let Some(dir) = dirs.pop() {
        for entry in fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                dirs.push(path);
            } else if path.extension().is_some_and(|extension| extension == "rs") {
                sources.push(path);
            }
        }
    }
    for source in sources {
        let text = fs::read_to_string(&source).unwrap();
        assert!(
            !text.contains("fake_tools"),
            "{} が偽のツールを参照している",
            source.display()
        );
    }
}
