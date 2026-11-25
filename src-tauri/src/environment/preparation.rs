//! 許可クレートの初回準備を、いつ始め、いつ止めるか（spec/exercise-runner/crates-and-workspace.md「初回準備」、
//! spec/architecture/commands/environment.md、ADR 0100、ADR 0236）。
//!
//! - 初回準備の状態（`not_started`・`preparing`・`ready`・`failed`）は [`Environment`] がメモリに持つ。状態が変わる
//!   たびに `environment_updated` を送る（[`notify`]）。
//! - 初回準備を始めてよいかを確かめて `preparing` にする処理は [`begin`]、動かす処理は [`spawn_preparation`] の
//!   1 か所ずつにまとめる。起動時（[`super::start_detection`]）、再試行（[`retry_crate_preparation`]）、演習のビルドの
//!   失敗（[`start_crate_preparation`]）、再検出（[`super::redetect::redetect_tools`]）のどれも、ここを通る。
//!   `preparing` のときは重ねて始めない。
//! - 起動時に「実行する条件」を判定して、当てはまれば始め、当てはまらなければ `ready` にする処理は
//!   [`prepare_crates_if_needed`]。判定そのものは [`judge`]。
//! - 初回準備の手順そのもの（手順の 1〜5）は [`crate::runner::preparation::prepare`] が行う。アプリでは手順の 4
//!   （`target-validate/` への複製）もする。
//! - アプリの終了では、[`stop_crate_preparation_on_exit`] で、動いている初回準備の `cargo` を止める。
//!   `prepared.json` は書かない（ADR 0236）。

use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::Path;
use std::thread;
use std::time::Duration;

use tauri::{AppHandle, Emitter, Manager, Runtime};

use super::{CratesState, CratesStatus, ENVIRONMENT_UPDATED, Environment, RustTools, State};
use crate::commands::error::{CommandError, ErrorCode};
use crate::runner::RunnerDir;
use crate::runner::preparation::{PreparationOutcome, PreparationSpec, prepare};
use crate::runner::prepared::{
    PreparedStatus, crates_hash, needs_preparation, read_prepared, rustc_verbose_version,
};
use crate::runner::process::Stopper;
use crate::runner::workspace::WorkspaceDir;
use crate::storage::AppDataDir;

/// アプリを終了するときに、止めた初回準備のスレッドが終わるのを待つ最長の時間。`cargo` はすぐに止まるので、
/// ふつうは待たない。
pub const EXIT_WAIT: Duration = Duration::from_secs(5);

/// 「実行する条件」を判定した結果。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Judgment {
    /// 条件のどれかに当てはまる。初回準備を実行する。
    Needed,
    /// どれにも当てはまらない。前の初回準備（`prepared_at` に終わった）が、そのまま使える。
    Ready { prepared_at: String },
}

/// 初回準備の「実行する条件」（spec/exercise-runner/crates-and-workspace.md「初回準備」）を判定する。
///
/// `prepared.json` を読めないとき、同梱のファイルを読めずにハッシュ値を求められないとき、`rustc -vV` を実行
/// できないときも、当てはまるとする。初回準備が、その理由を `failed` のエラーの内容として記録する。
pub fn judge(rustc: &Path, runner: &RunnerDir, workspace: &WorkspaceDir) -> Judgment {
    let Ok(Some(recorded)) = read_prepared(workspace) else {
        return Judgment::Needed;
    };
    let Ok(hash) = crates_hash(runner) else {
        return Judgment::Needed;
    };
    let Some(rustc_version) = rustc_verbose_version(rustc) else {
        return Judgment::Needed;
    };
    if needs_preparation(Some(&recorded), &hash, &rustc_version) {
        Judgment::Needed
    } else {
        Judgment::Ready {
            prepared_at: recorded.prepared_at,
        }
    }
}

/// 初回準備を始める。`preparing` のとき（と、アプリを終了するところのとき）は始めずに `false` を返す。
///
/// 始めたら、状態を `preparing` にして `environment_updated` を送り、別のスレッドで初回準備を動かす。終わったら、
/// 結果（`ready` か `failed`）を状態に入れて、また送る。`tools` の `cargo`・`rustc` を使い、作業領域はアプリの状態に
/// 置いた [`AppDataDir`] の `workspace/`、同梱の許可クレートは [`RunnerDir`] とする。
pub fn start_crate_preparation<R: Runtime>(app: &AppHandle<R>, tools: &RustTools) -> bool {
    let environment = app.state::<Environment>().inner().clone();
    start_on(app, &environment, tools, false).is_ok()
}

/// 「実行する条件」を判定し、当てはまれば初回準備を始め（[`start_crate_preparation`] と同じ処理）、当てはまらなければ
/// 状態を `ready` にする。`preparing` のときは、動いている初回準備を止めず、判定もし直さない。起動時の検出が、
/// アプリの状態に置いた `environment` に使う。
pub(super) fn prepare_crates_if_needed<R: Runtime>(
    app: &AppHandle<R>,
    environment: &Environment,
    tools: &RustTools,
) {
    if environment.crates_status().status == CratesState::Preparing {
        return;
    }
    let judgment = match places(app) {
        Ok((runner, workspace)) => judge(&tools.rustc, &runner, &workspace),
        // 置き場所が分からないときは、始めて、失敗として記録する（start_on）。
        Err(_) => Judgment::Needed,
    };
    match judgment {
        Judgment::Needed => {
            let _ = start_on(app, environment, tools, false);
        }
        Judgment::Ready { prepared_at } => {
            let changed = {
                let mut state = environment.lock();
                if state.crates.status == CratesState::Preparing {
                    false
                } else {
                    state.crates = CratesStatus::ready(prepared_at);
                    true
                }
            };
            if changed {
                notify(app, environment);
            }
        }
    }
}

/// 初回準備の再試行（`retry_crate_preparation`）。検出が終わるまで待つので、メインスレッドでは呼ばない。
///
/// `cargo` が見つからないときは `cargo_unavailable`、初回準備の状態が `failed` でないときは `invalid_state` を返す。
/// 始めたら、終わるのを待たずに返す。
pub fn retry_crate_preparation<R: Runtime>(app: &AppHandle<R>) -> Result<(), CommandError> {
    let environment = app.state::<Environment>().inner().clone();
    let tools = environment.rust_tools()?;
    start_on(app, &environment, &tools, true).map_err(|refused| CommandError {
        code: ErrorCode::InvalidState,
        message: "外部クレートの準備に失敗したときだけ、やり直せます。".into(),
        detail: Some(match refused {
            Refused::Preparing => "初回準備の状態が preparing".to_string(),
            Refused::NotFailed(state) => format!("初回準備の状態が {}", state_name(state)),
            Refused::Exiting => "アプリを終了するところ".to_string(),
        }),
    })
}

/// アプリを終了するときに、動いている初回準備の `cargo` を止め、初回準備のスレッドが終わるまで [`EXIT_WAIT`] だけ
/// 待つ。止めた初回準備は `prepared.json` を書かない。この後は、初回準備を新しく始めない。
pub fn stop_crate_preparation_on_exit<R: Runtime>(app: &AppHandle<R>) {
    if let Some(environment) = app.try_state::<Environment>() {
        environment.stop_preparation_for_exit(EXIT_WAIT);
    }
}

/// 初回準備を始めなかった理由。
pub(super) enum Refused {
    /// 動いている初回準備がある。
    Preparing,
    /// 再試行で、状態が `failed` でない。
    NotFailed(CratesState),
    /// アプリを終了するところ。
    Exiting,
}

fn state_name(state: CratesState) -> &'static str {
    match state {
        CratesState::NotStarted => "not_started",
        CratesState::Preparing => "preparing",
        CratesState::Ready => "ready",
        CratesState::Failed => "failed",
    }
}

/// 初回準備を始める、ただ 1 つの処理。`only_after_failure` のときは、状態が `failed` のときだけ始める（再試行）。
/// 状態を確かめて `preparing` にするまでを、1 回のロックの間に行う（[`begin`]）。
fn start_on<R: Runtime>(
    app: &AppHandle<R>,
    environment: &Environment,
    tools: &RustTools,
    only_after_failure: bool,
) -> Result<(), Refused> {
    let started = begin(&mut environment.lock(), only_after_failure)?;
    notify(app, environment);
    spawn_preparation(app, environment, tools, started);
    Ok(())
}

/// [`begin`] で始めると決めた初回準備（番号と、その `cargo` を止める操作）。[`spawn_preparation`] で動かす。
pub(super) struct Started {
    run: u64,
    stopper: Stopper,
}

/// 初回準備を始めてよいかを確かめ、状態を `preparing` にする。状態のロックを持つ間に呼ぶ。この後、ロックを
/// 放してから `environment_updated` を送り（[`notify`]）、[`spawn_preparation`] で動かす。
pub(super) fn begin(state: &mut State, only_after_failure: bool) -> Result<Started, Refused> {
    if state.exiting {
        return Err(Refused::Exiting);
    }
    if state.crates.status == CratesState::Preparing {
        return Err(Refused::Preparing);
    }
    if only_after_failure && state.crates.status != CratesState::Failed {
        return Err(Refused::NotFailed(state.crates.status));
    }
    state.last_run += 1;
    let run = state.last_run;
    // Stopper は一度止めると戻せないので、初回準備ごとに新しく作る。
    let stopper = Stopper::new();
    state.running = Some((run, stopper.clone()));
    state.crates = CratesStatus::preparing();
    Ok(Started { run, stopper })
}

/// [`begin`] で始めると決めた初回準備を、別のスレッドで動かす。終わったら、結果（`ready` か `failed`）を状態に
/// 入れて、`environment_updated` を送る。
pub(super) fn spawn_preparation<R: Runtime>(
    app: &AppHandle<R>,
    environment: &Environment,
    tools: &RustTools,
    started: Started,
) {
    let Started { run, stopper } = started;
    let app = app.clone();
    let environment = environment.clone();
    let places = places(&app);
    let tools = tools.clone();
    thread::spawn(move || {
        let result = match places {
            Ok((runner, workspace)) => run_preparation(
                PreparationSpec {
                    runner,
                    workspace,
                    cargo: tools.cargo,
                    rustc: tools.rustc,
                    copy_validate_target: true,
                },
                &stopper,
            ),
            Err(error) => Some(CratesStatus::failed(error)),
        };
        if environment.end_preparation(run, result) {
            notify(&app, &environment);
        }
    });
}

/// 初回準備を 1 回動かし、終わった後の状態を返す。止めたときは `None`（状態を変えない）。
fn run_preparation(spec: PreparationSpec, stopper: &Stopper) -> Option<CratesStatus> {
    let outcome = catch_unwind(AssertUnwindSafe(|| prepare(&spec, stopper)));
    match outcome {
        Ok(Ok(PreparationOutcome::Finished(record))) => Some(match record.status {
            PreparedStatus::Ready => CratesStatus::ready(record.prepared_at),
            PreparedStatus::Failed => CratesStatus::failed(record.error.unwrap_or_default()),
        }),
        Ok(Ok(PreparationOutcome::Stopped)) => None,
        Ok(Err(error)) => Some(CratesStatus::failed(format!(
            "初回準備の結果を prepared.json に書けませんでした: {error}"
        ))),
        Err(_) => Some(CratesStatus::failed(
            "初回準備の途中で、想定外のエラーが起きました。",
        )),
    }
}

/// 同梱の許可クレートの定義（アプリの状態の [`RunnerDir`]）と、作業領域（アプリの状態の [`AppDataDir`] の
/// `workspace/`）。アプリでは、検出を始める前に置いてある。
pub(super) fn places<R: Runtime>(app: &AppHandle<R>) -> Result<(RunnerDir, WorkspaceDir), String> {
    let runner = app
        .try_state::<RunnerDir>()
        .ok_or("同梱の許可クレートの場所が分かりません（内部エラー）。")?;
    let data_dir = app
        .try_state::<AppDataDir>()
        .ok_or("作業領域の場所が分かりません（内部エラー）。")?;
    Ok((runner.inner().clone(), WorkspaceDir::in_app_data(&data_dir)))
}

/// 検出が終わっていれば、いまの環境の状態を `environment_updated` で送る。終わっていなければ送らない（検出の
/// 終わりに送る）。送る順番をロックで守り、後に送ったものが、いつも新しい状態になるようにする。
pub(super) fn notify<R: Runtime>(app: &AppHandle<R>, environment: &Environment) {
    let _order = environment
        .shared
        .notify
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    if let Some(status) = environment.status_if_detected() {
        // 送れなくても（ウィンドウがまだ無いなど）、画面は get_environment_status で読む。
        let _ = app.emit(ENVIRONMENT_UPDATED, status);
    }
}

impl Environment {
    /// 番号 `run` の初回準備のスレッドが終わった。`result` があれば状態に入れる（止めたときは `None`）。
    /// 状態を変えたら `true`。
    fn end_preparation(&self, run: u64, result: Option<CratesStatus>) -> bool {
        let changed = {
            let mut state = self.lock();
            let current = state.running.as_ref().is_some_and(|(id, _)| *id == run);
            if current {
                state.running = None;
            }
            match result {
                Some(status) if current && !state.exiting => {
                    state.crates = status;
                    true
                }
                _ => false,
            }
        };
        self.shared.changed.notify_all();
        changed
    }

    /// 動いている初回準備の `cargo` を止め、スレッドが終わるまで `timeout` だけ待つ。この後は、初回準備を新しく
    /// 始めない。
    pub fn stop_preparation_for_exit(&self, timeout: Duration) {
        let mut state = self.lock();
        state.exiting = true;
        if let Some((_, stopper)) = &state.running {
            stopper.stop();
        }
        let _ = self
            .shared
            .changed
            .wait_timeout_while(state, timeout, |state| state.running.is_some())
            .unwrap_or_else(|poisoned| poisoned.into_inner());
    }

    /// 動いている初回準備のスレッドが終わるまで、`timeout` だけ待つ。終わった（動いていない）なら `true`。
    /// テストで、初回準備の終わりを待つのに使う。
    pub fn wait_preparation_finished(&self, timeout: Duration) -> bool {
        let (state, _) = self
            .shared
            .changed
            .wait_timeout_while(self.lock(), timeout, |state| state.running.is_some())
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        state.running.is_none()
    }
}
