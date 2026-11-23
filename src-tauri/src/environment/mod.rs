//! 外部ツールの検出と、環境の状態（spec/architecture/external-tools.md「検出」、
//! spec/architecture/commands/environment.md「環境の状態」）。
//!
//! アプリの起動時に [`start_detection`] で、画面の表示を待たせずに別のスレッドで `cargo`・`rustc` を
//! 探す。結果は [`Environment`] がメモリに持ち、`get_environment_status` と、実行・提出のコマンド
//! （[`Environment::rust_tools`]）が使う。検出が終わる前に読まれたときは、終わるまで待つ。

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
use tauri::{AppHandle, Emitter, Manager, Runtime};

use crate::commands::error::{CommandError, ErrorCode};
use crate::storage::AppDataDir;

/// `rustc` に必要な最低バージョン（spec/architecture/external-tools.md「検出」の表）。
pub const REQUIRED_RUSTC_VERSION: &str = "1.100.0";

/// 検出が終わったときと、検出し直したときに送るイベントの名前。
pub const ENVIRONMENT_UPDATED: &str = "environment_updated";

/// `rustc --version` が終わるのを待つ時間。過ぎたら止めて、その `rustc` は使えないとする。
const VERSION_TIMEOUT: Duration = Duration::from_secs(10);

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
    let mut child = Command::new(rustc)
        .arg("--version")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;
    let deadline = Instant::now() + VERSION_TIMEOUT;
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
    RustcVersion::from_version_output(&stdout)
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

/// 初回準備の状態（spec/exercise-runner/crates-and-workspace.md「初回準備の状態」）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CratesStatus {
    pub status: CratesState,
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

// ---- メモリに持つ検出結果 ----

/// 検出結果をメモリに持つ。Tauri の状態として `manage` で置く。複製は同じ結果を指す。
#[derive(Debug, Clone)]
pub struct Environment {
    shared: Arc<Shared>,
}

#[derive(Debug)]
struct Shared {
    /// `cargo`・`rustc` の検出結果。検出が終わるまでは `None`。
    cargo: Mutex<Option<Option<RustTools>>>,
    detected: Condvar,
}

impl Environment {
    /// 検出が終わっていない状態。[`Environment::finish_detection`] で結果を入れる。
    pub fn pending() -> Self {
        Self {
            shared: Arc::new(Shared {
                cargo: Mutex::new(None),
                detected: Condvar::new(),
            }),
        }
    }

    /// 検出が終わり、結果が `tools` の状態。テストでは、偽のツールのパスをここに与える。
    pub fn detected(tools: Option<RustTools>) -> Self {
        let environment = Self::pending();
        environment.finish_detection(tools);
        environment
    }

    /// 検出の結果を入れ、待っている読み手を起こす。
    pub fn finish_detection(&self, tools: Option<RustTools>) {
        *self.lock() = Some(tools);
        self.shared.detected.notify_all();
    }

    fn lock(&self) -> MutexGuard<'_, Option<Option<RustTools>>> {
        // 持っている値は検出結果の置き換えだけで、途中の状態にならないので、毒されていても使う。
        self.shared
            .cargo
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// 検出が終わるまで待って、`cargo`・`rustc` の検出結果を返す。
    fn wait_cargo(&self) -> Option<RustTools> {
        let guard = self
            .shared
            .detected
            .wait_while(self.lock(), |cargo| cargo.is_none())
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        guard.clone().flatten()
    }

    /// 検出が終わるまで待って、環境の状態を返す。待つので、メインスレッドでは呼ばない。
    pub fn status(&self) -> EnvironmentStatus {
        let tools = self.wait_cargo();
        EnvironmentStatus {
            cargo: CargoStatus::from_tools(tools.as_ref()),
            // claude は M4 で検出する。それまでは、見つからないとして返す。
            claude: ClaudeStatus {
                status: ClaudeState::NotFound,
                path: None,
                version: None,
            },
            // 初回準備は M3 で作る。それまでは、初回準備を始めていないので `not_started` を返す。
            crates: CratesStatus {
                status: CratesState::NotStarted,
                error: None,
            },
        }
    }

    /// 実行・テスト・提出で使う `cargo`・`rustc`。検出が終わるまで待つ。`rustc` のバージョンが
    /// 足りないときも返す（spec/architecture/external-tools.md「バージョンが足りない場合」）。
    /// 見つからないときは `cargo_unavailable`。待つので、メインスレッドでは呼ばない。
    pub fn rust_tools(&self) -> Result<RustTools, CommandError> {
        self.wait_cargo().ok_or_else(|| CommandError {
            code: ErrorCode::CargoUnavailable,
            message: "Rust（cargo）が見つかりません。rustup で Rust を導入してください。".into(),
            detail: None,
        })
    }
}

/// 検出を別のスレッドで始め、検出結果を持つ [`Environment`] をアプリの状態に置く。
/// 検出が終わったら、結果を入れてから `environment_updated` を送る。
pub fn start_detection<R: Runtime>(app: &AppHandle<R>, search: CargoSearch) {
    let environment = Environment::pending();
    app.manage(environment.clone());
    let app = app.clone();
    thread::spawn(move || {
        environment.finish_detection(search.detect());
        // 送れなくても（ウィンドウがまだ無いなど）、画面は get_environment_status で読む。
        let _ = app.emit(ENVIRONMENT_UPDATED, environment.status());
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
    fn a_relative_configured_path_is_not_used() {
        let search = CargoSearch {
            configured_cargo: Some("bin/cargo".into()),
            path_var: None,
            home: None,
        };
        assert_eq!(search.candidates(), []);
    }
}
