//! `claude -p` を 1 回呼ぶ部品（spec/claude-integration/invocation.md、spec/claude-integration/errors.md）。
//! 質疑応答・接続テスト・ジョブが、すべてこれを通して `claude` を呼ぶ。
//!
//! - [`ClaudeCall::new`] で呼び出しを作り、[`ClaudeCall::run`] で呼ぶ。`run` は、`claude` のプロセスが
//!   終わるまで、呼び出したスレッドで待つ。回答の断片を受け取る関数も、そのスレッドで呼ぶ。
//! - 中断は、[`ClaudeCall::canceller`] で先に受け取った [`Canceller`] で、別のスレッドから行う。
//! - 起動のフラグ、渡す環境変数（[`ClaudeEnv::command`]）、作業ディレクトリ、止め方は、仕様のとおりにする。
//!   学習者のプログラム用の部品（`runner/process.rs`）は使わない（ADR 0257「影響」）。
//! - `claude` が終わったら、同じプロセスグループに残ったプロセスを SIGKILL で止めてから、刈り取る（ADR 0260）。
//!
//! Tauri の実行時に頼らない（テストから直接使う）。

use std::{
    ffi::OsString,
    io::{self, BufRead, BufReader, Read, Write},
    os::unix::process::{CommandExt, ExitStatusExt},
    path::PathBuf,
    process::{Child, ExitStatus, Stdio},
    sync::{Arc, Condvar, Mutex, MutexGuard, mpsc},
    thread,
    time::{Duration, Instant},
};

use super::{
    ClaudeEnv, is_valid_model,
    stream::{ClaudeError, StreamReader},
};

/// SIGTERM を送ってから、プロセスが終わらないときに SIGKILL を送るまでの時間
/// （invocation.md「プロセスの止め方」）。
pub const STOP_GRACE: Duration = Duration::from_secs(3);

/// 「その他」の詳細に添える、標準エラー出力の先頭の量。
const STDERR_HEAD_BYTES: usize = 4 * 1024;

/// `claude` が終わった後に、残りの出力を待つ時間。同じプロセスグループに残ったプロセスは止めるので、
/// ふつうはすぐに出力の終わり（EOF）が来る。グループを抜けたプロセスがパイプを持ち続けるときも、
/// これより長くは待たない。
const OUTPUT_GRACE_AFTER_EXIT: Duration = Duration::from_millis(300);

/// 作業ディレクトリの名前の先頭。
const WORK_DIR_PREFIX: &str = "progrust-claude-";

/// `claude -p` の 1 回の呼び出しの条件。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClaudeRequest {
    /// `claude` の実行ファイルのパス（検出の結果か、設定で指定したもの）。
    pub claude: PathBuf,
    /// `claude` に渡す環境変数（ふつうは [`ClaudeEnv::from_process`]）。
    pub env: ClaudeEnv,
    /// `--model` に渡すモデル。[`is_valid_model`] を満たさないときは起動しない。
    pub model: String,
    /// `--system-prompt` に渡す、既定のシステムプロンプトに代わる文。
    pub system_prompt: String,
    /// 標準入力で渡す指示文。
    pub prompt: String,
    /// 制限時間（invocation.md「制限時間」）。
    pub time_limit: Duration,
    /// 呼び出しごとの空の作業ディレクトリを作る場所（ふつうは [`std::env::temp_dir`]）。
    pub work_dir_parent: PathBuf,
}

/// 呼び出しの終わり方。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClaudeOutcome {
    /// `result` の行で完了した。`text` は、その行の `result` の文（ジョブが結果として使う）。
    Completed { text: String },
    /// エラー（errors.md「分類」）。
    Failed(ClaudeError),
    /// [`Canceller::cancel`] で中断した。
    Cancelled,
}

/// `claude -p` の 1 回の呼び出し。
#[derive(Debug)]
pub struct ClaudeCall {
    request: ClaudeRequest,
    control: Arc<Control>,
}

/// 呼び出しを中断するためのもの。別のスレッドから使える。
#[derive(Debug, Clone)]
pub struct Canceller {
    control: Arc<Control>,
}

impl Canceller {
    /// 呼び出しを中断する。すぐに返り、止める操作（SIGTERM、3 秒後に SIGKILL）は、呼び出しの側で行う。
    /// 起動する前に中断したときは、起動しない。`claude` が終わった後に中断しても、何もしない。
    pub fn cancel(&self) {
        let mut state = self.control.state();
        // 止めると決めるのは、ここで 1 度だけにする。この後は断片を渡さず、必ず中断として返す。
        if !state.exited {
            state.stop.get_or_insert(StopReason::Cancelled);
        }
        drop(state);
        self.control.changed.notify_all();
    }
}

impl ClaudeCall {
    pub fn new(request: ClaudeRequest) -> Self {
        Self {
            request,
            control: Arc::new(Control::default()),
        }
    }

    /// この呼び出しを中断するためのもの。
    pub fn canceller(&self) -> Canceller {
        Canceller {
            control: Arc::clone(&self.control),
        }
    }

    /// `claude -p` を呼び、`claude` のプロセスが終わってから、終わり方を返す。
    ///
    /// `on_fragment` を渡すと（質疑応答）、`--include-partial-messages` を付けて呼び、回答の断片を
    /// 届いた順に渡す。止めると決めた後（中断・制限時間）は、断片を渡さない。
    ///
    /// 止めると決めたときは、`result` の行を読んだ後でも、中断かタイムアウトとして返す。
    /// 止める前に `claude` が終わっていれば、止めずに、出力のとおりに返す。
    pub fn run(self, on_fragment: Option<&mut dyn FnMut(&str)>) -> ClaudeOutcome {
        if self.control.stopping() {
            return ClaudeOutcome::Cancelled;
        }
        let request = &self.request;
        if !is_valid_model(&request.model) {
            return ClaudeOutcome::Failed(ClaudeError::other(format!(
                "モデルの値が正しくないので、claude を起動しませんでした: {:?}",
                request.model
            )));
        }
        // 終わったら（どの終わり方でも）、この値を捨てるときに消える。
        let work_dir = match tempfile::Builder::new()
            .prefix(WORK_DIR_PREFIX)
            .tempdir_in(&request.work_dir_parent)
        {
            Ok(dir) => dir,
            Err(error) => {
                return ClaudeOutcome::Failed(ClaudeError::other(format!(
                    "作業ディレクトリを作れませんでした: {error}"
                )));
            }
        };

        let mut command = request.env.command(&request.claude);
        command
            .args(args(request, on_fragment.is_some()))
            .current_dir(work_dir.path())
            // 新しいプロセスグループを作る（pgid は子の pid になる）。
            .process_group(0)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let child = match command.spawn() {
            Ok(child) => child,
            Err(error) => {
                return ClaudeOutcome::Failed(ClaudeError::other(format!(
                    "claude を起動できませんでした: {error}"
                )));
            }
        };
        let outcome = self.watch(child, on_fragment);
        drop(work_dir);
        outcome
    }

    /// 起動した `claude` の入出力を扱い、終わるまで待つ。
    fn watch(
        &self,
        mut child: Child,
        mut on_fragment: Option<&mut dyn FnMut(&str)>,
    ) -> ClaudeOutcome {
        let (events, received) = mpsc::channel();
        if let Some(mut stdin) = child.stdin.take() {
            let prompt = self.request.prompt.clone();
            // 書き終えたら、捨てて閉じる。`claude` が読まずに終わったときの失敗は無視する。
            thread::spawn(move || {
                let _ = stdin.write_all(prompt.as_bytes());
            });
        }
        // 抜けた子がパイプを持ったままだと EOF が来ないので、読むスレッドは待たない（join しない）。
        if let Some(stdout) = child.stdout.take() {
            let events = events.clone();
            thread::spawn(move || read_lines(stdout, &events));
        }
        if let Some(stderr) = child.stderr.take() {
            let events = events.clone();
            thread::spawn(move || {
                let _ = events.send(Event::Stderr(read_head(stderr)));
            });
        }

        let Ok(pgid) = libc::pid_t::try_from(child.id()) else {
            // pid は pid_t に収まるので、ここには来ない。
            let _ = child.kill();
            let _ = child.wait();
            return ClaudeOutcome::Failed(ClaudeError::other("claude の pid を扱えませんでした。"));
        };
        let watchdog = {
            let control = Arc::clone(&self.control);
            let time_limit = self.request.time_limit;
            thread::spawn(move || control.stop_when_needed(pgid, Instant::now() + time_limit))
        };
        {
            let control = Arc::clone(&self.control);
            thread::spawn(move || {
                let status = control.reap(pgid, &mut child);
                let _ = events.send(Event::Exited(status));
            });
        }

        let mut reader = StreamReader::default();
        let mut stdout_closed = false;
        let mut stderr_head: Option<Vec<u8>> = None;
        let mut status: Option<io::Result<ExitStatus>> = None;
        let mut exited_at: Option<Instant> = None;
        while !(stdout_closed && stderr_head.is_some() && status.is_some()) {
            let event = match exited_at {
                // 終わり方は、プロセスが終わってから返すので、終わるまでは待ち続ける。
                None => received.recv().ok(),
                Some(at) => {
                    let wait =
                        (at + OUTPUT_GRACE_AFTER_EXIT).saturating_duration_since(Instant::now());
                    received.recv_timeout(wait).ok()
                }
            };
            let Some(event) = event else { break };
            match event {
                Event::Line(line) => {
                    if let Some(fragment) = reader.read_line(&line)
                        && let Some(on_fragment) = on_fragment.as_mut()
                        && !self.control.stopping()
                    {
                        on_fragment(&fragment);
                    }
                }
                Event::StdoutClosed => stdout_closed = true,
                Event::Stderr(head) => stderr_head = Some(head),
                Event::Exited(exit) => {
                    status = Some(exit);
                    exited_at = Some(Instant::now());
                }
            }
        }
        // 終わったことを知らせてあるので、すぐに返る。
        let _ = watchdog.join();

        match self.control.state().stop {
            Some(StopReason::Cancelled) => return ClaudeOutcome::Cancelled,
            Some(StopReason::Timeout) => return ClaudeOutcome::Failed(ClaudeError::timeout()),
            None => {}
        }
        let stderr_head =
            String::from_utf8_lossy(stderr_head.as_deref().unwrap_or_default()).into_owned();
        let exit = match status {
            Some(Ok(status)) => describe_exit(status),
            Some(Err(error)) => format!("終わり方を確かめられません: {error}"),
            None => "終わり方が分かりません".to_string(),
        };
        match reader.finish(&stderr_head, &exit) {
            Ok(text) => ClaudeOutcome::Completed { text },
            Err(error) => ClaudeOutcome::Failed(error),
        }
    }
}

/// 起動のフラグ（invocation.md「起動のフラグ」「出力形式」）。`partial` のときは、回答の断片を出させる。
fn args(request: &ClaudeRequest, partial: bool) -> Vec<OsString> {
    let mut args: Vec<OsString> = [
        "-p",
        "--model",
        &request.model,
        "--safe-mode",
        "--setting-sources",
        "",
        "--strict-mcp-config",
        "--tools",
        "",
        "--disable-slash-commands",
        "--no-session-persistence",
        "--system-prompt",
        &request.system_prompt,
        "--output-format",
        "stream-json",
        "--verbose",
    ]
    .into_iter()
    .map(OsString::from)
    .collect();
    if partial {
        args.push("--include-partial-messages".into());
    }
    args
}

/// 読むスレッドなどから、呼び出しのスレッドへ知らせること。
enum Event {
    /// 標準出力の 1 行（改行を除く）。
    Line(Vec<u8>),
    /// 標準出力の終わり（EOF）。読めなくなったときも含む。
    StdoutClosed,
    /// 標準エラー出力の先頭。終わりまで読んでから知らせる。
    Stderr(Vec<u8>),
    /// `claude` が終わり、刈り取った。
    Exited(io::Result<ExitStatus>),
}

/// 標準出力を 1 行ずつ読んで知らせる。UTF-8 でない行も、そのまま渡す（読む側が飛ばす）。
/// 改行で終わらない最後の行も渡す。
fn read_lines(stdout: impl Read, events: &mpsc::Sender<Event>) {
    let mut stdout = BufReader::new(stdout);
    let mut line = Vec::new();
    loop {
        line.clear();
        match stdout.read_until(b'\n', &mut line) {
            Ok(0) | Err(_) => break,
            Ok(_) => {
                if line.last() == Some(&b'\n') {
                    line.pop();
                }
                if events.send(Event::Line(line.clone())).is_err() {
                    break;
                }
            }
        }
    }
    let _ = events.send(Event::StdoutClosed);
}

/// 標準エラー出力を終わりまで読み、先頭の [`STDERR_HEAD_BYTES`] だけを返す。残りは読み捨てる
/// （読まないと、`claude` が書き込みで止まる）。
fn read_head(mut stderr: impl Read) -> Vec<u8> {
    let mut head = Vec::new();
    let mut buffer = [0u8; 8 * 1024];
    loop {
        match stderr.read(&mut buffer) {
            Ok(0) => break,
            Ok(read) => {
                let room = STDERR_HEAD_BYTES.saturating_sub(head.len());
                head.extend_from_slice(&buffer[..read.min(room)]);
            }
            Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
            Err(_) => break,
        }
    }
    head
}

/// 「その他」の詳細に書く、`claude` の終わり方。
fn describe_exit(status: ExitStatus) -> String {
    match (status.code(), status.signal()) {
        (Some(code), _) => format!("終了コード {code}"),
        (None, Some(signal)) => format!("シグナル {signal}"),
        (None, None) => status.to_string(),
    }
}

/// 止めた理由。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StopReason {
    Timeout,
    Cancelled,
}

/// 呼び出しの状態。止めた理由と、`claude` が終わったかを、1 つのロックで扱う。
#[derive(Debug, Default)]
struct Control {
    state: Mutex<ControlState>,
    changed: Condvar,
}

#[derive(Debug, Default)]
struct ControlState {
    /// 止めると決めた理由。中断（[`Canceller::cancel`]）か、制限時間を超えたとき（watchdog）に、
    /// 先に決めたほうになる。`claude` が終わった後は決めない。
    stop: Option<StopReason>,
    /// `claude` が終わった。この後は、シグナルを送らない。刈り取った後の pgid は再利用されうるので、
    /// シグナルは、これを見てから、同じロックの中で送る。
    exited: bool,
}

impl Control {
    fn state(&self) -> MutexGuard<'_, ControlState> {
        // 中のスレッドは、ロックを持ったまま panic しない。panic しても、状態は壊れない。
        self.state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// 止めると決めたか。
    fn stopping(&self) -> bool {
        self.state().stop.is_some()
    }

    /// 制限時間を超えるか、中断を求められたら、プロセスグループに SIGTERM を送り、[`STOP_GRACE`] たっても
    /// 終わらなければ SIGKILL を送る。`claude` が終わったら、何もせずに返る。
    fn stop_when_needed(&self, pgid: libc::pid_t, deadline: Instant) {
        let mut state = self.state();
        loop {
            if state.exited {
                return;
            }
            if state.stop.is_some() {
                break;
            }
            let now = Instant::now();
            if now >= deadline {
                state.stop = Some(StopReason::Timeout);
                break;
            }
            state = self
                .changed
                .wait_timeout(state, deadline - now)
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .0;
        }
        signal_group(pgid, libc::SIGTERM);
        let kill_at = Instant::now() + STOP_GRACE;
        loop {
            if state.exited {
                return;
            }
            let now = Instant::now();
            if now >= kill_at {
                signal_group(pgid, libc::SIGKILL);
                return;
            }
            state = self
                .changed
                .wait_timeout(state, kill_at - now)
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .0;
        }
    }

    /// `claude` が終わるまで待ち、同じプロセスグループに残ったプロセスを SIGKILL で止めてから、刈り取る。
    fn reap(&self, pgid: libc::pid_t, child: &mut Child) -> io::Result<ExitStatus> {
        // 刈り取らずに終わりを知る。刈り取るまで pgid は再利用されないので、その間はグループにシグナルを送ってよい。
        // 待てないときも、下の wait で終わりを待つ。
        let _ = wait_for_exit_without_reaping(pgid);
        let mut state = self.state();
        state.exited = true;
        signal_group(pgid, libc::SIGKILL);
        // `claude` はもう終わっているので、すぐに返る。
        let status = child.wait();
        drop(state);
        self.changed.notify_all();
        status
    }
}

/// プロセスグループ全体にシグナルを送る。
fn signal_group(pgid: libc::pid_t, signal: libc::c_int) {
    // SAFETY: 引数を取らない関数で、失敗しない。
    let own = unsafe { libc::getpgrp() };
    // 誤って自分（アプリ）のプロセスグループや、init などのグループに送らない。
    if pgid <= 1 || pgid == own {
        return;
    }
    // SAFETY: シグナルを送るだけで、メモリを扱わない。グループがもう無いときは ESRCH が返るだけである。
    unsafe {
        libc::killpg(pgid, signal);
    }
}

/// `pid` のプロセスが終わるまで待つ。刈り取らない（`WNOWAIT`）。
fn wait_for_exit_without_reaping(pid: libc::pid_t) -> io::Result<()> {
    let id = libc::id_t::try_from(pid).map_err(io::Error::other)?;
    loop {
        // SAFETY: siginfo_t は、すべて 0 のバイトで正しい値になる C の構造体である。
        let mut info: libc::siginfo_t = unsafe { std::mem::zeroed() };
        // SAFETY: `info` は、この呼び出しの間だけ有効な、書き込める領域である。
        let result =
            unsafe { libc::waitid(libc::P_PID, id, &mut info, libc::WEXITED | libc::WNOWAIT) };
        if result == 0 {
            return Ok(());
        }
        let error = io::Error::last_os_error();
        if error.kind() != io::ErrorKind::Interrupted {
            return Err(error);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request() -> ClaudeRequest {
        ClaudeRequest {
            claude: PathBuf::from("/usr/local/bin/claude"),
            env: ClaudeEnv::default(),
            model: "haiku".into(),
            system_prompt: "あなたは Rust の先生です。".into(),
            prompt: "質問".into(),
            time_limit: Duration::from_secs(120),
            work_dir_parent: std::env::temp_dir(),
        }
    }

    #[test]
    fn args_follow_the_spec_flags() {
        let expected = [
            "-p",
            "--model",
            "haiku",
            "--safe-mode",
            "--setting-sources",
            "",
            "--strict-mcp-config",
            "--tools",
            "",
            "--disable-slash-commands",
            "--no-session-persistence",
            "--system-prompt",
            "あなたは Rust の先生です。",
            "--output-format",
            "stream-json",
            "--verbose",
        ];
        assert_eq!(args(&request(), false), expected.map(OsString::from));
        let mut with_partial = expected.map(OsString::from).to_vec();
        with_partial.push("--include-partial-messages".into());
        assert_eq!(args(&request(), true), with_partial);
    }

    #[test]
    fn the_stderr_head_is_limited() {
        let stderr = vec![b'x'; STDERR_HEAD_BYTES * 3];
        assert_eq!(read_head(&stderr[..]), vec![b'x'; STDERR_HEAD_BYTES]);
        assert_eq!(read_head(&b"short"[..]), b"short");
    }

    #[test]
    fn lines_without_a_final_newline_are_read() {
        let (events, received) = mpsc::channel();
        read_lines(&b"one\n\xff\ntwo"[..], &events);
        drop(events);
        let lines: Vec<Vec<u8>> = received
            .iter()
            .map(|event| match event {
                Event::Line(line) => line,
                Event::StdoutClosed => b"<closed>".to_vec(),
                _ => unreachable!(),
            })
            .collect();
        assert_eq!(
            lines,
            [
                b"one".to_vec(),
                b"\xff".to_vec(),
                b"two".to_vec(),
                b"<closed>".to_vec()
            ]
        );
    }

    #[test]
    fn exits_are_described() {
        assert_eq!(describe_exit(ExitStatus::from_raw(1 << 8)), "終了コード 1");
        assert_eq!(describe_exit(ExitStatus::from_raw(9)), "シグナル 9");
    }
}
