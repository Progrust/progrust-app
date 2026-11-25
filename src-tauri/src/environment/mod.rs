//! 外部ツールの検出と、環境の状態（spec/architecture/external-tools.md「検出」、
//! spec/architecture/commands/environment.md「環境の状態」）。
//!
//! アプリの起動時に [`start_detection`] で、画面の表示を待たせずに別のスレッドで `cargo`・`rustc` と
//! `claude` を探す。結果は [`Environment`] がメモリに持ち、`get_environment_status` と、実行・提出の
//! コマンド（[`Environment::rust_tools`]）と、`claude` を起動する処理（[`Environment::claude`]）が使う。
//! 検出が終わる前に読まれたときは、終わるまで待つ。設定画面と初回起動画面の「再検出」（`redetect_tools`）では、
//! [`redetect`] が検出し直して、メモリの検出結果を置き換える。
//!
//! 許可クレートの初回準備の状態（spec/exercise-runner/crates-and-workspace.md「初回準備の状態」）も
//! [`Environment`] が持つ。初回準備を始める・止める処理は [`preparation`] にある。

use std::{
    ffi::OsString,
    fmt,
    io::Read,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    str::FromStr,
    sync::{Arc, Condvar, Mutex, MutexGuard},
    thread,
    time::{Duration, Instant},
};

use serde::Serialize;
use tauri::{AppHandle, Manager, Runtime};

use crate::claude::ClaudeEnv;
use crate::commands::error::{CommandError, ErrorCode};
use crate::runner::process::Stopper;
use crate::storage::AppDataDir;

pub mod preparation;
pub mod redetect;

/// `rustc` に必要な最低バージョン（spec/architecture/external-tools.md「検出」の表）。
pub const REQUIRED_RUSTC_VERSION: &str = "1.100.0";

/// 検出が終わったときと、検出し直したときに送るイベントの名前。
pub const ENVIRONMENT_UPDATED: &str = "environment_updated";

/// `rustc --version`・`claude --version` が終わるのを待つ時間。過ぎたら止めて、その場所は使えないとする。
pub const VERSION_TIMEOUT: Duration = Duration::from_secs(10);

/// `claude` を探す npm のグローバルインストール先（spec/architecture/external-tools.md「検出」）。
/// Homebrew の Node.js（Apple シリコン）と、公式のインストーラーの Node.js の場所を、この順に調べる。
pub const NPM_GLOBAL_DIRS: [&str; 2] = ["/opt/homebrew/bin", "/usr/local/bin"];

// ---- rustc のバージョン ----

/// `rustc --version` から取り出したバージョン。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RustcVersion {
    /// 出力のとおりの文字列（例: `1.100.0`、`1.101.0-nightly`）。
    text: String,
    /// 比べるための数（メジャー、マイナー、パッチ）。`-nightly` などの後ろの部分は比べない。
    numbers: (u64, u64, u64),
}

impl RustcVersion {
    /// `rustc --version` の出力（例: `rustc 1.100.0 (0123456789 2026-11-09)`）から取り出す。
    pub fn from_version_output(output: &str) -> Option<Self> {
        let line = output.lines().next()?;
        let mut words = line.split_whitespace();
        if words.next()? != "rustc" {
            return None;
        }
        words.next()?.parse().ok()
    }

    /// 必要な最低バージョン（[`REQUIRED_RUSTC_VERSION`]）。
    pub fn required() -> Self {
        REQUIRED_RUSTC_VERSION
            .parse()
            .expect("REQUIRED_RUSTC_VERSION はバージョンの形")
    }

    /// 必要条件（[`REQUIRED_RUSTC_VERSION`] 以上）を満たすか。
    pub fn meets_requirement(&self) -> bool {
        self.numbers >= Self::required().numbers
    }

    pub fn as_str(&self) -> &str {
        &self.text
    }
}

impl FromStr for RustcVersion {
    type Err = ();

    /// `1.100.0` や `1.101.0-nightly` の形の文字列から作る。
    fn from_str(text: &str) -> Result<Self, ()> {
        let core = text.split_once('-').map_or(text, |(core, _)| core);
        let mut parts = core.split('.').map(|part| {
            if !part.is_empty() && part.bytes().all(|byte| byte.is_ascii_digit()) {
                part.parse::<u64>().map_err(|_| ())
            } else {
                Err(())
            }
        });
        let numbers = (
            parts.next().ok_or(())??,
            parts.next().ok_or(())??,
            parts.next().ok_or(())??,
        );
        if parts.next().is_some() {
            return Err(());
        }
        Ok(Self {
            text: text.to_string(),
            numbers,
        })
    }
}

impl fmt::Display for RustcVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.text)
    }
}

// ---- cargo・rustc の検出 ----

/// 見つかった `cargo`・`rustc`。実行・提出のコマンドは、これのパスでツールを起動する。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RustTools {
    /// `cargo` の絶対パス。
    pub cargo: PathBuf,
    /// `rustc` の絶対パス。`cargo` と同じディレクトリにある。
    pub rustc: PathBuf,
    pub rustc_version: RustcVersion,
}

/// `cargo`・`rustc` を探すときの入力。
///
/// 探す順は、`configured_cargo` → `path_var` の各ディレクトリ → `home` の `.cargo/bin` とする。
/// それぞれの場所で、`cargo` と、同じディレクトリの `rustc` が両方とも実行できるファイルで、
/// `rustc --version` からバージョンを取り出せたら、それを使う。バージョンが足りなくても、
/// そこで探すのをやめる。
#[derive(Debug, Clone, Default)]
pub struct CargoSearch {
    /// 設定の `tool_paths.cargo`（`cargo` の実行ファイルのパス）。
    pub configured_cargo: Option<PathBuf>,
    /// 環境変数 `PATH` の値。
    pub path_var: Option<OsString>,
    /// ホームディレクトリ。
    pub home: Option<PathBuf>,
}

/// 設定（`settings.json`）の `tool_paths.cargo`。`null` のとき、設定を読めないときは `None`。
pub fn configured_cargo(data_dir: &AppDataDir) -> Option<PathBuf> {
    data_dir
        .read_settings()
        .ok()
        .and_then(|settings| settings.tool_paths.cargo)
        .map(PathBuf::from)
}

impl CargoSearch {
    /// アプリのプロセスの `PATH` とホームディレクトリと、設定の `tool_paths.cargo`
    /// （[`configured_cargo`]）から作る。
    pub fn from_process(configured_cargo: Option<PathBuf>) -> Self {
        Self {
            configured_cargo,
            path_var: std::env::var_os("PATH"),
            home: std::env::home_dir(),
        }
    }

    /// 調べる `cargo` と `rustc` の組を、探す順に返す。相対パスの場所は使わない。
    fn candidates(&self) -> Vec<(PathBuf, PathBuf)> {
        let mut candidates = Vec::new();
        let mut push = |cargo: PathBuf| {
            if !cargo.is_absolute() {
                return;
            }
            let rustc = cargo.with_file_name("rustc");
            let pair = (cargo, rustc);
            if !candidates.contains(&pair) {
                candidates.push(pair);
            }
        };
        if let Some(cargo) = &self.configured_cargo {
            push(cargo.clone());
        }
        if let Some(path_var) = &self.path_var {
            for dir in std::env::split_paths(path_var) {
                push(dir.join("cargo"));
            }
        }
        if let Some(home) = &self.home {
            push(home.join(".cargo/bin/cargo"));
        }
        candidates
    }

    /// `cargo`・`rustc` を探す。見つからないときは `None`。
    pub fn detect(&self) -> Option<RustTools> {
        self.candidates().into_iter().find_map(|(cargo, rustc)| {
            if !is_executable_file(&cargo) || !is_executable_file(&rustc) {
                return None;
            }
            let rustc_version = rustc_version(&rustc)?;
            Some(RustTools {
                cargo,
                rustc,
                rustc_version,
            })
        })
    }
}

/// 実行できるファイル（シンボリックリンクの先を含む）か。
fn is_executable_file(path: &Path) -> bool {
    std::fs::metadata(path)
        .is_ok_and(|metadata| metadata.is_file() && metadata.permissions().mode() & 0o111 != 0)
}

/// `rustc --version` を実行して、バージョンを取り出す。失敗したとき、時間内に終わらないときは `None`。
fn rustc_version(rustc: &Path) -> Option<RustcVersion> {
    let stdout = tool_stdout(rustc, &["--version"], VERSION_TIMEOUT)?;
    RustcVersion::from_version_output(&stdout)
}

/// ツール `program` を引数 `args` で実行して、標準出力を返す。起動できないとき、終了コードが 0 でないとき、
/// 標準出力が UTF-8 でないとき、`timeout` の間に終わらないとき（止める）は `None`。
/// 標準出力は終わってから読むので、出力が短いもの（`--version` など）にだけ使う。
pub(crate) fn tool_stdout(program: &Path, args: &[&str], timeout: Duration) -> Option<String> {
    let mut command = Command::new(program);
    command.args(args);
    command_stdout(command, timeout)
}

/// [`tool_stdout`] と同じく、組み立てた `command` を実行して、標準出力を返す。環境変数などを
/// 呼び出し側で決めるときに使う。標準入力は空にし、標準エラー出力は捨てる。
fn command_stdout(mut command: Command, timeout: Duration) -> Option<String> {
    let mut child = command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;
    let deadline = Instant::now() + timeout;
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if Instant::now() < deadline => thread::sleep(Duration::from_millis(10)),
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                return None;
            }
        }
    };
    if !status.success() {
        return None;
    }
    let mut stdout = String::new();
    child.stdout.take()?.read_to_string(&mut stdout).ok()?;
    Some(stdout)
}

// ---- claude の検出 ----

/// 見つかった `claude`。`claude` を起動する処理は、これのパスで起動する。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClaudeTool {
    /// `claude` の絶対パス。
    pub path: PathBuf,
    /// `claude --version` から取り出したバージョン（例: `2.1.329`）。
    pub version: String,
}

/// `claude --version` の出力（例: `2.1.329 (Claude Code)`）から、1 行目の最初の語をバージョンとして
/// 取り出す。語が無いときは `None`。必要なバージョンの条件が無いので、語の形は確かめない。
pub fn claude_version_from_output(output: &str) -> Option<String> {
    output
        .lines()
        .next()?
        .split_whitespace()
        .next()
        .map(str::to_string)
}

/// `claude` を探すときの入力。
///
/// 探す順は、`configured_claude` → `path_var` の各ディレクトリ → `home` の `.local/bin` →
/// `home` の `.claude/local` → `npm_global_dirs` の各ディレクトリとする。それぞれの場所で、`claude` が
/// 実行できるファイルで、`claude --version` が `timeout` の間に終了コード 0 で終わり、出力の 1 行目の
/// 最初の語が空でなければ、それを使う。
#[derive(Debug, Clone)]
pub struct ClaudeSearch {
    /// 設定の `tool_paths.claude`（`claude` の実行ファイルのパス）。
    pub configured_claude: Option<PathBuf>,
    /// 環境変数 `PATH` の値。
    pub path_var: Option<OsString>,
    /// ホームディレクトリ。
    pub home: Option<PathBuf>,
    /// npm のグローバルインストール先。アプリでは [`NPM_GLOBAL_DIRS`]。
    pub npm_global_dirs: Vec<PathBuf>,
    /// `claude --version` に渡す環境変数。
    pub env: ClaudeEnv,
    /// `claude --version` が終わるのを待つ時間。アプリでは [`VERSION_TIMEOUT`]。
    pub timeout: Duration,
}

/// 設定（`settings.json`）の `tool_paths.claude`。`null` のとき、設定を読めないときは `None`。
pub fn configured_claude(data_dir: &AppDataDir) -> Option<PathBuf> {
    data_dir
        .read_settings()
        .ok()
        .and_then(|settings| settings.tool_paths.claude)
        .map(PathBuf::from)
}

impl ClaudeSearch {
    /// アプリのプロセスの `PATH`・ホームディレクトリ・環境変数と、設定の `tool_paths.claude`
    /// （[`configured_claude`]）から作る。
    pub fn from_process(configured_claude: Option<PathBuf>) -> Self {
        Self {
            configured_claude,
            path_var: std::env::var_os("PATH"),
            home: std::env::home_dir(),
            npm_global_dirs: NPM_GLOBAL_DIRS.iter().map(PathBuf::from).collect(),
            env: ClaudeEnv::from_process(),
            timeout: VERSION_TIMEOUT,
        }
    }

    /// どこも探さない。テストで、`claude` が見つからない状態にするときに使う。
    pub fn nowhere() -> Self {
        Self {
            configured_claude: None,
            path_var: None,
            home: None,
            npm_global_dirs: Vec::new(),
            env: ClaudeEnv::default(),
            timeout: VERSION_TIMEOUT,
        }
    }

    /// 調べる `claude` を、探す順に返す。相対パスの場所は使わない。
    fn candidates(&self) -> Vec<PathBuf> {
        let mut candidates = Vec::new();
        let mut push = |claude: PathBuf| {
            if claude.is_absolute() && !candidates.contains(&claude) {
                candidates.push(claude);
            }
        };
        if let Some(claude) = &self.configured_claude {
            push(claude.clone());
        }
        if let Some(path_var) = &self.path_var {
            for dir in std::env::split_paths(path_var) {
                push(dir.join("claude"));
            }
        }
        if let Some(home) = &self.home {
            push(home.join(".local/bin/claude"));
            push(home.join(".claude/local/claude"));
        }
        for dir in &self.npm_global_dirs {
            push(dir.join("claude"));
        }
        candidates
    }

    /// `claude` を探す。見つからないときは `None`。
    pub fn detect(&self) -> Option<ClaudeTool> {
        self.candidates().into_iter().find_map(|path| {
            if !is_executable_file(&path) {
                return None;
            }
            let version = self.version(&path)?;
            Some(ClaudeTool { path, version })
        })
    }

    /// `claude --version` を、[`ClaudeSearch::env`] の環境変数だけを渡して実行し、バージョンを取り出す。
    /// 失敗したとき、時間内に終わらないとき、出力が空のときは `None`。
    fn version(&self, claude: &Path) -> Option<String> {
        let mut command = self.env.command(claude);
        command.arg("--version");
        let stdout = command_stdout(command, self.timeout)?;
        claude_version_from_output(&stdout)
    }
}

// ---- cargo と claude の検出 ----

/// 検出の結果。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DetectedTools {
    /// 見つかった `cargo`・`rustc`。見つからないときは `None`。
    pub cargo: Option<RustTools>,
    /// 見つかった `claude`。見つからないときは `None`。
    pub claude: Option<ClaudeTool>,
}

/// `cargo`・`rustc` と `claude` を探すときの入力。
#[derive(Debug, Clone)]
pub struct ToolSearch {
    pub cargo: CargoSearch,
    pub claude: ClaudeSearch,
}

impl ToolSearch {
    /// アプリのプロセスの環境と、設定の `tool_paths` から作る。
    pub fn from_process(
        configured_cargo: Option<PathBuf>,
        configured_claude: Option<PathBuf>,
    ) -> Self {
        Self {
            cargo: CargoSearch::from_process(configured_cargo),
            claude: ClaudeSearch::from_process(configured_claude),
        }
    }

    /// `cargo`・`rustc` と `claude` を、並べて探す。どちらかに応答しない場所があっても、検出にかかる
    /// 時間が 2 つの合計にならないようにする。
    pub fn detect(&self) -> DetectedTools {
        thread::scope(|scope| {
            let claude = scope.spawn(|| self.claude.detect());
            let cargo = self.cargo.detect();
            DetectedTools {
                cargo,
                // 検出のスレッドが落ちたときは、見つからないとする。
                claude: claude.join().ok().flatten(),
            }
        })
    }
}

// ---- 環境の状態（get_environment_status の戻り値と environment_updated の内容）----

/// 環境の状態。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct EnvironmentStatus {
    pub cargo: CargoStatus,
    pub claude: ClaudeStatus,
    pub crates: CratesStatus,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CargoStatus {
    pub status: CargoState,
    pub cargo_path: Option<String>,
    pub rustc_path: Option<String>,
    pub rustc_version: Option<String>,
    pub required_rustc_version: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CargoState {
    Found,
    NotFound,
    VersionTooOld,
}

impl CargoStatus {
    fn from_tools(tools: Option<&RustTools>) -> Self {
        let required_rustc_version = REQUIRED_RUSTC_VERSION.to_string();
        match tools {
            None => Self {
                status: CargoState::NotFound,
                cargo_path: None,
                rustc_path: None,
                rustc_version: None,
                required_rustc_version,
            },
            Some(tools) => Self {
                status: if tools.rustc_version.meets_requirement() {
                    CargoState::Found
                } else {
                    CargoState::VersionTooOld
                },
                cargo_path: Some(tools.cargo.to_string_lossy().into_owned()),
                rustc_path: Some(tools.rustc.to_string_lossy().into_owned()),
                rustc_version: Some(tools.rustc_version.to_string()),
                required_rustc_version,
            },
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ClaudeStatus {
    pub status: ClaudeState,
    pub path: Option<String>,
    pub version: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ClaudeState {
    Found,
    NotFound,
}

impl ClaudeStatus {
    fn from_tool(tool: Option<&ClaudeTool>) -> Self {
        match tool {
            None => Self {
                status: ClaudeState::NotFound,
                path: None,
                version: None,
            },
            Some(tool) => Self {
                status: ClaudeState::Found,
                path: Some(tool.path.to_string_lossy().into_owned()),
                version: Some(tool.version.clone()),
            },
        }
    }
}

/// 初回準備の状態（spec/exercise-runner/crates-and-workspace.md「初回準備の状態」、
/// spec/architecture/commands/environment.md「環境の状態」の `crates`）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CratesStatus {
    pub status: CratesState,
    /// `ready` のときの、初回準備が終わった日時（`prepared.json` の `prepared_at`）。ほかの状態では `None`。
    pub prepared_at: Option<String>,
    /// `failed` のときのエラーの内容。ほかの状態では `None`。
    pub error: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CratesState {
    NotStarted,
    Preparing,
    Ready,
    Failed,
}

impl CratesStatus {
    /// 初回準備を始めていない（`cargo` が見つからず、判定していないときを含む）。
    pub fn not_started() -> Self {
        Self::with(CratesState::NotStarted)
    }

    /// 初回準備を実行している。
    pub fn preparing() -> Self {
        Self::with(CratesState::Preparing)
    }

    /// 初回準備が `prepared_at` に成功して終わり、「実行する条件」のどれにも当てはまらない。
    pub fn ready(prepared_at: impl Into<String>) -> Self {
        Self {
            prepared_at: Some(prepared_at.into()),
            ..Self::with(CratesState::Ready)
        }
    }

    /// 最後の初回準備が、エラーの内容 `error` で失敗した。
    pub fn failed(error: impl Into<String>) -> Self {
        Self {
            error: Some(error.into()),
            ..Self::with(CratesState::Failed)
        }
    }

    fn with(status: CratesState) -> Self {
        Self {
            status,
            prepared_at: None,
            error: None,
        }
    }
}

// ---- メモリに持つ検出結果と初回準備の状態 ----

/// 検出結果と初回準備の状態をメモリに持つ。Tauri の状態として `manage` で置く。複製は同じ状態を指す。
#[derive(Debug, Clone)]
pub struct Environment {
    shared: Arc<Shared>,
}

#[derive(Debug)]
struct Shared {
    state: Mutex<State>,
    /// 検出が終わったときと、初回準備のスレッドが終わったときに起こす。
    changed: Condvar,
    /// `environment_updated` を送る順番を守るロック（[`preparation::notify`]）。
    notify: Mutex<()>,
    /// 再検出（[`redetect::redetect_tools`]）で使う、探す場所（`PATH`・ホームディレクトリなど）。設定の
    /// `tool_paths` は、再検出のたびに読み直して入れ替える。起動時の検出（[`start_detection`]）が置く。置いて
    /// いないときは、アプリのプロセスの環境から作る。再検出を 1 つずつ行うためのロックも兼ねる。
    redetect: Mutex<Option<ToolSearch>>,
}

/// 1 つのロックで守る状態。検出結果と初回準備の状態を、一緒に読み書きする。
#[derive(Default)]
struct State {
    /// `cargo`・`rustc` と `claude` の検出結果。検出が終わるまでは `None`。
    tools: Option<DetectedTools>,
    /// 初回準備の状態。
    crates: CratesStatus,
    /// 動いている初回準備（番号と、その `cargo` を止める操作）。動いていないときは `None`。
    running: Option<(u64, Stopper)>,
    /// 最後に始めた初回準備の番号。
    last_run: u64,
    /// アプリを終了するところで、初回準備を新しく始めない。
    exiting: bool,
}

impl Default for CratesStatus {
    fn default() -> Self {
        Self::not_started()
    }
}

impl fmt::Debug for State {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("State")
            .field("tools", &self.tools)
            .field("crates", &self.crates)
            .field("running", &self.running.as_ref().map(|(run, _)| run))
            .field("exiting", &self.exiting)
            .finish()
    }
}

impl Environment {
    /// 検出が終わっていない状態。[`Environment::finish_detection`] で結果を入れる。
    /// 初回準備の状態は `not_started`。
    pub fn pending() -> Self {
        Self {
            shared: Arc::new(Shared {
                state: Mutex::new(State::default()),
                changed: Condvar::new(),
                notify: Mutex::new(()),
                redetect: Mutex::new(None),
            }),
        }
    }

    /// 検出が終わり、結果が `tools` の状態。初回準備は始めない（`not_started`）。テストでは、偽のツールの
    /// パスをここに与える。
    pub fn detected(tools: DetectedTools) -> Self {
        let environment = Self::pending();
        environment.finish_detection(tools);
        environment
    }

    /// 検出の結果を入れ、待っている読み手を起こす。初回準備の状態は変えない。
    pub fn finish_detection(&self, tools: DetectedTools) {
        self.lock().tools = Some(tools);
        self.shared.changed.notify_all();
    }

    fn lock(&self) -> MutexGuard<'_, State> {
        // 状態はロックを持つ間に書き換え終わり、途中の状態にならないので、毒されていても使う。
        self.shared
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// 検出が終わるまで待って、状態のロックを返す。
    fn wait_detected(&self) -> MutexGuard<'_, State> {
        self.shared
            .changed
            .wait_while(self.lock(), |state| state.tools.is_none())
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// 検出が終わるまで待って、検出結果を返す。
    fn wait_tools(&self) -> DetectedTools {
        self.wait_detected().tools.clone().unwrap_or_default()
    }

    /// 検出が終わるまで待って、環境の状態を返す。待つので、メインスレッドでは呼ばない。
    pub fn status(&self) -> EnvironmentStatus {
        Self::status_of(&self.wait_detected()).expect("検出が終わるまで待った")
    }

    /// 検出が終わっていれば、環境の状態を返す。終わっていなければ、待たずに `None` を返す。
    fn status_if_detected(&self) -> Option<EnvironmentStatus> {
        Self::status_of(&self.lock())
    }

    fn status_of(state: &State) -> Option<EnvironmentStatus> {
        let tools = state.tools.as_ref()?;
        Some(EnvironmentStatus {
            cargo: CargoStatus::from_tools(tools.cargo.as_ref()),
            claude: ClaudeStatus::from_tool(tools.claude.as_ref()),
            crates: state.crates.clone(),
        })
    }

    /// いまの初回準備の状態（spec/exercise-runner/crates-and-workspace.md「初回準備の状態」）。
    /// 検出が終わるのを待たない。
    pub fn crates_status(&self) -> CratesStatus {
        self.lock().crates.clone()
    }

    /// 許可クレートを使う演習を実行・提出できるか。初回準備の状態が `ready` でないときは `crates_not_ready`。
    pub fn ensure_crates_ready(&self) -> Result<(), CommandError> {
        let crates = self.crates_status();
        if crates.status == CratesState::Ready {
            return Ok(());
        }
        Err(CommandError {
            code: ErrorCode::CratesNotReady,
            message:
                "この演習で使うクレートの準備が終わっていません。準備が終わるまで待ってください。"
                    .into(),
            detail: crates.error,
        })
    }

    /// 実行・テスト・提出で使う `cargo`・`rustc`。検出が終わるまで待つ。`rustc` のバージョンが
    /// 足りないときも返す（spec/architecture/external-tools.md「バージョンが足りない場合」）。
    /// 見つからないときは `cargo_unavailable`。待つので、メインスレッドでは呼ばない。
    pub fn rust_tools(&self) -> Result<RustTools, CommandError> {
        self.wait_tools().cargo.ok_or_else(|| CommandError {
            code: ErrorCode::CargoUnavailable,
            message: "Rust（cargo）が見つかりません。rustup で Rust を導入してください。".into(),
            detail: None,
        })
    }

    /// `claude` を起動するときに使う、見つかった `claude`。検出が終わるまで待つ。見つからないときは
    /// `None`。待つので、メインスレッドでは呼ばない。
    pub fn claude(&self) -> Option<ClaudeTool> {
        self.wait_tools().claude
    }
}

/// 検出を別のスレッドで始め、検出結果を持つ [`Environment`] をアプリの状態に置く。
///
/// 検出が終わったら、`cargo` が見つかったときは、初回準備の「実行する条件」を判定し、当てはまれば初回準備を
/// 始める（[`preparation::prepare_crates_if_needed`]）。見つからないときは判定せず、`not_started` のままにする。
/// 判定を終えてから検出結果を入れ、`environment_updated` を送る。そのため、`get_environment_status` が返す
/// 最初の状態に、判定の結果（`preparing` か `ready`）が入る。
///
/// 初回準備は、アプリの状態に置いた [`RunnerDir`](crate::runner::RunnerDir) と
/// [`AppDataDir`] の作業領域を使う。この関数より先に置いておく。
///
/// `search` は、再検出（[`redetect::redetect_tools`]）でも、探す場所として使う（設定の `tool_paths` は読み直す）。
pub fn start_detection<R: Runtime>(app: &AppHandle<R>, search: ToolSearch) {
    let environment = Environment::pending();
    *environment
        .shared
        .redetect
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(search.clone());
    app.manage(environment.clone());
    let app = app.clone();
    thread::spawn(move || {
        let tools = search.detect();
        if let Some(rust) = &tools.cargo {
            preparation::prepare_crates_if_needed(&app, &environment, rust);
        }
        environment.finish_detection(tools);
        preparation::notify(&app, &environment);
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn version(text: &str) -> RustcVersion {
        text.parse()
            .unwrap_or_else(|()| panic!("{text} を読めない"))
    }

    #[test]
    fn parses_version_output() {
        let parsed =
            RustcVersion::from_version_output("rustc 1.100.0 (0123456789 2026-11-09)\n").unwrap();
        assert_eq!(parsed.as_str(), "1.100.0");
        let nightly =
            RustcVersion::from_version_output("rustc 1.101.0-nightly (0123456789 2026-11-20)")
                .unwrap();
        assert_eq!(nightly.as_str(), "1.101.0-nightly");
        assert_eq!(nightly.numbers, (1, 101, 0));
    }

    #[test]
    fn rejects_other_output() {
        for output in [
            "",
            "cargo 1.100.0 (0123456789 2026-11-09)",
            "rustc",
            "rustc version unknown",
            "rustc 1.100",
            "rustc 1.100.0.1",
            "rustc 1.x.0",
            "rustc +1.100.0",
        ] {
            assert_eq!(
                RustcVersion::from_version_output(output),
                None,
                "{output:?}"
            );
        }
    }

    #[test]
    fn compares_versions_as_numbers() {
        assert!(version("1.100.0").meets_requirement());
        assert!(version("1.100.1").meets_requirement());
        assert!(version("1.101.0-nightly").meets_requirement());
        assert!(version("2.0.0").meets_requirement());
        // 文字列として比べると、1.99.1 が 1.100.0 より大きくなる。
        assert!(!version("1.99.1").meets_requirement());
        assert!(!version("1.9.0").meets_requirement());
        assert!(!version("0.200.0").meets_requirement());
    }

    #[test]
    fn required_version_is_the_constant() {
        assert_eq!(RustcVersion::required().as_str(), REQUIRED_RUSTC_VERSION);
        assert_eq!(REQUIRED_RUSTC_VERSION, "1.100.0");
    }

    #[test]
    fn candidates_follow_the_search_order() {
        let search = CargoSearch {
            configured_cargo: Some("/opt/rust/bin/cargo".into()),
            path_var: Some("/usr/bin:relative/bin:/opt/rust/bin:/home/me/.cargo/bin".into()),
            home: Some("/home/me".into()),
        };
        let candidates: Vec<(PathBuf, PathBuf)> =
            ["/opt/rust/bin", "/usr/bin", "/home/me/.cargo/bin"]
                .iter()
                .map(|dir| (Path::new(dir).join("cargo"), Path::new(dir).join("rustc")))
                .collect();
        // 相対パスの場所は使わず、同じ場所は 2 度調べない。
        assert_eq!(search.candidates(), candidates);
    }

    #[test]
    fn tool_stdout_returns_the_standard_output() {
        assert_eq!(
            tool_stdout(Path::new("/bin/echo"), &["a", "b"], VERSION_TIMEOUT).as_deref(),
            Some("a b\n")
        );
    }

    #[test]
    fn tool_stdout_of_a_failing_or_missing_tool_is_none() {
        assert_eq!(
            tool_stdout(Path::new("/usr/bin/false"), &[], VERSION_TIMEOUT),
            None
        );
        assert_eq!(
            tool_stdout(Path::new("/nonexistent/rustc"), &["-vV"], VERSION_TIMEOUT),
            None
        );
    }

    /// 時間内に終わらないツールは、止めて `None` にする。
    #[test]
    fn tool_stdout_stops_a_tool_that_does_not_finish_in_time() {
        let started = Instant::now();
        assert_eq!(
            tool_stdout(Path::new("/bin/sleep"), &["30"], Duration::from_millis(100)),
            None
        );
        assert!(
            started.elapsed() < Duration::from_secs(10),
            "{:?}",
            started.elapsed()
        );
    }

    #[test]
    fn a_relative_configured_path_is_not_used() {
        let search = CargoSearch {
            configured_cargo: Some("bin/cargo".into()),
            path_var: None,
            home: None,
        };
        assert_eq!(search.candidates(), []);
    }

    #[test]
    fn parses_claude_version_output() {
        for (output, expected) in [
            ("2.1.329 (Claude Code)\n", "2.1.329"),
            ("2.1.329", "2.1.329"),
            ("  2.2.0-beta (Claude Code)\nsecond line\n", "2.2.0-beta"),
            // 語の形は確かめない。
            ("unknown\n", "unknown"),
        ] {
            assert_eq!(
                claude_version_from_output(output).as_deref(),
                Some(expected),
                "{output:?}"
            );
        }
    }

    #[test]
    fn empty_claude_version_output_has_no_version() {
        for output in ["", "\n", "   \n", "\n2.1.329 (Claude Code)\n"] {
            assert_eq!(claude_version_from_output(output), None, "{output:?}");
        }
    }

    #[test]
    fn claude_candidates_follow_the_search_order() {
        let search = ClaudeSearch {
            configured_claude: Some("/opt/claude/claude".into()),
            path_var: Some("/usr/bin:relative/bin:/opt/claude:/home/me/.local/bin".into()),
            home: Some("/home/me".into()),
            npm_global_dirs: vec!["/opt/homebrew/bin".into(), "/usr/local/bin".into()],
            ..ClaudeSearch::nowhere()
        };
        let expected: Vec<PathBuf> = [
            "/opt/claude/claude",
            "/usr/bin/claude",
            "/home/me/.local/bin/claude",
            "/home/me/.claude/local/claude",
            "/opt/homebrew/bin/claude",
            "/usr/local/bin/claude",
        ]
        .iter()
        .map(PathBuf::from)
        .collect();
        // 相対パスの場所は使わず、同じ場所は 2 度調べない。
        assert_eq!(search.candidates(), expected);
    }

    #[test]
    fn a_relative_configured_claude_is_not_used() {
        let search = ClaudeSearch {
            configured_claude: Some("bin/claude".into()),
            ..ClaudeSearch::nowhere()
        };
        assert_eq!(search.candidates(), Vec::<PathBuf>::new());
    }

    /// アプリでは、npm のグローバルインストール先として /opt/homebrew/bin → /usr/local/bin を調べ、
    /// `claude --version` を 10 秒で打ち切り、6 つの環境変数だけを渡す。
    #[test]
    fn claude_search_from_process_uses_the_app_settings() {
        let search = ClaudeSearch::from_process(Some("/opt/claude/claude".into()));
        assert_eq!(
            search.configured_claude.as_deref(),
            Some(Path::new("/opt/claude/claude"))
        );
        assert_eq!(
            search.npm_global_dirs,
            [
                PathBuf::from("/opt/homebrew/bin"),
                PathBuf::from("/usr/local/bin")
            ]
        );
        assert_eq!(search.timeout, Duration::from_secs(10));
        assert_eq!(VERSION_TIMEOUT, Duration::from_secs(10));
        assert_eq!(search.env, ClaudeEnv::from_process());
        assert_eq!(search.path_var, std::env::var_os("PATH"));
        assert_eq!(search.home, std::env::home_dir());
    }

    #[test]
    fn nowhere_has_no_candidates() {
        assert_eq!(ClaudeSearch::nowhere().candidates(), Vec::<PathBuf>::new());
    }
}
