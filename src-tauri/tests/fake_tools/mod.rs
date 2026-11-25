//! テスト用の偽の claude・cargo・rustc（spec/non-functional/testing.md「偽の claude と偽の cargo」）を、
//! テストごとの一時ディレクトリに置いて使うための手助け。
//!
//! 偽のツールの原本（このディレクトリの `claude`・`cargo`・`rustc`）を指すシンボリックリンクを
//! 一時ディレクトリに置き、返す出力をその隣のファイルに書く。実行ファイルを書き出してすぐに
//! 起動すると、テストを並べて実行したときに Linux で起動に失敗する（Text file busy）ことが
//! あるので、写さずにリンクにする。

use std::{
    fs,
    path::{Path, PathBuf},
};

use serde_json::json;

pub mod cargo_behavior;
pub mod claude;
pub mod rust;
pub use claude::FakeClaude;
pub use rust::FakeRust;

/// 偽のツールの原本を置くディレクトリ。
pub fn originals_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fake_tools")
}

fn write(path: &Path, content: impl AsRef<[u8]>) {
    fs::write(path, content).unwrap_or_else(|e| panic!("{} に書けない: {e}", path.display()));
}

/// `-p` の呼び出しの出力を決める。`--version` の出力を決めるもの（検出のテストでも使う）は、
/// `claude.rs` にある。
impl FakeClaude {
    /// `-p` の呼び出しで標準出力に書く内容。
    pub fn set_stdout(&self, stdout: &str) {
        write(&self.path().with_file_name("stdout"), stdout);
    }

    /// `-p` の呼び出しで標準エラー出力に書く内容。
    pub fn set_stderr(&self, stderr: &str) {
        write(&self.path().with_file_name("stderr"), stderr);
    }

    /// `-p` の呼び出しの終了コード。
    pub fn set_exit_code(&self, code: i32) {
        write(&self.path().with_file_name("exit-code"), code.to_string());
    }

    /// `-p` の呼び出しで、標準出力と標準エラー出力を書いた後、終了せずに止まる（応答しない）。
    pub fn set_hang(&self) {
        write(&self.path().with_file_name("hang"), "");
    }
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

    /// `--include-partial-messages` を付けた呼び出しで、回答が `fragments` の断片に分かれて
    /// 届き、成功で終わる出力。
    pub fn answer(fragments: &[&str]) -> String {
        let text = fragments.concat();
        let mut values = vec![
            system_init(),
            stream_event(json!({ "type": "message_start" })),
            stream_event(json!({
                "type": "content_block_start",
                "index": 0,
                "content_block": { "type": "text", "text": "" },
            })),
        ];
        values.extend(fragments.iter().map(|fragment| {
            stream_event(json!({
                "type": "content_block_delta",
                "index": 0,
                "delta": { "type": "text_delta", "text": fragment },
            }))
        }));
        values.extend([
            assistant("claude-haiku-5-5", &text),
            stream_event(json!({ "type": "content_block_stop", "index": 0 })),
            stream_event(json!({ "type": "message_delta" })),
            stream_event(json!({ "type": "message_stop" })),
            json!({
                "type": "rate_limit_event",
                "rate_limit_info": { "status": "allowed", "rateLimitType": "five_hour" },
                "session_id": SESSION_ID,
            }),
            result(false, &text, None),
        ]);
        lines(values)
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
