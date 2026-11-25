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
    2    仕組みの失敗（claude・gh・git の失敗、CI のチェックが報告されない、CI のジョブが開始されない、
         前提の不足）で止めた
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

# スキルの呼び方（{number} は Issue の番号、{numbers} は空白で区切った番号）
PROMPT_IMPLEMENT = "/implement-task {number} --loop"
PROMPT_FIX = "/implement-task {number} --loop fix\n\n{findings}"
PROMPT_REVIEW = "/review-task {number} --loop"
PROMPT_TRIAGE = "/triage-issues --loop {numbers}"
# 実装する側が結果を決まった形で返せなかったときに、引き上げるモデルで同じ会話の続きに送る（ADR 0229）
PROMPT_REPORT = (
    "この会話の前の応答は、作業の結果を決まった形（StructuredOutput）で返せずに終わった。"
    "作業はもう済んでいるので、ファイルを変えずに、この会話でした作業の結果を、決まった形で返して。"
)
# claude が、決まった形の結果を書き直しの上限まで返せずに終わったときの subtype
STRUCTURED_OUTPUT_FAILURE = "error_max_structured_output_retries"

# ループの仕分けが、ユーザーの判断が要るとして `未判断` のまま残した Issue に書くコメントの、1 行目の始まり。
# このコメントがあり、`未判断` が付いたままの Issue は、ループでは仕分け直さない（ADR 0193）
LEFT_FOR_USER_PREFIX = "ユーザーの判断が要るので、未判断のまま残した（"

# リポジトリのオーナーが書いたコメントの authorAssociation。ループは、オーナーが書いた Issue・コメント・
# pull request だけを判定に使う。書いた人が読めないものは、オーナーのものとして扱わない（ADR 0247）
OWNER_ASSOCIATION = "OWNER"

# 権限の設定（main にあるものを使う）
SETTINGS_IMPLEMENT = ".claude/loop-settings.json"
SETTINGS_REVIEW = ".claude/loop-review-settings.json"
SETTINGS_TRIAGE = ".claude/loop-triage-settings.json"
# claude の権限のモード。bypassPermissions では、設定の allow は使われず、deny だけが効く（ADR 0189）
PERMISSION_MODE = "bypassPermissions"

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
    ".gitleaks.toml",
    ".gitleaksignore",
    "documents/conventions/",
    "tools/check-documents.py",
    "tools/check-secrets.py",
    "tools/design-system.py",
    "tools/implement-loop.py",
    "tools/okf/",
    "tools/tests/test_check_secrets.py",
    "tools/tests/test_implement_loop.py",
)

# 機械のチェック（requires のファイルがあるときだけ実行する。requires_script があるときは、
# package.json の scripts にそのスクリプトがあるときだけ実行する）。
# stop_on_failure のチェック（秘密の情報の検査）は、タスクのブランチのものではなく、origin の main の道具
# （requires と with_files を main から取り出したもの）で実行する。失敗したら直しに回さず、push せずに止まる。
# push はすべて push_branch を通り、その直前にも実行する（ADR 0249）。
CHECKS: Tuple[Dict[str, Any], ...] = (
    {
        "name": "秘密の情報の検査",
        "argv": ["python3", "-I", "tools/check-secrets.py"],
        "requires": "tools/check-secrets.py",
        "with_files": [".gitleaks.toml"],
        "stop_on_failure": True,
    },
    {"name": "documents の検査", "argv": ["python3", "tools/check-documents.py"], "requires": "tools/check-documents.py"},
    {"name": "デザインシステムの検査", "argv": ["python3", "tools/design-system.py", "check"], "requires": "tools/design-system.py"},
    {"name": "npm run check", "argv": ["npm", "run", "check"], "requires": "package.json"},
    {
        "name": "教材チェックツール",
        "argv": ["npm", "run", "content:check"],
        "requires": "package.json",
        "requires_script": "content:check",
    },
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
# CI を待つ間に `gh pr checks` が続けて失敗したら止める回数（GitHub の一時的な失敗は、待ってからやり直す。ADR 0252）
CI_GH_MAX_FAILURES = 5

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
JOB_LINK_RE = re.compile(r"/actions/runs/\d+/job/(\d+)")


class InfraError(Exception):
    """仕組みの失敗。タスクにラベルを付けずに、ループを止める。"""


class StructuredOutputError(InfraError):
    """claude が、決まった形の結果を書き直しの上限まで返せずに終わった。作業の会話は session_id で続けられる。"""

    def __init__(self, message: str, session_id: Optional[str]) -> None:
        super().__init__(message)
        self.session_id = session_id


def author_login(raw: Any) -> str:
    """gh の JSON の `author` の login。無いか読めなければ空。"""
    author = raw.get("author") if isinstance(raw, dict) else None
    login = author.get("login") if isinstance(author, dict) else None
    return login.strip() if isinstance(login, str) else ""


def is_owner_login(login: str, owner: str) -> bool:
    """login がリポジトリのオーナーのものか。GitHub の login は大文字と小文字を区別しない。どちらかが空なら False。"""
    return bool(login) and bool(owner) and login.casefold() == owner.casefold()


def authored_by_owner(raw: Any, owner: str) -> bool:
    """Issue・pull request を書いた人が、リポジトリのオーナーか。書いた人が読めなければ False。"""
    return is_owner_login(author_login(raw), owner)


def owner_comments(comments: Any) -> List[Dict[str, Any]]:
    """コメントのうち、オーナーが書いたもの（authorAssociation が OWNER）だけ。無い・読めないものは外す。"""
    if not isinstance(comments, list):
        return []
    return [c for c in comments if isinstance(c, dict) and c.get("authorAssociation") == OWNER_ASSOCIATION]


def is_own_pr(raw: Any, branch: str, owner: str) -> bool:
    """pull request が、このリポジトリのブランチ `branch` から、オーナーが作ったものか。

    `gh pr list --head` は、fork のブランチからの pull request も、ブランチの名前だけで返す。
    fork からのものと、オーナー以外が作ったものは、ループの pull request として扱わない（ADR 0247）。
    """
    if not isinstance(raw, dict) or raw.get("isCrossRepository") is not False:
        return False
    if raw.get("headRefName") != branch:
        return False
    head_owner = raw.get("headRepositoryOwner")
    head_login = head_owner.get("login") if isinstance(head_owner, dict) else None
    if not isinstance(head_login, str) or not is_owner_login(head_login.strip(), owner):
        return False
    return authored_by_owner(raw, owner)


class Task:
    def __init__(self, raw: Dict[str, Any], owner: str) -> None:
        self.number: int = int(raw["number"])
        self.title: str = raw.get("title") or ""
        self.body: str = raw.get("body") or ""
        self.labels: Set[str] = {label["name"] for label in raw.get("labels") or []}
        milestone = raw.get("milestone") or {}
        self.milestone: str = milestone.get("title") or ""
        # オーナー以外が書いた Issue は、本文を指示として読まないために、選ばない（ADR 0247）
        self.owner_authored: bool = authored_by_owner(raw, owner)

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
        if not self.owner_authored:
            return "リポジトリのオーナー以外が書いた Issue である"
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

    def shown_title(self) -> str:
        """表示するタイトル。オーナー以外が書いた Issue は、タイトルも出さない（出力を AI が読むことがあるため）。"""
        return self.title if self.owner_authored else "（オーナー以外が書いた Issue なので、タイトルを出さない）"


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
        self.owner: Optional[str] = None
        # 始めるときに origin の main にあった、失敗したら止まるチェックの道具（途中で無くなったら止まる）
        self.required_tools: set = set()
        self.attempted: Set[int] = set()
        self.triaged: Set[int] = set()
        # 仕分けの会話で権限に拒否された操作があったので、もう一度仕分けることにした Issue（ADR 0194）
        self.triage_retried: Set[int] = set()
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
        env: Optional[Dict[str, str]] = None,
    ) -> subprocess.CompletedProcess:
        # 標準入力を渡さないときは、端末や親の標準入力を待たないように閉じておく
        source: Dict[str, Any] = {"input": stdin} if stdin is not None else {"stdin": subprocess.DEVNULL}
        try:
            return subprocess.run(
                argv,
                cwd=cwd,
                env=env,
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
        # ループ自身の git は、フックを動かさず、置き換えの参照（refs/replace）と grafts を使わない（ADR 0249）。
        # ループの claude が作業ツリーと共有する .git にフックや置き換えを置いても、ループのコミット・マージ・push と、
        # 検査の道具の取り出しが、それに左右されないようにする。
        env = dict(os.environ, GIT_NO_REPLACE_OBJECTS="1", GIT_GRAFT_FILE=os.devnull)
        # GIT_GRAFT_FILE を指すと、git が、grafts は廃止予定だという案内を毎回出すので、案内だけを止める
        quiet = ["-c", "advice.graftFileDeprecated=false"]
        result = self.run(["git", "-c", f"core.hooksPath={os.devnull}"] + quiet + args, cwd or self.worktree, GIT_TIMEOUT, env=env)
        if check and result.returncode != 0:
            raise InfraError(f"`git {' '.join(args[:3])}` が失敗しました: {tail(result.stderr).strip()}")
        return result

    # ---- Issue ----

    def repo_owner(self) -> str:
        """リポジトリの所有者の login。分からなければ、オーナーを見分けられないので止める。"""
        if self.owner is None:
            data = self.gh_json(["repo", "view", "--json", "owner"])
            owner = data.get("owner") if isinstance(data, dict) else None
            login = owner.get("login") if isinstance(owner, dict) else None
            if not isinstance(login, str) or not login.strip():
                raise InfraError("`gh repo view` から、リポジトリの所有者が分かりません。オーナーが書いた Issue を見分けられないので、止めます。")
            self.owner = login.strip()
        return self.owner

    def list_tasks(self) -> List[Task]:
        raw = self.gh_json(
            [
                "issue", "list", "--state", "open", "--label", LABEL_TASK,
                "--limit", "500", "--json", "number,title,labels,milestone,body,author",
            ]
        )
        owner = self.repo_owner()
        return [Task(item, owner) for item in raw]

    def prerequisite_state(self, number: int, cache: Dict[int, str]) -> str:
        """前提の Issue の状態。'closed'（オーナーが書いて閉じている）、'open'（閉じていないか、読めない）、
        'not_owner'（オーナー以外が書いた）のどれか。

        オーナー以外が書いた Issue は、書いた人が自分で閉じられるので、閉じていても前提が済んだとしない（ADR 0247）。
        """
        if number not in cache:
            result = self.gh(["issue", "view", str(number), "--json", "state,author"], check=False)
            state = "open"
            if result.returncode == 0:
                try:
                    data = json.loads(result.stdout)
                except ValueError:
                    data = None
                if isinstance(data, dict):
                    if not authored_by_owner(data, self.repo_owner()):
                        state = "not_owner"
                    elif data.get("state") == "CLOSED":
                        state = "closed"
            cache[number] = state
        return cache[number]

    def classify(self, tasks: List[Task]) -> Tuple[List[Task], List[Tuple[Task, str]]]:
        """選べるタスク（選ぶ順）と、選べないタスクとその理由。"""
        ready: List[Task] = []
        skipped: List[Tuple[Task, str]] = []
        cache: Dict[int, str] = {}
        for task in sorted(tasks, key=Task.sort_key):
            reason = task.skip_reason()
            if reason is None and task.number in self.attempted:
                reason = "この実行で既に扱った"
            if reason is None:
                prerequisites = parse_prerequisites(task.body) or []
                states = {n: self.prerequisite_state(n, cache) for n in prerequisites}
                waiting = [n for n in prerequisites if states[n] == "open"]
                foreign = [n for n in prerequisites if states[n] == "not_owner"]
                reasons = []
                if waiting:
                    reasons.append("前提の " + "、".join(f"#{n}" for n in waiting) + " が閉じていない")
                if foreign:
                    reasons.append("前提の " + "、".join(f"#{n}" for n in foreign) + " がオーナー以外が書いた Issue である")
                if reasons:
                    reason = "。".join(reasons)
            if reason is None:
                ready.append(task)
            else:
                skipped.append((task, reason))
        return ready, skipped

    def untriaged_issues(self) -> List[int]:
        raw = self.gh_json(
            [
                "issue", "list", "--state", "open", "--label", LABEL_AI_FILED,
                "--limit", "200", "--json", "number,labels,author",
            ]
        )
        owner = self.repo_owner()
        # 開いているタスクが「前提」で待っている Issue は、ユーザーの判断を求めるものなので、
        # ループでは仕分けない（ユーザーが決める前に、閉じたりタスクにしたりしないため）。
        # 本文を読むのは、オーナーが書いたタスクだけ（ADR 0247）。
        awaited: Set[int] = set()
        for task in self.list_tasks():
            if task.owner_authored:
                awaited.update(parse_prerequisites(task.body) or [])
        numbers = []
        for item in raw:
            # オーナー以外が書いた Issue は、仕分けない（本文を指示として読まないため。ADR 0247）
            if not authored_by_owner(item, owner):
                continue
            labels = {label["name"] for label in item.get("labels") or []}
            if LABEL_HOLD in labels or LABEL_TASK in labels:
                continue
            if int(item["number"]) in self.triaged or int(item["number"]) in awaited:
                continue
            # 前の実行の仕分けがユーザーに回した Issue は、ユーザーが答えるまで、ループでは仕分け直さない
            if LABEL_UNDECIDED in labels and self.left_for_user(int(item["number"])) is not False:
                continue
            numbers.append(int(item["number"]))
        return sorted(numbers)

    def left_for_user(self, number: int) -> Optional[bool]:
        """前の実行の仕分けが、ユーザーの判断が要るとして残した Issue か。コメントを全部読めなければ None。

        `gh issue list --json comments` は Issue ごとに最初の 100 件のコメントしか返さないので、
        コメントを全部返す `gh issue view --json comments` で読む。見るのは、オーナーが書いたコメントだけ
        （ループのコメントもオーナーのアカウントで書かれる）。外部の人のコメントで押し出されないようにする（ADR 0247）。
        """
        result = self.gh(["issue", "view", str(number), "--json", "comments"], check=False)
        if result.returncode != 0:
            self.say(f"警告: #{number} のコメントを読めなかったので、この回は仕分けない。")
            return None
        try:
            data = json.loads(result.stdout)
        except ValueError:
            data = None
        comments = data.get("comments") if isinstance(data, dict) else None
        if not isinstance(comments, list):
            self.say(f"警告: #{number} のコメントを読めなかったので、この回は仕分けない。")
            return None
        return any(
            str(comment.get("body") or "").lstrip().startswith(LEFT_FOR_USER_PREFIX)
            for comment in owner_comments(comments)
        )

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

    def head_state(self) -> str:
        """作業ツリーの HEAD のコミットと、ブランチの名前（切り離されていれば「detached」）。"""
        commit = self.git(["rev-parse", "HEAD"]).stdout.strip()
        branch = self.git(["symbolic-ref", "-q", "--short", "HEAD"], check=False).stdout.strip() or "detached"
        return f"{branch}@{commit[:12]}"

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

    def check_engines(self) -> None:
        """使われる node・npm の版が、package.json の engines の系列（`~X.Y` の形）に合うかを確かめる（ADR 0195）。

        ループは、始めた端末の PATH で node・npm を探す。系列の違う node で、チェックと npm ci を進めないために、
        始める前に比べる。
        """
        path = os.path.join(self.worktree, "package.json")
        if not os.path.isfile(path):
            return
        try:
            with open(path, encoding="utf-8") as handle:
                engines = json.load(handle).get("engines") or {}
        except (ValueError, AttributeError) as error:
            raise InfraError(f"`package.json` の engines を読めません: {error}")
        for tool in ("node", "npm"):
            wanted = engines.get(tool)
            match = re.fullmatch(r"~(\d+)\.(\d+)(?:\.\d+)?", wanted.strip()) if isinstance(wanted, str) else None
            if match is None:
                continue
            found = shutil.which(tool)
            if found is None:
                raise InfraError(f"`{tool}` が見つかりません。")
            result = self.run([tool, "--version"], self.worktree, GIT_TIMEOUT)
            version = result.stdout.strip().lstrip("v")
            if result.returncode != 0 or version.split(".")[:2] != [match.group(1), match.group(2)]:
                raise InfraError(
                    f"`{tool}` の版が {version or '不明'} で、package.json の engines（{wanted}）の系列に合いません。"
                    f"使われる {tool} は {found} です。`which -a {tool}` で確かめ、engines の系列の {tool} が"
                    f"先に見つかる端末から、始め直してください。"
                )

    def check_secret_scanner(self) -> None:
        """秘密の情報の検査に使う gitleaks が、固定した版で入っているかを確かめる（ADR 0249）。

        入っていないと、どのタスクも機械のチェックで止まるので、始める前に確かめる。
        """
        for check in CHECKS:
            if not check.get("stop_on_failure"):
                continue
            prepared = self.main_tool_argv(check)
            if prepared is None:
                continue
            argv, _ = prepared
            self.required_tools.add(check["requires"])
            result = self.run(argv + ["--check-version"], self.worktree, GIT_TIMEOUT)
            if result.returncode != 0:
                raise InfraError(
                    f"`{' '.join(check['argv'] + ['--check-version'])}`（`{MAIN_REF}` のもの）が失敗したので、始めません:\n"
                    f"{tail(result.stdout + result.stderr).strip()}"
                )

    def remote_sha(self, ref: str) -> Optional[str]:
        """origin（GitHub）にある ref の先頭のコミットを、手元の追跡ブランチを使わずに `git ls-remote` で得て、
        手元に無ければ取ってくる。ref が無ければ None。"""
        result = self.git(["ls-remote", REMOTE, ref])
        sha = None
        for line in result.stdout.splitlines():
            fields = line.split()
            if len(fields) == 2 and fields[1] == ref:
                sha = fields[0]
        if sha is None:
            return None
        if not re.fullmatch(r"[0-9a-f]{40}", sha):
            raise InfraError(f"`git ls-remote` の {ref} の値が、コミットの SHA ではありません: {sha}")
        if self.git(["cat-file", "-e", f"{sha}^{{commit}}"], check=False).returncode != 0:
            self.git(["fetch", "--quiet", REMOTE, sha])
        return sha

    def main_tool_argv(self, check: Dict[str, Any], branch: Optional[str] = None) -> Optional[Tuple[List[str], str]]:
        """失敗したら止まるチェックの道具を、origin の main から取り出し、実行する引数と、main の SHA を返す。

        タスクのブランチの道具と設定は、タスクの作業で書き換えられているかもしれないので使わない。手元の追跡ブランチ
        （origin/main）も、作業ツリーと共有する .git で動かせるので使わず、`git ls-remote` で得た main の SHA から
        取り出す。検査の範囲も、その SHA（と、origin に branch があれば、その SHA）を基点にする（`--base`）。
        main に道具が無ければ None（このチェックは無い）。ただし、始めるときに道具があったのに無くなったときと、
        main にあるのに取り出せないときは止まる。
        """
        requires = check["requires"]
        main_sha = self.remote_sha(f"refs/heads/{MAIN_BRANCH}")
        if main_sha is None:
            raise InfraError(f"origin に {MAIN_BRANCH} のブランチが見つからないので、{check['name']}を実行できません。止めます。")
        if self.git(["cat-file", "-e", f"{main_sha}:{requires}"], check=False).returncode != 0:
            if requires in self.required_tools:
                raise InfraError(
                    f"始めるときには origin の {MAIN_BRANCH} にあった `{requires}` が、見つからなくなりました。"
                    f"{check['name']}を飛ばさずに止めます。"
                )
            return None
        assert self.run_dir is not None
        base = os.path.join(self.run_dir, "main-tools")
        shutil.rmtree(base, ignore_errors=True)
        for path in [requires] + list(check.get("with_files", [])):
            result = self.git(["show", f"{main_sha}:{path}"], check=False)
            if result.returncode != 0:
                raise InfraError(f"origin の {MAIN_BRANCH}（{main_sha[:12]}）から `{path}` を取り出せないので、{check['name']}を実行できません。止めます。")
            target = os.path.join(base, path)
            os.makedirs(os.path.dirname(target), exist_ok=True)
            with open(target, "w", encoding="utf-8") as handle:
                handle.write(result.stdout)
        script_index = check["argv"].index(requires)
        argv = list(check["argv"])
        argv[script_index] = os.path.join(base, requires)
        argv += ["--repo", self.worktree, "--base", main_sha]
        if branch is not None:
            branch_sha = self.remote_sha(f"refs/heads/{branch}")
            if branch_sha is not None:
                argv += ["--base", branch_sha]
        return argv, main_sha

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
        head_before = self.head_state()
        result = self.run(argv, self.worktree, timeout)
        self.save(f"{name}.json", result.stdout)
        if result.stderr.strip():
            self.save(f"{name}.stderr.txt", result.stderr)
        head_after = self.head_state()
        if head_after != head_before:
            # コミットとブランチの切り替えはスクリプトだけが行う。deny をすり抜けたときに、ここで気づく
            raise InfraError(
                f"claude（{name}）を呼んだ前後で、作業ツリーの HEAD が変わりました（{head_before} → {head_after}）。"
                f"claude がコミットかブランチの切り替えをしたので、止めます。"
                f"禁止の一覧（.claude/loop-*.json の deny）か、スキルを見直してください。"
            )
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
            errors = data.get("errors")
            if not detail and isinstance(errors, list) and errors:
                # 決まった形の結果を返せなかったときなどは、result が無く、理由は errors にだけある
                detail = one_line(" ".join(str(error) for error in errors))[:300]
            if data.get("api_error_status"):
                detail = f"API のエラー {data['api_error_status']}: {detail}"
            if data.get("subtype") == STRUCTURED_OUTPUT_FAILURE:
                raise StructuredOutputError(
                    f"claude（{name}）が、結果を決まった形で返せずに終わりました。{detail}",
                    data.get("session_id"),
                )
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
            # 禁止の一覧（.claude/loop-*.json の deny）とスキルを見直すときの手がかりにする。
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
        escalate = MODELS[str(task.difficulty)]["escalate"]
        try:
            data = self.call_claude(name, prompt, model, SETTINGS_IMPLEMENT, IMPLEMENT_SCHEMA, IMPLEMENT_TIMEOUT, resume)
        except StructuredOutputError as error:
            if not escalate or escalate == model or not error.session_id:
                raise
            # 作業は済んでいて、結果の形だけを書き損じた。作業を捨てずに、引き上げるモデルで結果だけを返させる（ADR 0229）
            self.say(f"  結果を決まった形で返せなかったので、{escalate} で同じ会話の続きから結果を返させる")
            data = self.call_claude(
                name + "-report", PROMPT_REPORT, str(escalate), SETTINGS_IMPLEMENT, IMPLEMENT_SCHEMA, IMPLEMENT_TIMEOUT,
                error.session_id,
            )
            data["reported_by"] = str(escalate)
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

    def check_applies(self, check: Dict[str, Any]) -> bool:
        """機械のチェックを、いまの作業ツリーで実行するかどうか。"""
        path = os.path.join(self.worktree, check["requires"])
        if not os.path.exists(path):
            return False
        script = check.get("requires_script")
        if script is None:
            return True
        try:
            with open(path, encoding="utf-8") as handle:
                scripts = json.load(handle).get("scripts") or {}
        except (OSError, ValueError, AttributeError):
            return False
        return isinstance(scripts, dict) and script in scripts

    def run_checks(self, task: Task, step: int) -> List[str]:
        findings = []
        failure = self.ensure_node_modules()
        if failure:
            return [f"## 機械のチェックの失敗: npm ci\n\n```\n{failure}\n```"]
        for check in CHECKS:
            if check.get("stop_on_failure"):
                self.guard_before_push(task, f"task/{task.number}", f"check{step}")
                continue
            if not self.check_applies(check):
                continue
            self.say(f"機械のチェック: {check['name']}")
            result = self.run(check["argv"], self.worktree, CHECK_TIMEOUT)
            output = result.stdout + result.stderr
            self.save(f"task{task.number}-check{step}-{check['argv'][-1].replace('/', '_').replace(':', '_')}.txt", output)
            if result.returncode != 0:
                findings.append(
                    f"## 機械のチェックの失敗: {check['name']}\n\n"
                    f"`{' '.join(check['argv'])}` が終了コード {result.returncode} で終わった。出力の末尾:\n\n"
                    f"```\n{tail(output)}\n```"
                )
        self.discard_if_dirty("機械のチェックの後")
        return findings

    def guard_before_push(self, task: Task, branch: str, when: str) -> None:
        """失敗したら止まるチェック（秘密の情報の検査）を、origin の main の道具で実行する。失敗したら、push せずに止まる。

        コミットした秘密の情報は、後のコミットで消しても、push すればコミットの履歴に残る。ループはコミットを
        書き換えないので、直しに回さずに止まり、ユーザーに任せる（ADR 0249）。出力は、値を伏せたもので、
        記録のディレクトリにだけ残し、GitHub には書かない。
        """
        for check in CHECKS:
            if not check.get("stop_on_failure"):
                continue
            prepared = self.main_tool_argv(check, branch)
            if prepared is None:
                continue
            argv, main_sha = prepared
            self.say(f"機械のチェック: {check['name']}（origin の {MAIN_BRANCH} {main_sha[:12]} の道具で）")
            result = self.run(argv, self.worktree, CHECK_TIMEOUT)
            output = result.stdout + result.stderr
            name = f"task{task.number}-{when}-{os.path.basename(check['argv'][-1]).replace('.', '_')}.txt"
            self.save(name, output)
            if result.returncode != 0:
                raise InfraError(
                    f"{check['name']}（`{' '.join(check['argv'])}`）が、#{task.number} の作業で終了コード {result.returncode} で終わったので、"
                    f"push せずに止めます。記録のディレクトリの {name} を見てください。出力の末尾:\n{tail(output)}"
                )

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
        """このリポジトリのブランチ `branch` からの、オーナーが作った開いている pull request の番号。

        `gh pr list --head` は fork のブランチからの pull request も返すので、手元で外す（ADR 0247）。
        fork からの pull request をたくさん出されても見つけられるように、gh にログインしている人
        （ループではオーナー）が作ったものに GitHub の側で絞った一覧（検索を使う）も見る。検索は、作ったばかりの
        pull request をまだ返さないことがあるので、絞らない一覧も見る。どちらも、手元でオーナーのものか確かめる。
        """
        fields = ["--json", "number,headRefName,isCrossRepository,headRepositoryOwner,author", "--limit", "100"]
        owner = self.repo_owner()
        for extra in ([], ["--author", "@me"]):
            raw = self.gh_json(["pr", "list", "--head", branch, "--state", "open"] + extra + fields)
            own = [item for item in raw if is_own_pr(item, branch, owner)] if isinstance(raw, list) else []
            if own:
                return int(own[0]["number"])
        return None

    def remote_head(self, branch: str) -> Optional[str]:
        """origin のブランチの先頭のコミット。ブランチが無ければ None。"""
        out = self.git(["ls-remote", REMOTE, f"refs/heads/{branch}"]).stdout.split()
        return out[0] if out else None

    def push_branch(self, task: Task, branch: str, when: str) -> None:
        """タスクのブランチを push する。ループの push は、すべてここを通る。

        push の直前に、origin の main の道具で秘密の情報の検査を実行し、通ったときだけ push する（ADR 0249）。
        """
        self.guard_before_push(task, branch, when)
        push = self.git(["push", "--force", REMOTE, f"{branch}:{branch}"], check=False)
        if push.returncode != 0:
            raise InfraError(f"`git push` が失敗しました: {tail(push.stderr).strip()}")

    def publish(self, task: Task, branch: str, title: str, body: str) -> int:
        """push して、PR を作る（既にあれば本文を直す）。PR の番号を返す。"""
        self.push_branch(task, branch, "push")
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
        """CI が終わるのを待つ。失敗したら指摘を返す。

        チェックが報告されないとき、ジョブが開始されなかったとき、`gh pr checks` の失敗が続いたときは止める。
        """
        self.say(f"PR #{pr} の CI を待つ")
        time.sleep(CI_INITIAL_WAIT)
        started = time.monotonic()
        gh_failures = 0
        while True:
            result = self.gh(["pr", "checks", str(pr), "--json", "name,state,bucket,link"], check=False)
            checks: List[Dict[str, Any]] = []
            if result.stdout.strip():
                try:
                    checks = json.loads(result.stdout)
                except ValueError:
                    raise InfraError(f"`gh pr checks` の出力を JSON として読めません: {tail(result.stderr).strip()}")
            elif result.returncode != 0 and "no checks reported" not in result.stderr:
                # GitHub の一時的な失敗（HTTP 504 など）では、待ってからやり直す。続いたときだけ止める（ADR 0252）
                gh_failures += 1
                if gh_failures >= CI_GH_MAX_FAILURES:
                    raise InfraError(
                        f"`gh pr checks` が {gh_failures} 回続けて失敗しました。PR #{pr} は開いたままです: {tail(result.stderr).strip()}"
                    )
                self.say(f"  `gh pr checks` が失敗したので、待ってからやり直す（{gh_failures} 回目）: {one_line(tail(result.stderr))[:200]}")
                time.sleep(CI_POLL_SECONDS)
                continue
            gh_failures = 0
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
                    self.stop_if_not_started(pr, failed)
                    return [self.ci_finding(failed)]
                if not pending:
                    return []
                if elapsed >= CI_TIMEOUT:
                    raise InfraError(f"PR #{pr} の CI が {CI_TIMEOUT} 秒たっても終わりません。PR は開いたままです。")
            time.sleep(CI_POLL_SECONDS)

    def stop_if_not_started(self, pr: int, failed: List[Dict[str, Any]]) -> None:
        """失敗したチェックに、ステップを 1 つも実行せずに終わったジョブがあれば、直しに回さずに止める（ADR 0251）。

        支払いの失敗や利用上限などで、GitHub がジョブを開始しなかったときである。コードを直しても通らない。
        ジョブの様子が読めないときは、見分けずに、これまでどおり CI の失敗として直しに回す。
        """
        jobs_by_run: Dict[str, List[Dict[str, Any]]] = {}
        names: List[str] = []
        reasons: List[str] = []
        for check in failed:
            link = check.get("link") or ""
            run = RUN_LINK_RE.search(link)
            job = JOB_LINK_RE.search(link)
            if not run or not job:
                continue
            if run.group(1) not in jobs_by_run:
                jobs_by_run[run.group(1)] = self.run_jobs(run.group(1))
            for item in jobs_by_run[run.group(1)]:
                steps = item.get("steps")
                if str(item.get("databaseId")) == job.group(1) and item.get("status") == "completed" and isinstance(steps, list) and not steps:
                    names.append(str(check.get("name")))
                    reasons += [r for r in self.job_annotations(job.group(1)) if r not in reasons]
        if names:
            why = "".join(f" 注釈: {r}" for r in reasons[:3]) or " 注釈はありません。"
            raise InfraError(
                f"PR #{pr} の CI のジョブ（{'、'.join(names)}）が、ステップを 1 つも実行せずに失敗しました。"
                f"GitHub がジョブを開始しなかったので、直しに回さずに止めます。PR は開いたままです。{why}"
            )

    def run_jobs(self, run_id: str) -> List[Dict[str, Any]]:
        """CI の run のジョブの一覧。読めなければ空にする。"""
        result = self.gh(["run", "view", run_id, "--json", "jobs"], check=False)
        try:
            data = json.loads(result.stdout) if result.returncode == 0 else None
        except ValueError:
            data = None
        jobs = data.get("jobs") if isinstance(data, dict) else None
        return [job for job in jobs if isinstance(job, dict)] if isinstance(jobs, list) else []

    def job_annotations(self, job_id: str) -> List[str]:
        """ジョブの失敗の注釈の文。読めなければ空にする。"""
        result = self.gh(["api", f"repos/{{owner}}/{{repo}}/check-runs/{job_id}/annotations"], check=False)
        try:
            data = json.loads(result.stdout) if result.returncode == 0 else None
        except ValueError:
            data = None
        if not isinstance(data, list):
            return []
        messages = [one_line(str(a.get("message") or ""))[:300] for a in data if isinstance(a, dict) and a.get("annotation_level") == "failure"]
        return [m for m in messages if m]

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
                if data.get("permission_denials") and not result.get("blocking_issues"):
                    # 権限で拒否されて進められないのは、タスクではなく仕組みの問題なので、行き詰まりにしない
                    reason = one_line(result.get("blocked_reason") or "")[:300]
                    raise InfraError(
                        f"claude（task{task.number}-implement{step}）が、権限で拒否された操作があり、進められないと答えました。"
                        f"禁止の一覧（{SETTINGS_IMPLEMENT} の deny）か、スキルのコマンドを見直してください。{reason}"
                    )
                return self.handle_blocked(task, branch, result, history)
            fixed = merge_unique(fixed, result.get("fixed_problems") or [])
            human = merge_unique(human, result.get("human_checks") or [])
            title = one_line(result.get("pr_title") or "") or title
            summary = (result.get("summary") or "").strip() or summary

            label = f"{step} 回目（{model}）"
            if data.get("reported_by"):
                history.append(
                    f"{label}: 結果を決まった形で返せなかったので、{data['reported_by']} で同じ会話の続きから結果だけを返させた"
                )
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

        # 前の回に push したコミットと同じままだと、push しても CI が走らず、前の回の CI の結果を読んでしまう。
        # 空のコミットを足して、この回の CI を走らせる。
        if self.remote_head(branch) == self.git(["rev-parse", "HEAD"]).stdout.strip():
            self.git(["commit", "--quiet", "--allow-empty", "-m", f"CI をやり直す（#{task.number}）\n\n{CO_AUTHOR}"])
            history.append(f"{label}: 前の回に push したコミットから変わらなかったので、空のコミットを足して CI をやり直した")
            self.say("  前の回から変わらなかったので、空のコミットで CI をやり直す")

        body = self.pr_body(task, summary, fixed, human, notes, history + [f"{label}: 機械のチェックとレビューを通った"])
        pr = self.publish(task, branch, title, body)
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
            # 行き詰まりの作業は、機械のチェックを通っていないことがある（衝突したとき、進められないと答えたときなど）
            self.push_branch(task, branch, "stuck")
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
        data = self.call_claude("triage", prompt, TRIAGE_MODEL, SETTINGS_TRIAGE, None, TRIAGE_TIMEOUT)
        if data.get("permission_denials"):
            # 仕分けが済んだかどうか分からないので、仕分け済みにせず、次の切れ目でもう一度だけ仕分ける（ADR 0194）
            again = [n for n in numbers if n not in self.triage_retried]
            self.triage_retried.update(again)
            self.triaged.update(n for n in numbers if n not in again)
            if again:
                self.say(
                    "  仕分けの会話で権限に拒否された操作があったので、次の切れ目で、もう一度だけ仕分ける: "
                    + "、".join(f"#{n}" for n in again)
                )
        else:
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
        self.check_engines()
        self.check_secret_scanner()
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
            print(f"  #{task.number} {task.shown_title()}: {reason}")
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
                condition = f"`{check['requires']}` があるとき"
                if "requires_script" in check:
                    condition = f"`{check['requires']}` の scripts に `{check['requires_script']}` があるとき"
                if check.get("stop_on_failure"):
                    condition = f"`{MAIN_REF}` に `{check['requires']}` があるとき。`{MAIN_REF}` の道具で実行し、失敗したら push せずに止まる。push の直前にも実行する"
                print("  機械のチェック: " + " ".join(check["argv"]) + f"（{condition}）")
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
            stuck = self.gh_json(["issue", "list", "--state", "open", "--label", LABEL_STUCK, "--limit", "200", "--json", "number,title,author"])
            human = self.gh_json(["issue", "list", "--state", "all", "--label", LABEL_HUMAN_CHECK, "--limit", "200", "--json", "number,title,author"])
            undecided = self.gh_json(["issue", "list", "--state", "open", "--label", LABEL_UNDECIDED, "--limit", "200", "--json", "number,title,author"])
            owner = self.repo_owner()
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
                # オーナー以外が書いた Issue は、タイトルも出さない（この出力を AI が読むことがあるため。ADR 0247）
                title = item["title"] if authored_by_owner(item, owner) else "（オーナー以外が書いた Issue なので、タイトルを出さない）"
                print(f"  #{item['number']} {title}")
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
