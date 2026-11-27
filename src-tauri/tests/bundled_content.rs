//! アプリに同梱する教材・許可クレートの定義・指示文（spec/architecture/data-locations.md
//! 「アプリ同梱データ（`{app_resources}`）」）を確かめる。
//! 同梱するファイルは、ビルドスクリプトが `build/bundled_content.rs` で選んで写す。

#[path = "../build/bundled_content.rs"]
mod bundled_content;

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use serde_json::{Value, json};

/// 同梱するファイル。
const BUNDLED: [&str; 14] = [
    "chapters.json",
    "elements.json",
    "voice.json",
    "readings.json",
    "lessons/ch01-hello/lesson.json",
    "lessons/ch01-hello/video/script.json",
    "lessons/ch01-hello/video/voice/s1.m4a",
    "lessons/ch01-hello/video/voice/s2.m4a",
    "lessons/ch01-hello/exercise/problem.md",
    "lessons/ch01-hello/exercise/explanation.md",
    "lessons/ch01-hello/exercise/starter/src/main.rs",
    "lessons/ch01-hello/exercise/solution/src/main.rs",
    "lessons/ch01-hello/exercise/cases.json",
    "lessons/ch02-test/exercise/judge_tests.rs",
];

/// 同梱しないファイル（動画のコードと、動画が読み込む画像と、`.` で始まる名前のファイル）。
const NOT_BUNDLED: [&str; 11] = [
    "assets/common.png",
    "lessons/ch01-hello/assets/scene.png",
    "lessons/ch01-hello/video/index.tsx",
    "lessons/ch01-hello/video/scenes/Intro.tsx",
    "lessons/ch01-hello/video/timeline.ts",
    "lessons/ch01-hello/video/lesson.css",
    "lessons/ch01-hello/video/durations.json",
    "lessons/ch01-hello/video/cue-reference.json",
    ".DS_Store",
    "lessons/ch01-hello/video/voice/.DS_Store",
    "lessons/.hidden/lesson.json",
];

fn write_files(root: &Path, files: &[&str]) {
    for file in files {
        let path = root.join(file);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, format!("{file} の中身")).unwrap();
    }
}

/// `root` の下のファイルを、`root` からの相対パス（`/` 区切り）で集める。
fn list_files(root: &Path) -> BTreeSet<String> {
    fn walk(root: &Path, dir: &Path, files: &mut BTreeSet<String>) {
        for entry in fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                walk(root, &path, files);
            } else {
                let relative = path.strip_prefix(root).unwrap();
                files.insert(relative.to_str().unwrap().replace('\\', "/"));
            }
        }
    }
    let mut files = BTreeSet::new();
    walk(root, root, &mut files);
    files
}

fn strings(files: &[&str]) -> BTreeSet<String> {
    files.iter().map(|file| file.to_string()).collect()
}

#[test]
fn stages_content_without_video_code_and_images() {
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("content");
    let dest = temp.path().join("staged");
    write_files(&source, &BUNDLED);
    write_files(&source, &NOT_BUNDLED);

    bundled_content::stage(&source, &dest).unwrap();

    assert_eq!(list_files(&dest), strings(&BUNDLED));
    for file in BUNDLED {
        assert_eq!(
            fs::read_to_string(dest.join(file)).unwrap(),
            format!("{file} の中身")
        );
    }
}

/// `content/lessons/` にレッスンが 1 つも無くても写せる。
#[test]
fn stages_content_without_lessons() {
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("content");
    let dest = temp.path().join("staged");
    write_files(&source, &["chapters.json", "elements.json"]);

    bundled_content::stage(&source, &dest).unwrap();

    assert_eq!(
        list_files(&dest),
        strings(&["chapters.json", "elements.json"])
    );
}

/// 写し直すと、変わったファイルを書き直し、消えたファイルと同梱しないファイルを消す。
#[test]
fn restaging_follows_changes_in_source() {
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("content");
    let dest = temp.path().join("staged");
    write_files(&source, &BUNDLED);
    bundled_content::stage(&source, &dest).unwrap();

    fs::write(source.join("chapters.json"), "変えた中身").unwrap();
    fs::remove_dir_all(source.join("lessons/ch02-test")).unwrap();
    fs::remove_file(source.join("lessons/ch01-hello/video/voice/s2.m4a")).unwrap();
    // 前に写したもののうち、同梱しないファイル（前の版で写したものなど）も消す。
    write_files(&dest, &["lessons/ch01-hello/video/index.tsx"]);

    bundled_content::stage(&source, &dest).unwrap();

    let expected: BTreeSet<String> = strings(&BUNDLED)
        .into_iter()
        .filter(|file| !file.starts_with("lessons/ch02-test/") && !file.ends_with("voice/s2.m4a"))
        .collect();
    assert_eq!(list_files(&dest), expected);
    assert_eq!(
        fs::read_to_string(dest.join("chapters.json")).unwrap(),
        "変えた中身"
    );
    assert!(
        !dest.join("lessons/ch02-test").exists(),
        "空になったディレクトリを消す"
    );
}

/// 中身が同じファイルは書き直さない（cargo がビルドスクリプトを実行し直し続けないように）。
#[test]
fn restaging_keeps_unchanged_files() {
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("content");
    let dest = temp.path().join("staged");
    write_files(&source, &BUNDLED);
    bundled_content::stage(&source, &dest).unwrap();
    let staged = dest.join("lessons/ch01-hello/lesson.json");
    let old_time = std::time::SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1_000_000);
    fs::File::options()
        .write(true)
        .open(&staged)
        .unwrap()
        .set_modified(old_time)
        .unwrap();

    bundled_content::stage(&source, &dest).unwrap();

    assert_eq!(fs::metadata(&staged).unwrap().modified().unwrap(), old_time);
}

fn tauri_conf() -> Value {
    serde_json::from_str(include_str!("../tauri.conf.json")).expect("tauri.conf.json を読めない")
}

/// `bundle.resources` は、写したディレクトリだけを `{app_resources}/content/`・`runner/`・`prompts/` に入れる。
#[test]
fn bundle_resources_point_to_staging_dir() {
    assert_eq!(
        tauri_conf()["bundle"]["resources"],
        json!({
            (bundled_content::STAGING_DIR): "content/",
            (bundled_content::RUNNER_STAGING_DIR): "runner/",
            (bundled_content::PROMPTS_STAGING_DIR): "prompts/",
        })
    );
}

/// ビルドスクリプトが、リポジトリの `content/` から写したものと、写したディレクトリの中身が同じである。
#[test]
fn staging_dir_matches_repository_content() {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let temp = tempfile::tempdir().unwrap();
    bundled_content::stage(&manifest_dir.join("../content"), temp.path()).unwrap();
    assert_eq!(
        list_files(&manifest_dir.join(bundled_content::STAGING_DIR)),
        list_files(temp.path())
    );
}

/// 同梱する `runner/` のファイル。
const RUNNER: [&str; 2] = ["allowed-crates.toml", "Cargo.lock"];

/// `runner/` からは、`allowed-crates.toml` と `Cargo.lock` だけを写し、ほかのファイルは写さない。
#[test]
fn stages_runner_files_only() {
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("runner");
    let dest = temp.path().join("staged");
    write_files(&source, &RUNNER);
    write_files(&source, &["memo.md", ".DS_Store"]);
    write_files(&dest, &["old.txt"]);

    bundled_content::stage_runner(&source, &dest).unwrap();

    assert_eq!(list_files(&dest), strings(&RUNNER));
    assert_eq!(
        fs::read_to_string(dest.join("Cargo.lock")).unwrap(),
        "Cargo.lock の中身"
    );
}

/// 写したディレクトリと、開発時の起動で実行ファイルの隣に写されたものに、
/// リポジトリの `runner/` の 2 つのファイルが入っている。
#[test]
fn runner_files_are_staged_and_copied_next_to_executable() {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let repository = manifest_dir.join("../runner");
    // アプリの実行ファイル（`progrust`）の隣が、Tauri のビルドが資料を写す場所になる。
    let profile_dir = Path::new(env!("CARGO_BIN_EXE_progrust"))
        .parent()
        .unwrap()
        .to_path_buf();
    for dir in [
        manifest_dir.join(bundled_content::RUNNER_STAGING_DIR),
        profile_dir.join("runner"),
    ] {
        assert_eq!(list_files(&dir), strings(&RUNNER), "{}", dir.display());
        for file in RUNNER {
            assert_eq!(
                fs::read(dir.join(file)).unwrap(),
                fs::read(repository.join(file)).unwrap(),
                "{}",
                dir.join(file).display()
            );
        }
    }
}

/// 同梱する指示文（spec/claude-integration/prompts.md）。
const PROMPTS: [&str; 2] = ["chat-video.md", "chat-explanation.md"];

/// `prompts/` からは、直下の `.md` のファイルだけを写す。サブディレクトリの中と、`.md` 以外のファイルと、
/// `.` で始まる名前のファイルは写さない。前に写した、いまは無いファイルは消す。
#[test]
fn stages_prompt_markdown_files_only() {
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("prompts");
    let dest = temp.path().join("staged");
    write_files(&source, &PROMPTS);
    write_files(
        &source,
        &[
            "memo.txt",
            ".DS_Store",
            ".hidden.md",
            "drafts/chat-video.md",
            ".md",
        ],
    );
    write_files(&dest, &["old.md", "drafts/old.md"]);

    bundled_content::stage_prompts(&source, &dest).unwrap();

    assert_eq!(list_files(&dest), strings(&PROMPTS));
    assert!(
        !dest.join("drafts").exists(),
        "空になったディレクトリを消す"
    );
    assert_eq!(
        fs::read_to_string(dest.join("chat-video.md")).unwrap(),
        "chat-video.md の中身"
    );
}

/// 写したディレクトリと、開発時の起動で実行ファイルの隣に写されたもの（本番ビルドで
/// `{app_resources}/prompts/` に入るものと同じ）に、リポジトリの `prompts/` の
/// `chat-video.md` と `chat-explanation.md` が入っている。
#[test]
fn prompt_files_are_staged_and_copied_next_to_executable() {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let repository = manifest_dir.join("../prompts");
    let profile_dir = Path::new(env!("CARGO_BIN_EXE_progrust"))
        .parent()
        .unwrap()
        .to_path_buf();
    for dir in [
        manifest_dir.join(bundled_content::PROMPTS_STAGING_DIR),
        profile_dir.join("prompts"),
    ] {
        let files = list_files(&dir);
        for file in PROMPTS {
            assert!(files.contains(file), "{} に {file} が無い", dir.display());
            assert_eq!(
                fs::read(dir.join(file)).unwrap(),
                fs::read(repository.join(file)).unwrap(),
                "{}",
                dir.join(file).display()
            );
        }
    }
}

/// ビルドスクリプトが、リポジトリの `prompts/` から写したものと、写したディレクトリの中身が同じである。
#[test]
fn prompts_staging_dir_matches_repository_prompts() {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let temp = tempfile::tempdir().unwrap();
    bundled_content::stage_prompts(&manifest_dir.join("../prompts"), temp.path()).unwrap();
    assert_eq!(
        list_files(&manifest_dir.join(bundled_content::PROMPTS_STAGING_DIR)),
        list_files(temp.path())
    );
}
