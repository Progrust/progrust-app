//! 学習者による実行・テスト実行・提出を、同時に 1 つだけ行うための状態
//! （spec/exercise-runner/overview.md、spec/architecture/commands/run.md）。C の演習と Rust の演習で共通である。
//!
//! - [`Runs`] は、動いている要求（[`ActiveRun`]）を 1 つだけ持つ。アプリでは Tauri の状態として `manage` で置く。
//!   [`Runs::begin`] で要求を始め、終わったら [`Runs::release`] で外す。動いているものがあるときは `busy`、
//!   動いている要求と同じ `request_id` は `invalid_argument` にする。
//! - [`ActiveRun`] は、いまの段階（準備中・ビルド中・プログラムの実行中・提出の判定中・終わり）と、止める操作の受け付け、
//!   標準入力の書き込みと閉じる操作、学習者が入力した標準入力の記録を持つ。
//! - 止める操作が、止める相手（ビルドの cargo・`clang`、プログラム）を起動する前に来ても失わないように、止める指示を
//!   覚えておき、相手を起動して登録したときに、すぐに止める。
//! - [`Runs::stop_lesson_run_and_wait`] は、レッスンの実行・テスト実行を止め、要求が外れるまで待つ（`end_session`）。
//!   実行・テスト実行のタスクは、試行ログに追記してから要求を外すので、外れたときには追記が終わっている。

use std::sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError};
use std::time::Duration;

use super::build::{BUILD_TIME_LIMIT, BuildHandle};
use super::c_build::CBuildHandle;
use super::process::{ProcessHandle, Stopper};
use crate::commands::error::{CommandError, ErrorCode};

/// 実行時間の制限（run。テスト実行を含む。spec/exercise-runner/limits.md）。
pub const RUN_TIME_LIMIT: Duration = Duration::from_secs(60);
/// 出力量の上限（run。テスト実行を含む。標準出力と標準エラー出力の合計。spec/exercise-runner/limits.md）。
pub const RUN_OUTPUT_LIMIT: usize = 1024 * 1024;

/// 実行・テスト実行の制限値。アプリでは [`RunLimits::default`]（spec/exercise-runner/limits.md の値）を使い、
/// テストでは短くする。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RunLimits {
    /// ビルド時間の制限。
    pub build_time: Duration,
    /// 実行時間の制限。
    pub run_time: Duration,
    /// 出力量の上限（バイト）。
    pub output: usize,
}

impl Default for RunLimits {
    fn default() -> Self {
        Self {
            build_time: BUILD_TIME_LIMIT,
            run_time: RUN_TIME_LIMIT,
            output: RUN_OUTPUT_LIMIT,
        }
    }
}

/// 要求の種類。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunKind {
    /// `run_code` の実行・テスト実行。
    Run,
    /// `submit_code` の提出。
    Submit,
}

/// 止めた理由。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StopCause {
    /// 停止ボタンと、演習画面を離れたとき（`stop_run`）。
    User,
    /// アプリの終了。`run_finished` を送らず、試行ログに追記しない（spec/exercise-runner/run.md「停止」）。
    AppExit,
}

/// 動いている実行・提出を 1 つだけ持つ状態。複製は同じ状態を指す。
#[derive(Clone)]
pub struct Runs {
    active: Arc<Mutex<Option<Arc<ActiveRun>>>>,
    /// 要求を外したことを、外れるのを待っている側に知らせる。
    released: Arc<Condvar>,
    limits: RunLimits,
}

impl Default for Runs {
    fn default() -> Self {
        Self::new(RunLimits::default())
    }
}

impl Runs {
    /// 制限値を `limits` にする。
    pub fn new(limits: RunLimits) -> Self {
        Self {
            active: Arc::new(Mutex::new(None)),
            released: Arc::new(Condvar::new()),
            limits,
        }
    }

    /// 実行・テスト実行の制限値。
    pub fn limits(&self) -> RunLimits {
        self.limits
    }

    /// 要求を始める。動いている要求と同じ `request_id` のときは `invalid_argument`、ほかの要求が動いているときは
    /// `busy` を返す。始めた要求は、終わったら [`Runs::release`] で外す。
    pub fn begin(
        &self,
        request_id: &str,
        lesson_id: &str,
        kind: RunKind,
    ) -> Result<Arc<ActiveRun>, CommandError> {
        let mut active = lock(&self.active);
        if let Some(current) = active.as_ref() {
            if current.request_id == request_id {
                return Err(CommandError {
                    code: ErrorCode::InvalidArgument,
                    message: "同じ ID の実行・提出が、既に動いています。".into(),
                    detail: Some(format!("request_id {request_id} は動いている")),
                });
            }
            return Err(CommandError {
                code: ErrorCode::Busy,
                message: "ほかの実行・提出が終わるまで待ってください。".into(),
                detail: None,
            });
        }
        let run = Arc::new(ActiveRun {
            request_id: request_id.to_string(),
            lesson_id: lesson_id.to_string(),
            kind,
            state: Mutex::new(ActiveState::default()),
        });
        *active = Some(Arc::clone(&run));
        Ok(run)
    }

    /// 終わった要求を外す。ほかの要求が動いているときは、何もしない。
    pub fn release(&self, run: &Arc<ActiveRun>) {
        let mut active = lock(&self.active);
        if active
            .as_ref()
            .is_some_and(|current| Arc::ptr_eq(current, run))
        {
            *active = None;
            self.released.notify_all();
        }
    }

    /// `request_id` の、動いている要求。無いとき（既に終わったときを含む）は `None`。
    pub fn find(&self, request_id: &str) -> Option<Arc<ActiveRun>> {
        lock(&self.active)
            .as_ref()
            .filter(|run| run.request_id == request_id)
            .cloned()
    }

    /// 動いている要求。無いときは `None`。
    pub fn current(&self) -> Option<Arc<ActiveRun>> {
        lock(&self.active).clone()
    }

    /// `request_id` の要求が動いていれば止める。動いていないときは、何もしない（`stop_run`）。
    pub fn stop(&self, request_id: &str) {
        if let Some(run) = self.find(request_id) {
            run.stop(StopCause::User);
        }
    }

    /// レッスン `lesson_id` の実行・テスト実行が動いていれば止め（停止ボタンと同じ）、その要求が外れるまで待つ
    /// （`end_session`。spec/architecture/commands/progress.md「end_session」）。実行・テスト実行のタスクは、試行ログに
    /// 追記してから要求を外すので、返ったときには、止めた実行の追記が終わっている。止める指示の後に外れるのを待つので、
    /// 先に `stop_run` で止めていても同じである。
    ///
    /// 動いていないとき、ほかのレッスンの要求と、提出（止めた提出は追記しない）のときは、止めずにすぐ返る。
    pub fn stop_lesson_run_and_wait(&self, lesson_id: &str) {
        let Some(run) = self
            .current()
            .filter(|run| run.lesson_id == lesson_id && run.kind == RunKind::Run)
        else {
            return;
        };
        run.stop(StopCause::User);
        let active = lock(&self.active);
        let _released = self
            .released
            .wait_while(active, |active| {
                active
                    .as_ref()
                    .is_some_and(|current| Arc::ptr_eq(current, &run))
            })
            .unwrap_or_else(PoisonError::into_inner);
    }

    /// アプリを終了するときに、動いている要求を止める。止めた要求は、`run_finished` を送らず、記録しない。
    pub fn stop_for_app_exit(&self) {
        if let Some(run) = self.current() {
            run.stop(StopCause::AppExit);
        }
    }
}

/// 動いている要求（実行・テスト実行・提出）。
pub struct ActiveRun {
    request_id: String,
    lesson_id: String,
    kind: RunKind,
    state: Mutex<ActiveState>,
}

#[derive(Default)]
struct ActiveState {
    phase: Phase,
    stop: Option<StopCause>,
    /// 学習者が入力した標準入力を、書いた順に連結したもの。
    stdin_log: String,
}

/// 段階。
#[derive(Default)]
enum Phase {
    /// ビルドを始める前（`project/` の組み立てなど）。
    #[default]
    Preparing,
    /// ビルド中（Rust の演習の cargo か、C の演習の `clang`）。
    Building(BuildingHandle),
    /// 学習者のプログラム（テスト実行では `cargo test`）の実行中。
    Running {
        process: ProcessHandle,
        stdin: StdinState,
    },
    /// 提出の判定中（テストケースのプロセスを、続けて、または同時に動かす）。
    Judging(Stopper),
    /// プログラムが終わった後（提出では、判定が終わった後）。
    Finished,
}

/// ビルド中の、止める操作の相手。C の演習と Rust の演習の実行・提出を、同じ状態で 1 つだけ行う
/// （spec/exercise-runner/overview.md）。
pub enum BuildingHandle {
    /// Rust の演習の cargo。
    Cargo(BuildHandle),
    /// C の演習の `clang`。
    Clang(CBuildHandle),
}

impl BuildingHandle {
    /// ビルドのプロセスグループ全体を止める。
    fn stop(&self) {
        match self {
            Self::Cargo(build) => build.stop(),
            Self::Clang(build) => build.stop(),
        }
    }
}

impl From<BuildHandle> for BuildingHandle {
    fn from(build: BuildHandle) -> Self {
        Self::Cargo(build)
    }
}

impl From<CBuildHandle> for BuildingHandle {
    fn from(build: CBuildHandle) -> Self {
        Self::Clang(build)
    }
}

/// 実行中のプログラムの標準入力。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StdinState {
    /// 入力欄から書ける（実行）。
    Open,
    /// 閉じた（「入力終了」の後と、起動の直後に閉じるテスト実行・判定）。
    Closed,
}

impl ActiveRun {
    pub fn request_id(&self) -> &str {
        &self.request_id
    }

    pub fn lesson_id(&self) -> &str {
        &self.lesson_id
    }

    pub fn kind(&self) -> RunKind {
        self.kind
    }

    fn state(&self) -> MutexGuard<'_, ActiveState> {
        lock(&self.state)
    }

    /// 止める。ビルド中・実行中なら、そのプロセスグループ全体を止める。まだ起動していないときは、起動して登録したときに止める。
    /// アプリの終了は、先に止める操作があっても、それより優先する。
    pub fn stop(&self, cause: StopCause) {
        let mut state = self.state();
        match cause {
            StopCause::AppExit => state.stop = Some(cause),
            StopCause::User => {
                state.stop.get_or_insert(cause);
            }
        }
        match &state.phase {
            Phase::Building(build) => build.stop(),
            Phase::Running { process, .. } => process.stop(),
            Phase::Judging(stopper) => stopper.stop(),
            Phase::Preparing | Phase::Finished => {}
        }
    }

    /// 止める指示があったか。あったときは、その理由。
    pub fn stop_cause(&self) -> Option<StopCause> {
        self.state().stop
    }

    /// アプリの終了で止めたか。
    pub fn stopped_for_app_exit(&self) -> bool {
        self.stop_cause() == Some(StopCause::AppExit)
    }

    /// ビルドを始めたときに、止める操作の相手として登録する。既に止める指示があれば、すぐに止める。
    pub fn set_building(&self, build: impl Into<BuildingHandle>) {
        let build = build.into();
        let mut state = self.state();
        if state.stop.is_some() {
            build.stop();
        }
        state.phase = Phase::Building(build);
    }

    /// プログラムを起動したときに、止める操作と標準入力の相手として登録する。既に止める指示があれば、すぐに止める。
    pub fn set_running(&self, process: ProcessHandle, stdin: StdinState) {
        let mut state = self.state();
        if state.stop.is_some() {
            process.stop();
        }
        state.phase = Phase::Running { process, stdin };
    }

    /// 提出の判定を始めたときに、止める操作の相手として登録する。既に止める指示があれば、すぐに止める。
    pub fn set_judging(&self, stopper: Stopper) {
        let mut state = self.state();
        if state.stop.is_some() {
            stopper.stop();
        }
        state.phase = Phase::Judging(stopper);
    }

    /// プログラム（提出では判定）が終わったときに呼ぶ。この後は、標準入力を書けない。
    pub fn set_finished(&self) {
        self.state().phase = Phase::Finished;
    }

    /// 標準入力に `text` をそのまま書き、記録する（`send_stdin`）。プログラムがまだ起動していないとき（ビルド中）、
    /// 終わった後、標準入力を閉じた後は `invalid_state`。
    pub fn write_stdin(&self, text: &str) -> Result<(), CommandError> {
        let mut state = self.state();
        let process = open_stdin(self.kind, &state.phase)?;
        process.write_stdin(text).map_err(|_| stdin_closed())?;
        state.stdin_log.push_str(text);
        Ok(())
    }

    /// 標準入力を閉じる（`close_stdin`）。エラーは [`ActiveRun::write_stdin`] と同じ。
    pub fn close_stdin(&self) -> Result<(), CommandError> {
        let mut state = self.state();
        open_stdin(self.kind, &state.phase)?.close_stdin();
        if let Phase::Running { stdin, .. } = &mut state.phase {
            *stdin = StdinState::Closed;
        }
        Ok(())
    }

    /// 学習者が入力した標準入力を、書いた順に連結したもの。
    pub fn stdin_log(&self) -> String {
        self.state().stdin_log.clone()
    }
}

/// 標準入力を書ける段階なら、プログラムへの操作を返す。提出は、標準入力を受け付ける実行ではない。
fn open_stdin(kind: RunKind, phase: &Phase) -> Result<&ProcessHandle, CommandError> {
    if kind == RunKind::Submit {
        return Err(not_running());
    }
    match phase {
        Phase::Running {
            process,
            stdin: StdinState::Open,
        } => Ok(process),
        Phase::Running {
            stdin: StdinState::Closed,
            ..
        } => Err(stdin_closed()),
        Phase::Preparing | Phase::Building(_) => Err(invalid_state(
            "プログラムがまだ起動していません。",
            "ビルドが終わっていない",
        )),
        Phase::Judging(_) | Phase::Finished => Err(not_running()),
    }
}

/// `request_id` の実行が動いていないときの `send_stdin`・`close_stdin` のエラー。
pub fn not_running() -> CommandError {
    invalid_state("プログラムは実行されていません。", "実行が動いていない")
}

fn stdin_closed() -> CommandError {
    invalid_state("標準入力は閉じています。", "標準入力を閉じた後")
}

fn invalid_state(message: &str, detail: &str) -> CommandError {
    CommandError {
        code: ErrorCode::InvalidState,
        message: message.into(),
        detail: Some(detail.into()),
    }
}

/// ロックを取る。ほかのスレッドが panic してロックが壊れていても、中の値を使う。
fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

#[cfg(test)]
mod tests;
