#[path = "build/bundled_content.rs"]
mod bundled_content;

use std::path::Path;

fn main() {
    // 同梱する教材を、`tauri.conf.json` の `bundle.resources` が指すディレクトリに写す。
    // `tauri_build::build()` が、開発時の起動のために、そのディレクトリを実行ファイルの隣へ写すので、先に行う。
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let content = manifest_dir.join("../content");
    println!("cargo:rerun-if-changed={}", content.display());
    bundled_content::stage(&content, &manifest_dir.join(bundled_content::STAGING_DIR))
        .expect("同梱する教材を写せない");

    tauri_build::build()
}
