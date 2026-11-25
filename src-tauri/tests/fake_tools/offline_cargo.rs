//! 演習のビルドは実物の `cargo` で、初回準備は偽の `cargo` で動かすための、テスト用の cargo の振り分け
//! （原本の `offline-cargo` の説明）を置く手助け。
//!
//! 引数に `--offline` がある呼び出し（演習のビルド）は、このテストを動かしている実物の `cargo` を、テストごとの
//! 空の `CARGO_HOME` で動かす。許可クレートのソースもレジストリの索引も無いので、許可クレートを使う演習のビルドは、
//! 初回準備のやり直しが要る失敗になる。ほかの呼び出し（初回準備）は、偽の `cargo` に渡す。ネットワークは使わない。
//!
//! `fake_tools/rust.rs`（[`FakeRust`]）と `fake_tools/cargo_behavior.rs` と一緒に、
//! `#[path = "fake_tools/offline_cargo.rs"] mod offline_cargo;` で読み込む。

use std::{
    fs,
    os::unix::fs::symlink,
    path::{Path, PathBuf},
};

use progrust_lib::environment::RustTools;
use tempfile::TempDir;

use crate::fake_rust::FakeRust;

/// 置いた振り分けと、初回準備を受け持つ偽の `cargo`・`rustc`。
pub struct OfflineCargo {
    _dir: TempDir,
    /// 初回準備を受け持つ偽の `cargo`・`rustc`。動きは `fake_cargo::set_behavior` で決める。
    pub fake: FakeRust,
    cargo: PathBuf,
}

impl OfflineCargo {
    pub fn new() -> Self {
        let dir = TempDir::new().expect("一時ディレクトリを作れない");
        let fake = FakeRust::new("1.100.0");
        let cargo = dir.path().join("cargo");
        symlink(
            Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fake_tools/offline-cargo"),
            &cargo,
        )
        .unwrap_or_else(|e| panic!("{} を置けない: {e}", cargo.display()));
        let cargo_home = dir.path().join("empty-cargo-home");
        fs::create_dir(&cargo_home).unwrap();
        let write = |name: &str, path: &Path| {
            fs::write(dir.path().join(name), path.as_os_str().as_encoded_bytes()).unwrap();
        };
        write("real-cargo", Path::new(env!("CARGO")));
        write("cargo-home", &cargo_home);
        write("other-cargo", fake.cargo());
        Self {
            _dir: dir,
            fake,
            cargo,
        }
    }

    /// 検出の結果として与える `cargo`（振り分け）と `rustc`（偽の `rustc`。初回準備が `rustc -vV` に使う）。
    pub fn tools(&self) -> RustTools {
        RustTools {
            cargo: self.cargo.clone(),
            rustc: self.fake.rustc().to_path_buf(),
            rustc_version: "1.100.0".parse().unwrap(),
        }
    }
}
