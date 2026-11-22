#!/usr/bin/env python3
"""実装のループを回す。

GitHub の Issue（ラベル `実装タスク`）を 1 件ずつ選び、`claude -p` に実装とレビューを
させ、機械のチェックと CI が通ったら PR をマージする。人が見ていなくても進むように、
決まった手順（タスク選び、ブランチ、チェック、コミット、push、PR、マージ）はこの
スクリプトが行い、AI には実装・レビュー・仕分けだけをさせる。

使い方:
    python3 tools/implement-loop.py [--dry-run] [--status] [--max-tasks N]
                                    [--worktree PATH] [--log-dir PATH]

    --dry-run     選ぶタスクと実行するコマンドを表示するだけで、何も変えない
    --status      行き詰まり・人の確認待ち・未判断・残りのタスクを表示する（gh だけを使う）
    --max-tasks N タスクを N 件扱ったら止まる
    --worktree    ループ専用の作業ツリーの場所（既定はリポジトリの隣の「リポジトリ名-loop」）
    --log-dir     各呼び出しの出力を残す場所（既定は ~/Library/Logs/progrust-implement-loop/）

終了コード:
    0    選べるタスクが無くなった、または --max-tasks に達した
    1    行き詰まりが続いたので止めた
    2    仕組みの失敗（claude・gh・git の失敗、CI のチェックが報告されない、前提の不足）で止めた
    130  Ctrl-C で止めた

標準ライブラリだけを使う。Python 3.9 以降で動く。
"""

from __future__ import annotations

import argparse
import datetime
import fcntl
import hashlib
import json
import os
import re
import shlex
import shutil
import subprocess
import sys
import time
from typing import Any, Dict, List, Optional, Set, Tuple

REPO_ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))

REMOTE = "origin"
MAIN_BRANCH = "main"
MAIN_REF = f"{REMOTE}/{MAIN_BRANCH}"

# ラベル
LABEL_TASK = "実装タスク"
LABEL_STUCK = "行き詰まり"
LABEL_EXCLUDED = "ループ対象外"
LABEL_HOLD = "保留"
LABEL_AI_FILED = "AI起票"
LABEL_UNDECIDED = "未判断"
LABEL_HUMAN_CHECK = "人の確認待ち"
DIFFICULTY_LABELS = {"難易度:小": "小", "難易度:中": "中", "難易度:大": "大"}
DIFFICULTY_ORDER = ("小", "中", "大")

# 難易度とモデルの対応（escalate は、同じモデルで直せなかったときの引き上げ先）
MODELS: Dict[str, Dict[str, Optional[str]]] = {
    "小": {"implement": "haiku", "review": "sonnet", "escalate": "sonnet"},
    "中": {"implement": "sonnet", "review": "sonnet", "escalate": "opus"},
    "大": {"implement": "opus", "review": "opus", "escalate": None},
}
TRIAGE_MODEL = "sonnet"

# 直しの回数
MAX_SAME_MODEL_FIXES = 2
MAX_ESCALATED_FIXES = 1
MAX_CONSECUTIVE_STUCK = 3

# スキルの呼び方
# スキルの呼び方（{number} は Issue の番号、{numbers} は空白で区切った番号）
PROMPT_IMPLEMENT = "/implement-task {number} --loop"
PROMPT_FIX = "/implement-task {number} --loop fix\n\n{findings}"
PROMPT_REVIEW = "/review-task {number} --loop"
PROMPT_TRIAGE = "/triage-issues --loop {numbers}"

# 権限の設定（main にあるものを使う）
SETTINGS_IMPLEMENT = ".claude/loop-settings.json"
SETTINGS_REVIEW = ".claude/loop-review-settings.json"
SETTINGS_TRIAGE = ".claude/loop-triage-settings.json"
PERMISSION_MODE = "dontAsk"

# 始める前に main に無ければならないファイル
REQUIRED_FILES = (
    SETTINGS_IMPLEMENT,
    SETTINGS_REVIEW,
    SETTINGS_TRIAGE,
    ".claude/skills/implement-task/SKILL.md",
    ".claude/skills/review-task/SKILL.md",
    ".claude/skills/triage-issues/SKILL.md",
)
WORKFLOWS_DIR = ".github/workflows"

# ループが変えてはならないパス（末尾が / のものはディレクトリ）
PROTECTED_PATHS = (
    "CLAUDE.md",
    ".claude/",
    ".github/",
    "documents/conventions/",
    "tools/check-documents.py",
    "tools/design-system.py",
    "tools/implement-loop.py",
    "tools/okf/",
    "tools/tests/test_implement_loop.py",
)

# 機械のチェック（requires のファイルがあるときだけ実行する）
CHECKS: Tuple[Dict[str, Any], ...] = (
    {"name": "documents の検査", "argv": ["python3", "tools/check-documents.py"], "requires": "tools/check-documents.py"},
    {"name": "デザインシステムの検査", "argv": ["python3", "tools/design-system.py", "check"], "requires": "tools/design-system.py"},
    {"name": "npm run check", "argv": ["npm", "run", "check"], "requires": "package.json"},
)

# 時間切れ（秒）
IMPLEMENT_TIMEOUT = 90 * 60
REVIEW_TIMEOUT = 30 * 60
TRIAGE_TIMEOUT = 60 * 60
CHECK_TIMEOUT = 30 * 60
NPM_CI_TIMEOUT = 15 * 60
GIT_TIMEOUT = 5 * 60
GH_TIMEOUT = 2 * 60
CI_INITIAL_WAIT = 10
CI_APPEAR_TIMEOUT = 5 * 60
CI_TIMEOUT = 45 * 60
CI_POLL_SECONDS = 30

# 指摘やコメントに載せる出力の長さ
TAIL_LINES = 150
TAIL_CHARS = 8000
COMMENT_CHARS = 20000

NPM_LOCK_MARKER = os.path.join("node_modules", ".implement-loop-lock-hash")
CO_AUTHOR = "Co-Authored-By: Claude <noreply@anthropic.com>"
PR_FOOTER = "🤖 Generated with [Claude Code](https://claude.com/claude-code)"

IMPLEMENT_SCHEMA = {
    "type": "object",
    "properties": {
        "status": {"type": "string", "enum": ["done", "blocked"]},
        "pr_title": {"type": "string"},
        "summary": {"type": "string"},
        "fixed_problems": {"type": "array", "items": {"type": "string"}},
        "human_checks": {"type": "array", "items": {"type": "string"}},
        "blocked_reason": {"type": "string"},
        "blocking_issues": {"type": "array", "items": {"type": "integer"}},
    },
    "required": [
        "status",
        "pr_title",
        "summary",
        "fixed_problems",
        "human_checks",
        "blocked_reason",
        "blocking_issues",
    ],
    "additionalProperties": False,
}

REVIEW_SCHEMA = {
    "type": "object",
    "properties": {
        "verdict": {"type": "string", "enum": ["pass", "fail"]},
        "findings": {
            "type": "array",
            "items": {
                "type": "object",
                "properties": {
                    "severity": {"type": "string", "enum": ["must", "note"]},
                    "file": {"type": "string"},
                    "description": {"type": "string"},
                },
                "required": ["severity", "file", "description"],
                "additionalProperties": False,
            },
        },
        "summary": {"type": "string"},
    },
    "required": ["verdict", "findings", "summary"],
    "additionalProperties": False,
}

PREREQ_RE = re.compile(r"^- 前提[:：][ \t]*(.*)$", re.MULTILINE)
ISSUE_REF_RE = re.compile(r"#(\d+)")
HUMAN_CHECK_HEADING_RE = re.compile(r"^##[ \t]+人が確かめること[ \t]*$", re.MULTILINE)
NEXT_HEADING_RE = re.compile(r"^##[ \t]+", re.MULTILINE)
PR_URL_RE = re.compile(r"/pull/(\d+)")
RUN_LINK_RE = re.compile(r"/actions/runs/(\d+)")


class InfraError(Exception):
    """仕組みの失敗。タスクにラベルを付けずに、ループを止める。"""


class Task:
    def __init__(self, raw: Dict[str, Any]) -> None:
        self.number: int = int(raw["number"])
        self.title: str = raw.get("title") or ""
        self.body: str = raw.get("body") or ""
        self.labels: Set[str] = {label["name"] for label in raw.get("labels") or []}
        milestone = raw.get("milestone") or {}
        self.milestone: str = milestone.get("title") or ""

    @property
    def difficulty(self) -> Optional[str]:
        found = [DIFFICULTY_LABELS[name] for name in self.labels if name in DIFFICULTY_LABELS]
        if not found:
            return None
        # 複数あるときは、難しいほうにそろえる
        return max(found, key=DIFFICULTY_ORDER.index)

    @property
    def ai_filed(self) -> bool:
        return LABEL_AI_FILED in self.labels

    def skip_reason(self) -> Optional[str]:
        """ループが選ばない理由。選べるなら None。前提はここでは見ない。"""
        for label in (LABEL_STUCK, LABEL_EXCLUDED, LABEL_HOLD):
            if label in self.labels:
                return f"ラベル `{label}` が付いている"
        if self.difficulty is None:
            return "難易度のラベルが無い"
        return None

    def sort_key(self) -> Tuple[int, int, str, int]:
        if self.ai_filed:
            return (0, 0, "", self.number)
        return (1, 0 if self.milestone else 1, self.milestone, self.number)

    def describe(self) -> str:
        return f"#{self.number} [難易度:{self.difficulty or '?'}] {self.title}"


def parse_prerequisites(body: str) -> Optional[List[int]]:
    """本文の「- 前提: #12, #13」の行を読む。行が無ければ None、「なし」なら空のリスト。"""
    match = PREREQ_RE.search(body or "")
    if not match:
        return None
    return [int(n) for n in ISSUE_REF_RE.findall(match.group(1))]


def add_prerequisites(body: str, numbers: List[int]) -> Optional[str]:
    """本文の前提の行に番号を足す。行が無ければ None。"""
    match = PREREQ_RE.search(body or "")
    if not match:
        return None
    current = [int(n) for n in ISSUE_REF_RE.findall(match.group(1))]
    merged = current + [n for n in numbers if n not in current]
    line = "- 前提: " + ", ".join(f"#{n}" for n in merged)
    return body[: match.start()] + line + body[match.end():]


def human_check_items(body: str) -> List[str]:
    """本文の「## 人が確かめること」の節の項目。無いか「なし」なら空。"""
    match = HUMAN_CHECK_HEADING_RE.search(body or "")
    if not match:
        return []
    rest = body[match.end():]
    end = NEXT_HEADING_RE.search(rest)
    section = rest[: end.start()] if end else rest
    items = []
    for line in section.splitlines():
        text = line.strip()
        text = re.sub(r"^[-*][ \t]+(\[[ xX]\][ \t]+)?", "", text).strip()
        if text and text.rstrip("。.") != "なし":
            items.append(text)
    return items


def merge_unique(base: List[str], extra: List[str]) -> List[str]:
    out = list(base)
    for item in extra:
        item = (item or "").strip()
        if item and item not in out:
            out.append(item)
    return out


def is_protected(path: str) -> bool:
    for protected in PROTECTED_PATHS:
        if protected.endswith("/"):
            if path.startswith(protected):
                return True
        elif path == protected:
            return True
    return False


def tail(text: str) -> str:
    lines = (text or "").splitlines()[-TAIL_LINES:]
    out = "\n".join(lines)
    return out[-TAIL_CHARS:]


def one_line(text: str) -> str:
    return " ".join((text or "").split())


def today() -> str:
    return datetime.date.today().isoformat()


def bullet_list(items: List[str]) -> str:
    return "\n".join(f"- {one_line(item)}" for item in items)


class Loop:
    def __init__(self, args: argparse.Namespace) -> None:
        self.repo: str = os.path.abspath(args.repo)
        self.worktree: str = os.path.abspath(
            args.worktree
            or os.path.join(os.path.dirname(self.repo), os.path.basename(self.repo) + "-loop")
        )
        self.log_root: str = os.path.abspath(
            os.path.expanduser(args.log_dir or "~/Library/Logs/progrust-implement-loop")
        )
        self.max_tasks: Optional[int] = args.max_tasks
        self.dry_run: bool = args.dry_run
        self.run_dir: Optional[str] = None
        self.log_file: Optional[str] = None
        self.call_index = 0
        self.lock_handle: Any = None
        self.attempted: Set[int] = set()
        self.triaged: Set[int] = set()
        self.merged: List[str] = []
        self.stuck: List[str] = []
        self.blocked: List[str] = []
        self.consecutive_stuck = 0
        self.stop_reason = ""

    # ---- 出力と記録 ----

    def say(self, message: str) -> None:
        line = f"[{datetime.datetime.now().strftime('%H:%M:%S')}] {message}"
        print(line, flush=True)
        if self.log_file:
            with open(self.log_file, "a", encoding="utf-8") as handle:
                handle.write(line + "\n")

    def save(self, name: str, text: str) -> str:
        assert self.run_dir is not None
        self.call_index += 1
        path = os.path.join(self.run_dir, f"{self.call_index:03d}-{name}")
        with open(path, "w", encoding="utf-8") as handle:
            handle.write(text)
        return path

    def open_logs(self) -> None:
        stamp = datetime.datetime.now().strftime("%Y%m%d-%H%M%S")
        self.run_dir = os.path.join(self.log_root, stamp)
        os.makedirs(self.run_dir, exist_ok=True)
        self.log_file = os.path.join(self.run_dir, "loop.log")

    # ---- コマンドの実行 ----

    def run(
        self,
        argv: List[str],
        cwd: str,
        timeout: int,
        stdin: Optional[str] = None,
    ) -> subprocess.CompletedProcess:
        # 標準入力を渡さないときは、端末や親の標準入力を待たないように閉じておく
        source: Dict[str, Any] = {"input": stdin} if stdin is not None else {"stdin": subprocess.DEVNULL}
        try:
            return subprocess.run(
                argv,
                cwd=cwd,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                text=True,
                encoding="utf-8",
                errors="replace",
                timeout=timeout,
                **source,
            )
        except subprocess.TimeoutExpired as error:
            out = error.stdout if isinstance(error.stdout, str) else ""
            err = error.stderr if isinstance(error.stderr, str) else ""
            return subprocess.CompletedProcess(argv, 124, out, err + f"\n（{timeout} 秒で時間切れ）")
        except FileNotFoundError:
            return subprocess.CompletedProcess(argv, 127, "", f"`{argv[0]}` が見つかりません。")

    def gh(self, args: List[str], stdin: Optional[str] = None, check: bool = True) -> subprocess.CompletedProcess:
        result = self.run(["gh"] + args, self.repo, GH_TIMEOUT, stdin)
        if check and result.returncode != 0:
            raise InfraError(f"`gh {' '.join(args[:3])}` が失敗しました: {tail(result.stderr).strip()}")
        return result

    def gh_json(self, args: List[str]) -> Any:
        result = self.gh(args)
        try:
            return json.loads(result.stdout)
        except ValueError:
            raise InfraError(f"`gh {' '.join(args[:3])}` の出力を JSON として読めません。")

    def git(self, args: List[str], cwd: Optional[str] = None, check: bool = True) -> subprocess.CompletedProcess:
        result = self.run(["git"] + args, cwd or self.worktree, GIT_TIMEOUT)
        if check and result.returncode != 0:
            raise InfraError(f"`git {' '.join(args[:3])}` が失敗しました: {tail(result.stderr).strip()}")
        return result

    # ---- Issue ----

    def list_tasks(self) -> List[Task]:
        raw = self.gh_json(
            [
                "issue", "list", "--state", "open", "--label", LABEL_TASK,
                "--limit", "500", "--json", "number,title,labels,milestone,body",
            ]
        )
        return [Task(item) for item in raw]

    def issue_closed(self, number: int, cache: Dict[int, bool]) -> bool:
        if number not in cache:
            result = self.gh(["issue", "view", str(number), "--json", "state"], check=False)
            closed = False
            if result.returncode == 0:
                try:
                    closed = json.loads(result.stdout).get("state") == "CLOSED"
                except ValueError:
                    closed = False
            cache[number] = closed
        return cache[number]

    def classify(self, tasks: List[Task]) -> Tuple[List[Task], List[Tuple[Task, str]]]:
        """選べるタスク（選ぶ順）と、選べないタスクとその理由。"""
        ready: List[Task] = []
        skipped: List[Tuple[Task, str]] = []
        cache: Dict[int, bool] = {}
        for task in sorted(tasks, key=Task.sort_key):
            reason = task.skip_reason()
            if reason is None and task.number in self.attempted:
                reason = "この実行で既に扱った"
            if reason is None:
                waiting = [n for n in parse_prerequisites(task.body) or [] if not self.issue_closed(n, cache)]
                if waiting:
                    reason = "前提の " + "、".join(f"#{n}" for n in waiting) + " が閉じていない"
            if reason is None:
                ready.append(task)
            else:
                skipped.append((task, reason))
        return ready, skipped

    def untriaged_issues(self) -> List[int]:
        raw = self.gh_json(
            [
                "issue", "list", "--state", "open", "--label", LABEL_AI_FILED,
                "--limit", "200", "--json", "number,labels",
            ]
        )
        # 開いているタスクが「前提」で待っている Issue は、ユーザーの判断を求めるものなので、
        # ループでは仕分けない（ユーザーが決める前に、閉じたりタスクにしたりしないため）。
        awaited: Set[int] = set()
        for task in self.list_tasks():
            awaited.update(parse_prerequisites(task.body) or [])
        numbers = []
        for item in raw:
            labels = {label["name"] for label in item.get("labels") or []}
            if LABEL_HOLD in labels or LABEL_TASK in labels:
                continue
            if int(item["number"]) in self.triaged or int(item["number"]) in awaited:
                continue
            numbers.append(int(item["number"]))
        return sorted(numbers)

    def comment(self, number: int, body: str) -> None:
        self.gh(["issue", "comment", str(number), "--body-file", "-"], stdin=body[:COMMENT_CHARS * 3])

    # ---- 作業ツリー ----

    def common_dir(self, path: str) -> str:
        result = self.git(["rev-parse", "--path-format=absolute", "--git-common-dir"], cwd=path)
        return os.path.realpath(result.stdout.strip())

    def ensure_worktree(self) -> None:
        self.git(["fetch", REMOTE], cwd=self.repo)
        if not os.path.exists(self.worktree):
            self.say(f"ループ専用の作業ツリーを作る: {self.worktree}")
            self.git(["worktree", "add", "--detach", self.worktree, MAIN_REF], cwd=self.repo)
        else:
            check = self.git(["rev-parse", "--path-format=absolute", "--git-common-dir"], cwd=self.worktree, check=False)
            if check.returncode != 0 or os.path.realpath(check.stdout.strip()) != self.common_dir(self.repo):
                raise InfraError(f"`{self.worktree}` は、このリポジトリの作業ツリーではありません。")
        if os.path.realpath(self.worktree) == os.path.realpath(self.repo):
            raise InfraError("作業ツリーに、リポジトリ自身を指定できません。")
        self.reset_worktree()

    def reset_worktree(self) -> None:
        """作業ツリーの変更を捨てて、main の先頭に戻す。"""
        self.git(["merge", "--abort"], check=False)
        self.git(["reset", "--hard", "--quiet"])
        self.git(["clean", "-fd", "--quiet"])
        self.git(["clean", "-fdx", "--quiet", "--", ".claude"])
        self.git(["checkout", "--quiet", "--detach", MAIN_REF])

    def discard_if_dirty(self, when: str) -> None:
        status = self.git(["status", "--porcelain"]).stdout.strip()
        if status:
            self.say(f"警告: {when}に作業ツリーが変わっていたので、変更を捨てた。\n{status}")
            self.git(["reset", "--hard", "--quiet"])
            self.git(["clean", "-fd", "--quiet"])

    def preflight_files(self) -> None:
        missing = [path for path in REQUIRED_FILES if not os.path.isfile(os.path.join(self.worktree, path))]
        if missing:
            raise InfraError(
                f"`{MAIN_REF}` に、ループに要るファイルがありません: " + "、".join(f"`{p}`" for p in missing)
            )
        workflows = os.path.join(self.worktree, WORKFLOWS_DIR)
        has_ci = os.path.isdir(workflows) and any(
            name.endswith((".yml", ".yaml")) for name in os.listdir(workflows)
        )
        if not has_ci:
            raise InfraError(
                f"`{MAIN_REF}` の `{WORKFLOWS_DIR}/` に CI の定義がありません。CI が通ったことを確かめられないので、始めません。"
            )

    def settings_copy(self, path: str) -> str:
        """権限の設定を main から取り出して、作業ツリーの外に置く。"""
        assert self.run_dir is not None
        content = self.git(["show", f"{MAIN_REF}:{path}"]).stdout
        target = os.path.join(self.run_dir, os.path.basename(path))
        with open(target, "w", encoding="utf-8") as handle:
            handle.write(content)
        return target

    def ensure_node_modules(self) -> Optional[str]:
        """package-lock.json が変わっていたら npm ci を実行する。失敗したら出力を返す。"""
        lock = os.path.join(self.worktree, "package-lock.json")
        if not os.path.isfile(os.path.join(self.worktree, "package.json")) or not os.path.isfile(lock):
            return None
        with open(lock, "rb") as handle:
            digest = hashlib.sha256(handle.read()).hexdigest()
        marker = os.path.join(self.worktree, NPM_LOCK_MARKER)
        if os.path.isfile(marker):
            with open(marker, encoding="utf-8") as handle:
                if handle.read().strip() == digest:
                    return None
        self.say("npm ci を実行する")
        result = self.run(["npm", "ci"], self.worktree, NPM_CI_TIMEOUT)
        self.save("npm-ci.txt", result.stdout + result.stderr)
        if result.returncode != 0:
            return tail(result.stdout + result.stderr)
        with open(marker, "w", encoding="utf-8") as handle:
            handle.write(digest)
        return None

    # ---- claude ----

    def claude_argv(
        self,
        prompt: str,
        model: str,
        settings: str,
        schema: Optional[Dict[str, Any]],
        resume: Optional[str],
    ) -> List[str]:
        argv = [
            "claude", "-p", prompt,
            "--model", model,
            "--permission-mode", PERMISSION_MODE,
            "--settings", settings,
            "--output-format", "json",
        ]
        if schema is not None:
            argv += ["--json-schema", json.dumps(schema, ensure_ascii=False)]
        if resume:
            argv += ["--resume", resume]
        return argv

    def call_claude(
        self,
        name: str,
        prompt: str,
        model: str,
        settings_path: str,
        schema: Optional[Dict[str, Any]],
        timeout: int,
        resume: Optional[str] = None,
    ) -> Dict[str, Any]:
        settings = self.settings_copy(settings_path)
        argv = self.claude_argv(prompt, model, settings, schema, resume)
        self.say(f"claude を呼ぶ: {name}（{model}{'、会話の続き' if resume else ''}）")
        result = self.run(argv, self.worktree, timeout)
        self.save(f"{name}.json", result.stdout)
        if result.stderr.strip():
            self.save(f"{name}.stderr.txt", result.stderr)
        data: Any = None
        try:
            data = json.loads(result.stdout)
        except ValueError:
            data = None
        if isinstance(data, list):
            results = [item for item in data if isinstance(item, dict) and item.get("type") == "result"]
            data = results[-1] if results else None
        detail = ""
        if isinstance(data, dict):
            detail = one_line(str(data.get("result") or ""))[:300]
            if data.get("api_error_status"):
                detail = f"API のエラー {data['api_error_status']}: {detail}"
        if result.returncode != 0:
            raise InfraError(
                f"claude（{name}）が終了コード {result.returncode} で終わりました。{detail or tail(result.stderr).strip()[:300]}"
            )
        if not isinstance(data, dict):
            raise InfraError(f"claude（{name}）の出力を JSON として読めません。")
        if data.get("is_error"):
            raise InfraError(f"claude（{name}）がエラーを返しました。{detail}")
        if schema is not None:
            structured = data.get("structured_output")
            if not isinstance(structured, dict):
                try:
                    structured = json.loads(data.get("result") or "")
                except ValueError:
                    structured = None
            if not isinstance(structured, dict):
                raise InfraError(f"claude（{name}）が、決まった形の結果を返しませんでした。")
            data["structured_output"] = structured
        denials = data.get("permission_denials") or []
        if denials:
            # 許可の一覧（.claude/loop-*.json）を見直すときの手がかりにする。
            self.say(f"  権限で拒否された操作: {len(denials)} 件")
            for denial in denials[:10]:
                tool_input = denial.get("tool_input") or {}
                what = tool_input.get("command") or tool_input.get("file_path") or ""
                self.say(f"    - {denial.get('tool_name')}: {str(what)[:120]}")
        return data

    def call_implement(self, task: Task, model: str, findings: str, resume: Optional[str], step: int) -> Dict[str, Any]:
        if findings:
            prompt = PROMPT_FIX.format(number=task.number, findings=findings)
        else:
            prompt = PROMPT_IMPLEMENT.format(number=task.number)
        name = f"task{task.number}-implement{step}"
        try:
            data = self.call_claude(name, prompt, model, SETTINGS_IMPLEMENT, IMPLEMENT_SCHEMA, IMPLEMENT_TIMEOUT, resume)
        except InfraError:
            if not resume:
                raise
            # 会話の続きで失敗したときは、新しい会話で 1 回だけやり直す
            self.say("  会話の続きで失敗したので、新しい会話でやり直す")
            data = self.call_claude(name + "-retry", prompt, model, SETTINGS_IMPLEMENT, IMPLEMENT_SCHEMA, IMPLEMENT_TIMEOUT)
        structured = data["structured_output"]
        if structured.get("status") not in ("done", "blocked"):
            raise InfraError(f"claude（{name}）の結果の status が `done` でも `blocked` でもありません。")
        return data

    # ---- 1 タスクの流れ ----

    def start_branch(self, branch: str) -> None:
        self.git(["fetch", REMOTE])
        self.reset_worktree()
        self.git(["checkout", "--quiet", "-B", branch, MAIN_REF])
        failure = self.ensure_node_modules()
        if failure:
            raise InfraError(f"`{MAIN_REF}` の状態で npm ci が失敗しました:\n{failure}")

    def commit_all(self, message: str) -> Optional[str]:
        """変更をすべてコミットする。コミットに失敗したら出力を返す。"""
        self.git(["add", "-A"])
        if self.git(["diff", "--cached", "--quiet"], check=False).returncode == 0:
            return None
        result = self.git(["commit", "--quiet", "-m", message], check=False)
        if result.returncode != 0:
            return tail(result.stdout + result.stderr)
        return None

    def changed_paths(self) -> List[str]:
        out = self.git(["diff", "--name-only", f"{MAIN_REF}...HEAD"]).stdout
        return [line for line in out.splitlines() if line]

    def revert_protected(self, paths: List[str]) -> None:
        for path in paths:
            exists = self.git(["cat-file", "-e", f"{MAIN_REF}:{path}"], check=False).returncode == 0
            if exists:
                self.git(["checkout", MAIN_REF, "--", path])
            else:
                self.git(["rm", "-f", "--quiet", "--", path])
        failure = self.commit_all("保護するパスの変更を戻す")
        if failure:
            raise InfraError(f"保護するパスの変更を戻すコミットに失敗しました:\n{failure}")

    def run_checks(self, task: Task, step: int) -> List[str]:
        findings = []
        failure = self.ensure_node_modules()
        if failure:
            return [f"## 機械のチェックの失敗: npm ci\n\n```\n{failure}\n```"]
        for check in CHECKS:
            if not os.path.exists(os.path.join(self.worktree, check["requires"])):
                continue
            self.say(f"機械のチェック: {check['name']}")
            result = self.run(check["argv"], self.worktree, CHECK_TIMEOUT)
            output = result.stdout + result.stderr
            self.save(f"task{task.number}-check{step}-{check['argv'][-1].replace('/', '_')}.txt", output)
            if result.returncode != 0:
                findings.append(
                    f"## 機械のチェックの失敗: {check['name']}\n\n"
                    f"`{' '.join(check['argv'])}` が終了コード {result.returncode} で終わった。出力の末尾:\n\n"
                    f"```\n{tail(output)}\n```"
                )
        self.discard_if_dirty("機械のチェックの後")
        return findings

    def review(self, task: Task, model: str, step: int) -> Tuple[List[Dict[str, str]], List[Dict[str, str]]]:
        """レビューを呼び、must の指摘と note の指摘を返す。"""
        data = self.call_claude(
            f"task{task.number}-review{step}",
            PROMPT_REVIEW.format(number=task.number),
            model,
            SETTINGS_REVIEW,
            REVIEW_SCHEMA,
            REVIEW_TIMEOUT,
        )
        self.discard_if_dirty("レビューの後")
        structured = data["structured_output"]
        findings = [f for f in structured.get("findings") or [] if isinstance(f, dict)]
        must = [f for f in findings if f.get("severity") == "must"]
        notes = [f for f in findings if f.get("severity") != "must"]
        if structured.get("verdict") == "fail" and not must:
            must = [{"severity": "must", "file": "", "description": structured.get("summary") or "レビューが fail を返した。"}]
        return must, notes

    def find_pr(self, branch: str) -> Optional[int]:
        raw = self.gh_json(["pr", "list", "--head", branch, "--state", "open", "--json", "number", "--limit", "1"])
        return int(raw[0]["number"]) if raw else None

    def publish(self, branch: str, title: str, body: str) -> int:
        """push して、PR を作る（既にあれば本文を直す）。PR の番号を返す。"""
        push = self.git(["push", "--force", REMOTE, f"{branch}:{branch}"], check=False)
        if push.returncode != 0:
            raise InfraError(f"`git push` が失敗しました: {tail(push.stderr).strip()}")
        number = self.find_pr(branch)
        if number is not None:
            self.gh(["pr", "edit", str(number), "--title", title, "--body-file", "-"], stdin=body)
            return number
        created = self.gh(
            ["pr", "create", "--base", MAIN_BRANCH, "--head", branch, "--title", title, "--body-file", "-"],
            stdin=body,
        )
        match = PR_URL_RE.search(created.stdout)
        if match:
            return int(match.group(1))
        number = self.find_pr(branch)
        if number is None:
            raise InfraError("作った PR の番号が分かりません。")
        return number

    def wait_for_ci(self, pr: int) -> List[str]:
        """CI が終わるのを待つ。失敗したら指摘を返す。チェックが報告されなければ止める。"""
        self.say(f"PR #{pr} の CI を待つ")
        time.sleep(CI_INITIAL_WAIT)
        started = time.monotonic()
        while True:
            result = self.gh(["pr", "checks", str(pr), "--json", "name,state,bucket,link"], check=False)
            checks: List[Dict[str, Any]] = []
            if result.stdout.strip():
                try:
                    checks = json.loads(result.stdout)
                except ValueError:
                    raise InfraError(f"`gh pr checks` の出力を JSON として読めません: {tail(result.stderr).strip()}")
            elif result.returncode != 0 and "no checks reported" not in result.stderr:
                raise InfraError(f"`gh pr checks` が失敗しました: {tail(result.stderr).strip()}")
            elapsed = time.monotonic() - started
            if not checks:
                if elapsed >= CI_APPEAR_TIMEOUT:
                    raise InfraError(
                        f"PR #{pr} に CI のチェックが 1 つも報告されません。確かめられないので、マージせずに止めます。PR は開いたままです。"
                    )
            else:
                failed = [c for c in checks if c.get("bucket") in ("fail", "cancel")]
                pending = [c for c in checks if c.get("bucket") == "pending"]
                if failed:
                    return [self.ci_finding(failed)]
                if not pending:
                    return []
                if elapsed >= CI_TIMEOUT:
                    raise InfraError(f"PR #{pr} の CI が {CI_TIMEOUT} 秒たっても終わりません。PR は開いたままです。")
            time.sleep(CI_POLL_SECONDS)

    def ci_finding(self, failed: List[Dict[str, Any]]) -> str:
        lines = ["## CI の失敗", ""]
        run_ids: List[str] = []
        for check in failed:
            lines.append(f"- {check.get('name')}: {check.get('state')}（{check.get('link') or 'リンクなし'}）")
            match = RUN_LINK_RE.search(check.get("link") or "")
            if match and match.group(1) not in run_ids:
                run_ids.append(match.group(1))
        for run_id in run_ids[:2]:
            log = self.gh(["run", "view", run_id, "--log-failed"], check=False)
            if log.returncode == 0 and log.stdout.strip():
                lines += ["", f"失敗したジョブの出力の末尾（run {run_id}）:", "", "```", tail(log.stdout), "```"]
        return "\n".join(lines)

    def pr_body(
        self,
        task: Task,
        summary: str,
        fixed: List[str],
        human: List[str],
        notes: List[Dict[str, str]],
        history: List[str],
    ) -> str:
        parts = [f"Closes #{task.number}", "", "## 要約", "", summary.strip() or "（要約なし）"]
        if fixed:
            parts += ["", "## 起票せずに直した問題", "", bullet_list(fixed)]
        if human:
            parts += ["", "## 人が確かめること（実装中に足したもの）", "", bullet_list(human)]
        if notes:
            parts += ["", "## レビューの補足", ""]
            parts += [f"- {('`' + n['file'] + '`: ') if n.get('file') else ''}{one_line(n.get('description', ''))}" for n in notes]
        parts += ["", "## 経過", "", bullet_list(history), "", PR_FOOTER]
        return "\n".join(parts) + "\n"

    def run_task(self, task: Task) -> str:
        """1 タスクを扱う。'merged'・'stuck'・'blocked' のどれかを返す。"""
        self.say(f"タスクを始める: {task.describe()}")
        history: List[str] = []
        outcome = self.attempt(task, history, allow_restart=True)
        if outcome == "conflict":
            # main の取り込みで衝突したら、作業を捨てて、新しい main から 1 回だけやり直す
            history.append(f"`{MAIN_REF}` の取り込みで衝突したので、作業を捨てて、新しい `{MAIN_REF}` から最初からやり直した")
            self.say(f"  `{MAIN_REF}` と衝突したので、最初からやり直す")
            outcome = self.attempt(task, history, allow_restart=False)
        return outcome

    def attempt(self, task: Task, history: List[str], allow_restart: bool) -> str:
        """main の先頭から 1 タスクを通す。'merged'・'stuck'・'blocked'・'conflict' のどれかを返す。"""
        difficulty = task.difficulty
        assert difficulty is not None
        models = MODELS[difficulty]
        branch = f"task/{task.number}"
        self.start_branch(branch)

        plan: List[Tuple[str, bool]] = [(str(models["implement"]), False)]  # （モデル, 会話を続けるか）
        plan += [(str(models["implement"]), True)] * MAX_SAME_MODEL_FIXES
        if models["escalate"]:
            plan += [(str(models["escalate"]), False)] * MAX_ESCALATED_FIXES

        session: Optional[str] = None
        findings: List[str] = []
        fixed: List[str] = []
        human: List[str] = []
        title = task.title
        summary = ""

        for step, (model, resume) in enumerate(plan, start=1):
            data = self.call_implement(task, model, "\n\n".join(findings), session if resume else None, step)
            session = data.get("session_id") or session
            result = data["structured_output"]
            if result["status"] == "blocked":
                return self.handle_blocked(task, branch, result, history)
            fixed = merge_unique(fixed, result.get("fixed_problems") or [])
            human = merge_unique(human, result.get("human_checks") or [])
            title = one_line(result.get("pr_title") or "") or title
            summary = (result.get("summary") or "").strip() or summary

            label = f"{step} 回目（{model}）"
            checked = self.verify(task, branch, str(models["review"]), step, label, title, summary, fixed, human, history)
            if checked["status"] == "conflict":
                if allow_restart:
                    return "conflict"
                history.append(f"{label}: `{MAIN_REF}` の取り込みで、また衝突した")
                return self.mark_stuck(task, branch, checked["findings"], history)
            history.append(f"{label}: {checked['outcome']}")
            if checked["status"] == "pass":
                self.merge(task, branch, checked["pr"], title, summary, fixed, human, checked["notes"], history)
                return "merged"
            if checked["status"] == "stuck":
                return self.mark_stuck(task, branch, checked["findings"], history)
            findings = checked["findings"]
            self.say(f"  {checked['outcome']}")
        return self.mark_stuck(task, branch, findings, history)

    def protected_changes(self) -> List[str]:
        return [path for path in self.changed_paths() if is_protected(path)]

    def verify(
        self,
        task: Task,
        branch: str,
        review_model: str,
        step: int,
        label: str,
        title: str,
        summary: str,
        fixed: List[str],
        human: List[str],
        history: List[str],
    ) -> Dict[str, Any]:
        """実装の結果を確かめる。

        status は 'pass'（通った。pr に PR の番号）、'fix'（findings を直しに回す）、
        'conflict'（main の取り込みで衝突した）、'stuck'（直しに回さず行き詰まりにする）のどれか。
        """

        def outcome(status: str, text: str, findings: Optional[List[str]] = None, notes: Optional[List[Dict[str, str]]] = None, pr: Optional[int] = None) -> Dict[str, Any]:
            return {"status": status, "outcome": text, "findings": findings or [], "notes": notes or [], "pr": pr}

        failure = self.commit_all(f"{title}（#{task.number}）\n\n{CO_AUTHOR}")
        if failure:
            return outcome("fix", "コミットに失敗した", [f"## コミットの失敗\n\n```\n{failure}\n```"])

        self.git(["fetch", REMOTE])
        merge = self.git(["merge", "--no-edit", "--quiet", MAIN_REF], check=False)
        if merge.returncode != 0:
            conflict = self.git(["diff", "--name-only", "--diff-filter=U"], check=False).stdout.strip()
            self.git(["merge", "--abort"], check=False)
            return outcome(
                "conflict",
                f"`{MAIN_REF}` の取り込みで衝突した",
                [f"## `{MAIN_BRANCH}` との衝突\n\n`{MAIN_REF}` をマージできなかった。衝突したファイル:\n\n```\n{conflict or tail(merge.stdout + merge.stderr)}\n```"],
            )

        # 保護するパスの変更は、直しに回さずに、ここで戻す
        protected = self.protected_changes()
        if protected:
            self.revert_protected(protected)
            history.append(f"{label}: 保護するパスを変えていたので、スクリプトが元に戻した: " + "、".join(f"`{p}`" for p in protected))
            self.say("  保護するパスの変更を戻した: " + "、".join(protected))

        if not self.changed_paths():
            return outcome(
                "fix",
                "変更が無かった",
                [f"## 変更がない\n\n`{MAIN_REF}` との差分が空である。タスクの変更を作業ツリーに書く。"],
            )

        check_findings = self.run_checks(task, step)
        if check_findings:
            return outcome("fix", "機械のチェックで失敗した", check_findings)

        must, notes = self.review(task, review_model, step)
        if must:
            lines = ["## レビューの指摘（直す）", ""]
            lines += [f"- {('`' + f['file'] + '`: ') if f.get('file') else ''}{one_line(f.get('description', ''))}" for f in must]
            return outcome("fix", f"レビューで直す指摘が {len(must)} 件あった", ["\n".join(lines)], notes)

        # push の直前に、保護するパスをもう一度確かめる
        remaining = self.protected_changes()
        if remaining:
            listed = bullet_list([f"`{p}`" for p in remaining])
            return outcome(
                "stuck",
                "保護するパスの変更が差分に残っていたので、マージしなかった",
                [f"## 保護するパスの変更が残っている\n\n{listed}"],
                notes,
            )

        body = self.pr_body(task, summary, fixed, human, notes, history + [f"{label}: 機械のチェックとレビューを通った"])
        pr = self.publish(branch, title, body)
        ci_findings = self.wait_for_ci(pr)
        if ci_findings:
            return outcome("fix", "CI で失敗した", ci_findings, notes, pr)
        return outcome("pass", "機械のチェック、レビュー、CI を通った", [], notes, pr)

    def merge(
        self,
        task: Task,
        branch: str,
        pr: int,
        title: str,
        summary: str,
        fixed: List[str],
        human: List[str],
        notes: List[Dict[str, str]],
        history: List[str],
    ) -> None:
        self.gh(["pr", "edit", str(pr), "--title", title, "--body-file", "-"], stdin=self.pr_body(task, summary, fixed, human, notes, history))
        self.git(["checkout", "--quiet", "--detach", MAIN_REF])
        self.gh(
            [
                "pr", "merge", str(pr), "--squash",
                "--subject", f"{title} (#{pr})",
                "--body", f"Closes #{task.number}\n\n{CO_AUTHOR}",
            ]
        )
        self.say(f"マージした: #{task.number} → PR #{pr}")
        delete = self.git(["push", REMOTE, "--delete", branch], check=False)
        if delete.returncode != 0:
            self.say(f"警告: リモートのブランチ `{branch}` を消せなかった。")

        # 「Closes」で閉じるのを待たずに、ここで閉じる（次のタスク選びで、また選ばないため）
        state = self.gh_json(["issue", "view", str(task.number), "--json", "state"]).get("state")
        if state != "CLOSED":
            self.gh(["issue", "close", str(task.number), "--reason", "completed"])
        if LABEL_UNDECIDED in task.labels:
            self.gh(["issue", "edit", str(task.number), "--remove-label", LABEL_UNDECIDED])

        if task.ai_filed:
            lines = [f"直したので閉じる（{today()}、Claude の判断）。", "", f"PR #{pr} でマージした。", "", summary.strip() or "（要約なし）"]
            if fixed:
                lines += ["", "起票せずに直した問題:", "", bullet_list(fixed)]
            self.comment(task.number, "\n".join(lines))
        if human:
            self.comment(
                task.number,
                f"人が確かめることを足した（{today()}、Claude の判断）。\n\n{bullet_list(human)}",
            )
        if human or human_check_items(task.body):
            self.gh(["issue", "edit", str(task.number), "--add-label", LABEL_HUMAN_CHECK])
        self.merged.append(f"#{task.number} {task.title}（PR #{pr}）")

    def mark_stuck(self, task: Task, branch: str, findings: List[str], history: List[str]) -> str:
        failure = self.commit_all(f"WIP: 行き詰まり（#{task.number}）")
        if failure:
            self.git(["reset", "--quiet"], check=False)
        ahead = self.git(["rev-list", "--count", f"{MAIN_REF}..HEAD"]).stdout.strip()
        pushed = False
        if ahead not in ("", "0"):
            push = self.git(["push", "--force", REMOTE, f"{branch}:{branch}"], check=False)
            if push.returncode != 0:
                raise InfraError(f"`git push` が失敗しました: {tail(push.stderr).strip()}")
            pushed = True
        pr = self.find_pr(branch)
        last = "\n\n".join(findings)[:COMMENT_CHARS] or "（指摘なし）"
        body = "\n".join(
            [
                f"行き詰まりにした（{today()}、Claude の判断）。",
                "",
                "ループが、決めた回数の中でこのタスクを通せなかった。",
                "",
                "## 経過",
                "",
                bullet_list(history) or "- （なし）",
                "",
                "## ブランチ",
                "",
                f"`{branch}`（{'push してある' if pushed else '変更が無いので push していない'}）"
                + (f"。PR #{pr} が開いたままになっている。" if pr is not None else ""),
                "",
                "## 最後の指摘",
                "",
                last,
            ]
        )
        self.comment(task.number, body)
        self.gh(["issue", "edit", str(task.number), "--add-label", LABEL_STUCK])
        self.reset_worktree()
        self.say(f"行き詰まりにした: #{task.number}")
        self.stuck.append(f"#{task.number} {task.title}")
        return "stuck"

    def handle_blocked(self, task: Task, branch: str, result: Dict[str, Any], history: List[str]) -> str:
        reason = (result.get("blocked_reason") or "").strip() or "（理由なし）"
        blocking = []
        for number in result.get("blocking_issues") or []:
            if isinstance(number, int) and number > 0 and number != task.number and number not in blocking:
                blocking.append(number)
        new_body = None
        if blocking:
            current = self.gh_json(["issue", "view", str(task.number), "--json", "body"]).get("body") or ""
            new_body = add_prerequisites(current, blocking)
        if new_body is None:
            why = "進められないと答えたが、待つ Issue を挙げなかった" if not blocking else "本文に前提の行が無く、待つ Issue を足せなかった"
            history = history + [f"実装が進められないと答えた（{why}）"]
            return self.mark_stuck(task, branch, [f"## 進められない理由\n\n{reason}"], history)
        self.gh(["issue", "edit", str(task.number), "--body-file", "-"], stdin=new_body)
        refs = "、".join(f"#{n}" for n in blocking)
        self.comment(
            task.number,
            "\n".join(
                [
                    f"判断待ちにした（{today()}、Claude の判断）。",
                    "",
                    f"前提に {refs} を足した。閉じたら、ループがこのタスクをまた選ぶ。途中の作業は残していない。",
                    "",
                    "## 理由",
                    "",
                    reason,
                ]
            ),
        )
        self.reset_worktree()
        self.say(f"判断待ちにした: #{task.number}（{refs} を待つ）")
        self.blocked.append(f"#{task.number} {task.title}（{refs} を待つ）")
        return "blocked"

    # ---- 仕分け ----

    def triage(self) -> None:
        numbers = self.untriaged_issues()
        if not numbers:
            return
        self.say("AI起票 の Issue を仕分ける: " + "、".join(f"#{n}" for n in numbers))
        self.git(["fetch", REMOTE])
        self.reset_worktree()
        prompt = PROMPT_TRIAGE.format(numbers=" ".join(str(n) for n in numbers))
        self.call_claude("triage", prompt, TRIAGE_MODEL, SETTINGS_TRIAGE, None, TRIAGE_TIMEOUT)
        self.triaged.update(numbers)
        self.discard_if_dirty("仕分けの後")

    # ---- 全体 ----

    def lock(self) -> None:
        path = os.path.join(self.common_dir(self.repo), "implement-loop.lock")
        self.lock_handle = open(path, "w")
        try:
            fcntl.flock(self.lock_handle, fcntl.LOCK_EX | fcntl.LOCK_NB)
        except OSError:
            self.lock_handle.close()
            self.lock_handle = None
            raise InfraError("ほかの実装のループが、このリポジトリで動いています。")

    def unlock(self) -> None:
        if self.lock_handle is not None:
            fcntl.flock(self.lock_handle, fcntl.LOCK_UN)
            self.lock_handle.close()
            self.lock_handle = None

    def main_loop(self) -> int:
        for tool in ("git", "gh", "claude"):
            if shutil.which(tool) is None:
                raise InfraError(f"`{tool}` が見つかりません。")
        self.lock()
        self.open_logs()
        self.say(f"実装のループを始める。記録: {self.run_dir}")
        self.ensure_worktree()
        self.preflight_files()
        self.triage()
        handled = 0
        while True:
            if self.max_tasks is not None and handled >= self.max_tasks:
                self.stop_reason = f"--max-tasks の {self.max_tasks} 件に達した"
                return 0
            ready, _ = self.classify(self.list_tasks())
            if not ready:
                self.stop_reason = "選べるタスクが無い"
                return 0
            task = ready[0]
            self.attempted.add(task.number)
            handled += 1
            outcome = self.run_task(task)
            if outcome == "stuck":
                self.consecutive_stuck += 1
                if self.consecutive_stuck >= MAX_CONSECUTIVE_STUCK:
                    self.stop_reason = f"行き詰まりが {MAX_CONSECUTIVE_STUCK} 件続いた"
                    return 1
            else:
                self.consecutive_stuck = 0
            self.triage()

    def cleanup_after_failure(self) -> None:
        if os.path.isdir(self.worktree):
            try:
                self.reset_worktree()
            except InfraError as error:
                self.say(f"警告: 作業ツリーを元に戻せなかった: {error}")

    def summary(self) -> None:
        self.say("---- まとめ ----")
        for heading, items in (("マージしたタスク", self.merged), ("行き詰まり", self.stuck), ("判断待ち", self.blocked)):
            self.say(f"{heading}: {len(items)} 件")
            for item in items:
                self.say(f"  {item}")
        self.say(f"止まった理由: {self.stop_reason}")
        if self.run_dir:
            self.say(f"記録: {self.run_dir}")

    def execute(self) -> int:
        code = 2
        try:
            code = self.main_loop()
        except InfraError as error:
            self.stop_reason = f"仕組みの失敗: {error}"
            self.cleanup_after_failure()
            code = 2
        except KeyboardInterrupt:
            self.stop_reason = "Ctrl-C で止めた"
            self.cleanup_after_failure()
            code = 130
        finally:
            self.unlock()
        self.summary()
        return code

    # ---- 表示だけのモード ----

    def show_dry_run(self) -> int:
        print("実装のループ（--dry-run）: 何も変えずに、選ぶタスクと実行するコマンドを表示する。")
        print(f"作業ツリー: {self.worktree}{'' if os.path.exists(self.worktree) else '（まだ無い。実行すると作る）'}")
        try:
            untriaged = self.untriaged_issues()
            ready, skipped = self.classify(self.list_tasks())
        except InfraError as error:
            print(f"仕組みの失敗: {error}", file=sys.stderr)
            return 2
        print("")
        print("仕分ける AI起票 の Issue: " + ("、".join(f"#{n}" for n in untriaged) if untriaged else "なし"))
        print("")
        print(f"選べるタスク（選ぶ順）: {len(ready)} 件")
        for index, task in enumerate(ready, start=1):
            models = MODELS[task.difficulty or "中"]
            print(
                f"  {index}. {task.describe()}"
                f"  実装 {models['implement']}・レビュー {models['review']}・引き上げ先 {models['escalate'] or 'なし'}"
            )
        print(f"選べないタスク: {len(skipped)} 件")
        for task, reason in skipped:
            print(f"  #{task.number} {task.title}: {reason}")
        settings_dir = "（記録のディレクトリ）"
        print("")
        print("実行するコマンド（作業ツリーの中で実行する）:")
        if untriaged:
            argv = self.claude_argv(
                PROMPT_TRIAGE.format(numbers=" ".join(str(n) for n in untriaged)),
                TRIAGE_MODEL, f"{settings_dir}/{os.path.basename(SETTINGS_TRIAGE)}", None, None,
            )
            print("  仕分け: " + " ".join(shlex.quote(a) for a in argv))
        if ready:
            task = ready[0]
            models = MODELS[task.difficulty or "中"]
            argv = self.claude_argv(
                PROMPT_IMPLEMENT.format(number=task.number), str(models["implement"]),
                f"{settings_dir}/{os.path.basename(SETTINGS_IMPLEMENT)}", {"…": "実装の結果の形"}, None,
            )
            print("  実装: " + " ".join(shlex.quote(a) for a in argv))
            for check in CHECKS:
                print("  機械のチェック: " + " ".join(check["argv"]) + f"（`{check['requires']}` があるとき）")
            argv = self.claude_argv(
                PROMPT_REVIEW.format(number=task.number), str(models["review"]),
                f"{settings_dir}/{os.path.basename(SETTINGS_REVIEW)}", {"…": "レビューの結果の形"}, None,
            )
            print("  レビュー: " + " ".join(shlex.quote(a) for a in argv))
            print(f"  その後: git push、gh pr create、gh pr checks、gh pr merge --squash（ブランチ task/{task.number}）")
        else:
            print("  （選べるタスクが無いので、実装は行わない）")
        return 0

    def show_status(self) -> int:
        try:
            tasks = self.list_tasks()
            ready, skipped = self.classify(tasks)
            stuck = self.gh_json(["issue", "list", "--state", "open", "--label", LABEL_STUCK, "--limit", "200", "--json", "number,title"])
            human = self.gh_json(["issue", "list", "--state", "all", "--label", LABEL_HUMAN_CHECK, "--limit", "200", "--json", "number,title"])
            undecided = self.gh_json(["issue", "list", "--state", "open", "--label", LABEL_UNDECIDED, "--limit", "200", "--json", "number,title"])
        except InfraError as error:
            print(f"仕組みの失敗: {error}", file=sys.stderr)
            return 2
        print(f"残りのタスク: {len(tasks)} 件（いま選べるもの {len(ready)} 件）")
        for task in ready[:10]:
            print(f"  {task.describe()}")
        if len(ready) > 10:
            print(f"  … ほか {len(ready) - 10} 件")
        waiting = [(t, r) for t, r in skipped if r.startswith("前提")]
        if waiting:
            print(f"前提を待っているタスク: {len(waiting)} 件")
        for heading, items in (
            (f"行き詰まり（`{LABEL_STUCK}`）", stuck),
            (f"人の確認待ち（`{LABEL_HUMAN_CHECK}`）", human),
            (f"未判断（`{LABEL_UNDECIDED}`）", undecided),
        ):
            print(f"{heading}: {len(items)} 件")
            for item in sorted(items, key=lambda i: i["number"]):
                print(f"  #{item['number']} {item['title']}")
        return 0


def parse_args(argv: List[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        prog="python3 tools/implement-loop.py",
        description="実装のループを回す。ラベル「実装タスク」の Issue を 1 件ずつ、実装・チェック・レビュー・マージする。",
    )
    parser.add_argument("--dry-run", action="store_true", help="選ぶタスクと実行するコマンドを表示するだけで、何も変えない")
    parser.add_argument("--status", action="store_true", help="行き詰まり・人の確認待ち・未判断・残りのタスクを表示する")
    parser.add_argument("--max-tasks", type=int, metavar="N", help="タスクを N 件扱ったら止まる")
    parser.add_argument("--worktree", metavar="PATH", help="ループ専用の作業ツリーの場所（既定はリポジトリの隣の「リポジトリ名-loop」）")
    parser.add_argument("--log-dir", metavar="PATH", help="各呼び出しの出力を残す場所（既定は ~/Library/Logs/progrust-implement-loop/）")
    parser.add_argument("--repo", default=REPO_ROOT, help=argparse.SUPPRESS)  # テスト用
    args = parser.parse_args(argv)
    if args.max_tasks is not None and args.max_tasks < 1:
        parser.error("--max-tasks には 1 以上を指定します。")
    if args.dry_run and args.status:
        parser.error("--dry-run と --status は、同時に指定できません。")
    return args


def main(argv: List[str]) -> int:
    args = parse_args(argv)
    loop = Loop(args)
    if args.status:
        return loop.show_status()
    if args.dry_run:
        return loop.show_dry_run()
    return loop.execute()


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
