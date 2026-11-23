//! 試行ログ（`attempts/{session_id}.json`。spec/learning-record-and-analysis/attempt-log.md）の型と読み書き、
//! 試行セッションの開始と継続、実行・提出の `run`・`submit` イベントの追記（spec/exercise-runner/attempt-log.md）。
//!
//! - 読み込めない試行ログは `.broken-{日時}` を付けて退避し、作り直さずに、無いものとして扱う
//!   （spec/architecture/data-locations.md「書き込みの規則」）。
//! - 試行ログの「読んで、変えて、書く」は [`SESSION_LOCK`] で 1 つずつ行う。
//! - セッションの開始・継続は [`AppDataDir::start_or_continue_session`]、イベントの追記は
//!   [`AppDataDir::append_run`]・[`AppDataDir::append_submit`]、正解と終了は [`AppDataDir::mark_session_passed`]・
//!   [`AppDataDir::end_attempt_session`]、未完了のセッションは [`AppDataDir::unfinished_session`]・
//!   [`AppDataDir::unfinished_sessions`] で扱う。

pub mod output;

use std::collections::BTreeMap;
use std::sync::{Mutex, PoisonError};

use chrono::{DateTime, Datelike, Local, SecondsFormat, TimeDelta, Timelike};
use serde::{Deserialize, Serialize};

pub use output::{BuildOutput, OUTPUT_LIMIT_CHARS};

use crate::storage::{AppDataDir, StorageError};

/// 試行ログの `schema_version`。
pub const SCHEMA_VERSION: u64 = 1;

/// 試行ログを置くディレクトリの、`{app_data_dir}` からの相対パス。
const ATTEMPTS_DIR: &str = "attempts";

/// `session_id` の先頭の開始日時（`YYYYMMDDTHHMMSS`）の長さ。
const STAMP_LEN: usize = 15;

/// 試行ログの「読んで、変えて、書く」を 1 つずつ行うためのロック。セッションを探して始める処理と、
/// イベントの追記、`passed_at`・`ended_at` の記録を、この中で行う。
static SESSION_LOCK: Mutex<()> = Mutex::new(());

/// 試行セッションの種類（`kind`）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionKind {
    Lesson,
    ChapterReview,
    Generated,
}

impl SessionKind {
    /// `lesson.json` の `kind` のレッスンを開いたときに始めるセッションの種類。
    /// 動画のみのレッスン（`video_only`）では、セッションを作らないので `None`（ADR 0120）。
    pub fn for_lesson_kind(kind: &str) -> Option<Self> {
        match kind {
            "video_only" => None,
            "chapter_review" => Some(Self::ChapterReview),
            _ => Some(Self::Lesson),
        }
    }
}

/// 試行ログ 1 件（1 つの試行セッション）。日時は RFC 3339 の文字列（例: `2026-10-31T13:02:00+09:00`）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AttemptLog {
    pub schema_version: u64,
    pub session_id: String,
    pub lesson_id: String,
    pub kind: SessionKind,
    /// 生成問題の難易度（1〜5）。レッスンは `None`。
    pub difficulty: Option<u8>,
    pub started_at: String,
    pub passed_at: Option<String>,
    pub ended_at: Option<String>,
    pub events: Vec<Event>,
}

/// 試行ログのイベント。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Event {
    /// 質問のイベント。書くのは質疑応答（M4）で、ここでは、書かれた試行ログを読めるように形だけを持つ。
    Chat(ChatEvent),
    Run(RunEvent),
    Submit(SubmitEvent),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChatEvent {
    pub at: String,
    /// `video:{文の ID}` か `explanation`。
    pub context: String,
    pub question: String,
    /// 将来の拡張用。いまは常に `None`。
    pub answer_summary: Option<String>,
}

/// 実行（`main`）かテスト実行（`test`）か。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunMode {
    Main,
    Test,
}

/// 試行ログに記録する実行の結果の状態。`crates_unavailable` の実行は記録しないので、ここには無い（ADR 0125）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunStatus {
    Success,
    CompileError,
    RuntimeError,
    Timeout,
    Stopped,
}

/// 試行ログに記録する提出の判定結果の状態。`crates_unavailable` と、途中で止めた提出（`stopped`）は記録しないので、
/// ここには無い（spec/exercise-runner/attempt-log.md）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SubmitStatus {
    CompileError,
    Wrong,
    Passed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RunEvent {
    pub at: String,
    pub mode: RunMode,
    pub files: BTreeMap<String, String>,
    pub result: RunResult,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RunResult {
    pub status: RunStatus,
    pub error_codes: Vec<String>,
    pub stdout: String,
    pub stderr: String,
    /// `run_finished` の `run.exit_code` と同じ値（ADR 0206）。
    pub exit_code: Option<i32>,
    pub stdin: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SubmitEvent {
    pub at: String,
    pub files: BTreeMap<String, String>,
    pub result: SubmitResult,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SubmitResult {
    pub status: SubmitStatus,
    pub error_codes: Vec<String>,
    pub passed: u32,
    pub total: u32,
    /// 失敗したテストケースの表示名。
    pub failed_cases: Vec<String>,
    pub stderr: String,
}

/// 実行・テスト実行が終わったときに、実行のタスクが渡すもの。[`AppDataDir::append_run`] が試行ログの形に整える。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunRecord {
    /// 終わった日時（`run_finished` の `finished_at` と同じもの）。
    pub at: String,
    pub mode: RunMode,
    /// エディタ上の全ファイル。
    pub files: BTreeMap<String, String>,
    pub status: RunStatus,
    /// ビルドの JSON 出力のうち、レベルが `error` の診断の `code` を出現順に並べたもの。重複は、ここで除く。
    pub error_codes: Vec<String>,
    pub build: BuildOutput,
    /// 学習者のプログラム（テスト実行では `cargo test`）の標準出力。
    pub stdout: String,
    /// 学習者のプログラム（テスト実行では `cargo test`）の標準エラー出力。
    pub program_stderr: String,
    /// `run_finished` の `run.exit_code` と同じ値。
    pub exit_code: Option<i32>,
    /// 学習者が入力した標準入力を連結したもの。テスト実行では空。
    pub stdin: String,
}

/// 提出の判定が終わったときに、提出のタスクが渡すもの。[`AppDataDir::append_submit`] が試行ログの形に整える。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubmitRecord {
    /// 判定が終わった日時（`run_finished` の `finished_at` と同じもの）。
    pub at: String,
    /// エディタ上の全ファイル。
    pub files: BTreeMap<String, String>,
    pub status: SubmitStatus,
    /// [`RunRecord::error_codes`] と同じ。
    pub error_codes: Vec<String>,
    pub passed: u32,
    pub total: u32,
    pub failed_cases: Vec<String>,
    pub build: BuildOutput,
    /// 判定で動かしたプログラムの標準エラー出力。無いときは空。
    pub program_stderr: String,
}

impl RunRecord {
    fn into_event(self) -> Event {
        Event::Run(RunEvent {
            at: self.at,
            mode: self.mode,
            files: self.files,
            result: RunResult {
                status: self.status,
                error_codes: dedup(self.error_codes),
                stdout: output::attempt_stdout(self.stdout),
                stderr: output::attempt_stderr(&self.build, &self.program_stderr),
                exit_code: self.exit_code,
                stdin: self.stdin,
            },
        })
    }
}

impl SubmitRecord {
    fn into_event(self) -> Event {
        Event::Submit(SubmitEvent {
            at: self.at,
            files: self.files,
            result: SubmitResult {
                status: self.status,
                error_codes: dedup(self.error_codes),
                passed: self.passed,
                total: self.total,
                failed_cases: self.failed_cases,
                stderr: output::attempt_stderr(&self.build, &self.program_stderr),
            },
        })
    }
}

/// 重複を除く。最初に出てきた順を保つ。
fn dedup(codes: Vec<String>) -> Vec<String> {
    let mut unique: Vec<String> = Vec::with_capacity(codes.len());
    for code in codes {
        if !unique.contains(&code) {
            unique.push(code);
        }
    }
    unique
}

/// 試行ログの、`{app_data_dir}` からの相対パス。
fn attempt_file(session_id: &str) -> String {
    format!("{ATTEMPTS_DIR}/{session_id}.json")
}

/// `session_id`（`{開始日時 YYYYMMDDTHHMMSS}_{lesson_id}`）の `lesson_id` の部分。形に合わないときは `None`。
fn lesson_id_of(session_id: &str) -> Option<&str> {
    if !session_id.is_char_boundary(STAMP_LEN) {
        return None;
    }
    let (stamp, rest) = session_id.split_at(STAMP_LEN);
    let is_stamp = stamp.char_indices().all(|(index, c)| {
        if index == 8 {
            c == 'T'
        } else {
            c.is_ascii_digit()
        }
    });
    let lesson_id = rest.strip_prefix('_')?;
    (is_stamp && !lesson_id.is_empty()).then_some(lesson_id)
}

/// 日時の `YYYYMMDDTHHMMSS`。
fn stamp(at: &DateTime<Local>) -> String {
    format!(
        "{:04}{:02}{:02}T{:02}{:02}{:02}",
        at.year(),
        at.month(),
        at.day(),
        at.hour(),
        at.minute(),
        at.second()
    )
}

impl AppDataDir {
    /// 試行ログを読む。無いときと、読み込めない試行ログを退避したときは `None`。
    pub fn read_attempt_log(&self, session_id: &str) -> Result<Option<AttemptLog>, StorageError> {
        Ok(self
            .read_optional_versioned_json(attempt_file(session_id), SCHEMA_VERSION)?
            .value)
    }

    /// レッスン `lesson_id` の未完了のセッション（`ended_at` が無いもの。`passed_at` があるものを含む）を返す。
    /// 無いときは `None`。複数あるときは、最後に始めたものを返す。読み込めない試行ログは、退避して、無いものとして扱う。
    pub fn unfinished_session(&self, lesson_id: &str) -> Result<Option<AttemptLog>, StorageError> {
        let _guard = lock_sessions();
        self.find_unfinished_session(lesson_id)
    }

    /// すべてのレッスンの未完了のセッション（`ended_at` が無いもの。`passed_at` があるものを含む）を、
    /// `session_id` の順（同じレッスンでは始めた順）に返す。読み込めない試行ログは、退避して、無いものとして扱う。
    pub fn unfinished_sessions(&self) -> Result<Vec<AttemptLog>, StorageError> {
        let _guard = lock_sessions();
        let mut sessions = Vec::new();
        for session_id in self.list_json_stems(ATTEMPTS_DIR)? {
            let Some(lesson_id) = lesson_id_of(&session_id) else {
                continue;
            };
            if let Some(log) = self.read_unfinished(&session_id, lesson_id)? {
                sessions.push(log);
            }
        }
        Ok(sessions)
    }

    /// レッスンを開いたときに呼ぶ。レッスン `lesson_id` の未完了のセッションがあれば、それを返す（継続）。
    /// 無ければ、`kind` の新しいセッションを始め、試行ログを書いて返す（開始）。
    pub fn start_or_continue_session(
        &self,
        lesson_id: &str,
        kind: SessionKind,
    ) -> Result<AttemptLog, StorageError> {
        let _guard = lock_sessions();
        self.start_or_continue_session_at(lesson_id, kind, Local::now())
    }

    /// 実行・テスト実行が終わったときに呼ぶ。レッスン `lesson_id` の未完了のセッションに `run` イベントを追記し、
    /// そのセッションの ID を返す。未完了のセッションが無いとき（正解の後に演習画面を離れて、セッションの終了が
    /// 先に済んだときなど）は、追記せずに `None` を返す。新しいセッションは始めない。
    ///
    /// `crates_unavailable` の実行と、アプリの終了で止めた実行は、呼ばない（spec/exercise-runner/attempt-log.md）。
    pub fn append_run(
        &self,
        lesson_id: &str,
        record: RunRecord,
    ) -> Result<Option<String>, StorageError> {
        self.append_event(lesson_id, record.into_event())
    }

    /// 提出の判定が終わったときに呼ぶ。レッスン `lesson_id` の未完了のセッションに `submit` イベントを追記し、
    /// そのセッションの ID を返す。未完了のセッションが無いときは、追記せずに `None` を返す（[`AppDataDir::append_run`] と同じ）。
    ///
    /// `crates_unavailable` の提出と、途中で止めた提出は、呼ばない（spec/exercise-runner/attempt-log.md）。
    pub fn append_submit(
        &self,
        lesson_id: &str,
        record: SubmitRecord,
    ) -> Result<Option<String>, StorageError> {
        self.append_event(lesson_id, record.into_event())
    }

    /// 提出で正解したときに呼ぶ。セッション `session_id` の `passed_at` を `at` にし、更新した後の試行ログを返す。
    /// 既に `passed_at` があるときは、最初に正解した日時を残して変えない。試行ログが無いときは `None`。
    pub fn mark_session_passed(
        &self,
        session_id: &str,
        at: &str,
    ) -> Result<Option<AttemptLog>, StorageError> {
        self.update_attempt_log(session_id, |log| {
            if log.passed_at.is_none() {
                log.passed_at = Some(at.to_string());
            }
        })
    }

    /// セッション `session_id` を終了する（`ended_at` を `at` にする）。更新した後の試行ログを返す。
    /// 既に `ended_at` があるときは、変えない。試行ログが無いときは `None`。
    pub fn end_attempt_session(
        &self,
        session_id: &str,
        at: &str,
    ) -> Result<Option<AttemptLog>, StorageError> {
        self.update_attempt_log(session_id, |log| {
            if log.ended_at.is_none() {
                log.ended_at = Some(at.to_string());
            }
        })
    }

    fn append_event(&self, lesson_id: &str, event: Event) -> Result<Option<String>, StorageError> {
        let _guard = lock_sessions();
        let Some(mut log) = self.find_unfinished_session(lesson_id)? else {
            return Ok(None);
        };
        log.events.push(event);
        self.write_json(attempt_file(&log.session_id), &log)?;
        Ok(Some(log.session_id))
    }

    fn update_attempt_log(
        &self,
        session_id: &str,
        change: impl FnOnce(&mut AttemptLog),
    ) -> Result<Option<AttemptLog>, StorageError> {
        let _guard = lock_sessions();
        let Some(mut log) = self.read_attempt_log(session_id)? else {
            return Ok(None);
        };
        let before = log.clone();
        change(&mut log);
        if log != before {
            self.write_json(attempt_file(session_id), &log)?;
        }
        Ok(Some(log))
    }

    /// [`AppDataDir::start_or_continue_session`] の、開始日時 `now` を渡せるもの。呼ぶ側が [`SESSION_LOCK`] を持っていること。
    fn start_or_continue_session_at(
        &self,
        lesson_id: &str,
        kind: SessionKind,
        now: DateTime<Local>,
    ) -> Result<AttemptLog, StorageError> {
        if let Some(log) = self.find_unfinished_session(lesson_id)? {
            return Ok(log);
        }
        // 同じ秒に、同じレッスンの前のセッションが始まっていたとき（終えてすぐにやり直したときなど）は、
        // 前の試行ログを上書きしないように、ID の日時を 1 秒ずつ進めて、まだ無い ID にする。
        let mut id_time = now;
        let session_id = loop {
            let candidate = format!("{}_{lesson_id}", stamp(&id_time));
            let path = self.path(attempt_file(&candidate))?;
            let taken = path
                .try_exists()
                .map_err(|source| StorageError::Io { path, source })?;
            if !taken {
                break candidate;
            }
            id_time += TimeDelta::seconds(1);
        };
        let log = AttemptLog {
            schema_version: SCHEMA_VERSION,
            session_id,
            lesson_id: lesson_id.to_string(),
            kind,
            difficulty: None,
            started_at: now.to_rfc3339_opts(SecondsFormat::Secs, false),
            passed_at: None,
            ended_at: None,
            events: Vec::new(),
        };
        self.write_json(attempt_file(&log.session_id), &log)?;
        Ok(log)
    }

    /// 呼ぶ側が [`SESSION_LOCK`] を持っていること。
    fn find_unfinished_session(&self, lesson_id: &str) -> Result<Option<AttemptLog>, StorageError> {
        let mut found = None;
        // 名前の順は、始めた順。後に見つけたものほど新しい。
        for session_id in self.list_json_stems(ATTEMPTS_DIR)? {
            if lesson_id_of(&session_id) != Some(lesson_id) {
                continue;
            }
            if let Some(log) = self.read_unfinished(&session_id, lesson_id)? {
                found = Some(log);
            }
        }
        Ok(found)
    }

    /// 試行ログ `session_id` を読み、レッスン `lesson_id` の未完了のセッションなら返す。
    fn read_unfinished(
        &self,
        session_id: &str,
        lesson_id: &str,
    ) -> Result<Option<AttemptLog>, StorageError> {
        Ok(self.read_attempt_log(session_id)?.filter(|log| {
            log.ended_at.is_none() && log.session_id == session_id && log.lesson_id == lesson_id
        }))
    }
}

fn lock_sessions() -> std::sync::MutexGuard<'static, ()> {
    SESSION_LOCK.lock().unwrap_or_else(PoisonError::into_inner)
}

#[cfg(test)]
mod tests;
