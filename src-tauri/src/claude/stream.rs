//! `claude -p` の stream-json の出力の読み方と、エラーの分類
//! （spec/claude-integration/invocation.md「出力形式」、spec/claude-integration/errors.md）。

use serde::Serialize;
use serde_json::Value;

/// AI 呼び出しのエラーの分類（spec/architecture/commands/conventions.md「AI 呼び出しのエラー」の
/// `category`）。未検出は、呼び出す前にコマンドが `claude_unavailable` を返し、出力不正は各機能の
/// 出力検証で決めるので、ここには無い。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCategory {
    /// 未ログイン。
    NotLoggedIn,
    /// 利用上限。
    RateLimited,
    /// タイムアウト。
    Timeout,
    /// その他。
    Other,
}

/// AI 呼び出しのエラー（spec/architecture/commands/conventions.md「AI 呼び出しのエラー」の形）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ClaudeError {
    pub category: ErrorCategory,
    /// 「その他」の詳細。ほかの分類では `None`。
    pub detail: Option<String>,
}

impl ClaudeError {
    /// 詳細 `detail` の「その他」のエラー。
    pub fn other(detail: impl Into<String>) -> Self {
        Self {
            category: ErrorCategory::Other,
            detail: Some(detail.into()),
        }
    }

    fn without_detail(category: ErrorCategory) -> Self {
        Self {
            category,
            detail: None,
        }
    }

    /// タイムアウト。
    pub fn timeout() -> Self {
        Self::without_detail(ErrorCategory::Timeout)
    }
}

/// `result` の行から読んだもの。
#[derive(Debug, Clone, PartialEq, Eq)]
struct ResultLine {
    is_error: bool,
    /// `result` の文。無いとき、文字列でないときは空。
    text: String,
    /// この行より前の `assistant` の行の `error` の値（最後のもの）。
    assistant_error: Option<String>,
}

/// 標準出力の行を 1 行ずつ読んだ状態。
#[derive(Debug, Default)]
pub(crate) struct StreamReader {
    assistant_error: Option<String>,
    result: Option<ResultLine>,
}

impl StreamReader {
    /// 標準出力の 1 行（改行を除く）を読む。`stream_event` の行の `event.delta.text` があれば、
    /// 回答の断片として返す。JSON として読めない行と、知らない種類の行は飛ばす。
    pub(crate) fn read_line(&mut self, line: &[u8]) -> Option<String> {
        let Ok(value) = serde_json::from_slice::<Value>(line) else {
            return None;
        };
        match value.get("type").and_then(Value::as_str) {
            Some("stream_event") => value
                .pointer("/event/delta/text")
                .and_then(Value::as_str)
                .map(String::from),
            Some("assistant") => {
                if let Some(error) = value.get("error").and_then(Value::as_str) {
                    self.assistant_error = Some(error.to_string());
                }
                None
            }
            Some("result") => {
                // `subtype` はエラーのときも `success` になるので見ない（errors.md「エラーかどうかの判定」）。
                self.result = Some(ResultLine {
                    is_error: value.get("is_error").and_then(Value::as_bool) == Some(true),
                    text: value
                        .get("result")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .to_string(),
                    assistant_error: self.assistant_error.clone(),
                });
                None
            }
            _ => None,
        }
    }

    /// `claude` が（止められずに）終わった後に、終わり方を決める。完了なら、最後の `result` の行の
    /// `result` の文を返す。終了コードは分類に使わない（errors.md「エラーかどうかの判定」）。
    ///
    /// - `stderr_head`: 標準エラー出力の先頭。「その他」の詳細の補助にする。
    /// - `exit`: `result` の行が無いときに詳細に書く、`claude` の終わり方（例: `終了コード 1`）。
    pub(crate) fn finish(self, stderr_head: &str, exit: &str) -> Result<String, ClaudeError> {
        let Some(result) = self.result else {
            return Err(ClaudeError::other(other_detail(
                &format!("claude が result の行を出さずに終わりました（{exit}）。"),
                stderr_head,
            )));
        };
        if !result.is_error {
            return Ok(result.text);
        }
        Err(match result.assistant_error.as_deref() {
            Some("authentication_failed") => {
                ClaudeError::without_detail(ErrorCategory::NotLoggedIn)
            }
            Some("rate_limit" | "billing_error") => {
                ClaudeError::without_detail(ErrorCategory::RateLimited)
            }
            _ => ClaudeError::other(other_detail(&result.text, stderr_head)),
        })
    }
}

/// 「その他」の詳細。`main`（`result` の文など）を基本にし、標準エラー出力の先頭を補助として添える
/// （errors.md「分類」の下の決まり）。
fn other_detail(main: &str, stderr_head: &str) -> String {
    let main = main.trim();
    let stderr = stderr_head.trim();
    match (main.is_empty(), stderr.is_empty()) {
        (_, true) => main.to_string(),
        (true, false) => format!("標準エラー出力:\n{stderr}"),
        (false, false) => format!("{main}\n\n標準エラー出力:\n{stderr}"),
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn read_all(lines: &[Value]) -> (StreamReader, Vec<String>) {
        let mut reader = StreamReader::default();
        let fragments = lines
            .iter()
            .filter_map(|line| reader.read_line(line.to_string().as_bytes()))
            .collect();
        (reader, fragments)
    }

    fn delta(text: &str) -> Value {
        json!({
            "type": "stream_event",
            "event": { "type": "content_block_delta", "index": 0, "delta": { "type": "text_delta", "text": text } },
        })
    }

    fn assistant_error(error: &str) -> Value {
        json!({ "type": "assistant", "error": error, "message": { "model": "<synthetic>" } })
    }

    fn result(is_error: bool, text: &str) -> Value {
        json!({ "type": "result", "subtype": "success", "is_error": is_error, "result": text })
    }

    fn finish(lines: &[Value]) -> Result<String, ClaudeError> {
        read_all(lines).0.finish("", "終了コード 1")
    }

    #[test]
    fn fragments_are_returned_in_order_and_the_result_text_completes() {
        let (reader, fragments) = read_all(&[
            json!({ "type": "system", "subtype": "init" }),
            json!({ "type": "stream_event", "event": { "type": "message_start" } }),
            delta("所有権は、"),
            delta("1 つだけ"),
            json!({ "type": "assistant", "message": { "content": [] } }),
            json!({ "type": "rate_limit_event" }),
            result(false, "所有権は、1 つだけ"),
        ]);
        assert_eq!(fragments, ["所有権は、", "1 つだけ"]);
        assert_eq!(reader.finish("", ""), Ok("所有権は、1 つだけ".to_string()));
    }

    #[test]
    fn unreadable_and_unknown_lines_are_skipped() {
        let mut reader = StreamReader::default();
        for line in [
            &b"not json"[..],
            b"",
            b"\xff\xfe",
            b"[1, 2]",
            b"{\"type\": 3}",
            b"{\"type\": \"system\", \"subtype\": \"hook_started\"}",
            b"{\"type\": \"stream_event\", \"event\": {\"delta\": {\"text\": 1}}}",
            b"{\"no_type\": true}",
        ] {
            assert_eq!(reader.read_line(line), None);
        }
        assert!(reader.result.is_none());
        reader.read_line(result(false, "ok").to_string().as_bytes());
        assert_eq!(reader.finish("", ""), Ok("ok".to_string()));
    }

    #[test]
    fn errors_are_classified_by_the_assistant_error() {
        for (error, category) in [
            ("authentication_failed", ErrorCategory::NotLoggedIn),
            ("rate_limit", ErrorCategory::RateLimited),
            ("billing_error", ErrorCategory::RateLimited),
        ] {
            let outcome = finish(&[assistant_error(error), result(true, "message")]);
            assert_eq!(
                outcome,
                Err(ClaudeError {
                    category,
                    detail: None
                }),
                "{error}"
            );
        }
        for error in ["model_not_found", "overloaded", "unknown"] {
            let outcome = finish(&[assistant_error(error), result(true, "There's an issue")]);
            assert_eq!(
                outcome,
                Err(ClaudeError::other("There's an issue")),
                "{error}"
            );
        }
        // `assistant` の行に `error` が無いときも、その他になる。
        assert_eq!(
            finish(&[result(true, "API Error")]),
            Err(ClaudeError::other("API Error"))
        );
    }

    /// `subtype` は見ない。`is_error` が `true` でなければ完了になる。
    #[test]
    fn only_is_error_decides_whether_it_failed() {
        let mut line = result(false, "done");
        line["subtype"] = json!("error_during_execution");
        assert_eq!(finish(&[line]), Ok("done".to_string()));
        let mut line = result(true, "failed");
        line["subtype"] = json!("success");
        assert_eq!(finish(&[line]), Err(ClaudeError::other("failed")));
        // `is_error` が無いときと、真偽値でないときは、エラーとしない。
        assert_eq!(
            finish(&[json!({ "type": "result", "result": "text" })]),
            Ok("text".to_string())
        );
        assert_eq!(
            finish(&[json!({ "type": "result", "is_error": "true", "result": "text" })]),
            Ok("text".to_string())
        );
        // `result` の文が無い完了は、空の文になる。
        assert_eq!(
            finish(&[json!({ "type": "result", "is_error": false })]),
            Ok(String::new())
        );
    }

    /// 分類には、`result` の行より前の `assistant` の行を使う。
    #[test]
    fn the_assistant_error_after_the_result_line_is_not_used() {
        let outcome = finish(&[
            result(true, "message"),
            assistant_error("authentication_failed"),
        ]);
        assert_eq!(outcome, Err(ClaudeError::other("message")));
    }

    #[test]
    fn the_last_result_line_is_used() {
        assert_eq!(
            finish(&[result(true, "first"), result(false, "second")]),
            Ok("second".to_string())
        );
    }

    #[test]
    fn a_missing_result_line_is_other() {
        assert_eq!(
            StreamReader::default().finish("", "終了コード 1"),
            Err(ClaudeError::other(
                "claude が result の行を出さずに終わりました（終了コード 1）。"
            ))
        );
        let (reader, _) = read_all(&[delta("途中まで")]);
        assert_eq!(
            reader.finish("Error: boom\n", "シグナル 9"),
            Err(ClaudeError::other(
                "claude が result の行を出さずに終わりました（シグナル 9）。\n\n標準エラー出力:\nError: boom"
            ))
        );
    }

    #[test]
    fn the_other_detail_adds_the_stderr_head() {
        let (reader, _) = read_all(&[
            assistant_error("model_not_found"),
            result(true, "bad model"),
        ]);
        assert_eq!(
            reader.finish("[claude-code:unrecognized_model] {}\n", ""),
            Err(ClaudeError::other(
                "bad model\n\n標準エラー出力:\n[claude-code:unrecognized_model] {}"
            ))
        );
        let (reader, _) = read_all(&[result(true, "  ")]);
        assert_eq!(
            reader.finish("warning\n", ""),
            Err(ClaudeError::other("標準エラー出力:\nwarning"))
        );
    }

    /// `category` は commands/conventions.md「AI 呼び出しのエラー」の値で表し、その他以外の
    /// `detail` は `null` になる。
    #[test]
    fn errors_serialize_to_the_command_shape() {
        for (error, category) in [
            (
                ClaudeError::without_detail(ErrorCategory::NotLoggedIn),
                "not_logged_in",
            ),
            (
                ClaudeError::without_detail(ErrorCategory::RateLimited),
                "rate_limited",
            ),
            (ClaudeError::timeout(), "timeout"),
        ] {
            assert_eq!(
                serde_json::to_value(error).unwrap(),
                json!({ "category": category, "detail": null })
            );
        }
        assert_eq!(
            serde_json::to_value(ClaudeError::other("詳細")).unwrap(),
            json!({ "category": "other", "detail": "詳細" })
        );
    }
}
