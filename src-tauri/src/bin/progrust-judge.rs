//! 判定だけを行うコマンドラインのプログラム（spec/content-guidelines/check-tool.md、ADR 0208）。
//! 教材チェックツールが、`cargo run --features content-check --bin progrust-judge` で起動する（feature を付けたときだけ
//! ビルドし、アプリには入れない。ADR 0222）。中身は `progrust_lib::runner::lesson_judge`。

use std::process::ExitCode;

fn main() -> ExitCode {
    progrust_lib::runner::lesson_judge::main(std::env::args_os().skip(1))
}
