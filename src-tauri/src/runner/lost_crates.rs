//! 演習の `--offline` のビルドの失敗が、初回準備のやり直しが要る失敗（許可クレートのソースか、ビルド済みの依存が
//! 失われている）に当たるかの判定（spec/exercise-runner/crates-and-workspace.md「初回準備のやり直しが要る失敗」、
//! ADR 0105、ADR 0137）。
//!
//! - 終了コード 101 で失敗したときだけ当てはめる。
//! - `.crate` の欠落とレジストリの索引の欠落は、cargo 自身の標準エラー出力から探す。ビルド済みの依存の破損（E0786）は、
//!   コンパイラの診断から探す。探す出力を広げない（学習者のプログラムの文字列と取り違えないため。ADR 0137）。
//! - 文言は cargo の版で変わりうるので、どの状態も 2 つ以上の語の組で判定する。
//! - E0786 は、学習者のコードのコンパイルエラーと同じ形で出るので、`note:` の文がビルド先の中のファイルを指すときだけ
//!   当てはめる。ビルド先の中のどこを指すかは見ない。

use std::path::Path;

use super::build::ErrorDiagnostic;

/// cargo が、ビルドの失敗のときに返す終了コード。
const CARGO_FAILURE_EXIT_CODE: i32 = 101;

/// cargo 自身の標準エラー出力から探す語の組。組のどの語もあるときに当てはまる。
const CARGO_STDERR_PATTERNS: &[&[&str]] = &[
    // ダウンロード済みの `.crate` が無い。
    &[
        "failed to download",
        "attempting to make an HTTP request, but --offline was specified",
    ],
    // レジストリの索引が無い。
    &[
        "no matching package named",
        "location searched: crates.io index",
    ],
];

/// ビルド済みの依存の破損の診断の `code`。
const INVALID_METADATA_CODE: &str = "E0786";
/// ビルド済みの依存の破損の診断の `message` に入る語。
const INVALID_METADATA_MESSAGE: &str = "found invalid metadata files for crate";

/// ビルドの失敗が、初回準備のやり直しが要る失敗に当たるか。
///
/// - `exit_code`: ビルドの cargo の終了コード（シグナルで終わったときは `None`）。
/// - `cargo_stderr`: cargo 自身の標準エラー出力。
/// - `diagnostics`: コンパイラの診断（`compiler-message`）のうち、レベルが `error` のもの。
/// - `target_dir`: そのビルドのビルド先（`CARGO_TARGET_DIR`）。
pub fn needs_crate_preparation(
    exit_code: Option<i32>,
    cargo_stderr: &str,
    diagnostics: &[ErrorDiagnostic],
    target_dir: &Path,
) -> bool {
    if exit_code != Some(CARGO_FAILURE_EXIT_CODE) {
        return false;
    }
    let lost_sources = CARGO_STDERR_PATTERNS
        .iter()
        .any(|words| words.iter().all(|word| cargo_stderr.contains(word)));
    lost_sources
        || diagnostics
            .iter()
            .any(|diagnostic| is_broken_dependency(diagnostic, target_dir))
}

/// ビルド先の中のファイルが壊れていることを示す E0786 の診断か。
fn is_broken_dependency(diagnostic: &ErrorDiagnostic, target_dir: &Path) -> bool {
    diagnostic.code.as_deref() == Some(INVALID_METADATA_CODE)
        && diagnostic.message.contains(INVALID_METADATA_MESSAGE)
        && diagnostic
            .notes
            .iter()
            .any(|note| points_inside(note, target_dir))
}

/// `note` の文が、`dir` の中のパスを含むか。`note` の中のパスの形（引用符で囲むかなど）は rustc の版で変わり、
/// パスには空白が入りうる（`Application Support`）ので、文を区切らずに、`dir` に `/` を続けた文字列を探す。
/// `/` を続けるので、`target/` の中を探すときに、隣の `target-std/` などには当てはまらない。
/// rustc はシンボリックリンクをたどったパス（macOS の `/var` は `/private/var`）を出すことがあるので、`dir` は、
/// そのままの形と、たどった形の両方で探す。
fn points_inside(note: &str, dir: &Path) -> bool {
    let mut dirs = vec![dir.to_path_buf()];
    if let Ok(canonical) = dir.canonicalize()
        && canonical != dir
    {
        dirs.push(canonical);
    }
    dirs.iter()
        .any(|dir| note.contains(&with_trailing_slash(dir)))
}

fn with_trailing_slash(dir: &Path) -> String {
    let mut text = dir.to_string_lossy().trim_end_matches('/').to_string();
    text.push('/');
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `cfg-if` の `.crate` が無いときの、cargo 1.99.1 の出力（spec/verification/issue-18-build-time.md）。
    const MISSING_CRATE: &str = "error: failed to download `cfg-if v1.0.5`\n\n\
         Caused by:\n  attempting to make an HTTP request, but --offline was specified\n";

    /// レジストリの索引が無いときの、cargo 1.99.1 の出力。
    const MISSING_INDEX: &str = "error: no matching package named `rand` found\n\
         location searched: crates.io index\n\
         required by package `progrust_exercise v0.1.0 (/w/project)`\n\
         note: offline mode (via `--offline`) can sometimes cause surprising resolution failures\n";

    /// 学習者のコードのコンパイルエラーのときの、cargo 自身の標準エラー出力。
    const COULD_NOT_COMPILE: &str = "error: could not compile `progrust_exercise` (bin \"progrust_exercise\") due to 1 previous error\n";

    fn invalid_metadata(note: &str) -> ErrorDiagnostic {
        ErrorDiagnostic {
            code: Some("E0786".into()),
            message: "found invalid metadata files for crate `rand`".into(),
            notes: vec![note.into()],
        }
    }

    /// rustc 1.100.0 で、ビルド先の依存の rlib を 0 バイトにしたときの `note:` の文。
    fn mmap_note(dir: &str) -> String {
        format!(
            "failed to mmap file '{dir}/debug/build/rand/0123456789abcdef/out/librand-0123456789abcdef.rlib': \
             memory map must have a non-zero length"
        )
    }

    const TARGET: &str = "/w/workspace/target";

    fn judge(exit_code: Option<i32>, stderr: &str, diagnostics: &[ErrorDiagnostic]) -> bool {
        needs_crate_preparation(exit_code, stderr, diagnostics, Path::new(TARGET))
    }

    #[test]
    fn the_three_states_of_the_table_need_the_preparation() {
        assert!(judge(Some(101), MISSING_CRATE, &[]));
        assert!(judge(Some(101), MISSING_INDEX, &[]));
        assert!(judge(
            Some(101),
            COULD_NOT_COMPILE,
            &[invalid_metadata(&mmap_note(TARGET))]
        ));
    }

    #[test]
    fn only_exit_code_101_is_judged() {
        for exit_code in [None, Some(1), Some(0)] {
            assert!(!judge(exit_code, MISSING_CRATE, &[]), "{exit_code:?}");
            assert!(!judge(exit_code, MISSING_INDEX, &[]), "{exit_code:?}");
            assert!(
                !judge(exit_code, "", &[invalid_metadata(&mmap_note(TARGET))]),
                "{exit_code:?}"
            );
        }
    }

    #[test]
    fn a_single_matching_word_is_not_enough() {
        for stderr in [
            "error: failed to download `cfg-if v1.0.5`\n",
            "Caused by:\n  attempting to make an HTTP request, but --offline was specified\n",
            "error: no matching package named `rand` found\n",
            "location searched: crates.io index\n",
            // 2 つの状態の語が 1 つずつ。
            "error: failed to download `cfg-if v1.0.5`\nlocation searched: crates.io index\n",
        ] {
            assert!(!judge(Some(101), stderr, &[]), "{stderr:?}");
        }

        let note = mmap_note(TARGET);
        // E0786 の code だけ、message だけ、note のパスだけが合う診断。
        let code_only = ErrorDiagnostic {
            message: "別のエラー".into(),
            ..invalid_metadata(&note)
        };
        let message_only = ErrorDiagnostic {
            code: Some("E0463".into()),
            ..invalid_metadata(&note)
        };
        let without_note = ErrorDiagnostic {
            notes: Vec::new(),
            ..invalid_metadata(&note)
        };
        for diagnostic in [code_only, message_only, without_note] {
            assert!(
                !judge(
                    Some(101),
                    COULD_NOT_COMPILE,
                    std::slice::from_ref(&diagnostic)
                ),
                "{diagnostic:?}"
            );
        }
    }

    #[test]
    fn the_words_are_searched_only_in_their_own_output() {
        // `.crate` の欠落の文が、コンパイラの診断（学習者の文字列など）にあっても当てはまらない。
        let in_diagnostic = ErrorDiagnostic {
            code: None,
            message: MISSING_CRATE.into(),
            notes: vec![MISSING_INDEX.into()],
        };
        assert!(!judge(Some(101), COULD_NOT_COMPILE, &[in_diagnostic]));
        // E0786 の文が、cargo 自身の標準エラー出力にあっても当てはまらない。
        let stderr = format!(
            "error[E0786]: found invalid metadata files for crate `rand`\n  = note: {}\n",
            mmap_note(TARGET)
        );
        assert!(!judge(Some(101), &stderr, &[]));
    }

    #[test]
    fn an_e0786_like_error_pointing_outside_the_build_dir_is_the_learners() {
        for dir in [
            // 学習者のプロジェクトの中。
            "/w/workspace/project",
            // 隣のビルド先。`target` で始まるが、`target/` の中ではない。
            "/w/workspace/target-std",
            "/w/workspace/target-validate",
            "/elsewhere",
        ] {
            assert!(
                !judge(
                    Some(101),
                    COULD_NOT_COMPILE,
                    &[invalid_metadata(&mmap_note(dir))]
                ),
                "{dir}"
            );
        }
        // ビルド先のパスを含んでも、中のファイルを指していなければ当てはまらない。
        assert!(!judge(
            Some(101),
            COULD_NOT_COMPILE,
            &[invalid_metadata("crate `rand` in /w/workspace/target")]
        ));
    }

    #[test]
    fn the_place_inside_the_build_dir_does_not_matter() {
        for note in [
            format!(
                "failed to mmap file '{TARGET}/debug/deps/librand-0123.rlib': memory map must have a non-zero length"
            ),
            format!("failed to decompress metadata: {TARGET}/debug/deps/librand-0123.rmeta"),
        ] {
            assert!(
                judge(Some(101), COULD_NOT_COMPILE, &[invalid_metadata(&note)]),
                "{note}"
            );
        }
        // 学習者のコードのエラーと一緒に出ても当てはまる。
        let learner = ErrorDiagnostic {
            code: Some("E0308".into()),
            message: "mismatched types".into(),
            notes: Vec::new(),
        };
        assert!(judge(
            Some(101),
            COULD_NOT_COMPILE,
            &[learner, invalid_metadata(&mmap_note(TARGET))]
        ));
    }

    #[test]
    fn build_dirs_with_spaces_and_symbolic_links_are_found() {
        // macOS のアプリのデータのディレクトリ（`Application Support`）には空白が入る。
        let dir = "/Users/a/Library/Application Support/app/workspace/target";
        assert!(needs_crate_preparation(
            Some(101),
            "",
            &[invalid_metadata(&mmap_note(dir))],
            Path::new(dir),
        ));
        // 末尾に `/` を付けたビルド先も同じ。
        assert!(needs_crate_preparation(
            Some(101),
            "",
            &[invalid_metadata(&mmap_note(dir))],
            Path::new(&format!("{dir}/")),
        ));

        // rustc がシンボリックリンクをたどったパスを出しても当てはまる。
        let base = tempfile::tempdir().unwrap();
        let real = base.path().join("real");
        std::fs::create_dir_all(real.join("target")).unwrap();
        let link = base.path().join("link");
        std::os::unix::fs::symlink(&real, &link).unwrap();
        let canonical: std::path::PathBuf = real.join("target").canonicalize().unwrap();
        assert!(needs_crate_preparation(
            Some(101),
            "",
            &[invalid_metadata(&mmap_note(&canonical.to_string_lossy()))],
            &link.join("target"),
        ));
    }
}
