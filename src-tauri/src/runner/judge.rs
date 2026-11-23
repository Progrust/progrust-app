//! `stdout` 方式の判定の部品（spec/exercise-runner/submit.md「stdout 方式」「判定結果」、
//! spec/exercise-runner/limits.md の 1 テストケースの実行時間と出力量、ADR 0203、ADR 0214）。
//!
//! - [`judge_stdout`] は、ビルド済みの実行ファイルを、`cases.json` のテストケース（[`StdoutCase`]）ごとに
//!   子プロセスの部品（[`super::process`]）で起動し、テストケースの結果を決めて、判定結果（[`JudgeResult`]）を返す。
//!   テストケースは `cases.json` の順に 1 件ずつ実行し、1 件終わるたびに進み具合（[`JudgeProgress`]）を知らせる。
//!   1 件失敗しても、残りをすべて実行する。
//! - 出力の比較の前に、標準出力と期待値の両方を [`normalize`] で正規化する。
//! - 判定結果（[`JudgeResult`]）は、spec/exercise-runner/submit.md「判定結果」の形（`detail` は ADR 0216）で JSON にできる。
//!
//! Tauri の実行時に頼らない（判定だけを行うコマンドラインのプログラムからも使う。ADR 0208）。

use std::{ffi::OsString, io, path::PathBuf, time::Duration};

use serde::{Deserialize, Serialize};

use super::process::{
    self, CollectedOutput, Limits, OutputLimit, ProcessSpec, StdinMode, Termination, TimeoutOutput,
};

/// 1 テストケースの実行時間の制限（spec/exercise-runner/limits.md）。
pub const CASE_TIME_LIMIT: Duration = Duration::from_secs(5);
/// 1 テストケースの出力量の上限（バイト。標準出力だけで数える。spec/exercise-runner/limits.md、ADR 0214）。
pub const CASE_OUTPUT_LIMIT: usize = 64 * 1024;

/// `cases.json` のテストケース 1 件（spec/content-format/exercise.md「判定方式」）。
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct StdoutCase {
    /// 表示名。
    pub name: String,
    /// 標準入力として与える文字列。
    pub stdin: String,
    /// 期待する標準出力。
    pub expected_stdout: String,
}

/// `cases.json` の内容を読む。
pub fn parse_cases(json: &str) -> serde_json::Result<Vec<StdoutCase>> {
    serde_json::from_str(json)
}

/// 1 テストケースの制限値。アプリでは [`CaseLimits::default`]（spec/exercise-runner/limits.md の値）を使い、
/// テストでは変えられる。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CaseLimits {
    /// 実行時間の制限。
    pub time: Duration,
    /// 標準出力の量の上限（バイト）。
    pub stdout: usize,
}

impl Default for CaseLimits {
    fn default() -> Self {
        Self {
            time: CASE_TIME_LIMIT,
            stdout: CASE_OUTPUT_LIMIT,
        }
    }
}

/// `stdout` 方式で判定するもの。
#[derive(Debug, Clone)]
pub struct StdoutJudgeSpec {
    /// ビルド済みの実行ファイル（`src/main.rs` の実行ファイル）。
    pub executable: PathBuf,
    /// `project/`。実行ファイルの作業ディレクトリにする。
    pub project_dir: PathBuf,
    /// アプリの環境変数に加えて設定する環境変数。
    pub envs: Vec<(OsString, OsString)>,
    /// テストケース。この順に実行する。
    pub cases: Vec<StdoutCase>,
    pub limits: CaseLimits,
}

/// 判定結果の `status`（spec/exercise-runner/submit.md「判定結果」）のうち、この部品が決めるもの。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum JudgeStatus {
    /// すべてのテストケースが `pass`。
    Passed,
    /// 1 件以上のテストケースが `pass` でない。
    Wrong,
    /// テストケースの実行の途中で、止める操作で止めた（spec/exercise-runner/submit.md「途中で止めた提出」）。
    Stopped,
}

/// テストケースの結果の `status`。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CaseStatus {
    Pass,
    Fail,
    Timeout,
    RuntimeError,
}

/// テストケースの結果の `detail`（spec/exercise-runner/submit.md「判定結果」、ADR 0216）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(untagged)]
pub enum CaseDetail {
    /// `fail` の入力・期待する出力・実際の出力。オブジェクトにする。どれも正規化する前のもの。出力過多のときの
    /// 実際の出力は、上限までの標準出力である。
    Output {
        input: String,
        expected: String,
        actual: String,
    },
    /// `timeout`・`runtime_error` の標準エラー出力の先頭（[`process::STDERR_HEAD_BYTES`] まで）。文字列にする。
    Stderr(String),
}

/// テストケース 1 件の結果。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CaseResult {
    /// 表示名。
    pub name: String,
    pub status: CaseStatus,
    /// `pass` では `None`（JSON では `null`）。
    pub detail: Option<CaseDetail>,
}

/// 判定結果。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct JudgeResult {
    pub status: JudgeStatus,
    /// 通過したテストケースの数。
    pub passed: u32,
    /// 全テストケースの数。
    pub total: u32,
    /// テストケースごとの結果。`cases.json` の順に並ぶ。
    pub cases: Vec<CaseResult>,
}

impl JudgeResult {
    /// すべてのテストケースの結果から組み立てる。
    pub fn from_cases(cases: Vec<CaseResult>) -> Self {
        let passed = cases
            .iter()
            .filter(|case| case.status == CaseStatus::Pass)
            .count();
        let total = cases.len();
        Self {
            status: if passed == total {
                JudgeStatus::Passed
            } else {
                JudgeStatus::Wrong
            },
            passed: count(passed),
            total: count(total),
            cases,
        }
    }

    /// 途中で止めた判定結果。`passed` と `total` は 0、`cases` は空にする（spec/exercise-runner/submit.md
    /// 「途中で止めた提出」）。
    pub fn stopped() -> Self {
        Self {
            status: JudgeStatus::Stopped,
            passed: 0,
            total: 0,
            cases: Vec::new(),
        }
    }
}

/// テストケースの数を `u32` にする。数え切れない数のテストケースは無いので、超えたら上限にする。
fn count(value: usize) -> u32 {
    u32::try_from(value).unwrap_or(u32::MAX)
}

/// 判定の進み具合。テストケースが 1 件終わるたびに知らせる。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct JudgeProgress {
    /// 結果が決まったテストケースの数。
    pub completed: usize,
    /// 全テストケースの数。
    pub total: usize,
}

/// `spec` のテストケースを順に実行して判定する。テストケースが 1 件終わるたびに `on_progress` を呼ぶ。
///
/// 実行ファイルを起動できなかったときなど、アプリの側の失敗は `Err` で返し、残りのテストケースは実行しない。
pub fn judge_stdout<F>(spec: &StdoutJudgeSpec, mut on_progress: F) -> io::Result<JudgeResult>
where
    F: FnMut(JudgeProgress),
{
    let total = spec.cases.len();
    let mut results = Vec::with_capacity(total);
    for case in &spec.cases {
        let collected = process::collect_output(case_process(spec, case))?;
        let Some(result) = case_result(case, &collected) else {
            return Ok(JudgeResult::stopped());
        };
        results.push(result);
        on_progress(JudgeProgress {
            completed: results.len(),
            total,
        });
    }
    Ok(JudgeResult::from_cases(results))
}

/// テストケース 1 件を動かすプロセス。
fn case_process(spec: &StdoutJudgeSpec, case: &StdoutCase) -> ProcessSpec {
    let mut process = ProcessSpec::new(
        &spec.executable,
        &spec.project_dir,
        Limits {
            time: spec.limits.time,
            output: OutputLimit::Stdout(spec.limits.stdout),
        },
    );
    process.envs = spec.envs.clone();
    process.stdin = StdinMode::Input(case.stdin.clone().into_bytes());
    // 時間切れの `detail` に、止めるまでに受け取った標準エラー出力を使う。
    process.on_timeout = TimeoutOutput::Deliver;
    process
}

/// テストケースの結果を、spec/exercise-runner/submit.md「stdout 方式」の表の上の行から順に当てはめて決める
/// （ADR 0203）。止める操作で止めたときは `None`。
fn case_result(case: &StdoutCase, collected: &CollectedOutput) -> Option<CaseResult> {
    let stderr_detail = || Some(CaseDetail::Stderr(collected.stderr.clone()));
    let output_detail = || {
        Some(CaseDetail::Output {
            input: case.stdin.clone(),
            expected: case.expected_stdout.clone(),
            actual: collected.stdout.clone(),
        })
    };
    let (status, detail) = match collected.termination {
        Termination::Stopped => return None,
        Termination::TimedOut => (CaseStatus::Timeout, stderr_detail()),
        Termination::OutputLimitExceeded => (CaseStatus::Fail, output_detail()),
        Termination::Exited(code) if code != 0 => (CaseStatus::RuntimeError, stderr_detail()),
        Termination::Signaled(_) => (CaseStatus::RuntimeError, stderr_detail()),
        Termination::Exited(_) => {
            if normalize(&collected.stdout) == normalize(&case.expected_stdout) {
                (CaseStatus::Pass, None)
            } else {
                (CaseStatus::Fail, output_detail())
            }
        }
    };
    Some(CaseResult {
        name: case.name.clone(),
        status,
        detail,
    })
}

/// 出力の正規化（spec/exercise-runner/submit.md「stdout 方式」の「正規化」）。改行コード（CRLF）を LF にそろえ、
/// 各行の末尾の空白と、末尾の空行を取り除く。最後の行の改行も取り除くので、最後に改行があるかどうかは比べない。
pub fn normalize(text: &str) -> String {
    let text = text.replace("\r\n", "\n");
    let mut lines: Vec<&str> = text.split('\n').map(str::trim_end).collect();
    while lines.last().is_some_and(|line| line.is_empty()) {
        lines.pop();
    }
    lines.join("\n")
}

#[cfg(test)]
mod tests;
