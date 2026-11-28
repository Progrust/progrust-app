//! 設定画面と初回起動画面の「再検出」（spec/architecture/commands/environment.md「`redetect_tools`」、
//! spec/exercise-runner/crates-and-workspace.md「初回準備」、ADR 0234）。
//!
//! 設定の `tool_paths` を読み直して `cargo`・`claude`・`clang` を検出し直し、メモリの検出結果を置き換える。
//! 検出し直した後、`cargo` が見つかり、初回準備の状態が `preparing` でなければ、「実行する条件」を判定し、
//! 当てはまれば初回準備を始め、当てはまらなければ `ready` にする。
//!
//! 検出と判定（`rustc -vV` を動かす）は、状態のロックの外で行う。検出結果の置き換えと初回準備の状態の変更は、
//! 1 回のロックの間に行う。そのため、実行・提出のコマンドや `get_environment_status` は、置き換える前の状態か、
//! 置き換えた後の状態のどちらかを読み、途中の状態（新しい `rustc` と前の `ready` の組など）を読まない。

use std::path::PathBuf;

use tauri::{AppHandle, Manager, Runtime};

use super::preparation::{Judgment, begin, judge, notify, places, spawn_preparation};
use super::{
    CratesState, CratesStatus, Environment, EnvironmentStatus, ToolSearch, configured_cargo,
    configured_clang, configured_claude,
};
use crate::storage::AppDataDir;

/// `cargo`・`claude`・`clang` を検出し直し、検出し直した後の環境の状態を返す。`environment_updated` も送る。
/// 検出に時間がかかり、起動時の検出が終わるのも待つので、メインスレッドでは呼ばない。
///
/// 初回準備を始めたときは、戻り値の初回準備の状態は `preparing` になる。再検出を同時に呼んだときは、1 つずつ行う。
pub fn redetect_tools<R: Runtime>(app: &AppHandle<R>) -> EnvironmentStatus {
    let environment = app.state::<Environment>().inner().clone();
    // 起動時の検出が終わる前に置き換えると、後から終わった起動時の検出の結果で上書きされるので、終わるのを待つ。
    drop(environment.wait_detected());
    // 再検出を 1 つずつ行う。前の再検出の結果が、後の再検出の結果を上書きしないようにする。
    let base = environment
        .shared
        .redetect
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());

    let (cargo, claude, clang) = tool_paths(app);
    let mut search = base
        .clone()
        .unwrap_or_else(|| ToolSearch::from_process(None, None, None));
    search.cargo.configured_cargo = cargo;
    search.claude.configured_claude = claude;
    search.clang.configured_clang = clang;
    let tools = search.detect();

    // 判定は、置き換える前に、ロックの外で行う。`preparing` のときは判定しない。`failed` のときは、それ自体が
    // 「実行する条件」に当たるので、判定しない（prepared.json に書けずに failed になったときも始め直す）。
    let judgment = tools.cargo.as_ref().and_then(|rust| {
        match environment.crates_status().status {
            CratesState::Preparing => None,
            CratesState::Failed => Some(Judgment::Needed),
            CratesState::NotStarted | CratesState::Ready => Some(match places(app) {
                Ok((runner, workspace)) => judge(&rust.rustc, &runner, &workspace),
                // 置き場所が分からないときは、始めて、失敗として記録する（spawn_preparation）。
                Err(_) => Judgment::Needed,
            }),
        }
    });

    let (status, started) = {
        let mut state = environment.lock();
        state.tools = Some(tools.clone());
        let started = match judgment {
            // 判定の後に、ほかで初回準備が始まったときは、それを止めず、状態も変えない。
            _ if state.crates.status == CratesState::Preparing => None,
            // 判定の後に失敗したときも、始め直す。
            Some(_) if state.crates.status == CratesState::Failed => begin(&mut state, false).ok(),
            Some(Judgment::Needed) => begin(&mut state, false).ok(),
            Some(Judgment::Ready { prepared_at }) => {
                state.crates = CratesStatus::ready(prepared_at);
                None
            }
            // `cargo` が見つからないときは、初回準備の状態を変えない。
            None => None,
        };
        let status = Environment::status_of(&state).expect("検出結果を入れた");
        (status, started)
    };
    notify(app, &environment);
    // 始めるのは、`cargo` が見つかったときだけ。
    if let (Some(started), Some(rust)) = (started, &tools.cargo) {
        spawn_preparation(app, &environment, rust, started);
    }
    status
}

/// 設定（`settings.json`）の `tool_paths.cargo`・`tool_paths.claude`・`tool_paths.clang`。学習者データの
/// ディレクトリが分からないとき、設定を読めないときは、設定のパスを使わずに探す（起動時と同じ）。
fn tool_paths<R: Runtime>(
    app: &AppHandle<R>,
) -> (Option<PathBuf>, Option<PathBuf>, Option<PathBuf>) {
    match app.try_state::<AppDataDir>() {
        Some(data_dir) => (
            configured_cargo(&data_dir),
            configured_claude(&data_dir),
            configured_clang(&data_dir),
        ),
        None => (None, None, None),
    }
}
