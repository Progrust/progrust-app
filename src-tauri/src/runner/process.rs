//! 学習者のプログラムなどを、新しいプロセスグループの子プロセスとして動かす部品
//! （spec/exercise-runner/run.md「手順」「出力の送り方」「停止」「対話的な標準入力」、
//! spec/exercise-runner/limits.md）。実行・テスト実行・判定のすべてがこれを使う。
//!
//! - [`spawn`] で起動し、出力を [`BATCH_INTERVAL`] ごとにまとめて呼び出し側の関数に渡す。標準入力を書く・閉じる、
//!   止める操作は [`ProcessHandle`] で行い、終わり方は [`RunningProcess::wait`] で受け取る。
//! - 判定のように、標準入力に決まった文字列を書いて閉じ、出力を集めるだけのときは [`collect_output`] を使う。
//!   判定の途中で止める（提出を止める）ときは、[`Stopper`] で起動し、[`Stopper::stop`] でまとめて止める。
//! - 止めるときは、プロセスグループ全体に SIGKILL を送る。自分で別のプロセスグループやセッションに移った子は止めない。
//!   止める操作と制限時間で止めたときは、まだ渡していない出力を捨てる。ビルドでは、制限時間で止めたときも、
//!   受け取った出力を渡す（[`TimeoutOutput`]）。
//! - 起動したプロセスが終わったら、同じプロセスグループに残ったプロセスを止め、その後に届く出力を
//!   [`GRACE_AFTER_EXIT`] だけ待って終える。パイプの終わり（EOF）は待たない。
//! - 出力量の上限は、実行では標準出力と標準エラー出力の合計で、stdout 方式の判定では標準出力だけで数える
//!   （[`OutputLimit`]）。
//!
//! Tauri の実行時に頼らない（判定だけを行うコマンドラインのプログラムからも使う。ADR 0208）。

use std::{
    ffi::OsString,
    io::{self, Read, Write},
    os::unix::process::{CommandExt, ExitStatusExt},
    path::PathBuf,
    process::{Child, ChildStdin, Command, ExitStatus, Stdio},
    sync::{
        Arc, Mutex, MutexGuard,
        atomic::{AtomicUsize, Ordering},
        mpsc::{self, Receiver, RecvTimeoutError, Sender, SyncSender},
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

/// 出力をまとめて渡す間隔（run.md「出力の送り方」の 16〜50ms）。
pub const BATCH_INTERVAL: Duration = Duration::from_millis(16);
/// 起動したプロセスが終わった後に、残りの出力を待つ時間（run.md「手順」の 5）。
pub const GRACE_AFTER_EXIT: Duration = Duration::from_millis(300);

/// パイプから 1 回に読む量。
const READ_BUFFER_SIZE: usize = 8 * 1024;
/// 読み取りのスレッドから、出力をまとめるスレッドへの通り道に、ためておける数。
/// いっぱいになると読み取りが待ち、パイプを通して子プロセスの書き込みも待つ（端末と同じ流量制御）。
const CHANNEL_CAPACITY: usize = 64;

/// 出力の種類。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputStream {
    Stdout,
    Stderr,
}

/// 出力の断片。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutputChunk {
    pub stream: OutputStream,
    pub text: String,
}

/// 標準入力の扱い。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StdinMode {
    /// 実行中に [`ProcessHandle::write_stdin_line`]・[`ProcessHandle::write_stdin`] で書き、
    /// [`ProcessHandle::close_stdin`] で閉じる。
    Interactive,
    /// 起動の直後に閉じる（プログラムが読むと、すぐに EOF になる）。
    Closed,
    /// 起動の直後に、この内容を書いて閉じる（判定のテストケースの入力）。
    Input(Vec<u8>),
}

/// 制限値。値は呼び出し側が決める（spec/exercise-runner/limits.md）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits {
    /// 制限時間。起動から数え、標準入力を待つ間も含める（ADR 0013）。
    pub time: Duration,
    /// 出力量の上限と、その数え方。
    pub output: OutputLimit,
}

/// 出力量の上限と、その数え方。数える出力が上限を超えたら止めて [`Termination::OutputLimitExceeded`] にし、
/// 上限までの出力だけを渡す。上限で切れた UTF-8 の文字と、上限を超えた後に読んだ出力は渡さない（ADR 0102）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputLimit {
    /// 標準出力と標準エラー出力の合計で数える（実行。テスト実行を含む）。値は上限（バイト）。
    Total(usize),
    /// 標準出力だけで数える（stdout 方式の判定。spec/exercise-runner/submit.md「stdout 方式」）。値は上限（バイト）。
    /// 標準エラー出力は上限に数えず、先頭の [`STDERR_HEAD_BYTES`] だけを渡し、それより後は止めずに読んで捨てる。
    Stdout(usize),
}

/// [`OutputLimit::Stdout`] で渡す標準エラー出力の先頭の量（バイト）。判定の詳細（`detail`）に使う先頭の部分である。
pub const STDERR_HEAD_BYTES: usize = 8 * 1024;

/// 起動するプロセス。
#[derive(Debug, Clone)]
pub struct ProcessSpec {
    pub program: PathBuf,
    pub args: Vec<OsString>,
    pub current_dir: PathBuf,
    /// アプリの環境変数に加えて設定する環境変数。
    pub envs: Vec<(OsString, OsString)>,
    pub stdin: StdinMode,
    pub limits: Limits,
    /// 制限時間で止めたときに、まだ渡していない出力をどうするか。
    pub on_timeout: TimeoutOutput,
}

/// 制限時間で止めたときに、まだ渡していない出力をどうするか。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimeoutOutput {
    /// 捨てる（実行。run.md「停止」）。
    Discard,
    /// 止めるまでに受け取った出力を、すべて渡してから終える（ビルド。run.md「ビルドの出力の表示」）。
    Deliver,
}

impl ProcessSpec {
    /// 引数と追加の環境変数が無く、標準入力を起動の直後に閉じるプロセス。
    pub fn new(
        program: impl Into<PathBuf>,
        current_dir: impl Into<PathBuf>,
        limits: Limits,
    ) -> Self {
        Self {
            program: program.into(),
            args: Vec::new(),
            current_dir: current_dir.into(),
            envs: Vec::new(),
            stdin: StdinMode::Closed,
            limits,
            on_timeout: TimeoutOutput::Discard,
        }
    }
}

/// 終わり方。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Termination {
    /// 終了コードで終わった。
    Exited(i32),
    /// シグナルで終わった（abort など）。止める操作・制限で止めたときは、ほかの値になる。
    Signaled(i32),
    /// 制限時間を超えたので止めた。
    TimedOut,
    /// 出力量の上限を超えたので止めた。
    OutputLimitExceeded,
    /// 止める操作（[`ProcessHandle::stop`]）で止めた。
    Stopped,
}

/// 標準入力がもう閉じている（閉じた後、起動の直後に閉じる指定、プログラムが終わったなど）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StdinClosed;

/// 動いているプロセス。
pub struct RunningProcess {
    handle: ProcessHandle,
    dispatcher: JoinHandle<io::Result<Termination>>,
}

impl RunningProcess {
    /// 標準入力を書く・閉じる、止める操作。ほかのスレッドに渡せる。
    pub fn handle(&self) -> ProcessHandle {
        self.handle.clone()
    }

    /// 終わるまで待ち、終わり方を返す。返る前に、渡す出力はすべて渡し終えている。
    pub fn wait(self) -> io::Result<Termination> {
        self.dispatcher
            .join()
            .unwrap_or_else(|_| Err(io::Error::other("出力を渡すスレッドが panic した")))
    }
}

/// 動いているプロセスへの操作。
#[derive(Clone)]
pub struct ProcessHandle {
    group: Arc<Group>,
    stdin: Arc<Mutex<Option<Sender<String>>>>,
}

impl ProcessHandle {
    /// プロセスグループ全体を止める。まだ渡していない出力は捨てる。終わった後に呼んでも何もしない。
    pub fn stop(&self) {
        self.group.request_stop(StopReason::User);
    }

    /// 標準入力に、`line` に改行を付けて書く。書き込みは別のスレッドで行うので、プログラムが読まなくても待たない。
    pub fn write_stdin_line(&self, line: &str) -> Result<(), StdinClosed> {
        self.write_stdin(&format!("{line}\n"))
    }

    /// 標準入力に、`text` をそのまま書く（改行を付けない）。書き込みは別のスレッドで行うので、プログラムが読まなくても
    /// 待たない。
    pub fn write_stdin(&self, text: &str) -> Result<(), StdinClosed> {
        let stdin = lock(&self.stdin);
        let sender = stdin.as_ref().ok_or(StdinClosed)?;
        sender.send(text.to_string()).map_err(|_| StdinClosed)
    }

    /// 標準入力を閉じる。それまでに書いた行は、閉じる前にプログラムに渡る。
    pub fn close_stdin(&self) {
        lock(&self.stdin).take();
    }
}

/// `spec` のプロセスを新しいプロセスグループで起動する。
///
/// 出力は [`BATCH_INTERVAL`] ごとに、その間に受け取った断片を受け取った順に並べて `on_output` に渡す。
/// 同じ種類の断片が続く分は 1 つにまとめる。`on_output` は、出力を渡すための別のスレッドで呼ばれる。
pub fn spawn<F>(spec: ProcessSpec, on_output: F) -> io::Result<RunningProcess>
where
    F: FnMut(Vec<OutputChunk>) + Send + 'static,
{
    let mut command = Command::new(&spec.program);
    command
        .args(&spec.args)
        .current_dir(&spec.current_dir)
        .envs(spec.envs.iter().map(|(key, value)| (key, value)))
        // 新しいプロセスグループを作る（pgid は子の pid になる）。
        .process_group(0)
        .stdin(match spec.stdin {
            StdinMode::Closed => Stdio::null(),
            StdinMode::Interactive | StdinMode::Input(_) => Stdio::piped(),
        })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = command.spawn()?;

    let pgid = libc::pid_t::try_from(child.id()).map_err(io::Error::other)?;
    let group = Arc::new(Group {
        pgid,
        on_timeout: spec.on_timeout,
        state: Mutex::new(GroupState::default()),
    });
    let (events, receiver) = mpsc::sync_channel(CHANNEL_CAPACITY);
    let budgets = match spec.limits.output {
        OutputLimit::Total(limit) => {
            let budget = OutputBudget::new(limit, OverLimit::Stop);
            [budget.clone(), budget]
        }
        OutputLimit::Stdout(limit) => [
            OutputBudget::new(limit, OverLimit::Stop),
            OutputBudget::new(STDERR_HEAD_BYTES, OverLimit::Discard),
        ],
    };

    let pipes = [
        (OutputStream::Stdout, child.stdout.take().map(boxed_reader)),
        (OutputStream::Stderr, child.stderr.take().map(boxed_reader)),
    ];
    let mut readers = 0;
    for ((stream, pipe), budget) in pipes.into_iter().zip(budgets) {
        let Some(pipe) = pipe else { continue };
        readers += 1;
        let reader = Reader {
            stream,
            budget,
            group: Arc::clone(&group),
            events: events.clone(),
        };
        // 抜けた子がパイプを持ったままだと EOF が来ないので、このスレッドは待たない（join しない）。
        thread::spawn(move || reader.run(pipe));
    }

    let stdin_sender = match (spec.stdin, child.stdin.take()) {
        (StdinMode::Interactive, Some(stdin)) => Some(spawn_interactive_stdin(stdin)),
        (StdinMode::Input(input), Some(stdin)) => {
            spawn_input_stdin(stdin, input);
            None
        }
        _ => None,
    };

    let cancel_timer = spawn_timer(Arc::clone(&group), spec.limits.time);
    {
        let group = Arc::clone(&group);
        thread::spawn(move || {
            // 刈り取らずに終わりを知る。刈り取るまで pgid は再利用されないので、その間は killpg で止めてよい。
            let _ = wait_for_exit_without_reaping(pgid);
            let status = group.finish(&mut child);
            drop(cancel_timer);
            let _ = events.send(Event::Exited(status));
        });
    }

    let dispatcher = {
        let group = Arc::clone(&group);
        thread::spawn(move || dispatch(receiver, &group, readers, on_output))
    };

    Ok(RunningProcess {
        handle: ProcessHandle {
            group,
            stdin: Arc::new(Mutex::new(stdin_sender)),
        },
        dispatcher,
    })
}

/// [`collect_output`] の結果。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CollectedOutput {
    pub termination: Termination,
    pub stdout: String,
    pub stderr: String,
}

/// `spec` のプロセスを起動し、終わるまでの標準出力と標準エラー出力を集めて返す。
/// 出力量の上限で止めたときは、上限までの出力だけが入る。[`OutputLimit::Stdout`] のときは、
/// 標準エラー出力は先頭の [`STDERR_HEAD_BYTES`] までだけが入る。途中で止める操作が要るときは、
/// [`Stopper::collect_output`] を使う。
pub fn collect_output(spec: ProcessSpec) -> io::Result<CollectedOutput> {
    Stopper::new().collect_output(spec)
}

/// 判定のように、いくつものプロセスを続けて、または同時に起動するときに、まとめて止める操作。複製は同じものを指す。
///
/// [`Stopper::collect_output`] で起動したプロセスは、動いている間、止める相手として登録する。[`Stopper::stop`] の後は、
/// 動いているプロセスをすべて止め、新しいプロセスは起動せずに [`Termination::Stopped`] にする。止める操作が、
/// 起動してから登録するまでの間に来ても、登録したときに止める。
#[derive(Clone, Default)]
pub struct Stopper {
    state: Arc<Mutex<StopperState>>,
}

#[derive(Default)]
struct StopperState {
    stopped: bool,
    running: Vec<ProcessHandle>,
}

impl Stopper {
    pub fn new() -> Self {
        Self::default()
    }

    /// 動いているプロセスをすべて止め、この後は起動させない。
    pub fn stop(&self) {
        let mut state = lock(&self.state);
        state.stopped = true;
        for process in state.running.drain(..) {
            process.stop();
        }
    }

    /// [`Stopper::stop`] を呼んだか。
    pub fn is_stopped(&self) -> bool {
        lock(&self.state).stopped
    }

    /// [`collect_output`] と同じ。止めた後は起動せずに、出力が空の [`Termination::Stopped`] を返す。
    pub fn collect_output(&self, spec: ProcessSpec) -> io::Result<CollectedOutput> {
        let stopped = || CollectedOutput {
            termination: Termination::Stopped,
            stdout: String::new(),
            stderr: String::new(),
        };
        if self.is_stopped() {
            return Ok(stopped());
        }
        let collected = Arc::new(Mutex::new((String::new(), String::new())));
        let process = {
            let collected = Arc::clone(&collected);
            spawn(spec, move |chunks| {
                let mut collected = lock(&collected);
                for chunk in chunks {
                    match chunk.stream {
                        OutputStream::Stdout => collected.0.push_str(&chunk.text),
                        OutputStream::Stderr => collected.1.push_str(&chunk.text),
                    }
                }
            })?
        };
        let handle = process.handle();
        {
            let mut state = lock(&self.state);
            if state.stopped {
                handle.stop();
            } else {
                state.running.push(handle.clone());
            }
        }
        let termination = process.wait();
        lock(&self.state)
            .running
            .retain(|running| !Arc::ptr_eq(&running.group, &handle.group));
        let termination = termination?;
        let (stdout, stderr) = std::mem::take(&mut *lock(&collected));
        Ok(CollectedOutput {
            termination,
            stdout,
            stderr,
        })
    }
}

// ---- 止める ----

/// 止めた理由。最初に決まったものを使う。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StopReason {
    User,
    Timeout,
    OutputLimit,
}

impl StopReason {
    fn termination(self) -> Termination {
        match self {
            Self::User => Termination::Stopped,
            Self::Timeout => Termination::TimedOut,
            Self::OutputLimit => Termination::OutputLimitExceeded,
        }
    }

    /// まだ渡していない出力を捨てるか（run.md「停止」）。出力量の上限では、上限までの出力を渡す（limits.md）。
    /// 制限時間では、`on_timeout` に従う。
    fn discards_output(self, on_timeout: TimeoutOutput) -> bool {
        match self {
            Self::User => true,
            Self::Timeout => on_timeout == TimeoutOutput::Discard,
            Self::OutputLimit => false,
        }
    }
}

/// 起動したプロセスのプロセスグループ。
struct Group {
    pgid: libc::pid_t,
    on_timeout: TimeoutOutput,
    state: Mutex<GroupState>,
}

#[derive(Default)]
struct GroupState {
    stop: Option<StopReason>,
    /// 起動したプロセスが終わった。この後は、止める操作・制限で止めない。刈り取った後の pgid は再利用されうるので、
    /// killpg はこれを見てから、同じロックの中で送る。
    exited: bool,
}

impl Group {
    fn state(&self) -> MutexGuard<'_, GroupState> {
        lock(&self.state)
    }

    /// プロセスグループ全体を止める。起動したプロセスが終わった後は止めない。
    fn request_stop(&self, reason: StopReason) {
        let mut state = self.state();
        if state.exited {
            // 終わる直前に書かれた出力を、終わった後に読んで上限を超えたときも、上限までの出力だけを渡すので、
            // 上限で止めたものとする。止める操作と制限時間は、終わった後なら何もしない。
            if reason == StopReason::OutputLimit {
                state.stop.get_or_insert(reason);
            }
            return;
        }
        state.stop.get_or_insert(reason);
        kill_group(self.pgid);
    }

    /// 止めた理由。
    fn stop_reason(&self) -> Option<StopReason> {
        self.state().stop
    }

    /// まだ渡していない出力を捨てるか。
    fn discards_output(&self) -> bool {
        self.stop_reason()
            .is_some_and(|reason| reason.discards_output(self.on_timeout))
    }

    /// 起動したプロセスが終わった後に、同じプロセスグループに残ったプロセスを止めてから、刈り取る。
    fn finish(&self, child: &mut Child) -> io::Result<ExitStatus> {
        let mut state = self.state();
        state.exited = true;
        kill_group(self.pgid);
        // 上の killpg で起動したプロセスも止まっているので、すぐに返る。
        child.wait()
    }
}

/// プロセスグループ全体に SIGKILL を送る。
fn kill_group(pgid: libc::pid_t) {
    // SAFETY: 引数を取らない関数で、失敗しない。
    let own = unsafe { libc::getpgrp() };
    // 誤って自分（アプリ）のプロセスグループや、init などのグループを止めない。
    if pgid <= 1 || pgid == own {
        return;
    }
    // SAFETY: シグナルを送るだけで、メモリを扱わない。グループがもう無いときは ESRCH が返るだけである。
    unsafe {
        libc::killpg(pgid, libc::SIGKILL);
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

/// 制限時間を見るスレッドを起動する。返した送り手を捨てると、見るのをやめる。
fn spawn_timer(group: Arc<Group>, limit: Duration) -> Sender<()> {
    let (cancel, cancelled) = mpsc::channel::<()>();
    thread::spawn(move || {
        if let Err(RecvTimeoutError::Timeout) = cancelled.recv_timeout(limit) {
            group.request_stop(StopReason::Timeout);
        }
    });
    cancel
}

// ---- 標準入力 ----

/// 1 行ずつ書くスレッドを起動する。返した送り手を捨てると、送った行を書き終えてから標準入力を閉じる。
fn spawn_interactive_stdin(mut stdin: ChildStdin) -> Sender<String> {
    let (sender, lines) = mpsc::channel::<String>();
    thread::spawn(move || {
        for line in lines {
            // プログラムが終わっていると BrokenPipe になる。そのときは、書くのをやめる。
            if stdin.write_all(line.as_bytes()).is_err() || stdin.flush().is_err() {
                break;
            }
        }
    });
    sender
}

/// `input` を書いて閉じるスレッドを起動する。
fn spawn_input_stdin(mut stdin: ChildStdin, input: Vec<u8>) {
    thread::spawn(move || {
        let _ = stdin.write_all(&input);
    });
}

// ---- 出力 ----

enum Event {
    Output(OutputStream, String),
    ReaderDone,
    Exited(io::Result<ExitStatus>),
}

fn boxed_reader(pipe: impl Read + Send + 'static) -> Box<dyn Read + Send> {
    Box::new(pipe)
}

/// 上限を超えたときにすること。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OverLimit {
    /// プロセスグループを止める。
    Stop,
    /// 止めずに、残りを読んで捨てる。読むのをやめると、パイプがいっぱいになってプログラムが書けずに待つため。
    Discard,
}

/// 出力量を数える入れ物。数える量を共有する読み取りのスレッドには、同じ `read` を渡す。
#[derive(Debug, Clone)]
struct OutputBudget {
    limit: usize,
    /// これを共有する読み取りのスレッドが、合計で読んだ量。
    read: Arc<AtomicUsize>,
    over_limit: OverLimit,
}

impl OutputBudget {
    fn new(limit: usize, over_limit: OverLimit) -> Self {
        Self {
            limit,
            read: Arc::new(AtomicUsize::new(0)),
            over_limit,
        }
    }
}

/// 標準出力か標準エラー出力を読むスレッド。
struct Reader {
    stream: OutputStream,
    budget: OutputBudget,
    group: Arc<Group>,
    events: SyncSender<Event>,
}

impl Reader {
    fn run(self, mut pipe: Box<dyn Read + Send>) {
        let mut buffer = vec![0; READ_BUFFER_SIZE];
        let mut decoder = Utf8Decoder::default();
        let budget = &self.budget;
        loop {
            let read = match pipe.read(&mut buffer) {
                Ok(0) => break,
                Ok(read) => read,
                Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
                Err(_) => break,
            };
            let before = budget.read.fetch_add(read, Ordering::SeqCst);
            let allowed = budget.limit.saturating_sub(before).min(read);
            let text = decoder.decode(&buffer[..allowed]);
            if !text.is_empty() && self.events.send(Event::Output(self.stream, text)).is_err() {
                return;
            }
            if allowed < read {
                // 上限を超えた。上限で切れた文字（持ち越した分）と、それより後の出力は渡さない。
                match budget.over_limit {
                    OverLimit::Stop => self.group.request_stop(StopReason::OutputLimit),
                    OverLimit::Discard => discard_rest(&mut pipe, &mut buffer),
                }
                let _ = self.events.send(Event::ReaderDone);
                return;
            }
        }
        // 終わりに、文字の途中で切れたバイトが残っていたら、置換文字にして渡す。上限を超えた後なら捨てる。
        if budget.read.load(Ordering::SeqCst) <= budget.limit {
            let text = decoder.finish();
            if !text.is_empty() {
                let _ = self.events.send(Event::Output(self.stream, text));
            }
        }
        let _ = self.events.send(Event::ReaderDone);
    }
}

/// パイプの終わり（EOF）か、読めなくなるまで読んで捨てる。
fn discard_rest(pipe: &mut Box<dyn Read + Send>, buffer: &mut [u8]) {
    loop {
        match pipe.read(buffer) {
            Ok(0) => return,
            Ok(_) => {}
            Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
            Err(_) => return,
        }
    }
}

/// 出力をまとめて `on_output` に渡し、終わったら終わり方を返す。
fn dispatch<F>(
    events: Receiver<Event>,
    group: &Group,
    mut open_readers: usize,
    mut on_output: F,
) -> io::Result<Termination>
where
    F: FnMut(Vec<OutputChunk>),
{
    let mut pending: Vec<OutputChunk> = Vec::new();
    let mut flush_at: Option<Instant> = None;
    let mut exit: Option<io::Result<ExitStatus>> = None;
    let mut grace_until: Option<Instant> = None;
    let mut disconnected = false;
    loop {
        let event = match [flush_at, grace_until].into_iter().flatten().min() {
            _ if disconnected => None,
            None => events.recv().ok(),
            Some(deadline) => {
                match events.recv_timeout(deadline.saturating_duration_since(Instant::now())) {
                    Ok(event) => Some(event),
                    Err(RecvTimeoutError::Timeout) => None,
                    Err(RecvTimeoutError::Disconnected) => {
                        disconnected = true;
                        None
                    }
                }
            }
        };
        let now = Instant::now();
        match event {
            Some(Event::Output(stream, text)) => {
                push_chunk(&mut pending, stream, text);
                flush_at.get_or_insert(now + BATCH_INTERVAL);
            }
            Some(Event::ReaderDone) => open_readers = open_readers.saturating_sub(1),
            Some(Event::Exited(status)) => {
                exit = Some(status);
                grace_until = Some(now + GRACE_AFTER_EXIT);
            }
            None if flush_at.is_none() && grace_until.is_none() => disconnected = true,
            None => {}
        }

        let discarding = group.discards_output();
        if discarding {
            pending.clear();
            flush_at = None;
        }
        let finished = exit.is_some()
            && (discarding
                || disconnected
                || open_readers == 0
                || grace_until.is_some_and(|until| until <= now));
        if (finished || flush_at.is_some_and(|at| at <= now)) && !pending.is_empty() {
            on_output(std::mem::take(&mut pending));
            flush_at = None;
        }
        if finished {
            break;
        }
        if disconnected && exit.is_none() {
            return Err(io::Error::other(
                "プロセスの終わりを待つスレッドが、終わりを知らせずに止まった",
            ));
        }
    }

    // 止めた理由は、出力を渡し終えてから見る。終わった後に読んだ出力で、上限を超えたことがあるため。
    let status = exit.expect("終わったときだけ、ループを抜ける");
    if let Some(reason) = group.stop_reason() {
        return Ok(reason.termination());
    }
    let status = status?;
    match (status.code(), status.signal()) {
        (Some(code), _) => Ok(Termination::Exited(code)),
        (None, Some(signal)) => Ok(Termination::Signaled(signal)),
        (None, None) => Err(io::Error::other(format!("終わり方が分からない: {status}"))),
    }
}

/// 断片を足す。直前の断片と同じ種類なら、1 つにまとめる。
fn push_chunk(pending: &mut Vec<OutputChunk>, stream: OutputStream, text: String) {
    match pending.last_mut() {
        Some(last) if last.stream == stream => last.text.push_str(&text),
        _ => pending.push(OutputChunk { stream, text }),
    }
}

/// バイト列を UTF-8 の文字列にする。文字の途中で切れたバイトは、次に受け取った分に持ち越す。
/// UTF-8 として正しくないバイトは、置換文字（U+FFFD）にする。
#[derive(Debug, Default)]
struct Utf8Decoder {
    carry: Vec<u8>,
}

impl Utf8Decoder {
    fn decode(&mut self, bytes: &[u8]) -> String {
        let mut input = std::mem::take(&mut self.carry);
        input.extend_from_slice(bytes);
        let mut text = String::with_capacity(input.len());
        let mut rest = input.as_slice();
        loop {
            match std::str::from_utf8(rest) {
                Ok(valid) => {
                    text.push_str(valid);
                    break;
                }
                Err(error) => {
                    let (valid, after) = rest.split_at(error.valid_up_to());
                    text.push_str(&String::from_utf8_lossy(valid));
                    match error.error_len() {
                        Some(invalid) => {
                            text.push(char::REPLACEMENT_CHARACTER);
                            rest = &after[invalid..];
                        }
                        None => {
                            self.carry = after.to_vec();
                            break;
                        }
                    }
                }
            }
        }
        text
    }

    /// 持ち越したバイトを、置換文字にして返す。
    fn finish(self) -> String {
        String::from_utf8_lossy(&self.carry).into_owned()
    }
}

/// ロックを取る。ほかのスレッドが panic してロックが壊れていても、中の値を使う。
fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

#[cfg(test)]
mod tests;
