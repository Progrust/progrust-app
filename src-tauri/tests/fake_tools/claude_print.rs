//! テスト用の偽の `claude`（spec/non-functional/testing.md「偽の claude と偽の cargo」）の、
//! `-p` の呼び出しの出力と動きを決める手助けと、`-p` の呼び出しで書き残したプロセスと
//! 作業ディレクトリの記録を読む手助け。
//!
//! `claude.rs` を `claude` の名前で読み込んだモジュールの隣に読み込む（`claude.rs` の先頭の説明）。

use std::fs;

use serde_json::json;

use super::claude::FakeClaude;

impl FakeClaude {
    /// `-p` の呼び出しで標準出力に書く内容。
    pub fn set_stdout(&self, stdout: &str) {
        self.write_beside("stdout", stdout);
    }

    /// `-p` の呼び出しで標準エラー出力に書く内容。
    pub fn set_stderr(&self, stderr: &str) {
        self.write_beside("stderr", stderr);
    }

    /// `-p` の呼び出しの終了コード。
    pub fn set_exit_code(&self, code: i32) {
        self.write_beside("exit-code", code.to_string());
    }

    /// `-p` の呼び出しで、標準出力と標準エラー出力を書いた後、終了せずに止まる（応答しない）。
    pub fn set_hang(&self) {
        self.write_beside("hang", "");
    }

    /// `-p` の呼び出しで、[`FakeClaude::set_stdout`] の内容を書いた後、[`FakeClaude::go`] を
    /// 呼ぶまで待ってから、`rest` を標準出力に書く（出力を 2 回に分けて書く）。
    pub fn set_stdout_rest(&self, rest: &str) {
        self.write_beside("stdout-rest", rest);
    }

    /// [`FakeClaude::set_stdout_rest`] の続きを書かせる。
    pub fn go(&self) {
        self.write_beside("go", "");
    }

    /// `-p` の呼び出しで、SIGTERM を無視する（SIGKILL でだけ止まる）。
    pub fn set_ignore_sigterm(&self) {
        self.write_beside("ignore-sigterm", "");
    }

    /// `-p` の呼び出しで、同じプロセスグループに、終了しない子のプロセスを 1 つ作る。
    pub fn set_spawn_child(&self) {
        self.write_beside("spawn-child", "");
    }

    /// `number` 回目（1 から数える）の呼び出しが `-p` のときに書き残した、プロセスと
    /// 作業ディレクトリの記録。まだ書いていないとき、`--version` の呼び出しのときは `None`。
    pub fn process_record(&self, number: usize) -> Option<ProcessRecord> {
        let dir = self.call_dir(number);
        let read = |name: &str| fs::read_to_string(dir.join(name)).ok();
        // 偽の claude は、cwd-id を、pid・pgid・child-pid・cwd-entries の後に書く。
        let cwd_id = read("cwd-id")?;
        let (dev, ino) = cwd_id.split_once(' ').expect("cwd-id の形が違う");
        let number = |text: String| -> i32 { text.parse().expect("pid の記録の形が違う") };
        let mut cwd_entries: Vec<String> =
            read("cwd-entries")?.split('\0').map(String::from).collect();
        assert_eq!(
            cwd_entries.pop().as_deref(),
            Some(""),
            "記録が NUL で終わらない"
        );
        Some(ProcessRecord {
            pid: number(read("pid")?),
            pgid: number(read("pgid")?),
            child_pid: read("child-pid").map(number),
            cwd_entries,
            cwd_id: (
                dev.parse().expect("cwd-id の形が違う"),
                ino.parse().expect("cwd-id の形が違う"),
            ),
        })
    }
}

/// 偽の `claude` が `-p` の呼び出しで書き残した、プロセスと作業ディレクトリの記録。
#[derive(Debug)]
pub struct ProcessRecord {
    pub pid: i32,
    pub pgid: i32,
    /// [`FakeClaude::set_spawn_child`] で作った子のプロセスの pid。
    pub child_pid: Option<i32>,
    /// 起動したときに作業ディレクトリにあったファイルの名前（名前の順）。
    pub cwd_entries: Vec<String>,
    /// 作業ディレクトリの、デバイスの番号と inode の番号。
    pub cwd_id: (u64, u64),
}

/// 偽の `claude` に返させる stream-json の出力（spec/claude-integration/invocation.md「出力形式」）。
/// 行の種類と順は、実物の `claude -p` の出力（documents/verification/issue-9-claude-output-and-errors.md）に
/// 合わせる。各行には、アプリが読む項目と、行を見分ける項目だけを入れ、実物にあるほかの項目
/// （`usage`、`uuid` など）は省く。エラーの `assistant` の行で `error` と `is_api_error_message` を
/// 行の一番外側に、`model` を `message` の中に置くことは、Claude Code 2.1.329 の未ログインの出力で
/// 確かめた。Claude Code の出力の形が変わったら、ここを直す。
pub mod stream_json {
    use super::json;
    use serde_json::Value;

    const SESSION_ID: &str = "00000000-0000-4000-8000-000000000000";

    fn lines(values: impl IntoIterator<Item = Value>) -> String {
        values
            .into_iter()
            .map(|value| format!("{value}\n"))
            .collect()
    }

    fn system_init() -> Value {
        json!({ "type": "system", "subtype": "init", "session_id": SESSION_ID })
    }

    fn stream_event(event: Value) -> Value {
        json!({
            "type": "stream_event",
            "event": event,
            "session_id": SESSION_ID,
            "parent_tool_use_id": null,
        })
    }

    fn assistant(model: &str, text: &str) -> Value {
        json!({
            "type": "assistant",
            "message": {
                "model": model,
                "role": "assistant",
                "content": [{ "type": "text", "text": text }],
            },
            "parent_tool_use_id": null,
            "session_id": SESSION_ID,
        })
    }

    fn result(is_error: bool, text: &str, api_error_status: Option<u16>) -> Value {
        json!({
            "type": "result",
            // エラーのときも `success` になる（errors.md「エラーかどうかの判定」）。
            "subtype": "success",
            "is_error": is_error,
            "result": text,
            "api_error_status": api_error_status,
            "terminal_reason": if is_error { "api_error" } else { "completed" },
            "num_turns": 1,
            "session_id": SESSION_ID,
        })
    }

    /// 回答の断片 1 つの `stream_event` の行（`content_block_delta`）。
    pub fn fragment(text: &str) -> String {
        lines([stream_event(json!({
            "type": "content_block_delta",
            "index": 0,
            "delta": { "type": "text_delta", "text": text },
        }))])
    }

    /// `--include-partial-messages` を付けた呼び出しで、回答の断片が出始めるまでの行。
    pub fn answer_start() -> String {
        lines([
            system_init(),
            stream_event(json!({ "type": "message_start" })),
            stream_event(json!({
                "type": "content_block_start",
                "index": 0,
                "content_block": { "type": "text", "text": "" },
            })),
        ])
    }

    /// 回答の断片の後に続き、成功で終わる行。`text` は回答の全文。
    pub fn answer_end(text: &str) -> String {
        lines([
            assistant("claude-haiku-5-5", text),
            stream_event(json!({ "type": "content_block_stop", "index": 0 })),
            stream_event(json!({ "type": "message_delta" })),
            stream_event(json!({ "type": "message_stop" })),
            json!({
                "type": "rate_limit_event",
                "rate_limit_info": { "status": "allowed", "rateLimitType": "five_hour" },
                "session_id": SESSION_ID,
            }),
            result(false, text, None),
        ])
    }

    /// `--include-partial-messages` を付けた呼び出しで、回答が `fragments` の断片に分かれて
    /// 届き、成功で終わる出力。
    pub fn answer(fragments: &[&str]) -> String {
        let mut output = answer_start();
        for text in fragments {
            output.push_str(&fragment(text));
        }
        output.push_str(&answer_end(&fragments.concat()));
        output
    }

    /// API のエラー（`assistant` の行の `error` が `error`。errors.md「分類」）で終わる出力。
    /// `message` は、エラーの文（例: `Not logged in · Please run /login`）。
    pub fn api_error(error: &str, message: &str, api_error_status: Option<u16>) -> String {
        let mut assistant = assistant("<synthetic>", message);
        assistant["error"] = json!(error);
        assistant["is_api_error_message"] = json!(true);
        lines([
            system_init(),
            assistant,
            result(true, message, api_error_status),
        ])
    }
}
