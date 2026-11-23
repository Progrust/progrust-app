//! アプリに同梱する教材を選んで写す（spec/architecture/data-locations.md「アプリ同梱データ（`{app_resources}`）」）。
//!
//! Tauri の `bundle.resources` には、ディレクトリの中の一部のファイルを除く指定が無い。
//! そこで、ビルドスクリプト（`build.rs`）が、リポジトリの `content/` から同梱するファイルだけを
//! [`STAGING_DIR`] に写し、`tauri.conf.json` の `bundle.resources` は、そのディレクトリを
//! `{app_resources}/content/` に入れる。`build.rs` と、テスト（`tests/bundled_content.rs`）から使う。

use std::collections::BTreeSet;
use std::fs;
use std::io;
use std::path::{Component, Path, PathBuf};

/// 同梱する教材を写すディレクトリ。`src-tauri/` からの相対パスで、`tauri.conf.json` の
/// `bundle.resources` のキーと同じ。`src-tauri/gen/` は git に入れない。
pub const STAGING_DIR: &str = "gen/bundled/content/";

/// 許可クレートの定義（`runner/`）を写すディレクトリ。`STAGING_DIR` と同じく、`tauri.conf.json` の
/// `bundle.resources` のキーと同じ。
pub const RUNNER_STAGING_DIR: &str = "gen/bundled/runner/";

/// `runner/` から同梱するファイル。
pub const RUNNER_FILES: [&str; 2] = ["allowed-crates.toml", "Cargo.lock"];

/// `content/` からの相対パスのファイルを、同梱するかどうか。
///
/// 動画のコード（`lessons/{lesson_id}/video/` のうち `script.json` と `voice/` 以外のファイル）と、
/// 動画が読み込む画像（`assets/` と `lessons/{lesson_id}/assets/`）は同梱しない。
/// `.` で始まる名前のファイルとディレクトリ（`.DS_Store` など）も、教材ではないので同梱しない。
pub fn is_bundled(relative: &Path) -> bool {
    let mut names = Vec::new();
    for component in relative.components() {
        let Component::Normal(name) = component else {
            return false;
        };
        let Some(name) = name.to_str() else {
            return false;
        };
        if name.starts_with('.') {
            return false;
        }
        names.push(name);
    }
    match names.as_slice() {
        [] | ["assets", ..] => false,
        ["lessons", _, "assets", ..] => false,
        ["lessons", _, "video", "script.json"] => true,
        ["lessons", _, "video", "voice", _, ..] => true,
        ["lessons", _, "video", ..] => false,
        _ => true,
    }
}

/// `source`（リポジトリの `content/`）から同梱するファイルを選び、`dest` に同じ構成で写す。
///
/// 中身が同じファイルは書き直さない（ファイルの更新時刻が変わると、cargo がビルドスクリプトを
/// 実行し直すため）。`source` から消えたファイルと、同梱しなくなったファイルは、`dest` から消す。
pub fn stage(source: &Path, dest: &Path) -> io::Result<()> {
    let mut files = BTreeSet::new();
    collect(source, Path::new(""), &mut files)?;

    fs::create_dir_all(dest)?;
    for relative in &files {
        copy_if_changed(&source.join(relative), &dest.join(relative))?;
    }
    remove_unlisted(dest, Path::new(""), &files)?;
    Ok(())
}

/// `source`（リポジトリの `runner/`）から [`RUNNER_FILES`] だけを `dest` に写す。
/// 写し方は [`stage`] と同じで、`dest` にある、それ以外のファイルは消す。
pub fn stage_runner(source: &Path, dest: &Path) -> io::Result<()> {
    fs::create_dir_all(dest)?;
    let files: BTreeSet<PathBuf> = RUNNER_FILES.iter().map(PathBuf::from).collect();
    for relative in &files {
        copy_if_changed(&source.join(relative), &dest.join(relative))?;
    }
    remove_unlisted(dest, Path::new(""), &files)
}

/// `from` を `to` に写す。`to` が既に同じ中身なら書き直さない。
fn copy_if_changed(from: &Path, to: &Path) -> io::Result<()> {
    let content = fs::read(from)?;
    if fs::read(to).is_ok_and(|existing| existing == content) {
        return Ok(());
    }
    if let Some(parent) = to.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(to, content)
}

/// `source` の下の同梱するファイルを、`source` からの相対パスで集める。
fn collect(source: &Path, relative: &Path, files: &mut BTreeSet<PathBuf>) -> io::Result<()> {
    let mut entries = fs::read_dir(source.join(relative))?.collect::<Result<Vec<_>, _>>()?;
    entries.sort_by_key(|entry| entry.file_name());
    for entry in entries {
        let path = relative.join(entry.file_name());
        if entry.path().is_dir() {
            collect(source, &path, files)?;
        } else if is_bundled(&path) {
            files.insert(path);
        }
    }
    Ok(())
}

/// `dest` の下の、`files` に無いファイルと、空になったディレクトリを消す。
fn remove_unlisted(dest: &Path, relative: &Path, files: &BTreeSet<PathBuf>) -> io::Result<()> {
    for entry in fs::read_dir(dest.join(relative))? {
        let entry = entry?;
        let path = relative.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            remove_unlisted(dest, &path, files)?;
            if fs::read_dir(entry.path())?.next().is_none() {
                fs::remove_dir(entry.path())?;
            }
        } else if !files.contains(&path) {
            fs::remove_file(entry.path())?;
        }
    }
    Ok(())
}
