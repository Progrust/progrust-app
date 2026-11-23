//! 試行ログに保存する出力の整え方（spec/exercise-runner/attempt-log.md）。
//!
//! - `stdout`・`stderr` は、それぞれ先頭 [`OUTPUT_LIMIT_CHARS`] 文字までを保存する。
//! - `stderr` は、ビルドのコンパイラの診断、ビルドの cargo 自身の標準エラー出力、プログラムの標準エラー出力を、
//!   この順に連結してから切り詰める（ADR 0138。コンパイラの診断が出ない失敗でも、cargo の出力が先頭に残る）。
//! - ビルドの出力は、ANSI エスケープを除去する。プログラムの出力は、そのまま保存する。

/// `stdout`・`stderr` を保存する上限の文字数。
pub const OUTPUT_LIMIT_CHARS: usize = 20_000;

/// ビルドの出力（spec/exercise-runner/run.md「ビルドの出力の表示」）。ANSI エスケープを含んでいてよい。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BuildOutput {
    /// コンパイラの診断（cargo の標準出力の JSON のうち、`compiler-message` の診断を描いたもの）を、受け取った順に連結したもの。
    pub compiler_messages: String,
    /// ビルドの cargo 自身の標準エラー出力。
    pub cargo_stderr: String,
}

/// 試行ログの `stderr` を作る。ビルドの出力の ANSI エスケープを除き、プログラムの標準エラー出力をその後ろに付け、
/// 先頭 [`OUTPUT_LIMIT_CHARS`] 文字までにする。
pub fn attempt_stderr(build: &BuildOutput, program_stderr: &str) -> String {
    let mut stderr = strip_ansi(&build.compiler_messages);
    stderr.push_str(&strip_ansi(&build.cargo_stderr));
    stderr.push_str(program_stderr);
    truncate_chars(stderr, OUTPUT_LIMIT_CHARS)
}

/// 試行ログの `stdout` を作る。先頭 [`OUTPUT_LIMIT_CHARS`] 文字までにする。
pub fn attempt_stdout(stdout: String) -> String {
    truncate_chars(stdout, OUTPUT_LIMIT_CHARS)
}

/// `text` を先頭 `limit` 文字（Unicode のスカラー値の数）までにする。
fn truncate_chars(mut text: String, limit: usize) -> String {
    if let Some((index, _)) = text.char_indices().nth(limit) {
        text.truncate(index);
    }
    text
}

/// ANSI エスケープ（端末の色や太字などの指定）を除く。
///
/// - CSI（`ESC [` に続く引数の文字と中間の文字、最後の 1 文字）。色の指定（`ESC [ 1 ; 31 m` など）はこれである。
/// - OSC（`ESC ]` から BEL か `ESC \` まで）。端末へのリンクなど。
/// - 中間の文字を持つもの（`ESC ( B` など）。
/// - そのほかの `ESC` と次の 1 文字。
fn strip_ansi(text: &str) -> String {
    const ESC: char = '\u{1b}';
    const BEL: char = '\u{7}';
    let mut stripped = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if c != ESC {
            stripped.push(c);
            continue;
        }
        match chars.next() {
            Some('[') => {
                // 引数の文字（0x30〜0x3F）と中間の文字（0x20〜0x2F）を読み、最後の文字（0x40〜0x7E）で終わる。
                while chars
                    .next_if(|c| ('\u{20}'..='\u{3f}').contains(c))
                    .is_some()
                {}
                chars.next_if(|c| ('\u{40}'..='\u{7e}').contains(c));
            }
            Some(']') => {
                while let Some(c) = chars.next() {
                    if c == BEL {
                        break;
                    }
                    if c == ESC {
                        chars.next_if_eq(&'\\');
                        break;
                    }
                }
            }
            // 中間の文字（0x20〜0x2F）を持つもの（文字集合の指定 `ESC ( B` など）は、最後の文字（0x30〜0x7E）まで。
            Some(c) if ('\u{20}'..='\u{2f}').contains(&c) => {
                while chars
                    .next_if(|c| ('\u{20}'..='\u{2f}').contains(c))
                    .is_some()
                {}
                chars.next_if(|c| ('\u{30}'..='\u{7e}').contains(c));
            }
            Some(_) | None => {}
        }
    }
    stripped
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn color_codes_are_removed_from_compiler_messages() {
        let rendered = "\u{1b}[0m\u{1b}[1m\u{1b}[38;5;9merror[E0382]\u{1b}[0m\u{1b}[0m\u{1b}[1m: borrow of moved value: `s`\u{1b}[0m\n";
        assert_eq!(
            strip_ansi(rendered),
            "error[E0382]: borrow of moved value: `s`\n"
        );
    }

    #[test]
    fn other_escapes_are_removed_and_plain_text_is_kept() {
        assert_eq!(
            strip_ansi("前\u{1b}]8;;https://x\u{7}リンク\u{1b}]8;;\u{1b}\\後"),
            "前リンク後"
        );
        assert_eq!(strip_ansi("a\u{1b}[2Kb\u{1b}(Bc"), "abc");
        assert_eq!(strip_ansi("末尾\u{1b}"), "末尾");
        assert_eq!(strip_ansi("日本語のまま\n"), "日本語のまま\n");
    }

    #[test]
    fn stderr_is_compiler_messages_then_cargo_then_program() {
        let build = BuildOutput {
            compiler_messages: "\u{1b}[1merror[E0382]\u{1b}[0m: 診断\n".into(),
            cargo_stderr: "\u{1b}[31merror\u{1b}[0m: could not compile `exercise`\n".into(),
        };
        assert_eq!(
            attempt_stderr(&build, "\u{1b}[33mプログラム\u{1b}[0m\n"),
            "error[E0382]: 診断\nerror: could not compile `exercise`\n\u{1b}[33mプログラム\u{1b}[0m\n"
        );
    }

    /// 上限は文字数で数える（日本語の 1 文字も 1 文字）。連結してから切り詰めるので、前のものが先に残る。
    #[test]
    fn outputs_keep_the_first_20000_characters() {
        let long = "あ".repeat(OUTPUT_LIMIT_CHARS + 5);
        assert_eq!(
            attempt_stdout(long.clone()),
            "あ".repeat(OUTPUT_LIMIT_CHARS)
        );
        let exact = "x".repeat(OUTPUT_LIMIT_CHARS);
        assert_eq!(attempt_stdout(exact.clone()), exact);

        let build = BuildOutput {
            compiler_messages: String::new(),
            cargo_stderr: "error: 設定ファイルを読み込めない\n".into(),
        };
        let stderr = attempt_stderr(&build, &long);
        assert_eq!(stderr.chars().count(), OUTPUT_LIMIT_CHARS);
        assert!(
            stderr.starts_with("error: 設定ファイルを読み込めない\nあ"),
            "{}",
            &stderr[..60]
        );

        // ANSI エスケープは、除いた後の文字数で数える。
        let colored = BuildOutput {
            compiler_messages: format!("\u{1b}[1m{}\u{1b}[0m", "e".repeat(OUTPUT_LIMIT_CHARS)),
            cargo_stderr: String::new(),
        };
        assert_eq!(
            attempt_stderr(&colored, "後"),
            "e".repeat(OUTPUT_LIMIT_CHARS)
        );
    }
}
