//! メモリリークの検査のライブラリの部品のテスト。
//!
//! - 結果のファイルの読み取りと、ソースの書き出しと `clang` の起動（`clang` の代わりの `/bin/sh` のスクリプトで
//!   確かめる）は、どの OS でも実行する。
//! - 実物の `clang` でライブラリをビルドし、学習者のプログラムに差し込むテストは、macOS のときだけ実行する
//!   （spec/non-functional/testing.md「CI」、ADR 0276）。

use std::{
    collections::BTreeMap, fs, os::unix::fs::PermissionsExt, path::PathBuf, process::Command,
};

use super::*;
use crate::runner::{build::BuildStatus, c_project::assemble_c_project};

// ---- 結果のファイルの中身 ----

#[test]
fn the_report_path_is_reports_leak_n() {
    assert_eq!(leak_report_path(1), "reports/leak-1");
    assert_eq!(leak_report_path(12), "reports/leak-12");
}

#[test]
fn counts_and_bytes_are_read_from_two_lines() {
    for (text, count, bytes) in [
        ("0\n0\n", 0, 0),
        ("2\n18\n", 2, 18),
        ("1\n0\n", 1, 0),
        // 最後の改行は無くてもよい。
        ("4\n180", 4, 180),
        ("007\n010\n", 7, 10),
        (
            "18446744073709551615\n18446744073709551615\n",
            u64::MAX,
            u64::MAX,
        ),
    ] {
        assert_eq!(
            parse_leak_report(text.as_bytes()),
            LeakReport::Leaks { count, bytes },
            "{text:?}"
        );
    }
}

#[test]
fn overflow_is_read_from_its_single_line() {
    assert_eq!(parse_leak_report(b"overflow\n"), LeakReport::Overflow);
    assert_eq!(parse_leak_report(b"overflow"), LeakReport::Overflow);
}

#[test]
fn anything_else_is_malformed() {
    let malformed = LeakReport::Unavailable(LeakReportProblem::Malformed);
    for text in [
        "",
        "\n",
        "1\n",
        "1",
        "1\n2\n3\n",
        "1\n2\n\n",
        "\n1\n2\n",
        "1\n\n",
        "-1\n2\n",
        "+1\n2\n",
        " 1\n2\n",
        "1 \n2\n",
        "1\r\n2\r\n",
        "0x10\n2\n",
        "1.5\n2\n",
        "１\n2\n",
        // u64 に収まらない数。
        "18446744073709551616\n0\n",
        "0\n99999999999999999999\n",
        "OVERFLOW\n",
        "overflow \n",
        "overflow\n0\n",
        "overflow\n\n",
        "1\noverflow\n",
    ] {
        assert_eq!(parse_leak_report(text.as_bytes()), malformed, "{text:?}");
    }
    // UTF-8 でない中身。
    assert_eq!(parse_leak_report(b"\xff\n1\n"), malformed);
    assert_eq!(parse_leak_report(b"1\n2\xfe\n"), malformed);
    // ライブラリが書くより長い中身。
    let long = format!("1\n{}\n", "0".repeat(70));
    assert_eq!(parse_leak_report(long.as_bytes()), malformed);
}

// ---- 結果のファイルの読み取り ----

#[test]
fn the_report_of_the_case_is_read() {
    let temp = tempfile::tempdir().unwrap();
    let reports = temp.path().join("reports");
    fs::create_dir(&reports).unwrap();
    fs::write(reports.join("leak-1"), "3\n24\n").unwrap();
    fs::write(reports.join("leak-10"), "overflow\n").unwrap();
    fs::write(reports.join("case-2.100"), "0\n0\n").unwrap();

    assert_eq!(
        read_leak_report(&reports, 1),
        LeakReport::Leaks {
            count: 3,
            bytes: 24
        }
    );
    assert_eq!(read_leak_report(&reports, 10), LeakReport::Overflow);
    assert_eq!(
        read_leak_report(&reports, 2),
        LeakReport::Unavailable(LeakReportProblem::Missing)
    );
}

#[test]
fn a_missing_report_or_reports_dir_is_missing() {
    let temp = tempfile::tempdir().unwrap();
    let missing = LeakReport::Unavailable(LeakReportProblem::Missing);
    assert_eq!(read_leak_report(&temp.path().join("reports"), 1), missing);
    fs::create_dir(temp.path().join("reports")).unwrap();
    assert_eq!(read_leak_report(&temp.path().join("reports"), 1), missing);
}

#[test]
fn reports_that_are_not_regular_files_are_unreadable() {
    let temp = tempfile::tempdir().unwrap();
    let reports = temp.path().join("reports");
    fs::create_dir(&reports).unwrap();
    let unreadable = LeakReport::Unavailable(LeakReportProblem::Unreadable);

    fs::create_dir(reports.join("leak-1")).unwrap();
    assert_eq!(read_leak_report(&reports, 1), unreadable);

    // シンボリックリンクはたどらない。
    let target = temp.path().join("elsewhere");
    fs::write(&target, "0\n0\n").unwrap();
    std::os::unix::fs::symlink(&target, reports.join("leak-2")).unwrap();
    assert_eq!(read_leak_report(&reports, 2), unreadable);
}

#[test]
fn reports_that_cannot_be_read_are_unreadable() {
    let temp = tempfile::tempdir().unwrap();
    let reports = temp.path().join("reports");
    fs::create_dir(&reports).unwrap();
    let path = reports.join("leak-1");
    fs::write(&path, "0\n0\n").unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o000)).unwrap();
    // 管理者の権限で動いていると読めてしまうので、読めないことを確かめてから比べる。
    if fs::read(&path).is_err() {
        assert_eq!(
            read_leak_report(&reports, 1),
            LeakReport::Unavailable(LeakReportProblem::Unreadable)
        );
    }
    fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).unwrap();
}

#[test]
fn large_or_malformed_report_files_are_malformed() {
    let temp = tempfile::tempdir().unwrap();
    let reports = temp.path().join("reports");
    fs::create_dir(&reports).unwrap();
    let malformed = LeakReport::Unavailable(LeakReportProblem::Malformed);

    fs::write(reports.join("leak-1"), "").unwrap();
    assert_eq!(read_leak_report(&reports, 1), malformed);
    fs::write(reports.join("leak-2"), "1\n2\n".repeat(100_000)).unwrap();
    assert_eq!(read_leak_report(&reports, 2), malformed);
    fs::write(reports.join("leak-3"), "学習者のプログラムが書いたもの\n").unwrap();
    assert_eq!(read_leak_report(&reports, 3), malformed);
}

// ---- 埋め込んだソース ----

#[test]
fn the_embedded_source_matches_the_names_and_the_limit() {
    assert!(LEAK_CHECK_SOURCE.contains(&format!("#define REPORT_ENV \"{LEAK_REPORT_ENV}\"")));
    assert!(LEAK_CHECK_SOURCE.contains(&format!("#define INSERT_ENV \"{INSERT_LIBRARIES_ENV}\"")));
    assert!(LEAK_CHECK_SOURCE.contains("#define MAX_RECORDS ((size_t)1 << 19)"));
    assert_eq!(MAX_RECORDED_ALLOCATIONS, 1 << 19);
    for function in ["malloc", "calloc", "realloc", "free", "strdup"] {
        assert!(
            LEAK_CHECK_SOURCE
                .contains(&format!("PROGRUST_INTERPOSE(hook_{function}, {function});")),
            "{function}"
        );
    }
}

// ---- clang の代わりのスクリプトで確かめる ----

/// `script`（`/bin/sh` のスクリプトの本体）を `clang` の代わりにする。`c-project/` は `main.c` だけで組み立てる。
fn fake_fixture(script: &str) -> (tempfile::TempDir, WorkspaceDir, PathBuf) {
    let temp = tempfile::tempdir().unwrap();
    let workspace = WorkspaceDir::new(temp.path().join("workspace"));
    let files = BTreeMap::from([(
        "main.c".to_string(),
        "int main(void) { return 0; }\n".into(),
    )]);
    assemble_c_project(&workspace, &files).unwrap();
    // 書くために開いたファイルを、このプロセスから実行すると、ほかのテストのスレッドが同時に起動した子プロセスが
    // その書き込みの口を持っていて ETXTBSY で失敗することがある。書くのは別のプロセス（cp）に任せる。
    let draft = temp.path().join("fake-clang.txt");
    fs::write(&draft, format!("#!/bin/sh\n{script}\n")).unwrap();
    let clang = temp.path().join("bin/clang");
    fs::create_dir_all(clang.parent().unwrap()).unwrap();
    let copied = Command::new("cp").arg(&draft).arg(&clang).status().unwrap();
    assert!(copied.success());
    fs::set_permissions(&clang, fs::Permissions::from_mode(0o755)).unwrap();
    (temp, workspace, clang)
}

#[test]
fn the_source_is_written_and_clang_builds_the_library() {
    let (_temp, workspace, clang) = fake_fixture(
        "printf '%s\\n' \"$@\" > args.txt\ncp leakcheck/leakcheck.c seen.c\nprintf 'note\\n' >&2",
    );
    let outcome = build_leak_check_library(&clang, &workspace, Duration::from_secs(30)).unwrap();
    assert_eq!(outcome.status, BuildStatus::Succeeded);
    assert_eq!(outcome.clang_output(), "note\n");

    let dir = workspace.c_project();
    assert_eq!(
        fs::read_to_string(dir.join("args.txt")).unwrap(),
        "-dynamiclib\n-O2\n-o\nleakcheck/libprogrust_leakcheck.dylib\nleakcheck/leakcheck.c\n"
    );
    assert_eq!(
        fs::read_to_string(dir.join("seen.c")).unwrap(),
        LEAK_CHECK_SOURCE
    );
    assert_eq!(
        fs::read_to_string(dir.join("leakcheck/leakcheck.c")).unwrap(),
        LEAK_CHECK_SOURCE
    );
}

#[test]
fn a_failed_build_is_failed() {
    let (_temp, workspace, clang) = fake_fixture("echo 'error: x' >&2\nexit 1");
    let outcome = build_leak_check_library(&clang, &workspace, Duration::from_secs(30)).unwrap();
    assert_eq!(outcome.status, BuildStatus::Failed { exit_code: Some(1) });
}

#[test]
fn the_build_time_limit_is_applied() {
    let (_temp, workspace, clang) = fake_fixture("sleep 30");
    let started = std::time::Instant::now();
    let outcome = build_leak_check_library(&clang, &workspace, Duration::from_secs(1)).unwrap();
    assert_eq!(outcome.status, BuildStatus::TimedOut);
    assert!(started.elapsed() < Duration::from_secs(20));
}

#[test]
fn a_running_build_can_be_stopped() {
    let (_temp, workspace, clang) = fake_fixture("sleep 30");
    let running = start_leak_check_build(&clang, &workspace, Duration::from_secs(60)).unwrap();
    running.handle().stop();
    assert_eq!(running.wait().unwrap().status, BuildStatus::Stopped);
}

#[test]
fn failing_to_write_the_source_or_to_start_clang_is_an_error() {
    let (temp, workspace, clang) = fake_fixture("exit 0");
    // `leakcheck` がファイルなので、ソースを書けない。
    fs::write(workspace.c_project().join("leakcheck"), "ファイル").unwrap();
    assert!(build_leak_check_library(&clang, &workspace, Duration::from_secs(30)).is_err());

    fs::remove_file(workspace.c_project().join("leakcheck")).unwrap();
    let missing = temp.path().join("bin/no-clang");
    assert!(build_leak_check_library(&missing, &workspace, Duration::from_secs(30)).is_err());
}

// ---- 実物の clang でビルドして差し込む（macOS のときだけ） ----

#[cfg(target_os = "macos")]
mod macos {
    use super::*;

    use std::io::Write;
    use std::os::unix::process::{CommandExt, ExitStatusExt};
    use std::process::{ExitStatus, Stdio};

    use crate::environment::ClangSearch;
    use crate::runner::c_build::{CBuildKind, CBuildSpec, c_build};
    use crate::runner::c_project::leakcheck_library;

    /// 学習者のプログラムをビルドし、ライブラリをビルドした作業領域。パスに空白と日本語を含める。
    struct Fixture {
        _temp: tempfile::TempDir,
        workspace: WorkspaceDir,
    }

    /// 1 回の起動の結果。
    struct Ran {
        status: ExitStatus,
        stdout: Vec<u8>,
        /// 差し込んだときの結果のファイル。差し込まなかったときは `None`。
        report: Option<LeakReport>,
    }

    impl Fixture {
        fn new(main_c: &str) -> Self {
            let temp = tempfile::tempdir().unwrap();
            let workspace = WorkspaceDir::new(
                temp.path()
                    .join("Application Support/日本語 データ/workspace"),
            );
            let files = BTreeMap::from([("main.c".to_string(), main_c.to_string())]);
            assemble_c_project(&workspace, &files).unwrap();
            let clang = ClangSearch::from_process(None)
                .detect()
                .expect("clang が見つからない（Command Line Tools を入れる）")
                .path;
            let project_dir = workspace.c_project();
            let built =
                c_build(CBuildSpec::new(&clang, &project_dir, CBuildKind::Unchecked)).unwrap();
            assert_eq!(
                built.status,
                BuildStatus::Succeeded,
                "{}",
                built.clang_output()
            );
            let built =
                build_leak_check_library(&clang, &workspace, Duration::from_secs(120)).unwrap();
            assert_eq!(
                built.status,
                BuildStatus::Succeeded,
                "{}",
                built.clang_output()
            );
            fs::create_dir(project_dir.join(REPORTS_DIR)).unwrap();
            Self {
                _temp: temp,
                workspace,
            }
        }

        /// `c-project/main` を、`c-project/` を作業ディレクトリとして、新しいプロセスグループで起動する。`stdin` を
        /// 書き込んで標準入力を閉じる。`inject` のときは、1 番目のテストケースとしてライブラリを差し込み、
        /// 終わった後に結果のファイルを読む。
        fn run(&self, stdin: &str, inject: bool) -> Ran {
            let project_dir = self.workspace.c_project();
            let reports = project_dir.join(REPORTS_DIR);
            let _ = fs::remove_file(reports.join("leak-1"));
            let mut command = Command::new(project_dir.join(CBuildKind::Unchecked.output_file()));
            command
                .current_dir(&project_dir)
                .process_group(0)
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::null());
            if inject {
                command
                    .env(INSERT_LIBRARIES_ENV, leakcheck_library(&self.workspace))
                    .env(LEAK_REPORT_ENV, leak_report_path(1));
            }
            let mut child = command.spawn().unwrap();
            // 終わらないときは、プロセスグループ全体（fork した子を含む）を止めて、テストを失敗にする。
            let group = libc::pid_t::try_from(child.id()).unwrap();
            let (finished, wait_finish) = std::sync::mpsc::channel::<()>();
            let watchdog = std::thread::spawn(move || {
                let timed_out = matches!(
                    wait_finish.recv_timeout(RUN_LIMIT),
                    Err(std::sync::mpsc::RecvTimeoutError::Timeout)
                );
                if timed_out {
                    // SAFETY: 起動したプロセスグループにシグナルを送るだけである。
                    unsafe { libc::kill(-group, libc::SIGKILL) };
                }
                timed_out
            });
            child
                .stdin
                .take()
                .unwrap()
                .write_all(stdin.as_bytes())
                .unwrap();
            let output = child.wait_with_output().unwrap();
            drop(finished);
            assert!(
                !watchdog.join().unwrap(),
                "学習者のプログラムが {RUN_LIMIT:?} で終わらない"
            );
            Ran {
                status: output.status,
                stdout: output.stdout,
                report: inject.then(|| read_leak_report(&reports, 1)),
            }
        }

        /// 差し込んで起動し、終了コード 0 で終わったことを確かめて、結果のファイルを返す。
        fn report(&self, stdin: &str) -> LeakReport {
            let ran = self.run(stdin, true);
            assert_eq!(ran.status.code(), Some(0), "{:?}", ran.status);
            ran.report.unwrap()
        }
    }

    fn leaks(count: u64, bytes: u64) -> LeakReport {
        LeakReport::Leaks { count, bytes }
    }

    const MISSING: LeakReport = LeakReport::Unavailable(LeakReportProblem::Missing);

    /// 学習者のプログラムの 1 回の起動を待つ最長の時間。
    const RUN_LIMIT: Duration = Duration::from_secs(60);

    #[test]
    fn the_embedded_source_builds_without_warnings() {
        let temp = tempfile::tempdir().unwrap();
        let workspace = WorkspaceDir::new(temp.path().join("日本語 データ/workspace"));
        let files = BTreeMap::from([(
            "main.c".to_string(),
            "int main(void) { return 0; }\n".into(),
        )]);
        assemble_c_project(&workspace, &files).unwrap();
        let clang = ClangSearch::from_process(None)
            .detect()
            .expect("clang が見つからない（Command Line Tools を入れる）")
            .path;
        let outcome =
            build_leak_check_library(&clang, &workspace, Duration::from_secs(120)).unwrap();
        assert_eq!(
            outcome.status,
            BuildStatus::Succeeded,
            "{:?}",
            outcome.output
        );
        assert_eq!(outcome.clang_output(), "");
        assert!(leakcheck_library(&workspace).is_file());
    }

    #[test]
    fn unfreed_regions_are_counted_with_their_total_bytes() {
        let fx = Fixture::new(
            r#"#include <stdlib.h>

static char *lost_in_function(void) {
    char *p = malloc(40);
    p[0] = 'a';
    return NULL;
}

int main(void) {
    char *kept = malloc(100);
    kept[0] = 'k';
    lost_in_function();
    char *p = malloc(16);
    p = malloc(8);
    free(p);
    int *zeros = calloc(3, 8);
    (void)zeros;
    return 0;
}
"#,
        );
        // 100 + 40 + 16 + 24
        assert_eq!(fx.report(""), leaks(4, 180));
        // 2 回目も同じ。
        assert_eq!(fx.report(""), leaks(4, 180));
    }

    #[test]
    fn freeing_everything_gives_zero() {
        let fx = Fixture::new(
            r#"#include <stdlib.h>
#include <string.h>

struct node { int value; struct node *next; };
struct person { char *name; char *note; };

int main(void) {
    struct node *head = NULL;
    for (int i = 0; i < 5; i++) {
        struct node *n = malloc(sizeof(struct node));
        n->value = i;
        n->next = head;
        head = n;
    }
    while (head != NULL) {
        struct node *next = head->next;
        free(head);
        head = next;
    }
    struct person *p = calloc(1, sizeof(struct person));
    p->name = malloc(10);
    p->note = malloc(8);
    free(p->name);
    free(p->note);
    free(p);
    free(NULL);
    return 0;
}
"#,
        );
        assert_eq!(fx.report(""), leaks(0, 0));
    }

    #[test]
    fn allocations_inside_the_standard_library_are_not_counted() {
        let fx = Fixture::new(
            r#"#define _POSIX_C_SOURCE 200809L
#include <stdio.h>
#include <stdlib.h>

static int compare(const void *a, const void *b) {
    return *(const int *)a - *(const int *)b;
}

int main(void) {
    int n = 0;
    if (scanf("%d\n", &n) != 1) {
        return 1;
    }
    char line[64];
    if (fgets(line, sizeof(line), stdin) == NULL) {
        return 1;
    }
    int a = 0, b = 0;
    sscanf(line, "%d %d", &a, &b);

    char *got = NULL;
    size_t cap = 0;
    if (getline(&got, &cap, stdin) < 0) {
        return 1;
    }
    char *mine = malloc(2);
    size_t mine_cap = 2;
    if (getline(&mine, &mine_cap, stdin) < 0) {
        return 1;
    }

    int *values = malloc(sizeof(int) * 3);
    values[0] = 3; values[1] = 1; values[2] = 2;
    qsort(values, 3, sizeof(int), compare);
    for (int i = 0; i < 1000; i++) {
        printf("%d %d %d %s", n, a + b, values[0], got);
    }
    fprintf(stderr, "warning %s", mine);
    free(values);
    free(got);
    free(mine);
    return 0;
}
"#,
        );
        let stdin = "7\n2 3\nhello\na much longer line than two bytes\n";
        assert_eq!(fx.report(stdin), leaks(0, 0));
        let ran = fx.run(stdin, true);
        assert_eq!(ran.stdout, "7 5 1 hello\n".repeat(1000).into_bytes());
    }

    #[test]
    fn strdup_is_counted() {
        let fx = Fixture::new(
            r#"#define _POSIX_C_SOURCE 200809L
#include <stdlib.h>
#include <string.h>

int main(void) {
    char *kept = strdup("hello");
    char *freed = strdup("world");
    free(freed);
    char *grown = strdup("abc");
    grown = realloc(grown, 100);
    free(grown);
    (void)kept;
    return 0;
}
"#,
        );
        assert_eq!(fx.report(""), leaks(1, 6));
    }

    #[test]
    fn regions_moved_by_realloc_are_tracked() {
        let fx = Fixture::new(
            r#"#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>

int main(void) {
    int mode = 0;
    if (scanf("%d", &mode) != 1) {
        return 1;
    }
    /* 伸びる配列。領域が移る。 */
    size_t cap = 4;
    int *a = malloc(sizeof(int) * cap);
    int *first = a;
    for (size_t i = 0; i < 100000; i++) {
        if (i == cap) {
            cap *= 2;
            a = realloc(a, sizeof(int) * cap);
        }
        a[i] = (int)i;
    }
    printf("%s\n", a != first ? "moved" : "same");
    /* 縮める。 */
    char *s = malloc(1000);
    s = realloc(s, 10);
    /* realloc(NULL, n) は malloc と同じ。 */
    char *r = realloc(NULL, 40);
    /* 失敗した realloc は、古い領域を残す。 */
    volatile size_t huge = SIZE_MAX / 2;
    char *kept = malloc(24);
    char *failed = realloc(kept, huge);
    printf("%s\n", failed == NULL ? "failed" : "ok");
    if (mode == 1) {
        free(a);
        free(s);
        free(r);
        free(kept);
    }
    return 0;
}
"#,
        );
        assert_eq!(fx.report("1\n"), leaks(0, 0));
        // 伸びる配列（131,072 個の int）、縮めた 10 バイト、40 バイト、失敗した realloc の前の 24 バイト。
        assert_eq!(fx.report("0\n"), leaks(4, 524_288 + 10 + 40 + 24));
        assert_eq!(fx.run("0\n", true).stdout, b"moved\nfailed\n");
    }

    #[test]
    fn exit_atexit_and_a_nonzero_return_write_the_report() {
        let fx = Fixture::new(
            r#"#include <stdio.h>
#include <stdlib.h>

static char *global;

static void cleanup(void) {
    free(global);
}

static void quit(void) {
    exit(0);
}

int main(void) {
    int mode = 0;
    if (scanf("%d", &mode) != 1) {
        return 1;
    }
    if (mode == 0) {
        char *p = malloc(50);
        (void)p;
        quit();
    }
    if (mode == 1) {
        global = malloc(32);
        atexit(cleanup);
        return 0;
    }
    global = malloc(8);
    return 3;
}
"#,
        );
        assert_eq!(fx.report("0\n"), leaks(1, 50));
        assert_eq!(fx.report("1\n"), leaks(0, 0));
        let ran = fx.run("2\n", true);
        assert_eq!(ran.status.code(), Some(3));
        assert_eq!(ran.report, Some(leaks(1, 8)));
    }

    #[test]
    fn forked_and_executed_children_do_not_write() {
        let fx = Fixture::new(
            r#"#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/wait.h>
#include <unistd.h>

int main(int argc, char **argv) {
    if (argc > 1 && strcmp(argv[1], "child") == 0) {
        /* exec で起動された子。ライブラリが入っていれば、解放し忘れを書く。 */
        char *p = malloc(64);
        (void)p;
        exit(0);
    }
    int mode = 0;
    if (scanf("%d", &mode) != 1) {
        return 1;
    }
    pid_t pid = fork();
    if (pid == 0) {
        if (mode == 2) {
            execl("./main", "main", "child", (char *)NULL);
            _exit(127);
        }
        /* fork で作った子。解放し忘れを残して exit する。 */
        char *p = malloc(32);
        (void)p;
        exit(0);
    }
    int status = 0;
    waitpid(pid, &status, 0);
    if (!WIFEXITED(status) || WEXITSTATUS(status) != 0) {
        return 2;
    }
    if (mode == 0) {
        /* 親は書かずに終わる。子が書いていれば、結果のファイルが残る。 */
        _exit(0);
    }
    if (mode == 1) {
        char *q = malloc(10);
        free(q);
        return 0;
    }
    _exit(0);
}
"#,
        );
        // 子は書かない。親は _exit で終わるので、結果のファイルは無い。
        let ran = fx.run("0\n", true);
        assert_eq!(ran.status.code(), Some(0));
        assert_eq!(ran.report, Some(MISSING));
        // 結果は親のものだけ。
        assert_eq!(fx.report("1\n"), leaks(0, 0));
        // exec で起動した子にも、ライブラリは入らない。
        let ran = fx.run("2\n", true);
        assert_eq!(ran.status.code(), Some(0));
        assert_eq!(ran.report, Some(MISSING));
    }

    #[test]
    fn exit_without_atexit_abort_and_signals_do_not_write() {
        let fx = Fixture::new(
            r#"#include <stdio.h>
#include <stdlib.h>
#include <unistd.h>

int main(void) {
    int mode = 0;
    if (scanf("%d", &mode) != 1) {
        return 1;
    }
    char *p = malloc(16);
    (void)p;
    if (mode == 0) {
        _exit(0);
    }
    if (mode == 1) {
        abort();
    }
    volatile int *null = NULL;
    *null = 1;
    return 0;
}
"#,
        );
        let ran = fx.run("0\n", true);
        assert_eq!(ran.status.code(), Some(0));
        assert_eq!(ran.report, Some(MISSING));

        let ran = fx.run("1\n", true);
        assert_eq!(ran.status.signal(), Some(libc::SIGABRT), "{:?}", ran.status);
        assert_eq!(ran.report, Some(MISSING));

        let ran = fx.run("2\n", true);
        assert!(
            matches!(ran.status.signal(), Some(libc::SIGSEGV | libc::SIGBUS)),
            "{:?}",
            ran.status
        );
        assert_eq!(ran.report, Some(MISSING));
    }

    #[test]
    fn more_than_the_limit_of_live_regions_is_overflow() {
        let fx = Fixture::new(
            r#"#include <stdio.h>
#include <stdlib.h>

static void *regions[600000];

int main(void) {
    int n = 0, release = 0;
    if (scanf("%d %d", &n, &release) != 2 || n > 600000) {
        return 1;
    }
    for (int i = 0; i < n; i++) {
        regions[i] = malloc(1);
    }
    if (release) {
        for (int i = 0; i < n; i++) {
            free(regions[i]);
        }
    }
    return 0;
}
"#,
        );
        assert_eq!(fx.report("524288 1\n"), leaks(0, 0));
        assert_eq!(fx.report("524288 0\n"), leaks(524_288, 524_288));
        // 超えたら、後で解放しても overflow のまま。
        assert_eq!(fx.report("524289 1\n"), LeakReport::Overflow);
        assert_eq!(fx.report("600000 0\n"), LeakReport::Overflow);
    }

    #[test]
    fn stdout_and_the_exit_code_do_not_change_when_injected() {
        for (main_c, stdin) in [
            (
                r#"#include <stdio.h>
#include <stdlib.h>

int main(void) {
    char line[256];
    int count = 0;
    while (fgets(line, sizeof(line), stdin) != NULL) {
        count++;
        printf("%d: %s", count, line);
    }
    fprintf(stderr, "read %d lines\n", count);
    return count == 3 ? 0 : 4;
}
"#,
                "a\nいろは\nc\n",
            ),
            (
                r#"#include <stdio.h>
#include <stdlib.h>

int main(void) {
    char *p = malloc(8);
    for (int i = 0; i < 10000; i++) {
        printf("line %05d of the output\n", i);
    }
    (void)p;
    return 3;
}
"#,
                "",
            ),
            (
                r#"#include <stdio.h>
#include <stdlib.h>

int main(void) {
    const char *insert = getenv("DYLD_INSERT_LIBRARIES");
    const char *report = getenv("PROGRUST_LEAK_REPORT");
    printf("%s %s\n", insert == NULL ? "none" : insert, report == NULL ? "none" : report);
    return 0;
}
"#,
                "",
            ),
        ] {
            let fx = Fixture::new(main_c);
            let plain = fx.run(stdin, false);
            for _ in 0..2 {
                let injected = fx.run(stdin, true);
                assert_eq!(injected.stdout, plain.stdout, "{main_c}");
                assert_eq!(injected.status.code(), plain.status.code(), "{main_c}");
                assert!(
                    matches!(injected.report, Some(LeakReport::Leaks { .. })),
                    "{main_c}: {:?}",
                    injected.report
                );
            }
        }
        // 学習者のプログラムからは、差し込みの環境変数が見えない。
        let fx = Fixture::new(
            r#"#include <stdio.h>
#include <stdlib.h>

int main(void) {
    printf("%d %d\n", getenv("DYLD_INSERT_LIBRARIES") == NULL, getenv("PROGRUST_LEAK_REPORT") == NULL);
    return 0;
}
"#,
        );
        assert_eq!(fx.run("", true).stdout, b"1 1\n");
    }
}
