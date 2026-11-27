//! 回答中の質問（spec/architecture/commands/chat.md）。
//!
//! - 回答中の質問は、同時に 1 つだけにする（[`Chats::begin`]）。実行・提出（`runner::runs`）とは別に数えるので、
//!   実行・提出の間も質問できる。
//! - 回答は [`answer`] が、`send_chat` が起動したスレッドで `claude` を呼んで作り、断片を `chat_delta`、
//!   終わり方を `chat_finished` で 1 回だけ知らせる（[`ChatEvents`]）。
//! - 止めるのは [`Chats::cancel`]（`cancel_chat`）と [`Chats::stop_for_app_exit`]（アプリの終了）。どちらも
//!   [`Canceller`] で止め、`claude` のプロセスが終わるのを、呼び出しの部品（`claude::call`）が待つ。
//!
//! Tauri の実行時に頼らない（イベントの送り先は [`ChatEvents`] で受け取る）。

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant};

use serde::Serialize;

use crate::claude::{Canceller, ClaudeCall, ClaudeError, ClaudeOutcome, call::STOP_GRACE};
use crate::commands::error::{CommandError, ErrorCode};

/// 回答の断片のイベントの名前。
pub const CHAT_DELTA: &str = "chat_delta";

/// 回答の終わりのイベントの名前。
pub const CHAT_FINISHED: &str = "chat_finished";

/// 質疑応答の 1 回の呼び出しの制限時間（spec/claude-integration/invocation.md「制限時間」）。
pub const CHAT_TIME_LIMIT: Duration = Duration::from_secs(120);

/// アプリの終了で止めるときに、`claude` が終わるのを待つ最長の時間。SIGTERM から SIGKILL までの
/// [`STOP_GRACE`] と、終わった後に残りの出力を待つ時間より長くする。
const APP_EXIT_WAIT: Duration = Duration::from_secs(STOP_GRACE.as_secs() + 2);

/// イベント `chat_delta` の内容。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ChatDelta {
    pub request_id: String,
    /// 回答の断片。前の断片に続けてつなぐ。
    pub text: String,
}

/// `chat_finished` の終わり方。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ChatStatus {
    /// `result` の行を受け取った。
    Completed,
    /// `cancel_chat` で止めた。
    Cancelled,
    /// 失敗した。
    Error,
}

/// イベント `chat_finished` の内容。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ChatFinished {
    pub request_id: String,
    pub status: ChatStatus,
    /// `status` が `error` のときのエラー。ほかでは `None`（JSON では `null`）。
    pub error: Option<ClaudeError>,
}

/// 回答のイベントの送り先。
pub trait ChatEvents: Send + Sync {
    fn delta(&self, delta: ChatDelta);
    fn finished(&self, finished: ChatFinished);
}

/// 回答中の質問を、同時に 1 つだけ持つ状態。アプリの状態に置く。
#[derive(Clone)]
pub struct Chats {
    shared: Arc<Shared>,
    time_limit: Duration,
    work_dir_parent: PathBuf,
}

struct Shared {
    active: Mutex<Option<Arc<ActiveChat>>>,
    /// 回答中の質問を外したときに知らせる。
    released: Condvar,
}

impl Default for Chats {
    fn default() -> Self {
        Self::new(CHAT_TIME_LIMIT, std::env::temp_dir())
    }
}

impl Chats {
    /// 1 回の呼び出しの制限時間を `time_limit` にし、呼び出しごとの作業ディレクトリを `work_dir_parent` の中に作る。
    /// アプリでは [`CHAT_TIME_LIMIT`] と [`std::env::temp_dir`]（[`Chats::default`]）。
    pub fn new(time_limit: Duration, work_dir_parent: PathBuf) -> Self {
        Self {
            shared: Arc::new(Shared {
                active: Mutex::new(None),
                released: Condvar::new(),
            }),
            time_limit,
            work_dir_parent,
        }
    }

    /// 1 回の呼び出しの制限時間。
    pub fn time_limit(&self) -> Duration {
        self.time_limit
    }

    /// 呼び出しごとの作業ディレクトリを作る場所。
    pub fn work_dir_parent(&self) -> &Path {
        &self.work_dir_parent
    }

    /// `request_id` の質問を、回答中にする。`canceller` は、その質問の `claude` の呼び出しを止めるもの。
    /// 同じ ID の質問が回答中のときは `invalid_argument`、ほかの質問が回答中のときは `busy`。
    pub fn begin(
        &self,
        request_id: &str,
        canceller: Canceller,
    ) -> Result<Arc<ActiveChat>, CommandError> {
        let mut active = self.lock();
        if let Some(current) = active.as_ref() {
            if current.request_id == request_id {
                return Err(CommandError {
                    code: ErrorCode::InvalidArgument,
                    message: "同じ ID の質問が、既に回答中です。".into(),
                    detail: Some(format!("request_id {request_id} は回答中")),
                });
            }
            return Err(CommandError {
                code: ErrorCode::Busy,
                message: "回答が終わるまで待ってください。".into(),
                detail: None,
            });
        }
        let chat = Arc::new(ActiveChat {
            request_id: request_id.to_string(),
            canceller,
            app_exit: AtomicBool::new(false),
        });
        *active = Some(Arc::clone(&chat));
        Ok(chat)
    }

    /// 終わった質問を外す。ほかの質問が回答中のときは、何もしない。
    pub fn release(&self, chat: &Arc<ActiveChat>) {
        let mut active = self.lock();
        if active
            .as_ref()
            .is_some_and(|current| Arc::ptr_eq(current, chat))
        {
            *active = None;
            self.shared.released.notify_all();
        }
    }

    /// `request_id` の質問が回答中なら止める（`cancel_chat`）。回答中でないときは、何もしない。
    /// 止めた結果（`chat_finished` の `cancelled`）は、`claude` のプロセスが終わってから [`answer`] が送る。
    pub fn cancel(&self, request_id: &str) {
        let current = self.lock().clone();
        if let Some(chat) = current.filter(|chat| chat.request_id == request_id) {
            chat.canceller.cancel();
        }
    }

    /// アプリを終了するときに、回答中の質問を止め、その `claude` のプロセスが終わるまで待つ。止めた質問の
    /// `chat_finished` は送らない。
    ///
    /// [`Canceller::cancel`] は止めると決めるだけで、シグナルは呼び出しのスレッドが送る。待たずにアプリが
    /// 終わると、シグナルを送る前にスレッドが消え、別のプロセスグループの `claude` が残るので、待つ。
    pub fn stop_for_app_exit(&self) {
        let Some(chat) = self.lock().clone() else {
            return;
        };
        chat.app_exit.store(true, Ordering::SeqCst);
        chat.canceller.cancel();
        let deadline = Instant::now() + APP_EXIT_WAIT;
        let mut active = self.lock();
        while active
            .as_ref()
            .is_some_and(|current| Arc::ptr_eq(current, &chat))
        {
            let now = Instant::now();
            if now >= deadline {
                return;
            }
            active = self
                .shared
                .released
                .wait_timeout(active, deadline - now)
                .unwrap_or_else(PoisonError::into_inner)
                .0;
        }
    }

    /// 回答中の質問の ID。無いときは `None`。
    pub fn current_request_id(&self) -> Option<String> {
        self.lock().as_ref().map(|chat| chat.request_id.clone())
    }

    fn lock(&self) -> MutexGuard<'_, Option<Arc<ActiveChat>>> {
        // ロックを持つ間は、値を入れ替えるだけなので、毒されていても使う。
        self.shared
            .active
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
    }
}

/// 回答中の質問。
#[derive(Debug)]
pub struct ActiveChat {
    request_id: String,
    canceller: Canceller,
    /// アプリの終了で止めた。
    app_exit: AtomicBool,
}

impl ActiveChat {
    /// 質問の ID。
    pub fn request_id(&self) -> &str {
        &self.request_id
    }

    /// アプリの終了で止めたか。
    pub fn stopped_for_app_exit(&self) -> bool {
        self.app_exit.load(Ordering::SeqCst)
    }
}

/// `call` で `claude` を呼び、回答の断片を届いた順に `chat_delta` で送る。`claude` のプロセスが終わったら、
/// 質問を外してから（次の質問を送れるようにしてから）、`chat_finished` を 1 回送る。アプリの終了で止めたときは、
/// `chat_finished` を送らない。
///
/// `call` の [`Canceller`] は、`chat` を作るときに [`Chats::begin`] に渡したものであること。
pub fn answer(call: ClaudeCall, chat: Arc<ActiveChat>, chats: &Chats, events: &dyn ChatEvents) {
    let mut on_fragment = |text: &str| {
        events.delta(ChatDelta {
            request_id: chat.request_id.clone(),
            text: text.to_string(),
        });
    };
    let outcome = call.run(Some(&mut on_fragment));
    chats.release(&chat);
    if chat.stopped_for_app_exit() {
        return;
    }
    let (status, error) = match outcome {
        ClaudeOutcome::Completed { .. } => (ChatStatus::Completed, None),
        ClaudeOutcome::Cancelled => (ChatStatus::Cancelled, None),
        ClaudeOutcome::Failed(error) => (ChatStatus::Error, Some(error)),
    };
    events.finished(ChatFinished {
        request_id: chat.request_id.clone(),
        status,
        error,
    });
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use serde_json::json;

    use super::*;
    use crate::claude::{ClaudeEnv, ClaudeRequest};

    fn call() -> ClaudeCall {
        ClaudeCall::new(ClaudeRequest {
            claude: PathBuf::from("/nonexistent/claude"),
            env: ClaudeEnv::default(),
            model: "sonnet".into(),
            system_prompt: "system".into(),
            prompt: "prompt".into(),
            time_limit: Duration::from_secs(1),
            work_dir_parent: std::env::temp_dir(),
        })
    }

    #[derive(Default)]
    struct Recorded(Mutex<Vec<serde_json::Value>>);

    impl ChatEvents for Recorded {
        fn delta(&self, delta: ChatDelta) {
            self.0
                .lock()
                .unwrap()
                .push(serde_json::to_value(delta).unwrap());
        }
        fn finished(&self, finished: ChatFinished) {
            self.0
                .lock()
                .unwrap()
                .push(serde_json::to_value(finished).unwrap());
        }
    }

    #[test]
    fn the_time_limit_is_120_seconds() {
        assert_eq!(CHAT_TIME_LIMIT, Duration::from_secs(120));
        assert_eq!(Chats::default().time_limit(), CHAT_TIME_LIMIT);
        assert_eq!(Chats::default().work_dir_parent(), std::env::temp_dir());
    }

    #[test]
    fn only_one_chat_at_a_time() {
        let chats = Chats::default();
        let first = chats.begin("a", call().canceller()).unwrap();
        let same = chats.begin("a", call().canceller()).unwrap_err();
        assert_eq!(same.code, ErrorCode::InvalidArgument);
        let other = chats.begin("b", call().canceller()).unwrap_err();
        assert_eq!(other.code, ErrorCode::Busy);
        assert_eq!(chats.current_request_id().as_deref(), Some("a"));

        chats.release(&first);
        assert_eq!(chats.current_request_id(), None);
        let second = chats.begin("b", call().canceller()).unwrap();
        // 外した質問をもう一度外しても、回答中の質問は残る。
        chats.release(&first);
        assert_eq!(chats.current_request_id().as_deref(), Some("b"));
        chats.release(&second);
    }

    /// 起動する前に止めた呼び出しは、`claude` を起動せずに `cancelled` で終わり、質問が外れる。
    #[test]
    fn a_cancelled_chat_finishes_once_as_cancelled() {
        let chats = Chats::default();
        let call = call();
        let chat = chats.begin("a", call.canceller()).unwrap();
        chats.cancel("other");
        chats.cancel("a");
        let events = Recorded::default();
        answer(call, chat, &chats, &events);
        assert_eq!(
            *events.0.lock().unwrap(),
            [json!({ "request_id": "a", "status": "cancelled", "error": null })]
        );
        assert_eq!(chats.current_request_id(), None);
    }

    /// `claude` を起動できなかったときは、分類が `other` の `error` で終わる。
    #[test]
    fn a_chat_that_cannot_start_claude_finishes_as_other_error() {
        let chats = Chats::default();
        let call = call();
        let chat = chats.begin("a", call.canceller()).unwrap();
        let events = Recorded::default();
        answer(call, chat, &chats, &events);
        let events = events.0.lock().unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0]["status"], "error");
        assert_eq!(events[0]["error"]["category"], "other");
    }

    /// アプリの終了で止めた質問は、`chat_finished` を送らない。
    #[test]
    fn a_chat_stopped_for_app_exit_sends_no_event() {
        let chats = Chats::default();
        let call = call();
        let chat = chats.begin("a", call.canceller()).unwrap();
        let waiter = {
            let chats = chats.clone();
            std::thread::spawn(move || chats.stop_for_app_exit())
        };
        while !chat.stopped_for_app_exit() {
            std::thread::yield_now();
        }
        let events = Recorded::default();
        answer(call, chat, &chats, &events);
        waiter.join().unwrap();
        assert!(events.0.lock().unwrap().is_empty());
        assert_eq!(chats.current_request_id(), None);
    }

    /// 回答中の質問が無いときは、待たずに返る。
    #[test]
    fn stopping_for_app_exit_without_a_chat_returns_at_once() {
        let started = Instant::now();
        Chats::default().stop_for_app_exit();
        assert!(started.elapsed() < Duration::from_secs(1));
    }
}
