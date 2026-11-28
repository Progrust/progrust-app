//! C の演習の提出で、メモリの検査（AddressSanitizer・UndefinedBehaviorSanitizer）が書き出した報告のファイルから、
//! 誤りがあるかどうかと、誤りの種類（`kind`）・行（`location`）・報告の先頭（`report`）を取り出す部品
//! （spec/exercise-runner/c-submit.md「手順」の 3 の `memory_error` の行、「検査の報告の読み取り」「判定結果」、
//! ADR 0272）。
//!
//! - [`read_memory_error`] で、`reports/` の、名前が `case-{N}.` で始まるファイルを探し、誤りの行（`ERROR:
//!   AddressSanitizer` か `runtime error:` を含む行）があるファイルのうち、名前の後ろが起動したプロセスの
//!   プロセス ID のものを、無ければ名前の順で最初のものを読む。誤りの行のあるファイルが無ければ、`memory_error`
//!   にしない（`None`）。確保に失敗したときの警告（`WARNING: AddressSanitizer failed to allocate`）だけのファイルは、
//!   誤りの行が無いので、誤りにならない。
//! - [`parse_report`] で、1 つの報告から `kind`・`location`・`report` を決める。UndefinedBehaviorSanitizer の
//!   `runtime error:` の行を先に見て、無ければ AddressSanitizer の `SUMMARY:` の行の種類で決める。
//! - `kind` の `leak`（メモリリーク）は、メモリリークの検査の結果で決まるもので、この部品は返さない。
//! - 報告の書式は、技術検証（documents/verification/issue-293-parse-sanitizer-reports.md）で確かめた Apple clang の
//!   ものに合わせている。
//!
//! Tauri の実行時に頼らない（教材チェックツールの判定からも使う。ADR 0273）。

use std::fs;
use std::io;
use std::os::unix::ffi::OsStrExt;
use std::path::Path;

use serde::Serialize;

/// メモリの検査の報告を置くディレクトリ（`c-project/` からの相対パス）。
pub const REPORTS_DIR: &str = "reports";

/// 誤りの種類（c-submit.md「判定結果」の「誤りの種類」のうち、検査の報告から決まるもの）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MemoryErrorKind {
    /// 解放後の使用。
    UseAfterFree,
    /// 二重解放。
    DoubleFree,
    /// 範囲外アクセス。
    OutOfBounds,
    /// 戻った関数のローカル変数の使用。
    UseAfterReturn,
    /// 確保していない領域の解放。
    InvalidFree,
    /// NULL や無効なアドレスへのアクセス。
    InvalidAccess,
    /// 未定義動作。
    UndefinedBehavior,
    /// スタックのあふれ。
    StackOverflow,
    /// そのほかのメモリの誤り。
    Other,
}

impl MemoryErrorKind {
    /// 判定結果の `kind` の値。
    pub fn as_str(self) -> &'static str {
        match self {
            Self::UseAfterFree => "use_after_free",
            Self::DoubleFree => "double_free",
            Self::OutOfBounds => "out_of_bounds",
            Self::UseAfterReturn => "use_after_return",
            Self::InvalidFree => "invalid_free",
            Self::InvalidAccess => "invalid_access",
            Self::UndefinedBehavior => "undefined_behavior",
            Self::StackOverflow => "stack_overflow",
            Self::Other => "other",
        }
    }
}

/// 検査の報告から取り出したもの（c-submit.md「判定結果」の `memory_error` の `detail`）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MemoryError {
    pub kind: MemoryErrorKind,
    /// 誤りが起きた `main.c` の行（`main.c:{行番号}`）。報告から取り出せないときは `None`。
    pub location: Option<String>,
    /// 報告の先頭。区切りの行とレジスタの値の表を除き、行を改行でつなぐ。
    pub report: String,
}

/// `reports_dir`（`c-project/reports/`）から、`case_number` 番目（最初を 1 とする）のテストケースの検査の報告を
/// 読む。`pid` は、そのテストケースで起動した `main-checked` のプロセス ID。
///
/// 誤りの行のある報告のファイルが無ければ `None` を返す（`memory_error` にしない）。`reports_dir` が無いときも
/// `None` である。ディレクトリやファイルを読めなかったときは `Err` を返す。
pub fn read_memory_error(
    reports_dir: &Path,
    case_number: usize,
    pid: u32,
) -> io::Result<Option<MemoryError>> {
    let entries = match fs::read_dir(reports_dir) {
        Ok(entries) => entries,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error),
    };
    let prefix = report_file_prefix(case_number);
    let mut reports = Vec::new();
    for entry in entries {
        let entry = entry?;
        let name = entry.file_name();
        if !name.as_bytes().starts_with(prefix.as_bytes()) {
            continue;
        }
        let path = entry.path();
        // シンボリックリンクはたどらず、ふつうのファイルだけを読む。
        match fs::symlink_metadata(&path) {
            Ok(metadata) if metadata.is_file() => {}
            Ok(_) => continue,
            Err(error) if error.kind() == io::ErrorKind::NotFound => continue,
            Err(error) => return Err(error),
        }
        let text = match fs::read(&path) {
            Ok(bytes) => String::from_utf8_lossy(&bytes).into_owned(),
            Err(error) if error.kind() == io::ErrorKind::NotFound => continue,
            Err(error) => return Err(error),
        };
        reports.push((name.as_bytes().to_vec(), text));
    }
    let own = format!("{prefix}{pid}");
    Ok(select_report(&reports, own.as_bytes()).map(parse_report))
}

/// `case_number` 番目のテストケースの報告のファイルの名前の始まり（`case-{N}.`）。
fn report_file_prefix(case_number: usize) -> String {
    format!("case-{case_number}.")
}

/// 誤りの行のある報告（名前とテキストの組）のうち、名前が `own` のものを、無ければ名前の順で最初のものを選ぶ。
fn select_report<'a>(reports: &'a [(Vec<u8>, String)], own: &[u8]) -> Option<&'a str> {
    let mut candidates: Vec<&(Vec<u8>, String)> = reports
        .iter()
        .filter(|(_, text)| has_error_line(text))
        .collect();
    candidates.sort_by(|a, b| a.0.cmp(&b.0));
    candidates
        .iter()
        .find(|(name, _)| name.as_slice() == own)
        .or_else(|| candidates.first())
        .map(|(_, text)| text.as_str())
}

/// 報告に、誤りの行（`ERROR: AddressSanitizer` を含む行か `runtime error:` を含む行）があるか。
pub fn has_error_line(text: &str) -> bool {
    text.lines()
        .any(|line| line.contains("ERROR: AddressSanitizer") || line.contains("runtime error:"))
}

/// 1 つの報告から `kind`・`location`・`report` を決める（c-submit.md「検査の報告の読み取り」の 2〜5）。
pub fn parse_report(text: &str) -> MemoryError {
    let lines: Vec<&str> = text.lines().collect();
    let (kind, location) = if let Some(ubsan) = lines.iter().find_map(|line| parse_ubsan_line(line))
    {
        (ubsan.kind(), ubsan.location())
    } else if let Some(asan_type) = lines.iter().find_map(|line| parse_asan_summary(line)) {
        classify_asan(asan_type, &first_stack(&lines))
    } else {
        (MemoryErrorKind::Other, None)
    };
    MemoryError {
        kind,
        location,
        report: report_head(&lines),
    }
}

/// UndefinedBehaviorSanitizer の `{ファイル}:{行}:{桁}: runtime error: {文}` の行。
struct UbsanLine<'a> {
    file: &'a str,
    line: &'a str,
    message: &'a str,
}

impl UbsanLine<'_> {
    fn kind(&self) -> MemoryErrorKind {
        if self.message.starts_with("index ") && self.message.contains("out of bounds for type") {
            MemoryErrorKind::OutOfBounds
        } else if self.message.contains("null pointer") {
            MemoryErrorKind::InvalidAccess
        } else {
            MemoryErrorKind::UndefinedBehavior
        }
    }

    fn location(&self) -> Option<String> {
        (self.file == "main.c").then(|| format!("main.c:{}", self.line))
    }
}

/// `line` が `{ファイル}:{行}:{桁}: runtime error: {文}` の形なら、その部分を返す。
fn parse_ubsan_line(line: &str) -> Option<UbsanLine<'_>> {
    const MARKER: &str = ": runtime error: ";
    let at = line.find(MARKER)?;
    let message = &line[at + MARKER.len()..];
    // 前の部分を右から `{桁}`・`{行}` と切り、残りを `{ファイル}` とする。
    let mut parts = line[..at].rsplitn(3, ':');
    let column = parts.next()?;
    let line_number = parts.next()?;
    let file = parts.next()?;
    if file.is_empty() || !is_digits(line_number) || !is_digits(column) {
        return None;
    }
    Some(UbsanLine {
        file,
        line: line_number,
        message,
    })
}

/// `line` が `SUMMARY: AddressSanitizer: {種類}` の形なら、`{種類}`（空白までの語）を返す。
fn parse_asan_summary(line: &str) -> Option<&str> {
    let rest = line.strip_prefix("SUMMARY: AddressSanitizer: ")?;
    rest.split_whitespace().next()
}

/// 最初のスタック: 最初に `#0` で始まるフレームの行から、`#` で始まるフレームの行が続く範囲。
fn first_stack<'a>(lines: &[&'a str]) -> Vec<&'a str> {
    let Some(start) = lines
        .iter()
        .position(|line| frame_number(line) == Some("0"))
    else {
        return Vec::new();
    };
    lines[start..]
        .iter()
        .take_while(|line| frame_number(line).is_some())
        .copied()
        .collect()
}

/// フレームの行（空白の後に `#` と数字が続く行）なら、その番号を返す。
fn frame_number(line: &str) -> Option<&str> {
    let rest = line.trim_start().strip_prefix('#')?;
    let number = rest.split_whitespace().next()?;
    is_digits(number).then_some(number)
}

/// フレームの行の関数の名前から後ろ（` in ` の後ろ）。関数の名前は空白を含むことがある
/// （`__asan::Allocator::Deallocate(void*, unsigned long, …)+0x78`）。
fn frame_function(line: &str) -> Option<&str> {
    line.find(" in ").map(|at| &line[at + " in ".len()..])
}

/// フレームの行に `main.c:{行}` が出れば、`main.c:{行}` を返す。行の最後の語（`main.c:8` や `main.c:8:5`）で見る。
fn frame_main_c_location(line: &str) -> Option<String> {
    let last = line.split_whitespace().last()?;
    let rest = last.strip_prefix("main.c:")?;
    let line_number = rest.split(':').next()?;
    is_digits(line_number).then(|| format!("main.c:{line_number}"))
}

/// AddressSanitizer の `SUMMARY:` の種類と最初のスタックから、`kind` と `location` を決める。
fn classify_asan(asan_type: &str, stack: &[&str]) -> (MemoryErrorKind, Option<String>) {
    let main_c_frame = stack
        .iter()
        .enumerate()
        .find_map(|(index, line)| frame_main_c_location(line).map(|location| (index, location)));
    let location = main_c_frame.as_ref().map(|(_, location)| location.clone());
    let kind = match asan_type {
        "heap-use-after-free" => MemoryErrorKind::UseAfterFree,
        "double-free" => MemoryErrorKind::DoubleFree,
        "heap-buffer-overflow"
        | "stack-buffer-overflow"
        | "stack-buffer-underflow"
        | "global-buffer-overflow"
        | "dynamic-stack-buffer-overflow" => MemoryErrorKind::OutOfBounds,
        "stack-use-after-return" => MemoryErrorKind::UseAfterReturn,
        "bad-free" => MemoryErrorKind::InvalidFree,
        "SEGV" | "BUS" => {
            // `main.c` のフレームより前（`main.c` のフレームが無ければ、最初のスタックの全体）に、関数の名前が
            // `free` で始まるフレームがあれば、`free` の中で起きた誤りとする。
            let before = main_c_frame
                .as_ref()
                .map_or(stack.len(), |(index, _)| *index);
            let in_free = stack[..before]
                .iter()
                .any(|line| frame_function(line).is_some_and(|name| name.starts_with("free")));
            if in_free {
                MemoryErrorKind::InvalidFree
            } else {
                MemoryErrorKind::InvalidAccess
            }
        }
        "stack-overflow" => MemoryErrorKind::StackOverflow,
        _ => MemoryErrorKind::Other,
    };
    (kind, location)
}

/// 報告の先頭から `SUMMARY:` の行まで（無ければ全体）のうち、`=` だけの行と、レジスタの値の表を除いたもの。
fn report_head(lines: &[&str]) -> String {
    let end = lines
        .iter()
        .position(|line| line.starts_with("SUMMARY:"))
        .map_or(lines.len(), |at| at + 1);
    let mut kept = Vec::new();
    let mut in_registers = false;
    for line in &lines[..end] {
        if is_register_values_header(line) {
            in_registers = true;
            continue;
        }
        if in_registers {
            if is_register_values_row(line) {
                continue;
            }
            in_registers = false;
        }
        if is_separator(line) {
            continue;
        }
        kept.push(*line);
    }
    kept.join("\n")
}

/// `=` だけの行（`=` が 1 つ以上）。
fn is_separator(line: &str) -> bool {
    !line.is_empty() && line.chars().all(|c| c == '=')
}

/// `==`、プロセス ID、`==Register values:` の行。
fn is_register_values_header(line: &str) -> bool {
    line.strip_prefix("==")
        .and_then(|rest| rest.split_once("=="))
        .is_some_and(|(pid, rest)| is_digits(pid) && rest.trim_end() == "Register values:")
}

/// レジスタの値の行（`x[0] = 0x0000000000000010   x[1] = …` のように、`{名前} = 0x{16 進}` が並ぶ行）。
fn is_register_values_row(line: &str) -> bool {
    let words: Vec<&str> = line.split_whitespace().collect();
    !words.is_empty()
        && words.len().is_multiple_of(3)
        && words.chunks(3).all(|group| {
            group[1] == "="
                && group[2].strip_prefix("0x").is_some_and(|hex| {
                    !hex.is_empty() && hex.chars().all(|c| c.is_ascii_hexdigit())
                })
        })
}

fn is_digits(text: &str) -> bool {
    !text.is_empty() && text.bytes().all(|b| b.is_ascii_digit())
}

#[cfg(test)]
mod tests;
