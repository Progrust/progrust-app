//! 検査の報告の読み取りのテスト。
//!
//! - 報告の形は、技術検証（documents/verification/issue-293-parse-sanitizer-reports.md）と、Apple clang の実物の
//!   報告を写したテキストで確かめる（どの OS でも実行する）。プロセス ID・アドレスは見本の値である。
//! - 実物の `clang` で検査つきにビルドしたプログラムの報告は、macOS のときだけ確かめる
//!   （spec/non-functional/testing.md「CI」、ADR 0276）。

use std::fs;

use super::*;

/// 解放後の使用（技術検証の見本。`main.c` の 5 行目で確保、7 行目で解放、8 行目で読んだ）。シャドウメモリの表の
/// 中身は短くしている。
const USE_AFTER_FREE: &str = "\
=================================================================
==42290==ERROR: AddressSanitizer: heap-use-after-free on address 0x602000000930 at pc 0x000100d94984 bp 0x00016f06b5a0 sp 0x00016f06b598
READ of size 4 at 0x602000000930 thread T0
    #0 0x000100d94980 in main main.c:8
    #1 0x000193cc4d78 in start+0x19f8 (dyld:arm64e+0x31d78)

0x602000000930 is located 0 bytes inside of 16-byte region [0x602000000930,0x602000000940)
freed by thread T0 here:
    #0 0x0001015f9368 in free+0x7c (libclang_rt.asan_osx_dynamic.dylib:arm64e+0x41368)
    #1 0x000100d948f8 in main main.c:7
    #2 0x000193cc4d78 in start+0x19f8 (dyld:arm64e+0x31d78)

previously allocated by thread T0 here:
    #0 0x0001015f9274 in malloc+0x78 (libclang_rt.asan_osx_dynamic.dylib:arm64e+0x41274)
    #1 0x000100d94854 in main main.c:5
    #2 0x000193cc4d78 in start+0x19f8 (dyld:arm64e+0x31d78)

SUMMARY: AddressSanitizer: heap-use-after-free main.c:8 in main
Shadow bytes around the buggy address:
  0x602000000680: fa fa fd fd fa fa fd fd fa fa fd fd fa fa fd fd
=>0x602000000900: fa fa fd fd fa fa[fd]fd fa fa fa fa fa fa fa fa
Shadow byte legend (one shadow byte represents 8 application bytes):
  Addressable:           00
==42290==ABORTING
";

/// 符号付き整数のあふれ（技術検証の見本）。
const SIGNED_OVERFLOW: &str = "\
main.c:6:11: runtime error: signed integer overflow: 2147483647 + 1 cannot be represented in type 'int'
SUMMARY: UndefinedBehaviorSanitizer: undefined-behavior main.c:6:11
";

/// `strlen` に決まった数のアドレスを渡した（Apple clang の実物の報告を写した）。誤りは `_platform_strlen` の中で
/// 起き、`main.c` の 12 行目から呼んだ。
const SEGV_IN_STRLEN: &str = "\
=================================================================
==80882==ERROR: AddressSanitizer: SEGV on unknown address 0x000000000010 (pc 0x000198d5eb44 bp 0x00016b831fc0 sp 0x00016b831780 T0)
==80882==The signal is caused by a READ memory access.
==80882==Hint: address points to the zero page.
    #0 0x000198d5eb44 in _platform_strlen+0x4 (libsystem_platform.dylib:arm64e+0xb44)
    #1 0x0001045cc830 in main main.c:12
    #2 0x000193cc4d78 in start+0x19f8 (dyld:arm64e+0x31d78)

==80882==Register values:
 x[0] = 0x0000000000000010   x[1] = 0x0000000000000010   x[2] = 0x000000016b8326a0   x[3] = 0x000000016b832920
 x[4] = 0x0000000000000001   x[5] = 0x0000000000000010   x[6] = 0x0000000000000063   x[7] = 0x0000000000000000
 x[8] = 0x0000000000000001   x[9] = 0x0000000200d68d20  x[10] = 0x00000001045cc000  x[11] = 0x00000000000007c0
x[12] = 0x0000000000000138  x[13] = 0x00000001045cc688  x[14] = 0x0000000000000000  x[15] = 0x00000001984c0100
x[16] = 0x0000000198d5eb40  x[17] = 0x0000000104f453d8  x[18] = 0x0000000000000000  x[19] = 0x0000000000000001
x[20] = 0x0000000000000010  x[21] = 0x000000016b8320a0  x[22] = 0x000000016b832150  x[23] = 0x0000000201042bf0
x[24] = 0x0000000000000001  x[25] = 0x0000000fffffc10c  x[26] = 0x000000016b832208  x[27] = 0x0000000201042be0
x[28] = 0x0000000104968000     fp = 0x000000016b831fc0     lr = 0x0000000104eaa608     sp = 0x000000016b831780
AddressSanitizer can not provide additional info.
SUMMARY: AddressSanitizer: SEGV (libsystem_platform.dylib:arm64e+0xb44) in _platform_strlen+0x4
==80882==ABORTING
";

/// 文字列リテラルを `free` した（Apple clang の実物の報告を写した）。ASan の `free` の中で `BUS` になった。
const BUS_IN_FREE: &str = "\
=================================================================
==80887==ERROR: AddressSanitizer: BUS on unknown address (pc 0x000100f42588 bp 0x00016f729780 sp 0x00016f729740 T0)
==80887==The signal is caused by a WRITE memory access.
==80887==Hint: this fault was caused by a dereference of a high value address (see register values below).  Disassemble the provided pc to learn which register was used.
    #0 0x000100f42588 in __asan::Allocator::Deallocate(void*, unsigned long, unsigned long, __sanitizer::BufferedStackTrace*, __asan::AllocType)+0x78 (libclang_rt.asan_osx_dynamic.dylib:arm64e+0x2588)
    #1 0x000100f813c8 in free+0xdc (libclang_rt.asan_osx_dynamic.dylib:arm64e+0x413c8)
    #2 0x0001006d4818 in main main.c:8
    #3 0x000193cc4d78 in start+0x19f8 (dyld:arm64e+0x31d78)

==80887==Register values:
 x[0] = 0x0000000000000000   x[1] = 0x00000001006d4900   x[2] = 0x0000000000000000   x[3] = 0x0000000000000000
x[28] = 0x0000000100a14000     fp = 0x000000016f729780     lr = 0x0000000100f42554     sp = 0x000000016f729740
AddressSanitizer can not provide additional info.
SUMMARY: AddressSanitizer: BUS main.c:8 in main
==80887==ABORTING
";

/// 確保に失敗したときの警告だけのファイル（技術検証の見本）。
const ALLOCATION_WARNING: &str =
    "==47680==WARNING: AddressSanitizer failed to allocate 0x7fffffffffffffff bytes\n";

/// AddressSanitizer の報告を、種類と最初のスタックから組み立てる。
fn asan_report(asan_type: &str, stack: &[&str]) -> String {
    let mut text = format!(
        "=================================================================\n\
         ==100==ERROR: AddressSanitizer: {asan_type} on address 0x602000000930 at pc 0x1 bp 0x2 sp 0x3\n\
         READ of size 4 at 0x602000000930 thread T0\n"
    );
    for (index, frame) in stack.iter().enumerate() {
        text.push_str(&format!("    #{index} 0x000100d94980 in {frame}\n"));
    }
    text.push_str(&format!(
        "\nSUMMARY: AddressSanitizer: {asan_type} main.c:1 in main\n==100==ABORTING\n"
    ));
    text
}

/// UndefinedBehaviorSanitizer の報告を、`main.c` の 9 行目の文から組み立てる。
fn ubsan_report(message: &str) -> String {
    format!(
        "main.c:9:5: runtime error: {message}\nSUMMARY: UndefinedBehaviorSanitizer: undefined-behavior main.c:9:5\n"
    )
}

fn kind_and_location(text: &str) -> (&'static str, Option<String>) {
    let error = parse_report(text);
    (error.kind.as_str(), error.location)
}

fn at(line: &str) -> Option<String> {
    Some(line.to_string())
}

// ---- kind と location ----

#[test]
fn asan_summary_types_map_to_kinds() {
    let main_frame = ["main main.c:12", "start+0x19f8 (dyld:arm64e+0x31d78)"];
    for (asan_type, kind) in [
        ("heap-use-after-free", "use_after_free"),
        ("double-free", "double_free"),
        ("heap-buffer-overflow", "out_of_bounds"),
        ("stack-buffer-overflow", "out_of_bounds"),
        ("stack-buffer-underflow", "out_of_bounds"),
        ("global-buffer-overflow", "out_of_bounds"),
        ("dynamic-stack-buffer-overflow", "out_of_bounds"),
        ("stack-use-after-return", "use_after_return"),
        ("bad-free", "invalid_free"),
        ("SEGV", "invalid_access"),
        ("BUS", "invalid_access"),
        ("stack-overflow", "stack_overflow"),
        // 表に無い種類は `other`。
        ("stack-use-after-scope", "other"),
        ("strcpy-param-overlap", "other"),
        ("allocation-size-too-big", "other"),
    ] {
        assert_eq!(
            kind_and_location(&asan_report(asan_type, &main_frame)),
            (kind, at("main.c:12")),
            "{asan_type}"
        );
    }
}

#[test]
fn the_technical_verification_samples_are_read() {
    assert_eq!(
        kind_and_location(USE_AFTER_FREE),
        ("use_after_free", at("main.c:8"))
    );
    assert_eq!(
        kind_and_location(SIGNED_OVERFLOW),
        ("undefined_behavior", at("main.c:6"))
    );
}

#[test]
fn ubsan_messages_map_to_kinds() {
    for (message, kind) in [
        ("index 5 out of bounds for type 'int[5]'", "out_of_bounds"),
        ("index -1 out of bounds for type 'int[5]'", "out_of_bounds"),
        ("load of null pointer of type 'int'", "invalid_access"),
        ("store to null pointer of type 'int'", "invalid_access"),
        (
            "member access within null pointer of type 'struct node'",
            "invalid_access",
        ),
        (
            "applying non-zero offset 12 to null pointer",
            "invalid_access",
        ),
        (
            "signed integer overflow: 2147483647 + 1 cannot be represented in type 'int'",
            "undefined_behavior",
        ),
        ("division by zero", "undefined_behavior"),
        (
            "shift exponent 40 is too large for 32-bit type 'int'",
            "undefined_behavior",
        ),
        (
            "load of misaligned address 0x000102 for type 'int', which requires 4 byte alignment",
            "undefined_behavior",
        ),
        // `index ` で始まらなければ、範囲外の文があっても範囲外アクセスにしない。
        (
            "the index 5 out of bounds for type 'int[5]'",
            "undefined_behavior",
        ),
    ] {
        assert_eq!(
            kind_and_location(&ubsan_report(message)),
            (kind, at("main.c:9")),
            "{message}"
        );
    }
}

#[test]
fn ubsan_outside_main_c_has_no_location() {
    let text = "/usr/include/stdlib.h:30:7: runtime error: division by zero\n\
                SUMMARY: UndefinedBehaviorSanitizer: undefined-behavior /usr/include/stdlib.h:30:7\n";
    assert_eq!(kind_and_location(text), ("undefined_behavior", None));
}

#[test]
fn the_ubsan_line_is_read_before_the_asan_summary() {
    // 最初の `runtime error:` の行で決める。
    let text = format!(
        "{}main.c:4:2: runtime error: division by zero\n{}",
        ubsan_report("load of null pointer of type 'int'"),
        asan_report("heap-use-after-free", &["main main.c:20"]),
    );
    assert_eq!(kind_and_location(&text), ("invalid_access", at("main.c:9")));
}

#[test]
fn a_runtime_error_line_without_the_position_falls_through_to_the_asan_summary() {
    let text = format!(
        "runtime error: index 5 out of bounds for type 'int[5]'\nmain.c:x:3: runtime error: division by zero\n{}",
        asan_report("double-free", &["main main.c:15"]),
    );
    assert_eq!(kind_and_location(&text), ("double_free", at("main.c:15")));

    // ASan の `SUMMARY:` の行も無ければ `other`。
    let text = "main.c: runtime error: division by zero\n";
    assert!(has_error_line(text));
    assert_eq!(kind_and_location(text), ("other", None));
}

#[test]
fn the_location_is_the_first_main_c_frame_of_the_first_stack() {
    // 標準ライブラリの関数の中で起きた誤り。`SUMMARY:` の行は `main.c` を指さない（技術検証の fgets の例）。
    let text = "\
=================================================================
==100==ERROR: AddressSanitizer: stack-buffer-overflow on address 0x00016b8e2f64 at pc 0x0001047580ac bp 0x00016b8e2ee0 sp 0x00016b8e2690
WRITE of size 7 at 0x00016b8e2f64 thread T0
    #0 0x0001047580a8 in memcpy+0x260 (libclang_rt.asan_osx_dynamic.dylib:arm64e+0x900a8)
    #1 0x000193dd03e4 in fgets+0x184 (libsystem_c.dylib:arm64e+0x1f3e4)
    #2 0x00010474e848 in fgets+0x30 (libclang_rt.asan_osx_dynamic.dylib:arm64e+0x86848)
    #3 0x000104060940 in main main.c:5
    #4 0x000193cc4d78 in start+0x19f8 (dyld:arm64e+0x31d78)

Address 0x00016b8e2f64 is located in stack of thread T0 at offset 36 in frame
    #0 0x00010406083c in main main.c:3

SUMMARY: AddressSanitizer: stack-buffer-overflow (libsystem_c.dylib:arm64e+0x1f3e4) in fgets+0x184
";
    assert_eq!(kind_and_location(text), ("out_of_bounds", at("main.c:5")));

    // 関数の中で起きた誤りは、その関数の中の行。後ろのスタック（解放した行・確保した行）は見ない。
    assert_eq!(
        kind_and_location(&asan_report(
            "heap-use-after-free",
            &["read_first main.c:15", "main main.c:21"],
        )),
        ("use_after_free", at("main.c:15"))
    );

    // 桁が付いていても、行までにする。
    assert_eq!(
        kind_and_location(&asan_report("heap-buffer-overflow", &["main main.c:7:12"])),
        ("out_of_bounds", at("main.c:7"))
    );
}

#[test]
fn without_a_main_c_frame_the_location_is_null() {
    // `main-checked.dSYM/` が無いときの形（技術検証「行番号が出る条件」）。
    assert_eq!(
        kind_and_location(&asan_report(
            "heap-use-after-free",
            &[
                "main+0x150 (main-checked:arm64+0x100000980)",
                "start+0x19f8 (dyld:arm64e+0x31d78)",
            ],
        )),
        ("use_after_free", None)
    );
    // `PATH` に `/usr/bin` が無いときの形。絶対パスの `main.c` でもないので、行を取らない。
    assert_eq!(
        kind_and_location(&asan_report(
            "heap-use-after-free",
            &["read_first+0xa4 (/private/tmp/w/c-project/main-checked:arm64+0x1000009c8)"],
        )),
        ("use_after_free", None)
    );
    // `main.c` のほかのファイルの行は取らない。
    assert_eq!(
        kind_and_location(&asan_report(
            "double-free",
            &["helper other.c:3", "main xmain.c:4"]
        )),
        ("double_free", None)
    );
}

#[test]
fn segv_and_bus_inside_free_are_invalid_free() {
    assert_eq!(
        kind_and_location(BUS_IN_FREE),
        ("invalid_free", at("main.c:8"))
    );
    assert_eq!(
        kind_and_location(SEGV_IN_STRLEN),
        ("invalid_access", at("main.c:12"))
    );
    assert_eq!(
        kind_and_location(&asan_report(
            "SEGV",
            &[
                "free+0xdc (libclang_rt.asan_osx_dynamic.dylib:arm64e+0x413c8)",
                "main main.c:6"
            ],
        )),
        ("invalid_free", at("main.c:6"))
    );
    // `main.c` のフレームより後ろの `free` は見ない。
    assert_eq!(
        kind_and_location(&asan_report(
            "SEGV",
            &[
                "main main.c:6",
                "free+0xdc (libclang_rt.asan_osx_dynamic.dylib:arm64e+0x413c8)"
            ],
        )),
        ("invalid_access", at("main.c:6"))
    );
    // `main.c` のフレームが無ければ、最初のスタックの全体を見る。
    assert_eq!(
        kind_and_location(&asan_report(
            "BUS",
            &[
                "free+0xdc (libclang_rt.asan_osx_dynamic.dylib:arm64e+0x413c8)",
                "main+0x18 (main-checked:arm64+0x100000818)",
            ],
        )),
        ("invalid_free", None)
    );
    // ほかの種類では `free` のフレームを見ない。
    assert_eq!(
        kind_and_location(&asan_report(
            "heap-buffer-overflow",
            &[
                "free+0xdc (libclang_rt.asan_osx_dynamic.dylib:arm64e+0x413c8)",
                "main main.c:6"
            ],
        )),
        ("out_of_bounds", at("main.c:6"))
    );
}

#[test]
fn a_report_without_ubsan_line_nor_asan_summary_is_other() {
    let text = "==100==ERROR: AddressSanitizer: something new\n    #0 0x1 in main main.c:3\n";
    assert_eq!(kind_and_location(text), ("other", None));
    // UBSan の `SUMMARY:` の行は、ASan の `SUMMARY:` の行ではない。
    let text = "==100==ERROR: AddressSanitizer: x\nSUMMARY: UndefinedBehaviorSanitizer: undefined-behavior\n";
    assert_eq!(kind_and_location(text), ("other", None));
}

#[test]
fn kinds_serialize_to_the_spec_values() {
    let error = MemoryError {
        kind: MemoryErrorKind::UseAfterReturn,
        location: None,
        report: "報告".into(),
    };
    assert_eq!(
        serde_json::to_value(&error).unwrap(),
        serde_json::json!({ "kind": "use_after_return", "location": null, "report": "報告" })
    );
    for kind in [
        MemoryErrorKind::UseAfterFree,
        MemoryErrorKind::DoubleFree,
        MemoryErrorKind::OutOfBounds,
        MemoryErrorKind::UseAfterReturn,
        MemoryErrorKind::InvalidFree,
        MemoryErrorKind::InvalidAccess,
        MemoryErrorKind::UndefinedBehavior,
        MemoryErrorKind::StackOverflow,
        MemoryErrorKind::Other,
    ] {
        assert_eq!(
            serde_json::to_value(kind).unwrap(),
            serde_json::json!(kind.as_str())
        );
    }
}

// ---- report ----

#[test]
fn the_report_ends_at_the_summary_line_without_separators() {
    assert_eq!(
        parse_report(USE_AFTER_FREE).report,
        "\
==42290==ERROR: AddressSanitizer: heap-use-after-free on address 0x602000000930 at pc 0x000100d94984 bp 0x00016f06b5a0 sp 0x00016f06b598
READ of size 4 at 0x602000000930 thread T0
    #0 0x000100d94980 in main main.c:8
    #1 0x000193cc4d78 in start+0x19f8 (dyld:arm64e+0x31d78)

0x602000000930 is located 0 bytes inside of 16-byte region [0x602000000930,0x602000000940)
freed by thread T0 here:
    #0 0x0001015f9368 in free+0x7c (libclang_rt.asan_osx_dynamic.dylib:arm64e+0x41368)
    #1 0x000100d948f8 in main main.c:7
    #2 0x000193cc4d78 in start+0x19f8 (dyld:arm64e+0x31d78)

previously allocated by thread T0 here:
    #0 0x0001015f9274 in malloc+0x78 (libclang_rt.asan_osx_dynamic.dylib:arm64e+0x41274)
    #1 0x000100d94854 in main main.c:5
    #2 0x000193cc4d78 in start+0x19f8 (dyld:arm64e+0x31d78)

SUMMARY: AddressSanitizer: heap-use-after-free main.c:8 in main"
    );
    assert_eq!(
        parse_report(SIGNED_OVERFLOW).report,
        "main.c:6:11: runtime error: signed integer overflow: 2147483647 + 1 cannot be represented in type 'int'\n\
         SUMMARY: UndefinedBehaviorSanitizer: undefined-behavior main.c:6:11"
    );
}

#[test]
fn the_register_values_are_removed_from_the_report() {
    assert_eq!(
        parse_report(SEGV_IN_STRLEN).report,
        "\
==80882==ERROR: AddressSanitizer: SEGV on unknown address 0x000000000010 (pc 0x000198d5eb44 bp 0x00016b831fc0 sp 0x00016b831780 T0)
==80882==The signal is caused by a READ memory access.
==80882==Hint: address points to the zero page.
    #0 0x000198d5eb44 in _platform_strlen+0x4 (libsystem_platform.dylib:arm64e+0xb44)
    #1 0x0001045cc830 in main main.c:12
    #2 0x000193cc4d78 in start+0x19f8 (dyld:arm64e+0x31d78)

AddressSanitizer can not provide additional info.
SUMMARY: AddressSanitizer: SEGV (libsystem_platform.dylib:arm64e+0xb44) in _platform_strlen+0x4"
    );
    let report = parse_report(BUS_IN_FREE).report;
    assert!(!report.contains("Register values"), "{report}");
    assert!(!report.contains("x[0]"), "{report}");
    assert!(!report.contains("fp ="), "{report}");
    assert!(report.ends_with(
        "\n\nAddressSanitizer can not provide additional info.\nSUMMARY: AddressSanitizer: BUS main.c:8 in main"
    ));
}

#[test]
fn without_a_summary_line_the_whole_report_is_used() {
    let text = "\
=================================================================
==100==ERROR: AddressSanitizer: something new
==100==Register values:
 x[0] = 0x0000000000000000   x[1] = 0x00000001006d4900
    #0 0x1 in main main.c:3
==
==100==ABORTING
";
    assert_eq!(
        parse_report(text).report,
        "==100==ERROR: AddressSanitizer: something new\n    #0 0x1 in main main.c:3\n==100==ABORTING"
    );
}

// ---- 報告のファイルの選び方 ----

/// `reports/` に `files`（名前と中身）を置く。
fn reports_dir(files: &[(&str, &str)]) -> tempfile::TempDir {
    let temp = tempfile::tempdir().unwrap();
    for (name, text) in files {
        fs::write(temp.path().join(name), text).unwrap();
    }
    temp
}

#[test]
fn the_file_of_the_started_process_is_used() {
    // 学習者のプログラムが `fork` した。名前の順では子（41867）が先だが、起動したプロセス（41868）のものを使う。
    let dir = reports_dir(&[
        ("case-1.41867", SIGNED_OVERFLOW),
        ("case-1.41868", USE_AFTER_FREE),
    ]);
    let error = read_memory_error(dir.path(), 1, 41868).unwrap().unwrap();
    assert_eq!(error.kind, MemoryErrorKind::UseAfterFree);
}

#[test]
fn without_the_file_of_the_started_process_the_first_name_is_used() {
    let dir = reports_dir(&[
        ("case-1.500", USE_AFTER_FREE),
        ("case-1.41867", SIGNED_OVERFLOW),
    ]);
    // 名前の順（`case-1.41867` < `case-1.500`）で最初のもの。
    let error = read_memory_error(dir.path(), 1, 9).unwrap().unwrap();
    assert_eq!(error.kind, MemoryErrorKind::UndefinedBehavior);
}

#[test]
fn a_warning_only_file_is_not_a_memory_error() {
    let dir = reports_dir(&[("case-1.47680", ALLOCATION_WARNING)]);
    assert_eq!(read_memory_error(dir.path(), 1, 47680).unwrap(), None);
    assert!(!has_error_line(ALLOCATION_WARNING));

    // 起動したプロセスのファイルが警告だけで、子のファイルに誤りがあるときは、子のものを使う。
    let dir = reports_dir(&[
        ("case-1.47680", ALLOCATION_WARNING),
        ("case-1.47681", BUS_IN_FREE),
    ]);
    let error = read_memory_error(dir.path(), 1, 47680).unwrap().unwrap();
    assert_eq!(error.kind, MemoryErrorKind::InvalidFree);
}

#[test]
fn only_files_of_the_test_case_are_read() {
    let dir = reports_dir(&[
        ("case-10.100", USE_AFTER_FREE),
        ("case-1", USE_AFTER_FREE),
        ("leak-1", USE_AFTER_FREE),
        ("xcase-1.100", USE_AFTER_FREE),
        ("case-2.100", SIGNED_OVERFLOW),
    ]);
    assert_eq!(read_memory_error(dir.path(), 1, 100).unwrap(), None);
    let error = read_memory_error(dir.path(), 2, 100).unwrap().unwrap();
    assert_eq!(error.kind, MemoryErrorKind::UndefinedBehavior);
    let error = read_memory_error(dir.path(), 10, 100).unwrap().unwrap();
    assert_eq!(error.kind, MemoryErrorKind::UseAfterFree);
}

#[test]
fn no_reports_is_not_a_memory_error() {
    let dir = reports_dir(&[]);
    assert_eq!(read_memory_error(dir.path(), 1, 100).unwrap(), None);
    assert_eq!(
        read_memory_error(&dir.path().join("missing"), 1, 100).unwrap(),
        None
    );
}

#[test]
fn directories_and_symlinks_are_not_read() {
    let dir = reports_dir(&[]);
    fs::create_dir(dir.path().join("case-1.100")).unwrap();
    let outside = tempfile::tempdir().unwrap();
    let target = outside.path().join("report");
    fs::write(&target, USE_AFTER_FREE).unwrap();
    std::os::unix::fs::symlink(&target, dir.path().join("case-1.101")).unwrap();
    assert_eq!(read_memory_error(dir.path(), 1, 101).unwrap(), None);
}

#[test]
fn reports_that_are_not_utf8_are_read() {
    let dir = reports_dir(&[]);
    let mut bytes = SIGNED_OVERFLOW.as_bytes().to_vec();
    bytes.extend_from_slice(b"\xff\xfe\n");
    fs::write(dir.path().join("case-1.100"), bytes).unwrap();
    let error = read_memory_error(dir.path(), 1, 100).unwrap().unwrap();
    assert_eq!(error.kind, MemoryErrorKind::UndefinedBehavior);
    assert_eq!(error.location.as_deref(), Some("main.c:6"));
}

// ---- 実物の clang でビルドしたプログラムの報告（macOS のときだけ） ----

#[cfg(target_os = "macos")]
mod macos {
    use super::*;

    use std::collections::BTreeMap;
    use std::path::PathBuf;
    use std::process::{Command, Stdio};

    use crate::environment::ClangSearch;
    use crate::runner::build::BuildStatus;
    use crate::runner::c_build::{CBuildKind, CBuildSpec, c_build};
    use crate::runner::c_project::assemble_c_project;
    use crate::runner::workspace::WorkspaceDir;

    /// c-submit.md「手順」の 2 の `ASAN_OPTIONS`（1 番目のテストケース）。
    const ASAN_OPTIONS: &str = "detect_stack_use_after_return=1:allocator_may_return_null=1:\
                                external_symbolizer_path=/usr/bin/atos:log_path=reports/case-1";

    /// 検査つきの実行の結果。
    struct Checked {
        error: Option<MemoryError>,
        /// 作業領域を置いた一時ディレクトリ。報告に出ていないことを確かめるために使う（ディレクトリは消えている）。
        temp: PathBuf,
        /// `reports/` にできたファイルの中身。
        report_files: Vec<String>,
    }

    /// `main_c` を検査つきでビルドし、`c-project/` を作業ディレクトリとして 1 番目のテストケースとして動かし、
    /// 検査の報告を読む。作業領域のパスには、空白と日本語を含める。
    fn run_checked(main_c: &str) -> Checked {
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
        let outcome = c_build(CBuildSpec::new(&clang, &project_dir, CBuildKind::Checked)).unwrap();
        assert_eq!(
            outcome.status,
            BuildStatus::Succeeded,
            "{}",
            outcome.clang_output()
        );
        assert!(project_dir.join("main-checked.dSYM").is_dir());

        let reports = project_dir.join(REPORTS_DIR);
        fs::create_dir(&reports).unwrap();
        let child = Command::new(project_dir.join(CBuildKind::Checked.output_file()))
            .current_dir(&project_dir)
            .env("ASAN_OPTIONS", ASAN_OPTIONS)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        let pid = child.id();
        let output = child.wait_with_output().unwrap();
        let error = read_memory_error(&reports, 1, pid).unwrap();
        if error.is_some() {
            // 誤りを報告したプログラムは、どれも SIGABRT で終わる（技術検証「報告のファイルと書式」）。
            assert!(!output.status.success(), "{:?}", output.status);
        }
        let report_files = fs::read_dir(&reports)
            .unwrap()
            .map(|entry| fs::read_to_string(entry.unwrap().path()).unwrap())
            .collect();
        Checked {
            error,
            temp: temp.path().to_path_buf(),
            report_files,
        }
    }

    /// `main_c` の報告の `kind`・`location` が期待どおりで、`report` に読めない部分と作業領域のパスが無いこと。
    fn assert_memory_error(main_c: &str, kind: MemoryErrorKind, location: &str) {
        let Checked { error, temp, .. } = run_checked(main_c);
        let error = error.expect("誤りが報告されない");
        assert_eq!(
            (error.kind, error.location.as_deref()),
            (kind, Some(location)),
            "{}",
            error.report
        );
        let report = &error.report;
        assert!(
            report.lines().any(|line| line.starts_with("SUMMARY:")),
            "{report}"
        );
        assert!(!report.contains("Shadow bytes"), "{report}");
        assert!(!report.contains("Register values"), "{report}");
        assert!(!report.contains("ABORTING"), "{report}");
        assert!(!report.lines().any(is_separator), "{report}");
        let temp = temp.to_string_lossy();
        assert!(!report.contains(temp.as_ref()), "{report}");
        assert!(!report.contains("日本語 データ"), "{report}");
    }

    #[test]
    fn use_after_free() {
        assert_memory_error(
            "#include <stdio.h>\n#include <stdlib.h>\n\nint main(void) {\n    int *p = malloc(sizeof(int) * 4);\n    p[0] = 1;\n    free(p);\n    printf(\"%d\\n\", p[0]);\n    return 0;\n}\n",
            MemoryErrorKind::UseAfterFree,
            "main.c:8",
        );
    }

    #[test]
    fn double_free() {
        assert_memory_error(
            "#include <stdlib.h>\n\nint main(void) {\n    int *p = malloc(sizeof(int));\n    int *q = p;\n    free(p);\n    free(q);\n    return 0;\n}\n",
            MemoryErrorKind::DoubleFree,
            "main.c:7",
        );
    }

    #[test]
    fn out_of_bounds_on_the_heap() {
        assert_memory_error(
            "#include <stdlib.h>\n\nint main(void) {\n    int *a = malloc(sizeof(int) * 3);\n    for (int i = 0; i <= 3; i++) {\n        a[i] = i;\n    }\n    free(a);\n    return 0;\n}\n",
            MemoryErrorKind::OutOfBounds,
            "main.c:6",
        );
    }

    #[test]
    fn out_of_bounds_inside_a_library_function() {
        // `strcpy` の中で起きた誤りは、呼んだ `main.c` の行。
        assert_memory_error(
            "#include <stdio.h>\n#include <string.h>\n\nint main(void) {\n    char buf[4];\n    strcpy(buf, \"hello\");\n    printf(\"%s\\n\", buf);\n    return 0;\n}\n",
            MemoryErrorKind::OutOfBounds,
            "main.c:6",
        );
    }

    #[test]
    fn out_of_bounds_on_an_array_name_is_reported_by_ubsan() {
        assert_memory_error(
            "#include <stdio.h>\n\nint main(void) {\n    int a[5] = {0};\n    int sum = 0;\n    for (int i = 0; i <= 5; i++) {\n        sum += a[i];\n    }\n    printf(\"%d\\n\", sum);\n    return 0;\n}\n",
            MemoryErrorKind::OutOfBounds,
            "main.c:7",
        );
    }

    #[test]
    fn use_after_return() {
        assert_memory_error(
            "#include <stdio.h>\n\nint *make(void) {\n    int x = 42;\n    int *p = &x;\n    return p;\n}\n\nint main(void) {\n    int *p = make();\n    printf(\"%d\\n\", *p);\n    return 0;\n}\n",
            MemoryErrorKind::UseAfterReturn,
            "main.c:11",
        );
    }

    #[test]
    fn invalid_free_of_the_middle_of_a_region() {
        assert_memory_error(
            "#include <stdlib.h>\n\nint main(void) {\n    int *p = malloc(sizeof(int) * 4);\n    free(p + 1);\n    return 0;\n}\n",
            MemoryErrorKind::InvalidFree,
            "main.c:5",
        );
    }

    #[test]
    fn invalid_free_of_a_string_literal() {
        // ASan の `free` の中で `BUS` になる（レジスタの値の表が入る）。
        assert_memory_error(
            "#include <stdlib.h>\n\nint main(void) {\n    char *s = \"abc\";\n    free(s);\n    return 0;\n}\n",
            MemoryErrorKind::InvalidFree,
            "main.c:5",
        );
    }

    #[test]
    fn null_pointer_access_in_learner_code() {
        assert_memory_error(
            "#include <stdio.h>\n#include <stddef.h>\n\nint main(void) {\n    int *p = NULL;\n    printf(\"%d\\n\", *p);\n    return 0;\n}\n",
            MemoryErrorKind::InvalidAccess,
            "main.c:6",
        );
        // 書き込みも同じ。
        assert_memory_error(
            "#include <stddef.h>\n\nstruct node {\n    int value;\n};\n\nint main(void) {\n    struct node *n = NULL;\n    n->value = 1;\n    return 0;\n}\n",
            MemoryErrorKind::InvalidAccess,
            "main.c:9",
        );
    }

    #[test]
    fn null_pointer_access_inside_a_library_function() {
        // `strlen(NULL)` は ASan が `SEGV` と報告する（レジスタの値の表が入る）。
        assert_memory_error(
            "#include <stdio.h>\n#include <string.h>\n\nint main(int argc, char **argv) {\n    (void)argv;\n    char *s = argc > 5 ? \"x\" : NULL;\n    printf(\"%zu\\n\", strlen(s));\n    return 0;\n}\n",
            MemoryErrorKind::InvalidAccess,
            "main.c:7",
        );
    }

    #[test]
    fn signed_integer_overflow() {
        // 定数の式は、ビルドのときに畳まれて報告されないので、実行のときの値で溢れさせる。
        assert_memory_error(
            "#include <limits.h>\n#include <stdio.h>\n\nint main(int argc, char **argv) {\n    (void)argv;\n    int x = INT_MAX - 1 + argc;\n    x = x + 1;\n    printf(\"%d\\n\", x);\n    return 0;\n}\n",
            MemoryErrorKind::UndefinedBehavior,
            "main.c:7",
        );
    }

    #[test]
    fn endless_recursion() {
        // 最初のスタックの `#0` は、再帰する関数の最初の行（技術検証「行（location）」）。
        assert_memory_error(
            "#include <stdio.h>\n\nint depth(int n) {\n    return depth(n + 1) + 1;\n}\n\nint main(void) {\n    printf(\"%d\\n\", depth(0));\n    return 0;\n}\n",
            MemoryErrorKind::StackOverflow,
            "main.c:3",
        );
    }

    #[test]
    fn an_allocation_failure_that_is_checked_is_not_a_memory_error() {
        // `allocator_may_return_null=1` で `NULL` が返り、警告だけのファイルができる（技術検証「誤りの種類と報告の語」）。
        let checked = run_checked(
            "#include <stdint.h>\n#include <stdio.h>\n#include <stdlib.h>\n\nint main(void) {\n    char *p = malloc(SIZE_MAX / 2);\n    if (p == NULL) {\n        printf(\"failed\\n\");\n        return 0;\n    }\n    free(p);\n    return 0;\n}\n",
        );
        assert_eq!(checked.error, None);
        assert_eq!(checked.report_files.len(), 1, "{:?}", checked.report_files);
        assert!(
            checked.report_files[0].contains("WARNING: AddressSanitizer failed to allocate"),
            "{:?}",
            checked.report_files
        );
    }

    #[test]
    fn a_correct_program_is_not_a_memory_error() {
        let checked = run_checked(
            "#include <stdio.h>\n#include <stdlib.h>\n\nint main(void) {\n    int *p = malloc(sizeof(int));\n    *p = 1;\n    printf(\"%d\\n\", *p);\n    free(p);\n    return 0;\n}\n",
        );
        assert_eq!(checked.error, None);
        assert!(
            checked.report_files.is_empty(),
            "{:?}",
            checked.report_files
        );
    }
}
