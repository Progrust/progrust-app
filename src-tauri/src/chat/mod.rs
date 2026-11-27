//! 質疑応答で AI に渡す入力の組み立て（spec/qa-chat/ai-input.md）と、回答中の質問の扱い（[`answering`]）。
//!
//! 指示文（`{app_resources}/prompts/chat-video.md`・`chat-explanation.md`）の `{{name}}` に、教材と試行ログと
//! 会話履歴から作った文を埋め込む。Tauri の実行時に頼らずに呼べるように、教材のディレクトリ（[`ContentDir`]）と
//! 指示文のディレクトリ（[`PromptDir`]）と試行ログ（[`AttemptLog`]）を引数で受け取る。

pub mod answering;
pub mod template;

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use tauri::{AppHandle, Manager, Runtime};

use crate::attempts::AttemptLog;
use crate::content::{ContentDir, ContentError, Element};
use template::TemplateError;

/// 動画の再生中の指示文のファイル名（spec/claude-integration/prompts.md）。
pub const VIDEO_PROMPT: &str = "chat-video.md";

/// 解説表示中の指示文のファイル名（spec/claude-integration/prompts.md）。
pub const EXPLANATION_PROMPT: &str = "chat-explanation.md";

/// 入力に含める会話履歴の往復数の上限（spec/qa-chat/history.md）。
pub const HISTORY_TURNS: usize = 10;

/// 指示文のディレクトリ（`{app_resources}/prompts/`）。
#[derive(Debug, Clone)]
pub struct PromptDir {
    root: PathBuf,
}

impl PromptDir {
    /// `root` を指示文のディレクトリとして使う。
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    /// Tauri の `resource_dir()` の下の `prompts/` を指示文のディレクトリとして使う
    /// （spec/architecture/data-locations.md「アプリ同梱データ（`{app_resources}`）」）。
    pub fn from_app<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<Self> {
        Ok(Self::new(app.path().resource_dir()?.join("prompts")))
    }

    /// 指示文のディレクトリのパス。
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// 指示文のファイル `file`（例: [`VIDEO_PROMPT`]）を読む。
    pub fn read(&self, file: &str) -> Result<String, ChatInputError> {
        let path = self.root.join(file);
        fs::read_to_string(&path).map_err(|source| ChatInputError::Prompt { path, source })
    }
}

/// 会話履歴の 1 往復（spec/architecture/commands/chat.md「send_chat」の `history` の 1 件）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChatTurn {
    pub question: String,
    pub answer: String,
}

/// 入力を組み立てられなかった理由。
#[derive(Debug)]
pub enum ChatInputError {
    /// 教材を読めなかった（レッスンが無い、準備中である、を含む）。
    Content(ContentError),
    /// 指示文のファイルを読めなかった。
    Prompt { path: PathBuf, source: io::Error },
    /// 指示文の `{{name}}` を置き換えられなかった。
    Template { file: String, error: TemplateError },
    /// 教材のファイルが期待する形でない。
    Invalid {
        lesson_id: String,
        file: String,
        reason: String,
    },
    /// 動画の無いレッスン（章末復習レッスン）で、動画の再生中の入力を作ろうとした。
    NoVideo { lesson_id: String },
    /// 演習の無いレッスン（動画のみのレッスン）で、解説表示中の入力を作ろうとした。
    NoExercise { lesson_id: String },
    /// 台本に、再生中だった文の ID が無い。
    UnknownSentence {
        lesson_id: String,
        sentence_id: String,
    },
    /// 試行ログが、別のレッスンのものである。
    OtherLesson {
        lesson_id: String,
        session_id: String,
    },
    /// 試行ログに、正解した提出が無い。
    NotPassed { session_id: String },
}

impl fmt::Display for ChatInputError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Content(error) => write!(f, "{error}"),
            Self::Prompt { path, source } => write!(f, "{}: {source}", path.display()),
            Self::Template { file, error } => write!(f, "{file}: {error}"),
            Self::Invalid {
                lesson_id,
                file,
                reason,
            } => write!(f, "レッスン {lesson_id} の {file}: {reason}"),
            Self::NoVideo { lesson_id } => write!(f, "レッスン {lesson_id} には動画が無い"),
            Self::NoExercise { lesson_id } => write!(f, "レッスン {lesson_id} には演習が無い"),
            Self::UnknownSentence {
                lesson_id,
                sentence_id,
            } => write!(f, "レッスン {lesson_id} の台本に文 {sentence_id} が無い"),
            Self::OtherLesson {
                lesson_id,
                session_id,
            } => write!(
                f,
                "試行ログ {session_id} はレッスン {lesson_id} のものでない"
            ),
            Self::NotPassed { session_id } => {
                write!(f, "試行ログ {session_id} に正解した提出が無い")
            }
        }
    }
}

impl std::error::Error for ChatInputError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Content(error) => Some(error),
            Self::Prompt { source, .. } => Some(source),
            Self::Template { error, .. } => Some(error),
            _ => None,
        }
    }
}

impl From<ContentError> for ChatInputError {
    fn from(error: ContentError) -> Self {
        Self::Content(error)
    }
}

/// 動画の再生中の質問で AI に渡す入力を組み立てる（spec/qa-chat/ai-input.md「動画の再生中」）。
///
/// `sentence_id` は、質問を送信したときに再生中だった文の ID。`history` は古い順の会話履歴で、
/// 末尾から [`HISTORY_TURNS`] 往復までを入れる。演習の問題文・模範解答・テストケースは読まない。
pub fn build_video_input(
    content: &ContentDir,
    prompts: &PromptDir,
    lesson_id: &str,
    sentence_id: &str,
    history: &[ChatTurn],
    question: &str,
) -> Result<String, ChatInputError> {
    let lesson = read_lesson(content, lesson_id)?;
    if lesson.kind == "chapter_review" {
        return Err(ChatInputError::NoVideo {
            lesson_id: lesson_id.to_string(),
        });
    }
    let script: Script = parse(lesson_id, "video/script.json", content.script(lesson_id)?)?;
    let sentence = script
        .sentences
        .iter()
        .find(|sentence| sentence.id == sentence_id)
        .ok_or_else(|| ChatInputError::UnknownSentence {
            lesson_id: lesson_id.to_string(),
            sentence_id: sentence_id.to_string(),
        })?;
    let elements = content.elements()?;

    let lesson_text = render_lesson(&lesson, &elements, false);
    let script_text = render_script(&script);
    let ai_note = script
        .ai_note
        .as_deref()
        .filter(|note| !note.trim().is_empty())
        .unwrap_or("なし");
    let code_blocks = render_code_blocks(&script.code_blocks);
    let current_sentence = format!("[{}] {}", sentence.id, sentence.text);
    let learned = learned_elements(content, lesson_id, &lesson, &elements)?;
    let learned_text = render_element_list(&learned);
    let history_text = render_history(history);

    fill_prompt(
        prompts,
        VIDEO_PROMPT,
        &[
            ("lesson", &lesson_text),
            ("script", &script_text),
            ("ai_note", ai_note),
            ("code_blocks", &code_blocks),
            ("current_sentence", &current_sentence),
            ("learned_elements", &learned_text),
            ("history", &history_text),
            ("question", question),
        ],
    )
}

/// 解説表示中の質問で AI に渡す入力を組み立てる（spec/qa-chat/ai-input.md「解説表示中」）。
///
/// 学習者が正解したときのコードは、`attempt_log`（そのレッスンの試行ログ）の正解した `submit` の
/// ファイル（[`AttemptLog::passed_submit`]）を使う。`history` の扱いは [`build_video_input`] と同じ。
pub fn build_explanation_input(
    content: &ContentDir,
    prompts: &PromptDir,
    lesson_id: &str,
    attempt_log: &AttemptLog,
    history: &[ChatTurn],
    question: &str,
) -> Result<String, ChatInputError> {
    let lesson = read_lesson(content, lesson_id)?;
    if !crate::content::has_exercise(&lesson.kind) {
        return Err(ChatInputError::NoExercise {
            lesson_id: lesson_id.to_string(),
        });
    }
    if attempt_log.lesson_id != lesson_id {
        return Err(ChatInputError::OtherLesson {
            lesson_id: lesson_id.to_string(),
            session_id: attempt_log.session_id.clone(),
        });
    }
    let passed = attempt_log
        .passed_submit()
        .ok_or_else(|| ChatInputError::NotPassed {
            session_id: attempt_log.session_id.clone(),
        })?;
    let elements = content.elements()?;
    let answer = content.exercise_answer(lesson_id)?;

    let lesson_text = render_lesson(&lesson, &elements, true);
    let problem = fenced("markdown", &answer.problem_md);
    let explanation = fenced("markdown", &answer.explanation_md);
    let solution = render_files(&answer.solution_files);
    let learner_code = render_files(&passed.files);
    let history_text = render_history(history);

    fill_prompt(
        prompts,
        EXPLANATION_PROMPT,
        &[
            ("lesson", &lesson_text),
            ("problem", &problem),
            ("explanation", &explanation),
            ("solution", &solution),
            ("learner_code", &learner_code),
            ("history", &history_text),
            ("question", question),
        ],
    )
}

fn fill_prompt(
    prompts: &PromptDir,
    file: &str,
    values: &[(&str, &str)],
) -> Result<String, ChatInputError> {
    template::fill(&prompts.read(file)?, values).map_err(|error| ChatInputError::Template {
        file: file.to_string(),
        error,
    })
}

/// `lesson.json` のうち、入力に使うもの。
#[derive(Deserialize)]
struct Lesson {
    kind: String,
    title: String,
    #[serde(default)]
    primary_elements: Vec<String>,
    #[serde(default)]
    secondary_elements: Vec<String>,
}

/// `script.json` のうち、入力に使うもの（spec/content-format/video/script-json.md）。
#[derive(Deserialize)]
struct Script {
    #[serde(default)]
    ai_note: Option<String>,
    #[serde(default)]
    code_blocks: Vec<CodeBlock>,
    scenes: Vec<Scene>,
    sentences: Vec<Sentence>,
}

#[derive(Deserialize)]
struct CodeBlock {
    #[serde(default)]
    label: Option<String>,
    language: String,
    code: String,
    #[serde(default = "first_line")]
    start_line: u32,
    #[serde(default)]
    diff: bool,
}

fn first_line() -> u32 {
    1
}

#[derive(Deserialize)]
struct Scene {
    id: String,
    name: String,
}

#[derive(Deserialize)]
struct Sentence {
    id: String,
    scene: String,
    text: String,
}

fn read_lesson(content: &ContentDir, lesson_id: &str) -> Result<Lesson, ChatInputError> {
    parse(
        lesson_id,
        "lesson.json",
        content.lesson_definition(lesson_id)?,
    )
}

fn parse<T: for<'de> Deserialize<'de>>(
    lesson_id: &str,
    file: &str,
    value: Value,
) -> Result<T, ChatInputError> {
    serde_json::from_value(value).map_err(|error| ChatInputError::Invalid {
        lesson_id: lesson_id.to_string(),
        file: file.to_string(),
        reason: error.to_string(),
    })
}

/// あるレッスンにとっての既習の要素を、要素マスタの順に返す（spec/qa-chat/ai-input.md「動画の再生中」、
/// spec/content-format/chapters-json.md）。
///
/// アプリは章とレッスンの一覧を持たないので、同梱した教材から、次のどれかに当たる要素を既習とする。
///
/// - 要素マスタの `chapter`（導入レッスンの章）が、学習順でそのレッスンの章より前にある。
/// - 学習順でそのレッスンより前にある、準備中でない `kind` が `lesson` のレッスンの `primary_elements` にある。
/// - そのレッスンの `secondary_elements` にある（副要素は既習の要素である）。
///
/// そのレッスンと同じ章の、前にある準備中のレッスンだけが導入する要素は、求められないので含めない。
fn learned_elements<'a>(
    content: &ContentDir,
    lesson_id: &str,
    lesson: &Lesson,
    elements: &'a [Element],
) -> Result<Vec<&'a Element>, ChatInputError> {
    let order = content.learning_order()?;
    let position = order
        .iter()
        .position(|entry| entry.lesson_id == lesson_id)
        .ok_or_else(|| ContentError::LessonNotFound {
            lesson_id: lesson_id.to_string(),
        })?;

    let mut chapters: Vec<&str> = Vec::new();
    for entry in &order {
        if !chapters.contains(&entry.chapter_id.as_str()) {
            chapters.push(&entry.chapter_id);
        }
    }
    let chapter_index = |chapter: &str| chapters.iter().position(|id| *id == chapter);
    let lesson_chapter = chapter_index(&order[position].chapter_id);

    let mut learned: BTreeSet<String> = lesson.secondary_elements.iter().cloned().collect();
    for entry in &order[..position] {
        if !content.is_prepared(&entry.lesson_id) {
            continue;
        }
        let earlier = read_lesson(content, &entry.lesson_id)?;
        if earlier.kind == "lesson" {
            learned.extend(earlier.primary_elements);
        }
    }

    Ok(elements
        .iter()
        .filter(|element| !element.deprecated)
        .filter(|element| {
            learned.contains(&element.id)
                || chapter_index(&element.chapter)
                    .zip(lesson_chapter)
                    .is_some_and(|(element, lesson)| element < lesson)
        })
        .collect())
}

/// レッスンのタイトルと、主要素（`with_secondary` なら副要素も）の名前と説明。主要素の無いレッスン
/// （動画のみのレッスン）で副要素を入れないときは、タイトルだけにする。
fn render_lesson(lesson: &Lesson, elements: &[Element], with_secondary: bool) -> String {
    let mut text = format!("タイトル: {}", lesson.title);
    if lesson.primary_elements.is_empty() && !with_secondary {
        return text;
    }
    text.push_str("\n\n## 主要素\n\n");
    text.push_str(&render_element_ids(&lesson.primary_elements, elements));
    if with_secondary {
        text.push_str("\n\n## 副要素\n\n");
        text.push_str(&render_element_ids(&lesson.secondary_elements, elements));
    }
    text
}

/// 要素 ID の要素の名前と説明。要素マスタに無い要素 ID（教材チェックツールが検出するので、ふつうは無い）は、
/// ID だけを書く。
fn render_element_ids(ids: &[String], elements: &[Element]) -> String {
    if ids.is_empty() {
        return "なし".to_string();
    }
    ids.iter()
        .map(
            |id| match elements.iter().find(|element| element.id == *id) {
                Some(element) => element_line(element),
                None => format!("- {id}"),
            },
        )
        .collect::<Vec<_>>()
        .join("\n")
}

fn render_element_list(elements: &[&Element]) -> String {
    if elements.is_empty() {
        return "なし".to_string();
    }
    elements
        .iter()
        .map(|element| element_line(element))
        .collect::<Vec<_>>()
        .join("\n")
}

fn element_line(element: &Element) -> String {
    format!("- {}: {}", element.name, element.description)
}

/// 台本の全文。シーンごとにシーンの名前を見出しにし、そのシーンの各文の ID と `text` を台本の順に並べる。
fn render_script(script: &Script) -> String {
    script
        .scenes
        .iter()
        .map(|scene| {
            let sentences = script
                .sentences
                .iter()
                .filter(|sentence| sentence.scene == scene.id)
                .map(|sentence| format!("- [{}] {}", sentence.id, sentence.text))
                .collect::<Vec<_>>()
                .join("\n");
            format!("## {}\n\n{sentences}", scene.name)
        })
        .collect::<Vec<_>>()
        .join("\n\n")
}

/// 画面に出すコード（spec/qa-chat/ai-input.md「画面に出すコードの渡し方」）。
fn render_code_blocks(blocks: &[CodeBlock]) -> String {
    if blocks.is_empty() {
        return "なし".to_string();
    }
    blocks
        .iter()
        .map(render_code_block)
        .collect::<Vec<_>>()
        .join("\n\n")
}

fn render_code_block(block: &CodeBlock) -> String {
    let label = block.label.as_deref().filter(|label| !label.is_empty());
    if block.language == "text" {
        let heading = match label {
            Some(label) => format!("## ターミナルの出力（{label}）"),
            None => "## ターミナルの出力".to_string(),
        };
        return format!("{heading}\n\n{}", fenced("text", &block.code));
    }

    let heading = match label {
        Some(label) => format!("## {label}（言語: {}）", block.language),
        None => format!("## コード（言語: {}）", block.language),
    };
    let note = if block.diff {
        "Diff として表示する。行番号の後ろの 1 文字が `+` の行は追加した行、`-` の行は削除した行、\
         半角空白の行は変わらない行である。行番号は変更後のファイルの行番号で、削除した行には付けない。"
    } else {
        "各行の先頭の数字は、ファイルの行番号である。"
    };
    format!(
        "{heading}\n\n{note}\n\n{}",
        fenced(&block.language, &numbered_lines(block))
    )
}

/// 各行の先頭に行番号を付ける。行番号は `start_line` から数える。Diff では削除行（先頭が `-` の行）に
/// 行番号を付けず、数えもしない。行の先頭の `+`・`-`・半角空白は残す。
fn numbered_lines(block: &CodeBlock) -> String {
    let lines: Vec<&str> = block.code.split('\n').collect();
    let is_deleted = |line: &str| block.diff && line.starts_with('-');
    let numbered = lines.iter().filter(|line| !is_deleted(line)).count() as u32;
    let last = block.start_line + numbered.saturating_sub(1);
    let width = last.to_string().len();

    let mut number = block.start_line;
    lines
        .iter()
        .map(|line| {
            if is_deleted(line) {
                format!("{:>width$} | {line}", "")
            } else {
                let text = format!("{number:>width$} | {line}");
                number += 1;
                text
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// 会話履歴の末尾から [`HISTORY_TURNS`] 往復までを、古い順に並べる。
fn render_history(history: &[ChatTurn]) -> String {
    let recent = &history[history.len().saturating_sub(HISTORY_TURNS)..];
    if recent.is_empty() {
        return "なし".to_string();
    }
    recent
        .iter()
        .enumerate()
        .map(|(index, turn)| {
            let number = index + 1;
            format!(
                "## 学習者の質問 {number}\n\n{}\n\n## 回答 {number}\n\n{}",
                turn.question, turn.answer
            )
        })
        .collect::<Vec<_>>()
        .join("\n\n")
}

/// ファイルを、ファイルパスを見出しにして並べる。
fn render_files(files: &BTreeMap<String, String>) -> String {
    if files.is_empty() {
        return "なし".to_string();
    }
    files
        .iter()
        .map(|(path, text)| {
            let language = match Path::new(path).extension().and_then(|ext| ext.to_str()) {
                Some("rs") => "rust",
                Some("toml") => "toml",
                Some("md") => "markdown",
                _ => "",
            };
            let text = text.strip_suffix('\n').unwrap_or(text);
            format!("## {path}\n\n{}", fenced(language, text))
        })
        .collect::<Vec<_>>()
        .join("\n\n")
}

/// `text` をコードブロックで囲む。囲みの ``` は、`text` の中の最も長い ` の並びより長くする。
fn fenced(language: &str, text: &str) -> String {
    let longest = text.split(|c| c != '`').map(str::len).max().unwrap_or(0);
    let fence = "`".repeat(longest.max(2) + 1);
    let text = text.strip_suffix('\n').unwrap_or(text);
    format!("{fence}{language}\n{text}\n{fence}")
}

#[cfg(test)]
mod tests;
