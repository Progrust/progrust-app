//! C の演習の提出の、メモリリークの検査のライブラリ（確保と解放を数える動的ライブラリ）のビルドと、結果のファイルの
//! 読み取り（spec/exercise-runner/c-submit.md「メモリリークの検査」と「手順」の 1・4、
//! spec/exercise-runner/c-project-assembly.md「作業領域」「ビルド」、ADR 0273）。
//!
//! - ライブラリのソース（C の 1 ファイル `leakcheck.c`）は、[`LEAK_CHECK_SOURCE`] としてクレートに埋め込む。
//! - [`start_leak_check_build`] で、ソースを `c-project/leakcheck/leakcheck.c` に書き出し（[`super::c_project`]）、
//!   検出した `clang` で `c-project/leakcheck/libprogrust_leakcheck.dylib` にビルドする（[`super::c_build`] の
//!   [`CBuildKind::LeakCheckLibrary`]）。止める操作とビルド時間の制限は、ほかのビルドと同じく
//!   [`RunningCBuild`] で扱う。終わるまで待つだけなら [`build_leak_check_library`]。
//! - 学習者のプログラムに差し込むときは、環境変数 [`INSERT_LIBRARIES_ENV`] にライブラリの絶対パスを、
//!   [`LEAK_REPORT_ENV`] に結果のファイルのパス（[`leak_report_path`]。`c-project/` からの相対パス）を渡す。
//! - [`read_leak_report`] で、結果のファイルを読み、件数とバイト数・`overflow`・読めなかった（できなかった、
//!   読めなかった、形に合わない）のどれかを返す。テストケースの結果を決めるのは、呼び出し側である。
//!
//! Tauri の実行時に頼らない（教材チェックツールの判定からも使う。ADR 0273）。

use std::fs;
use std::io::{self, Read};
use std::path::Path;
use std::time::Duration;

use super::c_build::{CBuildKind, CBuildOutcome, CBuildSpec, RunningCBuild, start_c_build};
use super::c_project::{CAssemblyError, write_leakcheck_source};
use super::sanitizer_report::REPORTS_DIR;
use super::workspace::WorkspaceDir;

/// メモリリークの検査のライブラリのソース。
pub const LEAK_CHECK_SOURCE: &str = include_str!("leakcheck.c");

/// ライブラリを差し込む環境変数。値は、ライブラリの絶対パス。
pub const INSERT_LIBRARIES_ENV: &str = "DYLD_INSERT_LIBRARIES";
/// 結果のファイルのパスをライブラリに渡す環境変数。
pub const LEAK_REPORT_ENV: &str = "PROGRUST_LEAK_REPORT";

/// 同時に記録できる領域の数（`leakcheck.c` の `MAX_RECORDS`）。超えると、結果のファイルは `overflow` になる。
pub const MAX_RECORDED_ALLOCATIONS: usize = 524_288;

/// 結果のファイルとして読む最大のバイト数。ライブラリが書くのは、多くても 2 つの 20 桁の数と改行である。
const MAX_REPORT_BYTES: u64 = 64;

/// `case_number` 番目（最初を 1 とする）のテストケースの結果のファイルのパス（`c-project/` からの相対パス。
/// `reports/leak-{N}`）。[`LEAK_REPORT_ENV`] に渡す。
pub fn leak_report_path(case_number: usize) -> String {
    format!("{REPORTS_DIR}/leak-{case_number}")
}

/// `c-project/leakcheck/leakcheck.c` にソースを書き出し、`clang` を起動して、ライブラリのビルドを始める。
/// `clang` は検出したものの絶対パス、`time_limit` はビルド時間の制限（`clang` の起動 1 回に当てる）。
///
/// `c-project/` は、先に組み立てておく（[`super::c_project::assemble_c_project`]）。ソースを書けなかったときと
/// `clang` を起動できなかったときは `Err` を返す。どちらも、提出をアプリの側の失敗として終えるものである
/// （c-submit.md「手順」の 1）。
pub fn start_leak_check_build(
    clang: &Path,
    workspace: &WorkspaceDir,
    time_limit: Duration,
) -> io::Result<RunningCBuild> {
    write_leakcheck_source(workspace, LEAK_CHECK_SOURCE).map_err(into_io_error)?;
    let mut spec = CBuildSpec::new(clang, workspace.c_project(), CBuildKind::LeakCheckLibrary);
    spec.time_limit = time_limit;
    start_c_build(spec)
}

/// [`start_leak_check_build`] で始めたビルドが終わるまで待ち、結果を返す。
pub fn build_leak_check_library(
    clang: &Path,
    workspace: &WorkspaceDir,
    time_limit: Duration,
) -> io::Result<CBuildOutcome> {
    start_leak_check_build(clang, workspace, time_limit)?.wait()
}

fn into_io_error(error: CAssemblyError) -> io::Error {
    match error {
        CAssemblyError::Io { path, source } => {
            io::Error::new(source.kind(), format!("{}: {source}", path.display()))
        }
        // ソースを書くときは、ファイルパスを確かめない。
        CAssemblyError::InvalidPaths(invalid) => {
            io::Error::new(io::ErrorKind::InvalidInput, invalid)
        }
    }
}

/// 結果のファイルから読んだもの。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LeakReport {
    /// 解放されなかった領域の件数と合計のバイト数。
    Leaks { count: u64, bytes: u64 },
    /// 同時に記録できる領域の数を超えた（メモリリークを検査できなかった）。
    Overflow,
    /// 結果のファイルが無い・読めない・形に合わない（メモリリークを検査できなかった）。
    Unavailable(LeakReportProblem),
}

/// 結果のファイルを使えなかった理由。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LeakReportProblem {
    /// 結果のファイルができなかった（`_exit`・`abort`・シグナルで終わったときなど）。
    Missing,
    /// 結果のファイルを読めなかった（ふつうのファイルでないときを含む）。
    Unreadable,
    /// 中身が、件数とバイト数の 2 行でも `overflow` の 1 行でもない。
    Malformed,
}

/// `reports_dir`（`c-project/reports/`）から、`case_number` 番目（最初を 1 とする）のテストケースの結果のファイル
/// `leak-{N}` を読む。
///
/// 結果のファイルの中身は、件数と合計のバイト数を、この順に 10 進の整数で 1 行ずつ書いたもの（最後の行の改行は
/// 無くてもよい）か、`overflow` の 1 行である。それ以外は [`LeakReportProblem::Malformed`] にする。シンボリック
/// リンクはたどらない。
pub fn read_leak_report(reports_dir: &Path, case_number: usize) -> LeakReport {
    let path = reports_dir.join(format!("leak-{case_number}"));
    match read_report_bytes(&path) {
        Ok(bytes) => parse_leak_report(&bytes),
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            LeakReport::Unavailable(LeakReportProblem::Missing)
        }
        Err(_) => LeakReport::Unavailable(LeakReportProblem::Unreadable),
    }
}

/// ふつうのファイルなら、先頭の [`MAX_REPORT_BYTES`] より 1 バイト多くまでを読む。
fn read_report_bytes(path: &Path) -> io::Result<Vec<u8>> {
    let metadata = fs::symlink_metadata(path)?;
    if !metadata.is_file() {
        return Err(io::Error::other(format!(
            "{}: ふつうのファイルではない",
            path.display()
        )));
    }
    let mut bytes = Vec::new();
    fs::File::open(path)?
        .take(MAX_REPORT_BYTES + 1)
        .read_to_end(&mut bytes)?;
    Ok(bytes)
}

/// 結果のファイルの中身を読む。
pub fn parse_leak_report(bytes: &[u8]) -> LeakReport {
    const MALFORMED: LeakReport = LeakReport::Unavailable(LeakReportProblem::Malformed);
    if bytes.len() as u64 > MAX_REPORT_BYTES {
        return MALFORMED;
    }
    let Ok(text) = std::str::from_utf8(bytes) else {
        return MALFORMED;
    };
    let text = text.strip_suffix('\n').unwrap_or(text);
    let lines: Vec<&str> = text.split('\n').collect();
    match lines.as_slice() {
        ["overflow"] => LeakReport::Overflow,
        [count, bytes] => match (parse_decimal(count), parse_decimal(bytes)) {
            (Some(count), Some(bytes)) => LeakReport::Leaks { count, bytes },
            _ => MALFORMED,
        },
        _ => MALFORMED,
    }
}

/// 10 進の数字だけからなる文字列を読む。空・符号・空白・`u64` に収まらない数は `None`。
fn parse_decimal(text: &str) -> Option<u64> {
    if text.is_empty() || !text.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    text.parse().ok()
}

#[cfg(test)]
mod tests;
