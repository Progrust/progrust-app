/*
 * Progrust のメモリリークの検査のライブラリ（spec/exercise-runner/c-submit.md「メモリリークの検査」、ADR 0273）。
 *
 * C の演習の提出で、検査なしの実行ファイルに DYLD_INSERT_LIBRARIES で差し込み、学習者の実行ファイルのコードが
 * 確保して、プログラムが終わるまでに解放しなかった領域の件数と合計のバイト数を、PROGRUST_LEAK_REPORT のパスの
 * 結果のファイルに書く。
 *
 * - malloc・calloc・realloc・free・strdup を、dyld の interpose（__DATA,__interpose 節の置き換えの表）で置き換える。
 *   置き換えた関数の中から呼ぶ malloc などは、本物が呼ばれる。
 * - 学習者の実行ファイル（読み込まれたイメージのうち、ファイルの種類が MH_EXECUTE のもの。差し込むと 0 番目の
 *   イメージはこのライブラリになるので、0 番目は使わない）の __TEXT セグメントの中から呼ばれた確保だけを記録する。
 *   解放は、どこから呼ばれたものでも記録から消す。
 * - 記録は、固定の大きさの線形探査のハッシュ表に入れる。削除の印を残さずに後ろを詰める。同時に記録できる領域は
 *   524,288 個までとし、超えたら、それ以降を記録せず、結果のファイルに overflow と書く。
 * - 読み込まれたときに、結果のファイルのパスを覚え、DYLD_INSERT_LIBRARIES と PROGRUST_LEAK_REPORT を環境変数から
 *   消し（学習者のプログラムが起動する子プロセスに差し込まない）、atexit で書き出しを登録する。
 * - 書き出しは exit のとき（main から戻ったときを含む）に、読み込まれたプロセスだけが、open・write で行う。
 *   書き出しの中で確保しない。_exit・abort・シグナルで終わったときは、atexit の関数が動かないので書かない。
 *
 * このファイルは Rust のクレートに include_str! で埋め込み、提出のたびに c-project/leakcheck/leakcheck.c に書き出して
 * `clang -dynamiclib -O2` でビルドする（spec/exercise-runner/c-project-assembly.md「ビルド」）。
 */

#include <fcntl.h>
#include <limits.h>
#include <mach-o/dyld.h>
#include <mach-o/getsect.h>
#include <mach-o/loader.h>
#include <sched.h>
#include <stdatomic.h>
#include <stddef.h>
#include <stdint.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>

#if defined(__has_feature)
#if __has_feature(ptrauth_returns)
#include <ptrauth.h>
#define PROGRUST_STRIP_RETURN_ADDRESS(p) ptrauth_strip((p), ptrauth_key_return_address)
#endif
#endif
#ifndef PROGRUST_STRIP_RETURN_ADDRESS
#define PROGRUST_STRIP_RETURN_ADDRESS(p) (p)
#endif

/* 結果のファイルのパスを渡す環境変数。 */
#define REPORT_ENV "PROGRUST_LEAK_REPORT"
/* ライブラリを差し込む環境変数。 */
#define INSERT_ENV "DYLD_INSERT_LIBRARIES"

/* ハッシュ表の枠の数（2^20）。記録できる数の 2 倍にして、探す長さを短く保つ。 */
#define TABLE_BITS 20
#define TABLE_SIZE ((size_t)1 << TABLE_BITS)
#define TABLE_MASK (TABLE_SIZE - 1)
/* 同時に記録できる領域の数（2^19 = 524,288）。 */
#define MAX_RECORDS ((size_t)1 << 19)

struct slot {
    uintptr_t addr; /* 0 は空の枠 */
    size_t size;
};

static struct slot table[TABLE_SIZE];
static size_t record_count;
static unsigned long long record_bytes;
static int overflowed;
static atomic_flag table_lock = ATOMIC_FLAG_INIT;

/* 学習者の実行ファイルの __TEXT セグメントの範囲。読み込まれる前は空なので、何も記録しない。 */
static uintptr_t text_start;
static uintptr_t text_end;

/* 結果のファイルの絶対パス。空なら書かない。 */
static char report_path[PATH_MAX];
/* ライブラリが読み込まれたプロセスのプロセス ID。 */
static pid_t loaded_pid;

static void lock_table(void) {
    while (atomic_flag_test_and_set_explicit(&table_lock, memory_order_acquire)) {
        sched_yield();
    }
}

static void unlock_table(void) {
    atomic_flag_clear_explicit(&table_lock, memory_order_release);
}

/*
 * fork の前後に鍵を取る手当て（pthread_atfork）はしない。libSystem は、fork の子の中で、pthread_atfork の子の
 * 関数を呼ぶ前に free を呼ぶので、鍵を持ったまま fork すると、子が鍵を待ち続ける。鍵を持つのは、表を変える間と、
 * realloc の間だけなので、ほかのスレッドがそのどちらかをしている最中に fork したときだけ、子が止まりうる。
 */

static size_t home_slot(uintptr_t addr) {
    /* 確保した領域のアドレスは 16 の倍数なので、下の 4 ビットを捨ててから混ぜる。 */
    uint64_t h = (uint64_t)(addr >> 4) * UINT64_C(0x9E3779B97F4A7C15);
    return (size_t)(h >> (64 - TABLE_BITS));
}

/* 鍵を持って呼ぶ。 */
static void record_locked(uintptr_t addr, size_t size) {
    size_t i = home_slot(addr);
    while (table[i].addr != 0) {
        if (table[i].addr == addr) {
            /* 記録が残っていた（置き換えていない道で解放された）ときは、新しい大きさにする。 */
            record_bytes -= table[i].size;
            record_bytes += size;
            table[i].size = size;
            return;
        }
        i = (i + 1) & TABLE_MASK;
    }
    if (record_count >= MAX_RECORDS) {
        overflowed = 1;
        return;
    }
    table[i].addr = addr;
    table[i].size = size;
    record_count += 1;
    record_bytes += size;
}

/* 鍵を持って呼ぶ。i の枠を消し、後ろに続く枠を詰める。 */
static void remove_at_locked(size_t i) {
    size_t j = i;
    for (;;) {
        j = (j + 1) & TABLE_MASK;
        if (table[j].addr == 0) {
            break;
        }
        size_t k = home_slot(table[j].addr);
        /* k が (i, j] の中にあれば、j の枠はそのままでよい。 */
        int stays = (i <= j) ? (i < k && k <= j) : (i < k || k <= j);
        if (stays) {
            continue;
        }
        table[i] = table[j];
        i = j;
    }
    table[i].addr = 0;
    table[i].size = 0;
}

/* 鍵を持って呼ぶ。 */
static void forget_locked(uintptr_t addr) {
    size_t i = home_slot(addr);
    while (table[i].addr != 0) {
        if (table[i].addr == addr) {
            record_count -= 1;
            record_bytes -= table[i].size;
            remove_at_locked(i);
            return;
        }
        i = (i + 1) & TABLE_MASK;
    }
}

static int from_learner(void *return_address) {
    uintptr_t a = (uintptr_t)PROGRUST_STRIP_RETURN_ADDRESS(return_address);
    return a >= text_start && a < text_end;
}

static void record(void *p, size_t size) {
    if (p == NULL) {
        return;
    }
    lock_table();
    record_locked((uintptr_t)p, size);
    unlock_table();
}

static void *hook_malloc(size_t size) {
    void *caller = __builtin_return_address(0);
    void *p = malloc(size);
    if (from_learner(caller)) {
        record(p, size);
    }
    return p;
}

static void *hook_calloc(size_t count, size_t size) {
    void *caller = __builtin_return_address(0);
    void *p = calloc(count, size);
    /* 確保できたなら、count * size はあふれていない。 */
    if (from_learner(caller)) {
        record(p, count * size);
    }
    return p;
}

static void *hook_realloc(void *old, size_t size) {
    void *caller = __builtin_return_address(0);
    /*
     * 鍵を持ったまま本物の realloc を呼ぶ。古い領域が解放されてから記録を消すまでの間に、ほかのスレッドが同じ
     * アドレスを確保して記録しても、その記録を消さないためである。本物の realloc は置き換えた関数を呼ばない。
     */
    lock_table();
    void *p = realloc(old, size);
    /* 失敗して NULL を返したときは、古い領域は残っているので、記録も残す。 */
    if ((p != NULL || size == 0) && old != NULL) {
        forget_locked((uintptr_t)old);
    }
    if (p != NULL && from_learner(caller)) {
        record_locked((uintptr_t)p, size);
    }
    unlock_table();
    return p;
}

static void hook_free(void *p) {
    if (p != NULL) {
        /* 解放する前に記録を消す。解放した後だと、ほかのスレッドが同じアドレスを確保して記録しうる。 */
        lock_table();
        forget_locked((uintptr_t)p);
        unlock_table();
    }
    free(p);
}

static char *hook_strdup(const char *s) {
    void *caller = __builtin_return_address(0);
    /* 本物の strdup の中の malloc は、呼び出し元が標準ライブラリなので記録されない。 */
    char *p = strdup(s);
    if (p != NULL && from_learner(caller)) {
        record(p, strlen(p) + 1);
    }
    return p;
}

struct interpose {
    const void *replacement;
    const void *replacee;
};

#define PROGRUST_INTERPOSE(replacement, replacee)                                                   \
    __attribute__((used)) static const struct interpose interpose_##replacee                         \
        __attribute__((section("__DATA,__interpose"))) = {(const void *)(uintptr_t)&replacement,     \
                                                           (const void *)(uintptr_t)&replacee}

PROGRUST_INTERPOSE(hook_malloc, malloc);
PROGRUST_INTERPOSE(hook_calloc, calloc);
PROGRUST_INTERPOSE(hook_realloc, realloc);
PROGRUST_INTERPOSE(hook_free, free);
PROGRUST_INTERPOSE(hook_strdup, strdup);

/* 10 進で buf に書き、書いた長さを返す。buf は 20 バイト以上。 */
static size_t format_decimal(char *buf, unsigned long long value) {
    char digits[20];
    size_t n = 0;
    do {
        digits[n++] = (char)('0' + (value % 10));
        value /= 10;
    } while (value != 0);
    for (size_t i = 0; i < n; i++) {
        buf[i] = digits[n - 1 - i];
    }
    return n;
}

static void write_all(int fd, const char *buf, size_t len) {
    while (len > 0) {
        ssize_t written = write(fd, buf, len);
        if (written <= 0) {
            return;
        }
        buf += written;
        len -= (size_t)written;
    }
}

static void write_report(void) {
    /* fork で作った子プロセスは書かない。 */
    if (getpid() != loaded_pid || report_path[0] == '\0') {
        return;
    }
    char buf[64];
    size_t len = 0;
    lock_table();
    if (overflowed) {
        memcpy(buf, "overflow\n", 9);
        len = 9;
    } else {
        len += format_decimal(buf + len, (unsigned long long)record_count);
        buf[len++] = '\n';
        len += format_decimal(buf + len, record_bytes);
        buf[len++] = '\n';
    }
    unlock_table();
    int fd = open(report_path, O_WRONLY | O_CREAT | O_TRUNC | O_CLOEXEC, 0644);
    if (fd < 0) {
        return;
    }
    write_all(fd, buf, len);
    close(fd);
}

/* 学習者の実行ファイルの __TEXT セグメントの範囲を覚える。 */
static void find_learner_text(void) {
    uint32_t count = _dyld_image_count();
    for (uint32_t i = 0; i < count; i++) {
        const struct mach_header *header = _dyld_get_image_header(i);
        if (header == NULL || header->filetype != MH_EXECUTE) {
            continue;
        }
        unsigned long size = 0;
        uint8_t *start = getsegmentdata((const struct mach_header_64 *)header, "__TEXT", &size);
        if (start != NULL && size > 0) {
            text_start = (uintptr_t)start;
            text_end = (uintptr_t)start + size;
        }
        return;
    }
}

/* 結果のファイルのパスを、読み込まれたときの作業ディレクトリをもとに絶対パスにして覚える。 */
static void remember_report_path(const char *path) {
    size_t path_len = strlen(path);
    if (path_len == 0) {
        return;
    }
    if (path[0] == '/') {
        if (path_len < sizeof(report_path)) {
            memcpy(report_path, path, path_len + 1);
        }
        return;
    }
    char cwd[PATH_MAX];
    if (getcwd(cwd, sizeof(cwd)) == NULL) {
        return;
    }
    size_t cwd_len = strlen(cwd);
    if (cwd_len + 1 + path_len >= sizeof(report_path)) {
        return;
    }
    memcpy(report_path, cwd, cwd_len);
    report_path[cwd_len] = '/';
    memcpy(report_path + cwd_len + 1, path, path_len + 1);
}

__attribute__((constructor)) static void progrust_leakcheck_init(void) {
    const char *path = getenv(REPORT_ENV);
    if (path != NULL) {
        remember_report_path(path);
    }
    unsetenv(INSERT_ENV);
    unsetenv(REPORT_ENV);
    loaded_pid = getpid();
    find_learner_text();
    if (report_path[0] != '\0') {
        atexit(write_report);
    }
}
