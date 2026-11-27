//! 質疑応答のコマンド `send_chat`・`cancel_chat`（spec/architecture/commands/chat.md）。
//!
//! AI に渡す入力の組み立ては [`crate::chat`]、回答中の質問と回答のタスクは [`crate::chat::answering`]、
//! `claude` の呼び出しは [`crate::claude`] にある。ここでは、受け付ける前に確かめること（引数、場面、`claude`）と、
//! 試行ログへの `chat` イベントの追記（spec/qa-chat/attempt-log.md）と、イベントの送り先を扱う。

use serde::Deserialize;
use tauri::ipc::{InvokeBody, Request};
use tauri::{AppHandle, Emitter, Manager, Runtime, State};

use super::error::{CommandError, ErrorCode};
use crate::attempts::ChatEvent;
use crate::chat::answering::{
    self, CHAT_DELTA, CHAT_FINISHED, ChatDelta, ChatEvents, ChatFinished, Chats,
};
use crate::chat::{
    ChatInputError, ChatTurn, PromptDir, build_explanation_input, build_video_input,
};
use crate::claude::{ClaudeCall, ClaudeEnv, ClaudeRequest};
use crate::content::{ContentDir, has_exercise};
use crate::environment::Environment;
use crate::progress::now_string;
use crate::storage::AppDataDir;

/// 質問文の文字数の上限（spec/qa-chat/screen.md）。Unicode の文字（`char`）で数える。
pub const QUESTION_MAX_CHARS: usize = 2_000;

/// 質疑応答で `--system-prompt` に渡す、既定のシステムプロンプトに代わる文
/// （spec/claude-integration/invocation.md）。回答の方針は、標準入力で渡す指示文に書く。
pub const CHAT_SYSTEM_PROMPT: &str = "あなたは Rust 学習アプリ「Progrust」の質問回答係です。\
     ツールは使えません。入力の指示文にある回答の方針に従い、学習者の質問に日本語で答えてください。";

/// 質問の場面（spec/qa-chat/availability.md）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChatContext {
    /// 動画の再生中。
    Video,
    /// 演習の解説表示中。
    Explanation,
}

/// `send_chat` の引数（spec/architecture/commands/chat.md「send_chat」）。
#[derive(Debug, Clone, Deserialize)]
pub struct SendChatArgs {
    pub request_id: String,
    pub lesson_id: String,
    pub context: ChatContext,
    /// `video` のとき、質問を送信したときに再生中だった文の ID。`explanation` では `None`。
    pub sentence_id: Option<String>,
    pub question: String,
    /// 古い順の会話履歴。
    pub history: Vec<ChatTurn>,
    /// エラーの後の「再試行」で送り直すときは `true`。
    pub retry: bool,
}

impl SendChatArgs {
    /// `invoke` の引数（JSON のオブジェクト）から読む。形が違うときは `invalid_argument`。
    fn from_body(body: &InvokeBody) -> Result<Self, CommandError> {
        let InvokeBody::Json(value) = body else {
            return Err(invalid_argument(
                "質問の引数が正しくありません。",
                "引数が JSON でない",
            ));
        };
        serde_json::from_value(value.clone())
            .map_err(|error| invalid_argument("質問の引数が正しくありません。", error))
    }
}

/// 質問を受け付け、`claude` の呼び出しを始めたら返す。回答の断片は `chat_delta`、終わり方は `chat_finished` で送る。
///
/// 引数は 7 つより多いので、`invoke` の引数のオブジェクトをそのまま [`SendChatArgs`] として読む（フロントエンドからの
/// 渡し方は、ほかのコマンドと同じ。`invoke("send_chat", { request_id, lesson_id, … })`）。
/// `claude` の検出が終わるのを待つことがあるので、async のコマンドにして、別のスレッドで確かめる。
#[tauri::command(rename_all = "snake_case")]
pub async fn send_chat<R: Runtime>(
    app: AppHandle<R>,
    request: Request<'_>,
) -> Result<(), CommandError> {
    let args = SendChatArgs::from_body(request.body())?;
    tauri::async_runtime::spawn_blocking(move || start_chat(&app, args))
        .await
        .map_err(|error| CommandError {
            code: ErrorCode::Internal,
            message: "質問を送れませんでした。".into(),
            detail: Some(error.to_string()),
        })?
}

/// 回答中の質問を止める。回答中でない ID のときは、何もしない。止めた結果は、`claude` のプロセスが終わってから
/// `chat_finished`（`cancelled`）で送る。
#[tauri::command(rename_all = "snake_case")]
pub fn cancel_chat(chats: State<'_, Chats>, request_id: String) -> Result<(), CommandError> {
    chats.cancel(&request_id);
    Ok(())
}

/// アプリを終了するときに、回答中の質問の `claude` を止め、プロセスが終わるまで待つ。`chat_finished` は送らない。
pub fn stop_chats_on_exit<R: Runtime>(app: &AppHandle<R>) {
    if let Some(chats) = app.try_state::<Chats>() {
        chats.stop_for_app_exit();
    }
}

/// 確かめる順は、引数 → 場面（レッスン、文の ID、正解した提出）と入力の組み立て → `claude` → 回答中の質問。
/// 受け付けたら、試行ログに追記してから（再試行を除く）、回答のスレッドを起動する。
fn start_chat<R: Runtime>(app: &AppHandle<R>, args: SendChatArgs) -> Result<(), CommandError> {
    check_arguments(&args)?;
    let content = app.state::<ContentDir>();
    let prompts = app.state::<PromptDir>();
    let data_dir = app.state::<AppDataDir>();
    let chats = app.state::<Chats>().inner().clone();

    let (input, log_context) = match args.context {
        ChatContext::Video => {
            // 引数の検査で、`video` の `sentence_id` があることを確かめてある。
            let sentence_id = args.sentence_id.as_deref().unwrap_or_default();
            let input = build_video_input(
                &content,
                &prompts,
                &args.lesson_id,
                sentence_id,
                &args.history,
                &args.question,
            )
            .map_err(input_error)?;
            (input, format!("video:{sentence_id}"))
        }
        ChatContext::Explanation => {
            let kind = content.lesson_kind(&args.lesson_id)?;
            if !has_exercise(&kind) {
                return Err(not_answerable(format!(
                    "レッスン {} には演習が無い",
                    args.lesson_id
                )));
            }
            let log = data_dir
                .unfinished_session(&args.lesson_id)?
                .filter(|log| log.passed_submit().is_some())
                .ok_or_else(|| {
                    not_answerable(format!(
                        "レッスン {} の終わっていない試行セッションに、正解した提出が無い",
                        args.lesson_id
                    ))
                })?;
            let input = build_explanation_input(
                &content,
                &prompts,
                &args.lesson_id,
                &log,
                &args.history,
                &args.question,
            )
            .map_err(input_error)?;
            (input, "explanation".to_string())
        }
    };

    let claude = app
        .state::<Environment>()
        .claude()
        .ok_or_else(|| CommandError {
            code: ErrorCode::ClaudeUnavailable,
            message: "Claude Code が見つかりません。".into(),
            detail: None,
        })?;
    // モデルの値が正しくないときは、呼び出しの部品が `claude` を起動せず、`chat_finished` の「その他」のエラーにする。
    let model = data_dir.read_settings()?.models.chat;
    let call = ClaudeCall::new(ClaudeRequest {
        claude: claude.path,
        env: ClaudeEnv::from_process(),
        model,
        system_prompt: CHAT_SYSTEM_PROMPT.into(),
        prompt: input,
        time_limit: chats.time_limit(),
        work_dir_parent: chats.work_dir_parent().to_path_buf(),
    });

    let chat = chats.begin(&args.request_id, call.canceller())?;
    if !args.retry {
        let event = ChatEvent {
            at: now_string(),
            context: log_context,
            question: args.question,
            answer_summary: None,
        };
        // 試行セッションが無いとき（動画のみのレッスンなど）は、記録しない。
        if let Err(error) = data_dir.append_chat(&args.lesson_id, event) {
            chats.release(&chat);
            return Err(error.into());
        }
    }

    let events = AppEvents(app.clone());
    std::thread::spawn(move || answering::answer(call, chat, &chats, &events));
    Ok(())
}

/// 引数だけで決まる条件（spec/architecture/commands/chat.md「send_chat」の「エラー」）。
fn check_arguments(args: &SendChatArgs) -> Result<(), CommandError> {
    if args.question.is_empty() {
        return Err(invalid_argument("質問を入力してください。", "質問が空"));
    }
    let chars = args.question.chars().count();
    if chars > QUESTION_MAX_CHARS {
        return Err(invalid_argument(
            "質問は 2,000 文字までにしてください。",
            format!("質問が {chars} 文字"),
        ));
    }
    match (args.context, &args.sentence_id) {
        (ChatContext::Video, None) => Err(invalid_argument(
            "再生中の文が分かりません。",
            "context が video で sentence_id が null",
        )),
        (ChatContext::Explanation, Some(_)) => Err(invalid_argument(
            "質問の引数が正しくありません。",
            "context が explanation で sentence_id が null でない",
        )),
        _ => Ok(()),
    }
}

fn invalid_argument(message: &str, detail: impl ToString) -> CommandError {
    CommandError {
        code: ErrorCode::InvalidArgument,
        message: message.into(),
        detail: Some(detail.to_string()),
    }
}

/// その場面で質問できない（spec/qa-chat/availability.md）。
fn not_answerable(detail: String) -> CommandError {
    CommandError {
        code: ErrorCode::InvalidState,
        message: "この場面では質問できません。".into(),
        detail: Some(detail),
    }
}

/// 入力を組み立てられなかった理由を、コマンドのエラーにする。
fn input_error(error: ChatInputError) -> CommandError {
    let detail = error.to_string();
    match error {
        ChatInputError::Content(error) => error.into(),
        ChatInputError::UnknownSentence { .. } => {
            invalid_argument("再生中の文が、このレッスンの台本にありません。", detail)
        }
        ChatInputError::NoVideo { .. }
        | ChatInputError::NoExercise { .. }
        | ChatInputError::NotPassed { .. } => not_answerable(detail),
        ChatInputError::Prompt { .. } => CommandError {
            code: ErrorCode::IoError,
            message: "AI への指示文の読み込みに失敗しました。".into(),
            detail: Some(detail),
        },
        ChatInputError::Template { .. }
        | ChatInputError::Invalid { .. }
        | ChatInputError::OtherLesson { .. } => CommandError {
            code: ErrorCode::Internal,
            message: "想定外のエラーが起きました。".into(),
            detail: Some(detail),
        },
    }
}

/// イベントを、アプリのウィンドウに `emit` で送る。
struct AppEvents<R: Runtime>(AppHandle<R>);

impl<R: Runtime> ChatEvents for AppEvents<R> {
    fn delta(&self, delta: ChatDelta) {
        // 送れなくても（ウィンドウが無いなど）、回答は続ける。
        let _ = self.0.emit(CHAT_DELTA, delta);
    }

    fn finished(&self, finished: ChatFinished) {
        let _ = self.0.emit(CHAT_FINISHED, finished);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(context: ChatContext, sentence_id: Option<&str>, question: &str) -> SendChatArgs {
        SendChatArgs {
            request_id: "r".into(),
            lesson_id: "ch01-01".into(),
            context,
            sentence_id: sentence_id.map(String::from),
            question: question.into(),
            history: Vec::new(),
            retry: false,
        }
    }

    fn code(args: &SendChatArgs) -> Option<ErrorCode> {
        check_arguments(args).err().map(|error| error.code)
    }

    #[test]
    fn the_question_must_not_be_empty_and_has_a_limit_of_2000_chars() {
        let invalid = Some(ErrorCode::InvalidArgument);
        assert_eq!(code(&args(ChatContext::Video, Some("m1"), "")), invalid);
        // 2,000 文字は送れ、2,001 文字は送れない。日本語も 1 文字と数える。
        let max = "あ".repeat(QUESTION_MAX_CHARS);
        assert_eq!(code(&args(ChatContext::Video, Some("m1"), &max)), None);
        let over = format!("{max}a");
        assert_eq!(code(&args(ChatContext::Video, Some("m1"), &over)), invalid);
    }

    #[test]
    fn sentence_id_is_required_only_for_video() {
        let invalid = Some(ErrorCode::InvalidArgument);
        assert_eq!(code(&args(ChatContext::Video, None, "質問")), invalid);
        assert_eq!(code(&args(ChatContext::Explanation, None, "質問")), None);
        assert_eq!(
            code(&args(ChatContext::Explanation, Some("m1"), "質問")),
            invalid
        );
    }

    #[test]
    fn the_system_prompt_is_not_empty() {
        assert!(!CHAT_SYSTEM_PROMPT.trim().is_empty());
    }
}
