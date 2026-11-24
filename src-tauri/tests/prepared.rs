//! 初回準備の「実行する条件」のうち、`rustc -vV` の出力を使う判定
//! （spec/exercise-runner/crates-and-workspace.md「初回準備」、ADR 0100）を、偽の `rustc` で確かめる。

#[path = "fake_tools/rust.rs"]
mod fake_rust;

use std::fs;

use fake_rust::FakeRust;
use progrust_lib::environment::{CargoSearch, RustTools};
use progrust_lib::runner::prepared::{
    PreparedRecord, needs_preparation, read_prepared, rustc_verbose_version, write_prepared,
};
use progrust_lib::runner::workspace::WorkspaceDir;
use tempfile::TempDir;

const HASH: &str = "3f5a";

/// アプリの起動時と同じく、`cargo` を検出し、その隣の `rustc` を使う（spec/architecture/external-tools.md「検出」）。
fn detect(fake: &FakeRust) -> RustTools {
    CargoSearch {
        configured_cargo: Some(fake.cargo().to_path_buf()),
        path_var: None,
        home: None,
    }
    .detect()
    .expect("偽の cargo・rustc を検出できない")
}

#[test]
fn the_rustc_verbose_version_is_the_output_as_is() {
    let fake = FakeRust::new("1.100.0");
    let tools = detect(&fake);
    assert_eq!(
        rustc_verbose_version(&tools.rustc).as_deref(),
        Some(
            "rustc 1.100.0 (0123456789 2026-01-01)\n\
             binary: rustc\n\
             commit-hash: 0123456789012345678901234567890123456789\n\
             commit-date: 2026-01-01\n\
             host: aarch64-apple-darwin\n\
             release: 1.100.0\n\
             LLVM version: 23.1.5\n"
        )
    );
}

#[test]
fn a_rustc_that_cannot_run_gives_none() {
    let temp = TempDir::new().unwrap();
    assert_eq!(rustc_verbose_version(&temp.path().join("rustc")), None);
    // 失敗する（終了コードが 0 でない）`rustc`。偽の `rustc` は版のファイルが無いと失敗する。
    let fake = FakeRust::new("1.100.0");
    fs::remove_file(fake.rustc().with_file_name("rust-version")).unwrap();
    assert_eq!(rustc_verbose_version(fake.rustc()), None);
}

/// 初回準備をした後に `rustc` の版を変える（`rustup update` など）と、次の起動で初回準備をやり直すと判定する。
#[test]
fn changing_the_rustc_version_after_preparation_needs_preparation_again() {
    let fake = FakeRust::new("1.100.0");
    let rustc = detect(&fake).rustc;
    let temp = TempDir::new().unwrap();
    let workspace = WorkspaceDir::new(temp.path());

    let prepared_with = rustc_verbose_version(&rustc).unwrap();
    write_prepared(
        &workspace,
        &PreparedRecord::ready(HASH, &prepared_with, "2026-11-24T19:02:00+09:00"),
    )
    .unwrap();

    // 同じ rustc のままなら、やり直さない。
    let now = rustc_verbose_version(&rustc).unwrap();
    assert!(!needs_preparation(
        read_prepared(&workspace).unwrap().as_ref(),
        HASH,
        &now
    ));

    fs::write(fake.rustc().with_file_name("rust-version"), "1.101.0\n").unwrap();
    let updated = rustc_verbose_version(&rustc).unwrap();
    assert_ne!(updated, prepared_with);
    assert!(needs_preparation(
        read_prepared(&workspace).unwrap().as_ref(),
        HASH,
        &updated
    ));
}
