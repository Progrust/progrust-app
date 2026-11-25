//! テスト用の偽の `cargo`（spec/non-functional/testing.md「偽の claude と偽の cargo」）の、`build` と
//! `test --no-run` の動きを決めることと、動いた跡を読むことの手助け。
//!
//! 偽の `cargo` は、実行ファイルと同じディレクトリのファイルで動きを決める（環境変数では決めない）。動きは、次の
//! どれかの 1 語で書く（原本の `cargo` の説明）。
//!
//! - `succeed`: 環境変数 `CARGO_TARGET_DIR` のディレクトリ（ビルド先）を作って、成功する。
//! - `fail`: 標準エラー出力に失敗の文を書いて、終了コード 101 で失敗する。
//! - `hang`: 自分のプロセス ID を書いてから、止められるまで終わらない。
//!
//! 偽の `cargo`・`rustc` を置く手助け（`rust.rs`）に頼らず、偽の `cargo` のパスを受け取る。初回準備を動かすテスト
//! だけが `#[path = "fake_tools/cargo_behavior.rs"] mod fake_cargo;` で読み込む。

use std::{fs, path::Path};

fn write(cargo: &Path, name: &str, content: &str) {
    let path = cargo.with_file_name(name);
    fs::write(&path, content).unwrap_or_else(|e| panic!("{} に書けない: {e}", path.display()));
}

/// 偽の `cargo`（パス `cargo`）の、`cargo build` の動きを `build` に、`cargo test --no-run` の動きを `test_no_run` に
/// する。どちらも `succeed`・`fail`・`hang` のどれか。
pub fn set_behavior(cargo: &Path, build: &str, test_no_run: &str) {
    write(cargo, "cargo-build", build);
    write(cargo, "cargo-test-no-run", test_no_run);
}

/// `hang` の `cargo` が書いた、自分のプロセス ID。まだ書いていなければ `None`。
pub fn hanging_pid(cargo: &Path) -> Option<u32> {
    fs::read_to_string(cargo.with_file_name("cargo-pid"))
        .ok()
        .and_then(|text| text.trim().parse().ok())
}

/// これまでの `cargo build`・`cargo test` の呼び出しの引数（1 回を空白でつないだ 1 行）。呼び出した順に並ぶ。
pub fn cargo_calls(cargo: &Path) -> Vec<String> {
    fs::read_to_string(cargo.with_file_name("cargo-calls"))
        .map(|text| text.lines().map(str::to_string).collect())
        .unwrap_or_default()
}
