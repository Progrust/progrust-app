#!/usr/bin/env python3
"""tools/implement-loop.py のテスト。

実行:
    python3 -m unittest tools/tests/test_implement_loop.py

一時ディレクトリに git のリポジトリ（bare の origin と clone）を作り、PATH の先頭に置いた
偽の `gh` と偽の `claude` で、スクリプトを端から端まで動かす。GitHub にも Claude にも
つながない。偽のツールが返す内容は、実行ファイルの隣のファイルで決める。

標準ライブラリだけを使う。Python 3.9 以降で動く。
"""

from __future__ import annotations

import contextlib
import importlib.util
import io
import json
import os
import shutil
import stat
import subprocess
import sys
import tempfile
import unittest
from typing import Any, Dict, List, Optional

TOOLS_DIR = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
SCRIPT = os.path.join(TOOLS_DIR, "implement-loop.py")


def load_module():
    """スクリプトを、テストごとに新しいモジュールとして読み込む（定数を差し替えるため）。"""
    spec = importlib.util.spec_from_file_location("implement_loop", SCRIPT)
    module = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    saved = sys.dont_write_bytecode
    sys.dont_write_bytecode = True  # tools/ に __pycache__ を作らない
    try:
        spec.loader.exec_module(module)
    finally:
        sys.dont_write_bytecode = saved
    return module


FAKE_GH = r'''#!/usr/bin/env python3
"""偽の gh。状態は隣の gh-state.json に持ち、呼び出しを gh-calls.jsonl に残す。"""
import json, os, subprocess, sys

HERE = os.path.dirname(os.path.abspath(__file__))
STATE = os.path.join(HERE, "gh-state.json")
CALLS = os.path.join(HERE, "gh-calls.jsonl")
FLAGS_WITHOUT_VALUE = {"--squash", "--log-failed"}


def main():
    argv = sys.argv[1:]
    stdin = "" if sys.stdin.isatty() else sys.stdin.read()
    with open(CALLS, "a", encoding="utf-8") as handle:
        handle.write(json.dumps({"argv": argv, "stdin": stdin}, ensure_ascii=False) + "\n")
    with open(STATE, encoding="utf-8") as handle:
        state = json.load(handle)

    positional, flags = [], {}
    index = 0
    while index < len(argv):
        arg = argv[index]
        if arg.startswith("--"):
            if arg in FLAGS_WITHOUT_VALUE:
                flags.setdefault(arg, []).append("")
            else:
                index += 1
                flags.setdefault(arg, []).append(argv[index])
        else:
            positional.append(arg)
        index += 1
    command = " ".join(positional[:2])

    def flag(name, default=None):
        return flags[name][-1] if name in flags else default

    def save():
        with open(STATE, "w", encoding="utf-8") as handle:
            json.dump(state, handle, ensure_ascii=False, indent=1)

    def fail(message, code=1):
        sys.stderr.write(message + "\n")
        sys.exit(code)

    if command in state.get("fail", {}):
        fail(state["fail"][command])
    # fail_times のコマンドは、count の回数だけ失敗し、その後は成功する
    failing = state.get("fail_times", {}).get(command)
    if failing and failing["count"] > 0:
        failing["count"] -= 1
        save()
        fail(failing["message"])

    def comment_json(comment):
        # 文字列のコメントは、オーナーのアカウント（ループ）が書いたもの。辞書のコメントは、そのまま返す
        if isinstance(comment, str):
            return {"author": {"login": state["owner"]}, "authorAssociation": "OWNER", "body": comment}
        return comment

    def issue_json(issue, fields, comment_limit=None):
        # gh issue list --json comments は、Issue ごとに最初の 100 件のコメントしか返さない。gh issue view は全部返す
        author = issue.get("author", state["owner"])
        full = {
            "number": issue["number"],
            "title": issue["title"],
            "body": issue["body"],
            "state": issue["state"],
            "labels": [{"name": name} for name in issue["labels"]],
            "milestone": {"title": issue["milestone"]} if issue.get("milestone") else None,
            "comments": [comment_json(comment) for comment in issue.get("comments", [])][:comment_limit],
            "author": {"login": author} if author is not None else None,
        }
        return {key: full[key] for key in fields}

    def pr_json(pr):
        # gh pr list --head は、fork のブランチからの pull request も、ブランチの名前だけで返す
        author = pr.get("author", state["owner"])
        head_owner = pr.get("head_owner", state["owner"])
        return {
            "number": pr["number"],
            "headRefName": pr["head"],
            "isCrossRepository": bool(pr.get("cross", False)),
            "headRepositoryOwner": {"login": head_owner} if head_owner is not None else None,
            "author": {"login": author} if author is not None else None,
        }

    def get_issue(number):
        issue = state["issues"].get(str(number))
        if issue is None:
            fail(f"GraphQL: Could not resolve to an Issue with the number of {number}.")
        return issue

    if command == "issue list":
        wanted = flag("--state", "open")
        labels = flags.get("--label", [])
        fields = flag("--json").split(",")
        out = []
        for issue in sorted(state["issues"].values(), key=lambda i: -i["number"]):
            if wanted != "all" and issue["state"].lower() != wanted:
                continue
            if any(label not in issue["labels"] for label in labels):
                continue
            out.append(issue_json(issue, fields, comment_limit=100))
        print(json.dumps(out, ensure_ascii=False))
    elif command == "issue view":
        if int(positional[2]) in state.get("fail_view", []):
            fail("HTTP 502")
        print(json.dumps(issue_json(get_issue(positional[2]), flag("--json").split(",")), ensure_ascii=False))
    elif command == "issue edit":
        issue = get_issue(positional[2])
        for names in flags.get("--add-label", []):
            for name in names.split(","):
                if name not in issue["labels"]:
                    issue["labels"].append(name)
        for names in flags.get("--remove-label", []):
            for name in names.split(","):
                if name in issue["labels"]:
                    issue["labels"].remove(name)
        if "--body-file" in flags:
            issue["body"] = stdin
        save()
    elif command == "issue comment":
        get_issue(positional[2])["comments"].append(stdin)
        save()
    elif command == "issue close":
        issue = get_issue(positional[2])
        issue["state"] = "CLOSED"
        issue["close_reason"] = flag("--reason", "")
        save()
    elif command == "repo view":
        if state.get("owner_missing"):
            print(json.dumps({"owner": None}))
        else:
            print(json.dumps({"owner": {"login": state["owner"]}}))
    elif command == "pr list":
        head = flag("--head")
        fields = flag("--json").split(",")
        prs = sorted(state["prs"].values(), key=lambda pr: -pr["number"])
        # --author "@me" は、gh にログインしている人（ここではオーナー）が作ったものに絞る
        author = {"@me": state["owner"]}.get(flag("--author"), flag("--author"))
        out = [
            {key: pr_json(pr)[key] for key in fields} for pr in prs
            if pr["head"] == head and pr["state"] == "OPEN" and (author is None or pr.get("author", state["owner"]) == author)
        ]
        print(json.dumps(out[: int(flag("--limit", "30"))]))
    elif command == "pr create":
        number = state["next_pr"]
        state["next_pr"] += 1
        state["prs"][str(number)] = {
            "number": number, "head": flag("--head"), "base": flag("--base"),
            "title": flag("--title"), "body": stdin, "state": "OPEN",
        }
        save()
        print(f"https://github.com/example/repo/pull/{number}")
    elif command == "pr edit":
        pr = state["prs"][positional[2]]
        if "--title" in flags:
            pr["title"] = flag("--title")
        if "--body-file" in flags:
            pr["body"] = stdin
        save()
    elif command == "pr checks":
        pr = state["prs"][positional[2]]
        queue = state.get("checks_queue", [])
        checks = queue.pop(0) if queue else state.get("checks", [])
        save()
        if not checks:
            fail(f"no checks reported on the '{pr['head']}' branch")
        print(json.dumps(checks))
        buckets = {check["bucket"] for check in checks}
        sys.exit(8 if "pending" in buckets else 1 if buckets & {"fail", "cancel"} else 0)
    elif command == "pr merge":
        pr = state["prs"][positional[2]]
        scratch = state["scratch"]
        if not os.path.isdir(scratch):
            subprocess.run(["git", "clone", "--quiet", state["origin"], scratch], check=True)
        def git(*args):
            return subprocess.run(["git", "-C", scratch] + list(args), stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
        git("fetch", "--quiet", "origin")
        git("checkout", "--quiet", "-B", "main", "origin/main")
        merged = git("merge", "--squash", "origin/" + pr["head"])
        if merged.returncode != 0:
            git("reset", "--hard", "--quiet")
            fail("Pull request is not mergeable: " + merged.stderr)
        message = flag("--subject", pr["title"]) + "\n\n" + flag("--body", "")
        committed = git("commit", "--quiet", "-m", message)
        if committed.returncode != 0:
            fail("commit failed: " + committed.stdout + committed.stderr)
        pushed = git("push", "--quiet", "origin", "main")
        if pushed.returncode != 0:
            fail("push failed: " + pushed.stderr)
        pr["state"] = "MERGED"
        save()
    elif command == "run view":
        if "--json" in flags:
            print(json.dumps({"jobs": state.get("run_jobs", {}).get(positional[2], [])}))
        else:
            print(state.get("run_log", "テストが失敗した行"))
    elif positional[:1] == ["api"] and positional[1].endswith("/annotations"):
        job_id = positional[1].split("/")[-2]
        print(json.dumps(state.get("annotations", {}).get(job_id, [])))
    else:
        fail(f"偽の gh が知らないコマンド: {argv}", 2)


main()
'''

FAKE_CLAUDE = r'''#!/usr/bin/env python3
"""偽の claude。返す内容は隣の claude-script.json で決め、呼び出しを claude-calls.jsonl に残す。"""
import json, os, subprocess, sys, tempfile

HERE = os.path.dirname(os.path.abspath(__file__))
SCRIPT = os.path.join(HERE, "claude-script.json")
CALLS = os.path.join(HERE, "claude-calls.jsonl")


def main():
    argv = sys.argv[1:]
    def value(name):
        return argv[argv.index(name) + 1] if name in argv else None
    prompt = value("-p") or ""
    kind = "implement" if prompt.startswith("/implement-task") else "review" if prompt.startswith("/review-task") else "triage" if prompt.startswith("/triage-issues") else "other"
    with open(SCRIPT, encoding="utf-8") as handle:
        script = json.load(handle)
    count = script.get("count", 0) + 1
    script["count"] = count
    settings = value("--settings")
    settings_text = None
    if settings and os.path.isfile(settings):
        with open(settings, encoding="utf-8") as handle:
            settings_text = handle.read()
    with open(CALLS, "a", encoding="utf-8") as handle:
        handle.write(json.dumps({
            "kind": kind, "prompt": prompt, "model": value("--model"), "resume": value("--resume"),
            "settings": settings, "settings_text": settings_text, "cwd": os.getcwd(),
            "permission_mode": value("--permission-mode"), "has_schema": "--json-schema" in argv,
            "output_format": value("--output-format"),
        }, ensure_ascii=False) + "\n")
    queue = script.get(kind, [])
    entry = queue.pop(0) if queue else script.get(kind + "_default")
    with open(SCRIPT, "w", encoding="utf-8") as handle:
        json.dump(script, handle, ensure_ascii=False, indent=1)
    if entry is None:
        sys.stderr.write(f"偽の claude: {kind} の返す内容が残っていない\n")
        sys.exit(3)

    for path, content in (entry.get("files") or {}).items():
        os.makedirs(os.path.dirname(os.path.abspath(path)), exist_ok=True)
        with open(path, "w", encoding="utf-8") as handle:
            handle.write(content)
    for path in entry.get("delete") or []:
        if os.path.exists(path):
            os.remove(path)
    if entry.get("commit"):
        # 禁止をすり抜けて、claude がコミットしたことにする
        subprocess.run(["git", "add", "-A"], check=True)
        subprocess.run(["git", "commit", "--quiet", "--allow-empty", "-m", "claude のコミット"], check=True)
    if entry.get("push_main"):
        # ほかの作業で main が進んだことにする
        with tempfile.TemporaryDirectory() as tmp:
            subprocess.run(["git", "clone", "--quiet", script["origin"], tmp], check=True)
            for path, content in entry["push_main"].items():
                if content is None:
                    os.remove(os.path.join(tmp, path))
                    continue
                with open(os.path.join(tmp, path), "w", encoding="utf-8") as handle:
                    handle.write(content)
            subprocess.run(["git", "-C", tmp, "add", "-A"], check=True)
            subprocess.run(["git", "-C", tmp, "commit", "--quiet", "-m", "ほかの作業"], check=True)
            subprocess.run(["git", "-C", tmp, "push", "--quiet", "origin", "HEAD:main"], check=True)
    for command in entry.get("shell") or []:
        # 作業ツリーと共有する .git を書き換えるなど、禁止をすり抜ける操作をしたことにする
        subprocess.run(["sh", "-c", command], check=True)

    if "raw_stdout" in entry:
        sys.stdout.write(entry["raw_stdout"])
        sys.exit(entry.get("exit", 0))
    structured = entry.get("structured_output")
    out = {
        "type": "result",
        "subtype": entry.get("subtype", "success"),
        "is_error": bool(entry.get("is_error", False)),
        "result": entry.get("result", json.dumps(structured, ensure_ascii=False) if structured else "ok"),
        "session_id": value("--resume") or f"sess-{count}",
        "permission_denials": entry.get("permission_denials", []),
        "api_error_status": entry.get("api_error_status"),
    }
    if structured is not None:
        out["structured_output"] = structured
    print(json.dumps(out, ensure_ascii=False))
    sys.exit(entry.get("exit", 0))


main()
'''

CHECK_STUB = '''#!/usr/bin/env python3
import os, sys
if os.path.exists("FAIL_DOCS"):
    print("documents/x.md:1: link-target: FAIL_DOCS がある")
    sys.exit(1)
'''

SECRETS_STUB = '''#!/usr/bin/env python3
# 偽の tools/check-secrets.py。--repo のリポジトリの、push していないコミットか作業ツリーに LEAK があると失敗する。
# 呼び出しを、リポジトリの外のファイル（環境変数 SECRETS_CALLS）に、どこから実行されたかと一緒に残す。
import os, re, subprocess, sys
args = sys.argv[1:]
repo = args[args.index("--repo") + 1]
bases = [args[i + 1] for i, a in enumerate(args) if a == "--base"]
here = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
where = "main-tools" if os.path.basename(here) == "main-tools" else here
with open(os.environ["SECRETS_CALLS"], "a", encoding="utf-8") as handle:
    handle.write(" ".join("REPO" if a == repo else "SHA" if re.fullmatch("[0-9a-f]{40}", a) else a for a in args) + f" @{where}\\n")
if not os.path.isfile(os.path.join(here, ".gitleaks.toml")):
    print("設定が無い", file=sys.stderr)
    sys.exit(2)
os.chdir(repo)
if "--check-version" in args:
    version = os.environ.get("FAKE_GITLEAKS_VERSION", "8.30.1")
    if version != "8.30.1":
        print(f"check-secrets: gitleaks の版が {version} で、固定した版 8.30.1 と違います", file=sys.stderr)
        sys.exit(2)
    sys.exit(0)
log = subprocess.run(["git", "log", "-p", "HEAD", "--not"] + (bases or ["--remotes"]), stdout=subprocess.PIPE, text=True).stdout
tree = subprocess.run(["git", "diff", "HEAD"], stdout=subprocess.PIPE, text=True).stdout
added = [l for l in (log + tree).splitlines() if l.startswith("+") and not l.startswith("+++")]
if any("LEAK" in l for l in added):
    print("Finding:     REDACTED\\nFile:        a.txt\\nleaks found: 1")
    sys.exit(1)
print("no leaks found")
'''


def done(**extra: Any) -> Dict[str, Any]:
    result = {
        "status": "done", "pr_title": "機能を足す", "summary": "機能を足した。",
        "fixed_problems": [], "human_checks": [], "blocked_reason": "", "blocking_issues": [],
    }
    result.update(extra)
    return result


def review_pass(*notes: str) -> Dict[str, Any]:
    return {
        "structured_output": {
            "verdict": "pass", "summary": "問題なし。",
            "findings": [{"severity": "note", "file": "a.txt", "description": note} for note in notes],
        }
    }


def review_fail(*musts: str) -> Dict[str, Any]:
    return {
        "structured_output": {
            "verdict": "fail", "summary": "直す点がある。",
            "findings": [{"severity": "must", "file": "a.txt", "description": must} for must in musts],
        }
    }


PASSING = [{"name": "check", "state": "SUCCESS", "bucket": "pass", "link": "https://github.com/example/repo/actions/runs/1/job/2"}]
FAILING = [{"name": "check", "state": "FAILURE", "bucket": "fail", "link": "https://github.com/example/repo/actions/runs/7/job/8"}]
PENDING = [{"name": "check", "state": "IN_PROGRESS", "bucket": "pending", "link": ""}]


class LoopTestCase(unittest.TestCase):
    def setUp(self) -> None:
        self.tmp = tempfile.mkdtemp(prefix="implement-loop-test-")
        self.addCleanup(shutil.rmtree, self.tmp, ignore_errors=True)
        self.bin = os.path.join(self.tmp, "bin")
        self.origin = os.path.join(self.tmp, "origin.git")
        self.repo = os.path.join(self.tmp, "repo")
        self.worktree = os.path.join(self.tmp, "repo-loop")
        self.logs = os.path.join(self.tmp, "logs")
        os.makedirs(self.bin)

        self.saved_env = dict(os.environ)
        self.addCleanup(self.restore_env)
        os.environ["PATH"] = self.bin + os.pathsep + os.environ.get("PATH", "")
        os.environ.update(
            {
                "GIT_CONFIG_GLOBAL": os.devnull,
                "GIT_CONFIG_NOSYSTEM": "1",
                "GIT_AUTHOR_NAME": "tester", "GIT_AUTHOR_EMAIL": "tester@example.invalid",
                "GIT_COMMITTER_NAME": "tester", "GIT_COMMITTER_EMAIL": "tester@example.invalid",
            }
        )

        for name, source in (("gh", FAKE_GH), ("claude", FAKE_CLAUDE)):
            path = os.path.join(self.bin, name)
            with open(path, "w", encoding="utf-8") as handle:
                handle.write(source)
            os.chmod(path, os.stat(path).st_mode | stat.S_IXUSR)

        self.sh(["git", "init", "--quiet", "--bare", "--initial-branch=main", self.origin], cwd=self.tmp)
        self.sh(["git", "init", "--quiet", "--initial-branch=main", self.repo], cwd=self.tmp)
        self.sh(["git", "remote", "add", "origin", self.origin])
        files = {
            ".claude/loop-settings.json": '{"permissions": {"allow": ["Edit"]}}\n',
            ".claude/loop-review-settings.json": '{"permissions": {"allow": ["Read"]}}\n',
            ".claude/loop-triage-settings.json": '{"permissions": {"allow": ["Bash(gh issue *)"]}}\n',
            ".claude/skills/implement-task/SKILL.md": "実装\n",
            ".claude/skills/review-task/SKILL.md": "レビュー\n",
            ".claude/skills/triage-issues/SKILL.md": "仕分け\n",
            ".github/workflows/ci.yml": "name: ci\n",
            "CLAUDE.md": "決まり\n",
            "tools/check-documents.py": CHECK_STUB,
            "tools/design-system.py": "import sys\nsys.exit(0)\n",
            "shared.txt": "最初\n",
        }
        for path, content in files.items():
            full = os.path.join(self.repo, path)
            os.makedirs(os.path.dirname(full), exist_ok=True)
            with open(full, "w", encoding="utf-8") as handle:
                handle.write(content)
        self.sh(["git", "add", "-A"])
        self.sh(["git", "commit", "--quiet", "-m", "最初のコミット"])
        self.sh(["git", "push", "--quiet", "origin", "main"])

        self.state: Dict[str, Any] = {
            "issues": {}, "prs": {}, "next_pr": 100, "checks": PASSING, "checks_queue": [],
            "fail": {}, "origin": self.origin, "scratch": os.path.join(self.tmp, "merge-clone"),
            "owner": "repo-owner",
        }
        self.script: Dict[str, Any] = {"origin": self.origin, "implement": [], "review": [], "triage": []}

        self.mod = load_module()
        self.mod.CI_INITIAL_WAIT = 0
        self.mod.CI_POLL_SECONDS = 0.01
        self.mod.CI_APPEAR_TIMEOUT = 0.2
        self.mod.CI_TIMEOUT = 5

    def restore_env(self) -> None:
        os.environ.clear()
        os.environ.update(self.saved_env)

    # ---- 道具 ----

    def sh(self, argv: List[str], cwd: Optional[str] = None) -> str:
        result = subprocess.run(argv, cwd=cwd or self.repo, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
        if result.returncode != 0:
            raise AssertionError(f"{argv} が失敗: {result.stderr}")
        return result.stdout

    def add_issue(self, number: int, labels: List[str], body: str = "- 前提: なし\n", title: Optional[str] = None,
                  milestone: Optional[str] = None, state: str = "OPEN", author: Any = "repo-owner") -> None:
        """Issue を足す。author は書いた人の login（既定はリポジトリのオーナー）。None なら書いた人が読めない。"""
        self.state["issues"][str(number)] = {
            "number": number, "title": title or f"タスク{number}", "body": body, "labels": list(labels),
            "state": state, "milestone": milestone, "comments": [], "author": author,
        }

    def task(self, number: int, difficulty: str = "小", **kwargs: Any) -> None:
        labels = ["実装タスク", f"難易度:{difficulty}"] + kwargs.pop("extra_labels", [])
        self.add_issue(number, labels, **kwargs)

    def write_fakes(self) -> None:
        with open(os.path.join(self.bin, "gh-state.json"), "w", encoding="utf-8") as handle:
            json.dump(self.state, handle, ensure_ascii=False)
        with open(os.path.join(self.bin, "claude-script.json"), "w", encoding="utf-8") as handle:
            json.dump(self.script, handle, ensure_ascii=False)

    def run_loop(self, *extra: str) -> int:
        self.write_fakes()
        argv = ["--repo", self.repo, "--worktree", self.worktree, "--log-dir", self.logs] + list(extra)
        out = io.StringIO()
        with contextlib.redirect_stdout(out):
            code = self.mod.main(argv)
        self.output = out.getvalue()
        with open(os.path.join(self.bin, "gh-state.json"), encoding="utf-8") as handle:
            self.state = json.load(handle)
        return code

    def read_calls(self, name: str) -> List[Dict[str, Any]]:
        path = os.path.join(self.bin, name)
        if not os.path.exists(path):
            return []
        with open(path, encoding="utf-8") as handle:
            return [json.loads(line) for line in handle if line.strip()]

    def claude_calls(self, kind: Optional[str] = None) -> List[Dict[str, Any]]:
        calls = self.read_calls("claude-calls.jsonl")
        return [c for c in calls if kind is None or c["kind"] == kind]

    def gh_calls(self, command: str) -> List[Dict[str, Any]]:
        return [c for c in self.read_calls("gh-calls.jsonl") if " ".join(c["argv"][:2]) == command]

    def issue(self, number: int) -> Dict[str, Any]:
        return self.state["issues"][str(number)]

    def main_file(self, path: str) -> Optional[str]:
        result = subprocess.run(["git", "--git-dir", self.origin, "show", f"main:{path}"], stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
        return result.stdout if result.returncode == 0 else None

    def origin_branches(self) -> List[str]:
        out = self.sh(["git", "--git-dir", self.origin, "for-each-ref", "--format=%(refname:short)", "refs/heads"])
        return out.split()


class PureFunctionTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.mod = load_module()

    def test_parse_prerequisites(self) -> None:
        parse = self.mod.parse_prerequisites
        self.assertIsNone(parse("本文だけ"))
        self.assertEqual(parse("x\n- 前提: なし\ny"), [])
        self.assertEqual(parse("- 前提: #12, #13\n"), [12, 13])
        self.assertEqual(parse("- 前提：#7\n"), [7])

    def test_add_prerequisites(self) -> None:
        add = self.mod.add_prerequisites
        self.assertIsNone(add("本文だけ", [5]))
        self.assertEqual(add("a\n- 前提: なし\nb", [5]), "a\n- 前提: #5\nb")
        self.assertEqual(add("- 前提: #3\n", [3, 9]), "- 前提: #3, #9\n")

    def test_human_check_items(self) -> None:
        items = self.mod.human_check_items
        self.assertEqual(items("## 概要\nx\n"), [])
        self.assertEqual(items("## 人が確かめること\n\n- なし\n\n## 次\n- y\n"), [])
        self.assertEqual(items("## 人が確かめること\n\nなし。\n"), [])
        self.assertEqual(items("## 人が確かめること\n\n- [ ] 画面を見る\n- 音を聞く\n\n## 次\n- y\n"), ["画面を見る", "音を聞く"])

    def test_is_protected(self) -> None:
        protected = self.mod.is_protected
        for path in ("CLAUDE.md", ".claude/skills/x/SKILL.md", ".github/workflows/ci.yml", "documents/conventions/issues.md",
                     "tools/implement-loop.py", "tools/okf/validate.py", "tools/tests/test_implement_loop.py",
                     ".gitleaks.toml", ".gitleaksignore", "tools/check-secrets.py", "tools/tests/test_check_secrets.py"):
            self.assertTrue(protected(path), path)
        for path in ("src/main.ts", "documents/spec/overview.md", "tools/check-content.ts", "docs/CLAUDE.md", "src/.gitleaks.toml"):
            self.assertFalse(protected(path), path)

    def test_merge_unique(self) -> None:
        self.assertEqual(self.mod.merge_unique(["a"], ["b", "a", " ", "b"]), ["a", "b"])

    def test_authored_by_owner(self) -> None:
        by_owner = self.mod.authored_by_owner
        self.assertTrue(by_owner({"author": {"login": "Owner"}}, "Owner"))
        self.assertTrue(by_owner({"author": {"login": "owner"}}, "Owner"))  # login は大文字と小文字を区別しない
        self.assertFalse(by_owner({"author": {"login": "outsider"}}, "Owner"))
        # 書いた人が読めないものは、オーナーのものとして扱わない
        for raw in ({}, {"author": None}, {"author": {}}, {"author": {"login": ""}}, {"author": {"login": 3}}, None):
            self.assertFalse(by_owner(raw, "Owner"), raw)
        self.assertFalse(by_owner({"author": {"login": "Owner"}}, ""))  # オーナーが分からないときも

    def test_owner_comments(self) -> None:
        comments = [
            {"authorAssociation": "OWNER", "body": "オーナー"},
            {"authorAssociation": "NONE", "body": "外部の人"},
            {"authorAssociation": "CONTRIBUTOR", "body": "貢献した人"},
            {"authorAssociation": "owner", "body": "小文字"},
            {"body": "書いた人の関係が無い"},
            "文字列",
        ]
        self.assertEqual([c["body"] for c in self.mod.owner_comments(comments)], ["オーナー"])
        self.assertEqual(self.mod.owner_comments(None), [])
        self.assertEqual(self.mod.owner_comments("x"), [])

    def test_is_own_pr(self) -> None:
        own = {"number": 1, "headRefName": "task/1", "isCrossRepository": False,
               "headRepositoryOwner": {"login": "Owner"}, "author": {"login": "Owner"}}
        self.assertTrue(self.mod.is_own_pr(own, "task/1", "Owner"))
        self.assertFalse(self.mod.is_own_pr(own, "task/2", "Owner"))
        for change in (
            {"isCrossRepository": True},  # fork のブランチから
            {"isCrossRepository": None},
            {"headRepositoryOwner": {"login": "outsider"}},
            {"headRepositoryOwner": None},
            {"author": {"login": "outsider"}},  # オーナー以外が作った
            {"author": None},
        ):
            self.assertFalse(self.mod.is_own_pr(dict(own, **change), "task/1", "Owner"), change)
        missing = dict(own)
        del missing["isCrossRepository"]
        self.assertFalse(self.mod.is_own_pr(missing, "task/1", "Owner"))
        self.assertFalse(self.mod.is_own_pr(own, "task/1", ""))


class SelectionTest(LoopTestCase):
    def classify(self):
        self.write_fakes()
        loop = self.mod.Loop(self.mod.parse_args(["--repo", self.repo, "--worktree", self.worktree, "--log-dir", self.logs]))
        return loop.classify(loop.list_tasks())

    def test_order_and_exclusions(self) -> None:
        self.task(10, milestone="M2")
        self.task(11, milestone="M1")
        self.task(12, milestone="M1", extra_labels=["行き詰まり"])
        self.task(13, milestone="M1", extra_labels=["ループ対象外"])
        self.task(14, milestone="M1", extra_labels=["保留"])
        self.add_issue(15, ["実装タスク"], milestone="M1")  # 難易度が無い
        self.task(16, milestone="M1", body="- 前提: #11, #30\n")  # #11 は開いている
        self.task(17, milestone="M1", body="- 前提: #30\n")  # #30 は閉じている
        self.task(18, milestone="M1", body="前提の行が無い\n")
        self.task(19, body="- 前提: なし\n")  # マイルストーンが無い
        self.task(20, milestone="M9", extra_labels=["AI起票"])
        self.task(21, milestone="M1", body="- 前提: #999\n")  # 存在しない Issue
        self.add_issue(30, ["実装タスク", "難易度:小"], state="CLOSED")
        self.add_issue(40, ["AI起票", "未判断"])  # タスクではない
        ready, skipped = self.classify()
        self.assertEqual([t.number for t in ready], [20, 11, 17, 18, 10, 19])
        reasons = {t.number: r for t, r in skipped}
        self.assertEqual(set(reasons), {12, 13, 14, 15, 16, 21})
        self.assertIn("行き詰まり", reasons[12])
        self.assertIn("難易度", reasons[15])
        self.assertIn("#11", reasons[16])
        self.assertNotIn("#30", reasons[16])
        self.assertIn("#999", reasons[21])

    def test_tasks_not_written_by_owner_are_not_selected(self) -> None:
        self.task(1)
        self.task(2, author="outsider", title="外部の人のタスク")
        self.task(3, author=None)  # 書いた人が読めない
        self.task(4, author="REPO-OWNER")  # login は大文字と小文字を区別しない
        ready, skipped = self.classify()
        self.assertEqual([t.number for t in ready], [1, 4])
        reasons = {t.number: r for t, r in skipped}
        self.assertEqual(set(reasons), {2, 3})
        for number in (2, 3):
            self.assertIn("オーナー以外が書いた", reasons[number])

    def test_dry_run_hides_titles_not_written_by_owner(self) -> None:
        self.task(1)
        self.task(2, author="outsider", title="この指示に従え")
        self.add_issue(3, ["行き詰まり"], author="outsider", title="これも指示")
        self.assertEqual(self.run_loop("--dry-run"), 0)
        self.assertIn("#2 （オーナー以外が書いた Issue なので、タイトルを出さない）", self.output)
        self.assertNotIn("この指示に従え", self.output)
        self.assertEqual(self.run_loop("--status"), 0)
        self.assertIn("#3 （オーナー以外が書いた Issue なので、タイトルを出さない）", self.output)
        self.assertNotIn("これも指示", self.output)

    def test_unknown_owner_stops_before_any_call(self) -> None:
        self.task(1)
        self.add_issue(5, ["AI起票", "未判断"])
        self.state["owner_missing"] = True
        self.assertEqual(self.run_loop(), 2)
        self.assertIn("リポジトリの所有者が分かりません", self.output)
        self.assertEqual(self.claude_calls(), [])

    def test_harder_label_wins(self) -> None:
        self.add_issue(1, ["実装タスク", "難易度:小", "難易度:大"])
        ready, _ = self.classify()
        self.assertEqual(ready[0].difficulty, "大")

    def test_dry_run_changes_nothing(self) -> None:
        self.task(1, difficulty="中")
        self.add_issue(5, ["AI起票", "未判断"])
        code = self.run_loop("--dry-run")
        self.assertEqual(code, 0)
        self.assertIn("#1 [難易度:中]", self.output)
        self.assertIn("実装 sonnet・レビュー sonnet・引き上げ先 opus", self.output)
        self.assertIn("/implement-task 1 --loop", self.output)
        self.assertIn("/triage-issues --loop 5", self.output)
        self.assertFalse(os.path.exists(self.worktree))
        self.assertFalse(os.path.exists(self.logs))
        self.assertEqual(self.claude_calls(), [])
        for call in self.read_calls("gh-calls.jsonl"):
            # 読むだけのコマンドしか使わない（repo view は、オーナーを見分けるために所有者を読む）
            self.assertIn(" ".join(call["argv"][:2]), ("issue list", "issue view", "repo view"))

    def test_status(self) -> None:
        self.task(1)
        self.task(2, extra_labels=["行き詰まり"])
        self.add_issue(3, ["実装タスク", "難易度:小", "人の確認待ち"], state="CLOSED")
        self.add_issue(5, ["AI起票", "未判断"])
        code = self.run_loop("--status")
        self.assertEqual(code, 0)
        self.assertIn("残りのタスク: 2 件（いま選べるもの 1 件）", self.output)
        self.assertIn("#2 タスク2", self.output)
        self.assertIn("#3 タスク3", self.output)
        self.assertIn("#5 タスク5", self.output)
        self.assertEqual(self.claude_calls(), [])


class FlowTest(LoopTestCase):
    def test_pass_and_merge(self) -> None:
        self.task(1, body="- 前提: なし\n\n## 人が確かめること\n\n- 画面を見る\n")
        self.task(2, body="- 前提: #1\n\n## 人が確かめること\n\n- なし\n")
        self.script["implement"] = [
            {"structured_output": done(pr_title="A を足す", summary="A を足した。", fixed_problems=["誤記を直した"]), "files": {"a.txt": "A\n"}},
            {"structured_output": done(pr_title="B を足す"), "files": {"b.txt": "B\n"}},
        ]
        self.script["review"] = [review_pass("名前を短くできる"), review_pass()]
        code = self.run_loop()
        self.assertEqual(code, 0, self.output)
        self.assertIn("選べるタスクが無い", self.output)
        self.assertEqual(self.main_file("a.txt"), "A\n")
        self.assertEqual(self.main_file("b.txt"), "B\n")
        self.assertEqual(self.issue(1)["state"], "CLOSED")
        self.assertEqual(self.issue(2)["state"], "CLOSED")
        self.assertIn("人の確認待ち", self.issue(1)["labels"])
        self.assertNotIn("人の確認待ち", self.issue(2)["labels"])
        self.assertEqual(self.issue(1)["comments"], [])

        pr = self.state["prs"]["100"]
        self.assertEqual(pr["state"], "MERGED")
        self.assertEqual(pr["title"], "A を足す")
        self.assertEqual(pr["head"], "task/1")
        self.assertTrue(pr["body"].startswith("Closes #1\n"))
        for text in ("A を足した。", "誤記を直した", "名前を短くできる", "1 回目（haiku）: 機械のチェック、レビュー、CI を通った"):
            self.assertIn(text, pr["body"])
        self.assertEqual(self.origin_branches(), ["main"])  # タスクのブランチは消してある

        implement = self.claude_calls("implement")
        self.assertEqual([c["prompt"] for c in implement], ["/implement-task 1 --loop", "/implement-task 2 --loop"])
        self.assertEqual([c["model"] for c in implement], ["haiku", "haiku"])
        review = self.claude_calls("review")
        self.assertEqual([c["prompt"] for c in review], ["/review-task 1 --loop", "/review-task 2 --loop"])
        self.assertEqual([c["model"] for c in review], ["sonnet", "sonnet"])
        for call in self.claude_calls():
            self.assertEqual(os.path.realpath(call["cwd"]), os.path.realpath(self.worktree))
            self.assertEqual(call["permission_mode"], "bypassPermissions")
            self.assertEqual(call["output_format"], "json")
            self.assertTrue(call["has_schema"])
            # 権限の設定は、作業ツリーの外に置いた main のコピーを渡す
            self.assertFalse(os.path.realpath(call["settings"]).startswith(os.path.realpath(self.worktree) + os.sep))
        self.assertIn('"Edit"', implement[0]["settings_text"])
        self.assertIn('"Read"', review[0]["settings_text"])
        # ユーザーのチェックアウトは触らない
        self.assertEqual(self.sh(["git", "status", "--porcelain"]), "")
        self.assertEqual(self.sh(["git", "rev-parse", "--abbrev-ref", "HEAD"]).strip(), "main")
        self.assertTrue(os.path.isfile(os.path.join(self.logs, os.listdir(self.logs)[0], "loop.log")))

    def test_max_tasks(self) -> None:
        self.task(1)
        self.task(2)
        self.script["implement"] = [{"structured_output": done(), "files": {"a.txt": "A\n"}}]
        self.script["review"] = [review_pass()]
        code = self.run_loop("--max-tasks", "1")
        self.assertEqual(code, 0, self.output)
        self.assertIn("--max-tasks の 1 件に達した", self.output)
        self.assertEqual(self.issue(2)["state"], "OPEN")
        self.assertEqual(len(self.claude_calls("implement")), 1)

    def test_review_finding_then_fix(self) -> None:
        self.task(1)
        self.script["implement"] = [
            {"structured_output": done(fixed_problems=["一つ目"], human_checks=["音を聞く"]), "files": {"a.txt": "A1\n"}},
            {"structured_output": done(pr_title="直した題", fixed_problems=["一つ目", "二つ目"], human_checks=["音を聞く", "色を見る"]), "files": {"a.txt": "A2\n"}},
        ]
        self.script["review"] = [review_fail("境界の値を扱っていない"), review_pass()]
        code = self.run_loop()
        self.assertEqual(code, 0, self.output)
        self.assertEqual(self.main_file("a.txt"), "A2\n")
        implement = self.claude_calls("implement")
        self.assertEqual(len(implement), 2)
        self.assertIsNone(implement[0]["resume"])
        self.assertEqual(implement[1]["resume"], "sess-1")
        self.assertTrue(implement[1]["prompt"].startswith("/implement-task 1 --loop fix\n\n"))
        self.assertIn("境界の値を扱っていない", implement[1]["prompt"])
        pr = self.state["prs"]["100"]
        self.assertEqual(pr["title"], "直した題")
        self.assertEqual(pr["body"].count("一つ目"), 1)
        self.assertIn("二つ目", pr["body"])
        self.assertIn("1 回目（haiku）: レビューで直す指摘が 1 件あった", pr["body"])
        # 実装中に足した「人が確かめること」は、重複なしでコメントに書き、ラベルを付ける
        self.assertIn("人の確認待ち", self.issue(1)["labels"])
        comments = self.issue(1)["comments"]
        self.assertEqual(len(comments), 1)
        self.assertRegex(comments[0], r"^人が確かめることを足した（\d{4}-\d{2}-\d{2}、Claude の判断）。\n\n- 音を聞く\n- 色を見る$")

    def test_check_failure_then_fix(self) -> None:
        self.task(1)
        self.script["implement"] = [
            {"structured_output": done(), "files": {"a.txt": "A\n", "FAIL_DOCS": "x\n"}},
            {"structured_output": done(), "delete": ["FAIL_DOCS"]},
        ]
        self.script["review"] = [review_pass()]
        code = self.run_loop()
        self.assertEqual(code, 0, self.output)
        implement = self.claude_calls("implement")
        self.assertEqual(len(implement), 2)
        self.assertIn("機械のチェックの失敗: documents の検査", implement[1]["prompt"])
        self.assertIn("FAIL_DOCS がある", implement[1]["prompt"])
        self.assertEqual(len(self.claude_calls("review")), 1)  # チェックが落ちた回はレビューしない
        self.assertIsNone(self.main_file("FAIL_DOCS"))
        self.assertEqual(self.main_file("a.txt"), "A\n")

    def write_npm_fake(self, scripts: Dict[str, str]) -> str:
        """package.json と、引数を記録し、FAIL_CONTENT があると content:check で失敗する偽の npm を置く。"""
        with open(os.path.join(self.repo, "package.json"), "w", encoding="utf-8") as handle:
            json.dump({"name": "x", "scripts": scripts}, handle)
        self.sh(["git", "add", "-A"])
        self.sh(["git", "commit", "--quiet", "-m", "package.json"])
        self.sh(["git", "push", "--quiet", "origin", "main"])
        log = os.path.join(self.bin, "npm-calls.txt")
        path = os.path.join(self.bin, "npm")
        with open(path, "w", encoding="utf-8") as handle:
            handle.write(
                "#!/bin/sh\n"
                f'echo "$*" >> "{log}"\n'
                'if [ "$2" = "content:check" ] && [ -e FAIL_CONTENT ]; then\n'
                '  echo "content/x/lesson.json: エラー: FAIL_CONTENT がある"\n'
                "  exit 1\n"
                "fi\n"
            )
        os.chmod(path, os.stat(path).st_mode | stat.S_IXUSR)
        return log

    def test_content_check_runs_and_failure_is_fixed(self) -> None:
        log = self.write_npm_fake({"check": "x", "content:check": "x"})
        self.task(1)
        self.script["implement"] = [
            {"structured_output": done(), "files": {"a.txt": "A\n", "FAIL_CONTENT": "x\n"}},
            {"structured_output": done(), "delete": ["FAIL_CONTENT"]},
        ]
        self.script["review"] = [review_pass()]
        code = self.run_loop()
        self.assertEqual(code, 0, self.output)
        with open(log, encoding="utf-8") as handle:
            calls = handle.read().splitlines()
        self.assertEqual(calls, ["run check", "run content:check"] * 2)
        implement = self.claude_calls("implement")
        self.assertEqual(len(implement), 2)
        self.assertIn("機械のチェックの失敗: 教材チェックツール", implement[1]["prompt"])
        self.assertIn("`npm run content:check` が終了コード 1 で終わった", implement[1]["prompt"])
        self.assertIn("FAIL_CONTENT がある", implement[1]["prompt"])
        self.assertEqual(len(self.claude_calls("review")), 1)  # チェックが落ちた回はレビューしない
        self.assertEqual(self.main_file("a.txt"), "A\n")

    def test_content_check_skipped_without_script(self) -> None:
        log = self.write_npm_fake({"check": "x"})
        self.task(1)
        self.script["implement"] = [{"structured_output": done(), "files": {"a.txt": "A\n", "FAIL_CONTENT": "x\n"}}]
        self.script["review"] = [review_pass()]
        code = self.run_loop()
        self.assertEqual(code, 0, self.output)
        with open(log, encoding="utf-8") as handle:
            self.assertEqual(handle.read().splitlines(), ["run check"])
        self.assertEqual(len(self.claude_calls("implement")), 1)

    def test_stuck_after_escalation_and_continue(self) -> None:
        self.task(1)
        self.task(2)
        self.script["implement"] = [{"structured_output": done(), "files": {"a.txt": f"A{i}\n"}} for i in range(4)]
        self.script["implement"].append({"structured_output": done(), "files": {"b.txt": "B\n"}})
        self.script["review"] = [review_fail("まだ違う")] * 4 + [review_pass()]
        code = self.run_loop()
        self.assertEqual(code, 0, self.output)
        implement = self.claude_calls("implement")
        self.assertEqual([c["model"] for c in implement], ["haiku", "haiku", "haiku", "sonnet", "haiku"])
        self.assertEqual([c["resume"] for c in implement], [None, "sess-1", "sess-1", None, None])
        issue = self.issue(1)
        self.assertEqual(issue["state"], "OPEN")
        self.assertIn("行き詰まり", issue["labels"])
        self.assertEqual(len(issue["comments"]), 1)
        self.assertRegex(issue["comments"][0], r"^行き詰まりにした（\d{4}-\d{2}-\d{2}、Claude の判断）。")
        for text in ("4 回目（sonnet）: レビューで直す指摘が 1 件あった", "`task/1`（push してある）", "まだ違う"):
            self.assertIn(text, issue["comments"][0])
        self.assertIsNone(self.main_file("a.txt"))
        self.assertIn("task/1", self.origin_branches())  # 途中の作業はブランチに残す
        self.assertEqual(self.state["prs"].get("100", {}).get("head"), "task/2")
        # 次のタスクへ進む
        self.assertEqual(self.issue(2)["state"], "CLOSED")
        self.assertEqual(self.main_file("b.txt"), "B\n")

    def test_large_task_has_no_escalation(self) -> None:
        self.task(1, difficulty="大")
        self.script["implement"] = [{"structured_output": done(), "files": {"a.txt": f"A{i}\n"}} for i in range(3)]
        self.script["review"] = [review_fail("違う")] * 3
        code = self.run_loop()
        self.assertEqual(code, 0, self.output)
        self.assertEqual([c["model"] for c in self.claude_calls("implement")], ["opus", "opus", "opus"])
        self.assertEqual([c["model"] for c in self.claude_calls("review")], ["opus", "opus", "opus"])
        self.assertIn("行き詰まり", self.issue(1)["labels"])

    def test_stop_after_three_stuck_in_a_row(self) -> None:
        for number in (1, 2, 3, 4):
            self.task(number, difficulty="大")
        self.script["implement_default"] = {"structured_output": done(), "files": {"a.txt": "A\n"}}
        self.script["review_default"] = review_fail("違う")
        code = self.run_loop()
        self.assertEqual(code, 1, self.output)
        self.assertIn("行き詰まりが 3 件続いた", self.output)
        self.assertEqual([n for n in (1, 2, 3, 4) if "行き詰まり" in self.issue(n)["labels"]], [1, 2, 3])

    def test_claude_failure_stops_without_label(self) -> None:
        self.task(1)
        self.task(2)
        self.script["implement"] = [
            {"exit": 1, "is_error": True, "result": "usage limit reached", "api_error_status": 429, "files": {"a.txt": "A\n"}},
        ]
        code = self.run_loop()
        self.assertEqual(code, 2, self.output)
        self.assertIn("仕組みの失敗", self.output)
        self.assertIn("429", self.output)
        self.assertEqual(self.issue(1)["labels"], ["実装タスク", "難易度:小"])
        self.assertEqual(self.issue(1)["comments"], [])
        self.assertEqual(self.issue(1)["state"], "OPEN")
        self.assertEqual(len(self.claude_calls()), 1)  # 次のタスクへ進まない
        self.assertEqual(self.state["prs"], {})
        self.assertEqual(self.origin_branches(), ["main"])
        # 作業ツリーの変更は捨ててある
        self.assertEqual(self.sh(["git", "status", "--porcelain"], cwd=self.worktree), "")
        self.assertFalse(os.path.exists(os.path.join(self.worktree, "a.txt")))

    def test_is_error_with_exit_zero_stops(self) -> None:
        self.task(1)
        self.script["implement"] = [{"is_error": True, "result": "error_max_turns"}]
        self.assertEqual(self.run_loop(), 2)
        self.assertEqual(self.issue(1)["labels"], ["実装タスク", "難易度:小"])

    def test_failure_reason_only_in_errors_is_shown(self) -> None:
        self.task(1)
        failed = {
            "type": "result",
            "subtype": "error_max_structured_output_retries",
            "is_error": True,
            "errors": ["Failed to provide valid structured output after 5 attempts"],
        }
        self.script["implement"] = [{"raw_stdout": json.dumps(failed), "exit": 1}]
        self.assertEqual(self.run_loop(), 2)
        self.assertIn("Failed to provide valid structured output after 5 attempts", self.output)
        self.assertEqual(self.issue(1)["labels"], ["実装タスク", "難易度:小"])

    def test_structured_output_failure_is_reported_by_escalated_model(self) -> None:
        self.task(1)
        self.script["implement"] = [
            {"exit": 1, "is_error": True, "subtype": "error_max_structured_output_retries", "result": "", "files": {"a.txt": "A\n"}},
        ]
        self.script["other"] = [{"structured_output": done()}]
        self.script["review"] = [review_pass()]
        code = self.run_loop()
        self.assertEqual(code, 0, self.output)
        implement = self.claude_calls("implement")
        report = self.claude_calls("other")
        self.assertEqual([(c["model"], c["resume"]) for c in implement], [("haiku", None)])
        # 作業を捨てずに、引き上げるモデルで同じ会話の続きから結果だけを返させる
        self.assertEqual([(c["model"], c["resume"], c["has_schema"]) for c in report], [("sonnet", "sess-1", True)])
        self.assertEqual(self.main_file("a.txt"), "A\n")
        self.assertEqual(self.issue(1)["state"], "CLOSED")
        self.assertIn("結果を決まった形で返せなかったので、sonnet で同じ会話の続きから結果だけを返させた", self.state["prs"]["100"]["body"])

    def test_structured_output_failure_after_escalated_report_stops(self) -> None:
        self.task(1)
        failed = {"exit": 1, "is_error": True, "subtype": "error_max_structured_output_retries", "result": ""}
        self.script["implement"] = [dict(failed, files={"a.txt": "A\n"})]
        self.script["other"] = [failed]
        self.assertEqual(self.run_loop(), 2)
        self.assertIn("結果を決まった形で返せずに終わりました", self.output)
        self.assertEqual(len(self.claude_calls("other")), 1)
        self.assertEqual(self.issue(1)["labels"], ["実装タスク", "難易度:小"])
        self.assertEqual(self.sh(["git", "status", "--porcelain"], cwd=self.worktree), "")

    def test_structured_output_failure_without_escalation_stops(self) -> None:
        self.task(1, difficulty="大")
        self.script["implement"] = [{"exit": 1, "is_error": True, "subtype": "error_max_structured_output_retries", "result": ""}]
        self.assertEqual(self.run_loop(), 2)
        self.assertIn("結果を決まった形で返せずに終わりました", self.output)
        self.assertEqual(self.claude_calls("other"), [])
        self.assertEqual(self.issue(1)["labels"], ["実装タスク", "難易度:大"])

    def test_unreadable_output_stops(self) -> None:
        self.task(1)
        self.script["implement"] = [{"raw_stdout": "これは JSON ではない"}]
        self.assertEqual(self.run_loop(), 2)
        self.assertIn("JSON として読めません", self.output)
        self.assertEqual(self.issue(1)["labels"], ["実装タスク", "難易度:小"])

    def test_review_failure_stops_without_label(self) -> None:
        self.task(1)
        self.script["implement"] = [{"structured_output": done(), "files": {"a.txt": "A\n"}}]
        self.script["review"] = [{"exit": 1, "is_error": True, "result": "overloaded"}]
        self.assertEqual(self.run_loop(), 2)
        self.assertEqual(self.issue(1)["labels"], ["実装タスク", "難易度:小"])
        self.assertEqual(self.state["prs"], {})

    def test_gh_failure_stops_without_label(self) -> None:
        self.task(1)
        self.state["fail"] = {"pr create": "HTTP 502"}
        self.script["implement"] = [{"structured_output": done(), "files": {"a.txt": "A\n"}}]
        self.script["review"] = [review_pass()]
        self.assertEqual(self.run_loop(), 2)
        self.assertIn("HTTP 502", self.output)
        self.assertEqual(self.issue(1)["labels"], ["実装タスク", "難易度:小"])
        self.assertIsNone(self.main_file("a.txt"))

    def test_protected_paths_are_reverted_without_fix_round(self) -> None:
        self.task(1)
        self.script["implement"] = [
            {
                "structured_output": done(),
                "files": {
                    "a.txt": "A\n",
                    "CLAUDE.md": "書き換えた\n",
                    ".claude/loop-settings.json": '{"permissions": {"allow": ["Bash"]}}\n',
                    ".claude/skills/new/SKILL.md": "新しいスキル\n",
                    "tools/implement-loop.py": "print('x')\n",
                },
            },
        ]
        self.script["review"] = [review_pass()]
        code = self.run_loop()
        self.assertEqual(code, 0, self.output)
        self.assertEqual(len(self.claude_calls("implement")), 1)  # 直しに回さない
        self.assertEqual(self.main_file("a.txt"), "A\n")
        self.assertEqual(self.main_file("CLAUDE.md"), "決まり\n")
        self.assertIn('"Edit"', self.main_file(".claude/loop-settings.json"))
        self.assertIsNone(self.main_file(".claude/skills/new/SKILL.md"))
        self.assertIsNone(self.main_file("tools/implement-loop.py"))
        body = self.state["prs"]["100"]["body"]
        self.assertIn("保護するパスを変えていたので、スクリプトが元に戻した", body)
        for path in ("`CLAUDE.md`", "`.claude/loop-settings.json`", "`.claude/skills/new/SKILL.md`", "`tools/implement-loop.py`"):
            self.assertIn(path, body)
        # レビューに渡す権限の設定は、書き換えられる前の main のもの
        self.assertIn('"Read"', self.claude_calls("review")[0]["settings_text"])

    def test_only_protected_changes_count_as_no_change(self) -> None:
        self.task(1, difficulty="大")
        self.script["implement_default"] = {"structured_output": done(), "files": {"CLAUDE.md": "書き換えた\n"}}
        code = self.run_loop()
        self.assertEqual(code, 0, self.output)
        self.assertIn("行き詰まり", self.issue(1)["labels"])
        self.assertIn("変更が無かった", self.issue(1)["comments"][0])
        self.assertEqual(self.claude_calls("review"), [])
        self.assertEqual(self.main_file("CLAUDE.md"), "決まり\n")

    def test_protected_path_left_before_push_is_not_merged(self) -> None:
        self.task(1)
        self.script["implement"] = [{"structured_output": done(), "files": {"a.txt": "A\n", "CLAUDE.md": "書き換えた\n"}}]
        self.script["review"] = [review_pass()]
        self.mod.Loop.revert_protected = lambda loop, paths: None  # 戻す処理が効かなかったことにする
        code = self.run_loop()
        self.assertEqual(code, 0, self.output)
        self.assertIn("行き詰まり", self.issue(1)["labels"])
        self.assertIn("保護するパスの変更が残っている", self.issue(1)["comments"][0])
        self.assertEqual(self.state["prs"], {})
        self.assertEqual(self.main_file("CLAUDE.md"), "決まり\n")
        self.assertEqual(len(self.claude_calls("implement")), 1)

    def test_blocked_adds_prerequisite(self) -> None:
        self.task(1, body="## 概要\n\nx\n\n- 前提: なし\n")
        self.add_issue(50, ["AI起票", "未判断", "仕様"])
        self.script["implement"] = [
            {"structured_output": done(status="blocked", blocked_reason="戻る先が決まっていない。", blocking_issues=[50, 1]), "files": {"a.txt": "A\n"}},
        ]
        self.script["triage_default"] = {"result": "仕分けた"}
        code = self.run_loop()
        self.assertEqual(code, 0, self.output)
        issue = self.issue(1)
        self.assertEqual(issue["body"], "## 概要\n\nx\n\n- 前提: #50\n")
        self.assertEqual(issue["state"], "OPEN")
        self.assertNotIn("行き詰まり", issue["labels"])
        self.assertRegex(issue["comments"][0], r"^判断待ちにした（\d{4}-\d{2}-\d{2}、Claude の判断）。")
        self.assertIn("戻る先が決まっていない。", issue["comments"][0])
        self.assertEqual(self.state["prs"], {})
        self.assertEqual(self.origin_branches(), ["main"])
        self.assertEqual(len(self.claude_calls("implement")), 1)  # 同じ実行では選び直さない
        self.assertIn("判断待ち: 1 件", self.output)

    def test_blocked_without_issue_or_line_is_stuck(self) -> None:
        self.task(1, difficulty="大")
        self.task(2, difficulty="大", body="前提の行が無い\n")
        self.script["implement"] = [
            {"structured_output": done(status="blocked", blocked_reason="分からない。")},
            {"structured_output": done(status="blocked", blocked_reason="決まっていない。", blocking_issues=[50])},
        ]
        code = self.run_loop()
        self.assertEqual(code, 0, self.output)
        for number, text in ((1, "分からない。"), (2, "決まっていない。")):
            self.assertIn("行き詰まり", self.issue(number)["labels"])
            self.assertIn(text, self.issue(number)["comments"][0])
            self.assertIn("変更が無いので push していない", self.issue(number)["comments"][0])
        self.assertEqual(self.issue(2)["body"], "前提の行が無い\n")

    def test_blocked_by_permission_denial_stops_without_label(self) -> None:
        self.task(1)
        self.task(2)
        denial = [{"tool_name": "Bash", "tool_input": {"command": "gh issue view 1; git branch --show-current"}}]
        self.script["implement"] = [
            {"structured_output": done(status="blocked", blocked_reason="Bash が許可されない。"), "permission_denials": denial, "files": {"a.txt": "A\n"}},
        ]
        code = self.run_loop()
        self.assertEqual(code, 2, self.output)
        self.assertIn("仕組みの失敗", self.output)
        self.assertIn("権限で拒否された操作があり", self.output)
        self.assertEqual(self.issue(1)["labels"], ["実装タスク", "難易度:小"])
        self.assertEqual(self.issue(1)["comments"], [])
        self.assertEqual(len(self.claude_calls()), 1)  # 次のタスクへ進まない
        self.assertEqual(self.origin_branches(), ["main"])
        self.assertFalse(os.path.exists(os.path.join(self.worktree, "a.txt")))

    def test_commit_by_claude_stops_without_label(self) -> None:
        self.task(1)
        self.task(2)
        self.script["implement"] = [
            {"structured_output": done(), "files": {"a.txt": "A\n"}, "commit": True},
        ]
        code = self.run_loop()
        self.assertEqual(code, 2, self.output)
        self.assertIn("仕組みの失敗", self.output)
        self.assertIn("HEAD が変わりました", self.output)
        self.assertEqual(self.issue(1)["labels"], ["実装タスク", "難易度:小"])
        self.assertEqual(self.issue(1)["comments"], [])
        self.assertEqual(len(self.claude_calls()), 1)  # レビューにも次のタスクにも進まない
        self.assertEqual(self.origin_branches(), ["main"])

    def test_blocked_with_issue_and_denial_waits_for_decision(self) -> None:
        self.task(1, body="## 概要\n\nx\n\n- 前提: なし\n")
        self.add_issue(50, ["AI起票", "未判断", "仕様"])
        denial = [{"tool_name": "Bash", "tool_input": {"command": "git branch --show-current"}}]
        self.script["implement"] = [
            {"structured_output": done(status="blocked", blocked_reason="決まっていない。", blocking_issues=[50]), "permission_denials": denial},
        ]
        self.script["triage_default"] = {"result": "仕分けた"}
        code = self.run_loop()
        self.assertEqual(code, 0, self.output)
        self.assertEqual(self.issue(1)["body"], "## 概要\n\nx\n\n- 前提: #50\n")
        self.assertNotIn("行き詰まり", self.issue(1)["labels"])

    def test_no_ci_checks_means_no_merge(self) -> None:
        self.task(1)
        self.task(2)
        self.state["checks"] = []
        self.script["implement"] = [{"structured_output": done(), "files": {"a.txt": "A\n"}}]
        self.script["review"] = [review_pass()]
        code = self.run_loop()
        self.assertEqual(code, 2, self.output)
        self.assertIn("CI のチェックが 1 つも報告されません", self.output)
        self.assertEqual(self.state["prs"]["100"]["state"], "OPEN")
        self.assertEqual(self.gh_calls("pr merge"), [])
        self.assertIsNone(self.main_file("a.txt"))
        self.assertEqual(self.issue(1)["labels"], ["実装タスク", "難易度:小"])
        self.assertEqual(self.issue(1)["state"], "OPEN")
        self.assertEqual(len(self.claude_calls("implement")), 1)

    def test_ci_appears_late_and_pending_then_pass(self) -> None:
        self.task(1)
        self.mod.CI_APPEAR_TIMEOUT = 5
        self.state["checks_queue"] = [[], PENDING, PENDING]
        self.script["implement"] = [{"structured_output": done(), "files": {"a.txt": "A\n"}}]
        self.script["review"] = [review_pass()]
        code = self.run_loop()
        self.assertEqual(code, 0, self.output)
        self.assertEqual(len(self.gh_calls("pr checks")), 4)
        self.assertEqual(self.main_file("a.txt"), "A\n")

    def test_ci_failure_then_fix_reuses_pull_request(self) -> None:
        self.task(1)
        self.state["checks_queue"] = [FAILING]
        self.state["run_log"] = "test_x が失敗した"
        self.script["implement"] = [
            {"structured_output": done(), "files": {"a.txt": "A1\n"}},
            {"structured_output": done(), "files": {"a.txt": "A2\n"}},
        ]
        self.script["review"] = [review_pass(), review_pass()]
        code = self.run_loop()
        self.assertEqual(code, 0, self.output)
        implement = self.claude_calls("implement")
        self.assertIn("## CI の失敗", implement[1]["prompt"])
        self.assertIn("test_x が失敗した", implement[1]["prompt"])
        self.assertEqual(len(self.gh_calls("pr create")), 1)
        self.assertEqual(list(self.state["prs"]), ["100"])
        self.assertEqual(self.state["prs"]["100"]["state"], "MERGED")
        self.assertEqual(self.main_file("a.txt"), "A2\n")
        self.assertIn("1 回目（haiku）: CI で失敗した", self.state["prs"]["100"]["body"])
        self.assertNotIn("空のコミット", self.state["prs"]["100"]["body"])

    def test_ci_failure_with_unchanged_fix_reruns_ci_with_empty_commit(self) -> None:
        # 直す側がタスクと関係しない失敗と見て変えなかったときも、前の回の CI の結果を使わずに、CI をやり直す
        self.task(1)
        self.state["checks_queue"] = [FAILING]
        self.script["implement"] = [
            {"structured_output": done(), "files": {"a.txt": "A\n"}},
            {"structured_output": done(summary="タスクと関係しない失敗なので変えなかった。"), "files": {"a.txt": "A\n"}},
        ]
        self.script["review"] = [review_pass(), review_pass()]
        heads: List[str] = []
        publish = self.mod.Loop.publish

        def record_head(loop: Any, task: Any, branch: str, title: str, body: str) -> int:
            number = publish(loop, task, branch, title, body)
            heads.append(loop.remote_head(branch))
            return number

        self.mod.Loop.publish = record_head
        self.addCleanup(setattr, self.mod.Loop, "publish", publish)
        code = self.run_loop()
        self.assertEqual(code, 0, self.output)
        self.assertEqual(len(heads), 2)
        self.assertNotEqual(heads[0], heads[1])
        self.assertEqual(len(self.gh_calls("pr checks")), 2)
        self.assertEqual(self.state["prs"]["100"]["state"], "MERGED")
        self.assertEqual(self.main_file("a.txt"), "A\n")
        self.assertIn("2 回目（haiku）: 前の回に push したコミットから変わらなかったので、空のコミットを足して CI をやり直した", self.state["prs"]["100"]["body"])

    def test_ci_failure_until_stuck_leaves_pull_request_open(self) -> None:
        self.task(1, difficulty="大")
        self.state["checks"] = FAILING
        self.script["implement_default"] = {"structured_output": done(), "files": {"a.txt": "A\n"}}
        self.script["review_default"] = review_pass()
        code = self.run_loop()
        self.assertEqual(code, 0, self.output)
        self.assertIn("行き詰まり", self.issue(1)["labels"])
        self.assertIn("PR #100 が開いたままになっている", self.issue(1)["comments"][0])
        # 直しで変わらなかった回も、前の回の結果を使わずに、空のコミットで CI をやり直してから失敗としている
        self.assertIn("2 回目（opus）: 前の回に push したコミットから変わらなかったので、空のコミットを足して CI をやり直した", self.issue(1)["comments"][0])
        self.assertIn("3 回目（opus）: 前の回に push したコミットから変わらなかったので、空のコミットを足して CI をやり直した", self.issue(1)["comments"][0])
        self.assertEqual(self.state["prs"]["100"]["state"], "OPEN")
        self.assertEqual(self.gh_calls("pr merge"), [])
        self.assertIsNone(self.main_file("a.txt"))

    def test_ci_job_not_started_stops_without_fix_round(self) -> None:
        # 支払いの失敗などで GitHub がジョブを開始しなかったときは、コードの失敗として直しに回さず、仕組みの失敗として止まる
        self.task(1)
        self.task(2)
        self.state["checks"] = FAILING
        self.state["run_jobs"] = {"7": [{"databaseId": 8, "name": "check", "status": "completed", "conclusion": "failure", "steps": []}]}
        message = "The job was not started because recent account payments have failed or your spending limit needs to be increased."
        self.state["annotations"] = {"8": [{"annotation_level": "failure", "message": message}]}
        self.script["implement"] = [{"structured_output": done(), "files": {"a.txt": "A\n"}}]
        self.script["review"] = [review_pass()]
        code = self.run_loop()
        self.assertEqual(code, 2, self.output)
        self.assertIn("ステップを 1 つも実行せずに失敗しました", self.output)
        self.assertIn(message, self.output)
        self.assertEqual(len(self.claude_calls("implement")), 1)
        self.assertEqual(self.issue(1)["labels"], ["実装タスク", "難易度:小"])
        self.assertEqual(self.issue(1)["comments"], [])
        self.assertEqual(self.state["prs"]["100"]["state"], "OPEN")
        self.assertEqual(self.gh_calls("pr merge"), [])
        self.assertIsNone(self.main_file("a.txt"))

    def test_ci_job_with_steps_goes_to_fix_round(self) -> None:
        # ステップを実行して失敗したジョブは、これまでどおり CI の失敗として直しに回す
        self.task(1)
        self.state["checks_queue"] = [FAILING]
        self.state["run_jobs"] = {"7": [{"databaseId": 8, "name": "check", "status": "completed", "conclusion": "failure", "steps": [{"name": "Set up job"}]}]}
        self.script["implement"] = [
            {"structured_output": done(), "files": {"a.txt": "A1\n"}},
            {"structured_output": done(), "files": {"a.txt": "A2\n"}},
        ]
        self.script["review"] = [review_pass(), review_pass()]
        code = self.run_loop()
        self.assertEqual(code, 0, self.output)
        self.assertIn("## CI の失敗", self.claude_calls("implement")[1]["prompt"])
        self.assertEqual(self.main_file("a.txt"), "A2\n")

    def test_transient_pr_checks_failure_is_retried(self) -> None:
        # CI を待つ間の GitHub の一時的な失敗では止まらず、待ってからやり直す
        self.task(1)
        self.state["fail_times"] = {"pr checks": {"count": 2, "message": "HTTP 504: 504 Gateway Timeout (https://api.github.com/graphql)"}}
        self.script["implement"] = [{"structured_output": done(), "files": {"a.txt": "A\n"}}]
        self.script["review"] = [review_pass()]
        code = self.run_loop()
        self.assertEqual(code, 0, self.output)
        self.assertEqual(len(self.gh_calls("pr checks")), 3)
        self.assertEqual(self.state["prs"]["100"]["state"], "MERGED")
        self.assertEqual(self.main_file("a.txt"), "A\n")

    def test_persistent_pr_checks_failure_stops_without_label(self) -> None:
        self.task(1)
        self.state["fail"] = {"pr checks": "HTTP 504: 504 Gateway Timeout"}
        self.script["implement"] = [{"structured_output": done(), "files": {"a.txt": "A\n"}}]
        self.script["review"] = [review_pass()]
        code = self.run_loop()
        self.assertEqual(code, 2, self.output)
        self.assertIn(f"`gh pr checks` が {self.mod.CI_GH_MAX_FAILURES} 回続けて失敗しました", self.output)
        self.assertEqual(len(self.gh_calls("pr checks")), self.mod.CI_GH_MAX_FAILURES)
        self.assertEqual(self.issue(1)["labels"], ["実装タスク", "難易度:小"])
        self.assertEqual(self.state["prs"]["100"]["state"], "OPEN")
        self.assertIsNone(self.main_file("a.txt"))

    def test_conflict_restarts_once_from_new_main(self) -> None:
        self.task(1)
        self.script["implement"] = [
            {"structured_output": done(), "files": {"shared.txt": "タスクの変更\n"}, "push_main": {"shared.txt": "ほかの変更\n"}},
            {"structured_output": done(), "files": {"shared.txt": "ほかの変更\nタスクの変更\n"}},
        ]
        self.script["review"] = [review_pass()]
        code = self.run_loop()
        self.assertEqual(code, 0, self.output)
        implement = self.claude_calls("implement")
        self.assertEqual([c["prompt"] for c in implement], ["/implement-task 1 --loop", "/implement-task 1 --loop"])
        self.assertEqual([c["resume"] for c in implement], [None, None])
        self.assertEqual(self.main_file("shared.txt"), "ほかの変更\nタスクの変更\n")
        self.assertIn("最初からやり直した", self.state["prs"]["100"]["body"])
        self.assertNotIn("行き詰まり", self.issue(1)["labels"])

    def test_second_conflict_is_stuck(self) -> None:
        self.task(1)
        self.script["implement"] = [
            {"structured_output": done(), "files": {"shared.txt": "タスクの変更\n"}, "push_main": {"shared.txt": "ほかの変更 1\n"}},
            {"structured_output": done(), "files": {"shared.txt": "タスクの変更\n"}, "push_main": {"shared.txt": "ほかの変更 2\n"}},
        ]
        code = self.run_loop()
        self.assertEqual(code, 0, self.output)
        self.assertEqual(len(self.claude_calls("implement")), 2)
        self.assertIn("行き詰まり", self.issue(1)["labels"])
        self.assertIn("また衝突した", self.issue(1)["comments"][0])
        self.assertEqual(self.main_file("shared.txt"), "ほかの変更 2\n")

    def test_main_moving_without_conflict_is_merged_in(self) -> None:
        self.task(1)
        self.script["implement"] = [
            {"structured_output": done(), "files": {"a.txt": "A\n"}, "push_main": {"other.txt": "ほかの変更\n"}},
        ]
        self.script["review"] = [review_pass()]
        code = self.run_loop()
        self.assertEqual(code, 0, self.output)
        self.assertEqual(self.main_file("a.txt"), "A\n")
        self.assertEqual(self.main_file("other.txt"), "ほかの変更\n")
        self.assertEqual(len(self.claude_calls("implement")), 1)

    def test_triage_runs_for_untriaged_ai_issues(self) -> None:
        self.add_issue(5, ["AI起票", "未判断", "仕様"])
        self.add_issue(6, ["AI起票", "仕様", "保留"])
        self.add_issue(7, ["AI起票", "不具合", "実装タスク", "難易度:小", "未判断"])
        self.task(8)
        self.script["triage_default"] = {"result": "仕分けた", "files": {"stray.txt": "仕分けが書いた\n"}}
        self.script["implement"] = [
            {"structured_output": done(summary="不具合を直した。", fixed_problems=["近くの誤記"]), "files": {"a.txt": "A\n"}},
            {"structured_output": done(), "files": {"b.txt": "B\n"}},
        ]
        self.script["review"] = [review_pass(), review_pass()]
        code = self.run_loop()
        self.assertEqual(code, 0, self.output)
        triage = self.claude_calls("triage")
        self.assertEqual(len(triage), 1)  # 同じ Issue を、同じ実行で 2 回仕分けない
        self.assertEqual(triage[0]["prompt"], "/triage-issues --loop 5")
        self.assertEqual(triage[0]["model"], "sonnet")
        self.assertIn("gh issue", triage[0]["settings_text"])
        self.assertFalse(triage[0]["has_schema"])
        self.assertIn("仕分けの後に作業ツリーが変わっていた", self.output)
        self.assertIsNone(self.main_file("stray.txt"))
        # AI起票 のタスクは、ほかのタスクより先に選び、閉じるときにコメントを書いて `未判断` を外す
        self.assertEqual([c["prompt"] for c in self.claude_calls("implement")], ["/implement-task 7 --loop", "/implement-task 8 --loop"])
        issue = self.issue(7)
        self.assertEqual(issue["state"], "CLOSED")
        self.assertNotIn("未判断", issue["labels"])
        self.assertRegex(issue["comments"][0], r"^直したので閉じる（\d{4}-\d{2}-\d{2}、Claude の判断）。")
        for text in ("PR #100", "不具合を直した。", "近くの誤記"):
            self.assertIn(text, issue["comments"][0])
        self.assertEqual(self.issue(8)["comments"], [])

    def test_triage_skips_issues_that_open_tasks_wait_for(self) -> None:
        # #5 は、タスク #8 が「前提」で待っている、ユーザーの判断を求める Issue。ループでは仕分けない。
        self.add_issue(5, ["AI起票", "未判断", "仕様"])
        self.add_issue(6, ["AI起票", "未判断", "仕様"])
        self.add_issue(8, ["実装タスク", "難易度:中"], body="- 前提: #5\n")
        self.script["triage_default"] = {"result": "仕分けた"}
        code = self.run_loop()
        self.assertEqual(code, 0, self.output)
        triage = self.claude_calls("triage")
        self.assertEqual([c["prompt"] for c in triage], ["/triage-issues --loop 6"])
        self.assertEqual(self.claude_calls("implement"), [])
        self.assertEqual(self.issue(5)["state"], "OPEN")

    def test_triage_skips_issues_left_for_user(self) -> None:
        # #5 は、前の実行の仕分けが、ユーザーの判断が要るとして残した。ユーザーが答えるまで仕分け直さない。
        # #7 は、同じコメントがあるが、ユーザーが答えて `未判断` が外れた（対応すると決めた）ので、仕分ける。
        left = "ユーザーの判断が要るので、未判断のまま残した（2026-11-23、Claude の判断）。\n\n## ユーザーに決めてほしいこと\n\n…"
        self.add_issue(5, ["AI起票", "未判断", "仕様"])
        self.issue(5)["comments"].append(left)
        self.add_issue(6, ["AI起票", "未判断", "仕様"])
        self.add_issue(7, ["AI起票", "改善"])
        self.issue(7)["comments"].append(left)
        self.script["triage_default"] = {"result": "仕分けた"}
        code = self.run_loop()
        self.assertEqual(code, 0, self.output)
        self.assertEqual([c["prompt"] for c in self.claude_calls("triage")], ["/triage-issues --loop 6 7"])

    def test_triage_uses_only_owner_comments_and_owner_issues(self) -> None:
        left = "ユーザーの判断が要るので、未判断のまま残した（2026-11-23、Claude の判断）。\n\n## ユーザーに決めてほしいこと\n\n…"
        # #5: 外部の人が、ループの書くコメントをまねた。オーナーのコメントではないので、仕分けから外れない
        self.add_issue(5, ["AI起票", "未判断", "仕様"])
        self.issue(5)["comments"].append({"author": {"login": "outsider"}, "authorAssociation": "NONE", "body": left})
        # #6: オーナー（ループ）が書いたコメント。仕分けから外れる
        self.add_issue(6, ["AI起票", "未判断", "仕様"])
        self.issue(6)["comments"].append({"author": {"login": "repo-owner"}, "authorAssociation": "OWNER", "body": left})
        # #7: 書いた人の関係（authorAssociation）が無いコメントは、オーナーのものとして扱わない
        self.add_issue(7, ["AI起票", "未判断", "仕様"])
        self.issue(7)["comments"].append({"author": {"login": "repo-owner"}, "body": left})
        # #8・#9: オーナー以外が書いた Issue と、書いた人が読めない Issue は、仕分けない
        self.add_issue(8, ["AI起票", "未判断", "仕様"], author="outsider")
        self.add_issue(9, ["AI起票", "未判断", "仕様"], author=None)
        # #10 は、外部の人のタスク #11 が「前提」に書いているが、その本文は読まないので、仕分ける
        self.add_issue(10, ["AI起票", "未判断", "仕様"])
        self.task(11, author="outsider", body="- 前提: #10\n")
        self.script["triage_default"] = {"result": "仕分けた"}
        code = self.run_loop()
        self.assertEqual(code, 0, self.output)
        self.assertEqual([c["prompt"] for c in self.claude_calls("triage")], ["/triage-issues --loop 5 7 10"])
        self.assertEqual(self.claude_calls("implement"), [])  # 外部の人のタスクは実装しない

    def test_owner_comment_after_100_outsider_comments_is_seen(self) -> None:
        # gh issue list は最初の 100 件のコメントしか返さない。外部の人のコメント 100 件で、
        # ループの「ユーザーに回した」コメントを押し出されても、見落とさない
        left = "ユーザーの判断が要るので、未判断のまま残した（2026-11-23、Claude の判断）。\n\n…"
        noise = [{"author": {"login": "outsider"}, "authorAssociation": "NONE", "body": f"雑音{i}"} for i in range(100)]
        self.add_issue(5, ["AI起票", "未判断", "仕様"])
        self.issue(5)["comments"] = noise + [left]
        self.add_issue(6, ["AI起票", "未判断", "仕様"])
        self.issue(6)["comments"] = list(noise)
        # #7 は、コメントを読めないので、この回は仕分けない（安全な側に倒す）
        self.add_issue(7, ["AI起票", "未判断", "仕様"])
        self.state["fail_view"] = [7]
        self.script["triage_default"] = {"result": "仕分けた"}
        code = self.run_loop()
        self.assertEqual(code, 0, self.output)
        self.assertEqual([c["prompt"] for c in self.claude_calls("triage")], ["/triage-issues --loop 6"])
        self.assertIn("#7 のコメントを読めなかった", self.output)

    def test_prerequisite_written_by_outsider_does_not_unblock(self) -> None:
        # 前提の Issue を外部の人が書いていると、その人が自分で閉じられるので、閉じていても済んだとしない
        self.add_issue(30, ["仕様"], state="CLOSED", author="outsider")
        self.add_issue(31, ["仕様"], state="CLOSED")
        self.task(1, body="- 前提: #30\n")
        self.task(2, body="- 前提: #31\n")
        self.write_fakes()
        loop = self.mod.Loop(self.mod.parse_args(["--repo", self.repo, "--worktree", self.worktree, "--log-dir", self.logs]))
        ready, skipped = loop.classify(loop.list_tasks())
        self.assertEqual([t.number for t in ready], [2])
        self.assertEqual([(t.number, r) for t, r in skipped], [(1, "前提の #30 がオーナー以外が書いた Issue である")])

    def test_own_pull_request_is_found_behind_many_fork_pull_requests(self) -> None:
        # fork から同じ名前のブランチで pull request を 100 件より多く出され、絞らない一覧から押し出されても、
        # 直しの回に自分の pull request を見つけて使い続ける
        self.task(1)
        self.state["next_pr"] = 1
        for number in range(200, 301):
            self.state["prs"][str(number)] = {"number": number, "head": "task/1", "base": "main", "title": "外部", "body": "外部",
                                              "state": "OPEN", "cross": True, "author": "outsider", "head_owner": "outsider"}
        self.state["checks_queue"] = [FAILING]
        self.script["implement"] = [
            {"structured_output": done(), "files": {"a.txt": "A1\n"}},
            {"structured_output": done(), "files": {"a.txt": "A2\n"}},
        ]
        self.script["review"] = [review_pass(), review_pass()]
        code = self.run_loop()
        self.assertEqual(code, 0, self.output)
        self.assertEqual(len(self.gh_calls("pr create")), 1)
        self.assertEqual(self.state["prs"]["1"]["state"], "MERGED")
        self.assertEqual({c["argv"][2] for c in self.gh_calls("pr merge")}, {"1"})

    def test_pull_requests_from_forks_or_others_are_not_used(self) -> None:
        # 外部の人が、fork のブランチ task/1 から pull request を出した。同じリポジトリのブランチでも、
        # オーナー以外が作った pull request は使わない。ループは自分の pull request を作ってマージする。
        self.state["prs"]["50"] = {"number": 50, "head": "task/1", "base": "main", "title": "外部", "body": "外部",
                                   "state": "OPEN", "cross": True, "author": "outsider", "head_owner": "outsider"}
        self.state["prs"]["51"] = {"number": 51, "head": "task/1", "base": "main", "title": "別の人", "body": "別の人",
                                   "state": "OPEN", "author": "someone"}
        self.task(1)
        self.state["checks_queue"] = [FAILING]
        self.script["implement"] = [
            {"structured_output": done(), "files": {"a.txt": "A1\n"}},
            {"structured_output": done(), "files": {"a.txt": "A2\n"}},
        ]
        self.script["review"] = [review_pass(), review_pass()]
        code = self.run_loop()
        self.assertEqual(code, 0, self.output)
        self.assertEqual(self.state["prs"]["100"]["state"], "MERGED")
        self.assertEqual(self.main_file("a.txt"), "A2\n")
        self.assertEqual(len(self.gh_calls("pr create")), 1)  # 直しの回も、自分の pull request を使い続ける
        for number, text in (("50", "外部"), ("51", "別の人")):
            self.assertEqual(self.state["prs"][number]["state"], "OPEN")
            self.assertEqual(self.state["prs"][number]["title"], text)
            self.assertEqual(self.state["prs"][number]["body"], text)
        for command in ("pr edit", "pr merge", "pr checks"):
            self.assertEqual({c["argv"][2] for c in self.gh_calls(command)}, {"100"}, command)

    def test_stuck_comment_does_not_name_pull_request_from_fork(self) -> None:
        self.state["prs"]["50"] = {"number": 50, "head": "task/1", "base": "main", "title": "外部", "body": "外部",
                                   "state": "OPEN", "cross": True, "author": "outsider", "head_owner": "outsider"}
        self.task(1, difficulty="大")
        self.script["implement_default"] = {"structured_output": done(), "files": {"a.txt": "A\n"}}
        self.script["review_default"] = review_fail("違う")
        code = self.run_loop()
        self.assertEqual(code, 0, self.output)
        self.assertIn("行き詰まり", self.issue(1)["labels"])
        self.assertNotIn("PR #50", self.issue(1)["comments"][0])

    def test_triage_with_permission_denial_is_retried_once(self) -> None:
        self.add_issue(5, ["AI起票", "未判断", "仕様"])
        self.task(8)
        self.task(9)
        denial = [{"tool_name": "Bash", "tool_input": {"command": "gh api repos"}}]
        self.script["triage_default"] = {"result": "仕分けた", "permission_denials": denial}
        self.script["implement"] = [
            {"structured_output": done(), "files": {"a.txt": "A\n"}},
            {"structured_output": done(), "files": {"b.txt": "B\n"}},
        ]
        self.script["review"] = [review_pass(), review_pass()]
        code = self.run_loop()
        self.assertEqual(code, 0, self.output)
        # 始めたとき（拒否された）と、#8 の後（もう一度だけ）。#9 の後は仕分けない
        self.assertEqual([c["prompt"] for c in self.claude_calls("triage")], ["/triage-issues --loop 5"] * 2)
        self.assertIn("もう一度だけ仕分ける: #5", self.output)

    def write_engine_fakes(self, node: str, npm: str) -> None:
        with open(os.path.join(self.repo, "package.json"), "w", encoding="utf-8") as handle:
            json.dump({"name": "x", "engines": {"node": "~25.9", "npm": "~11.12"}}, handle)
        self.sh(["git", "add", "-A"])
        self.sh(["git", "commit", "--quiet", "-m", "package.json"])
        self.sh(["git", "push", "--quiet", "origin", "main"])
        for name, version in (("node", node), ("npm", npm)):
            path = os.path.join(self.bin, name)
            with open(path, "w", encoding="utf-8") as handle:
                handle.write(f"#!/bin/sh\necho {version}\n")
            os.chmod(path, os.stat(path).st_mode | stat.S_IXUSR)

    def test_engine_mismatch_stops_before_any_call(self) -> None:
        self.task(1)
        self.add_issue(5, ["AI起票", "未判断"])
        self.write_engine_fakes("v26.13.1", "11.20.0")
        self.assertEqual(self.run_loop(), 2)
        self.assertIn("`node` の版が 26.13.1 で、package.json の engines（~25.9）の系列に合いません", self.output)
        self.assertEqual(self.claude_calls(), [])

    def test_engine_series_match_starts(self) -> None:
        self.add_issue(5, ["AI起票", "未判断"])
        self.write_engine_fakes("v25.9.3", "11.12.1")
        self.script["triage_default"] = {"result": "仕分けた"}
        self.assertEqual(self.run_loop(), 0, self.output)
        self.assertEqual(len(self.claude_calls("triage")), 1)

    def test_triage_failure_stops(self) -> None:
        self.add_issue(5, ["AI起票", "未判断"])
        self.task(8)
        self.script["triage"] = [{"exit": 1, "is_error": True, "result": "limit"}]
        self.assertEqual(self.run_loop(), 2)
        self.assertEqual(self.claude_calls("implement"), [])

    def test_missing_required_files_stops_before_any_call(self) -> None:
        self.task(1)
        self.sh(["git", "rm", "--quiet", ".claude/loop-triage-settings.json"])
        self.sh(["git", "commit", "--quiet", "-m", "消す"])
        self.sh(["git", "push", "--quiet", "origin", "main"])
        self.assertEqual(self.run_loop(), 2)
        self.assertIn(".claude/loop-triage-settings.json", self.output)
        self.assertEqual(self.claude_calls(), [])

    def test_missing_ci_definition_stops_before_any_call(self) -> None:
        self.task(1)
        self.sh(["git", "rm", "--quiet", ".github/workflows/ci.yml"])
        self.sh(["git", "commit", "--quiet", "-m", "消す"])
        self.sh(["git", "push", "--quiet", "origin", "main"])
        self.assertEqual(self.run_loop(), 2)
        self.assertIn("CI の定義がありません", self.output)
        self.assertEqual(self.claude_calls(), [])

    # ---- 秘密の情報の検査（ADR 0249） ----

    def write_secrets_stub(self) -> str:
        """偽の tools/check-secrets.py と .gitleaks.toml を main に入れる。呼び出しの記録のファイルを返す。"""
        path = os.path.join(self.repo, "tools", "check-secrets.py")
        with open(path, "w", encoding="utf-8") as handle:
            handle.write(SECRETS_STUB)
        with open(os.path.join(self.repo, ".gitleaks.toml"), "w", encoding="utf-8") as handle:
            handle.write("[extend]\nuseDefault = true\n")
        self.sh(["git", "add", "-A"])
        self.sh(["git", "commit", "--quiet", "-m", "check-secrets"])
        self.sh(["git", "push", "--quiet", "origin", "main"])
        calls = os.path.join(self.tmp, "secrets-calls.txt")
        os.environ["SECRETS_CALLS"] = calls
        return calls

    def read_lines(self, path: str) -> List[str]:
        if not os.path.exists(path):
            return []
        with open(path, encoding="utf-8") as handle:
            return handle.read().splitlines()

    def test_secret_check_runs_before_push_and_passes(self) -> None:
        calls = self.write_secrets_stub()
        self.task(1)
        self.script["implement"] = [{"structured_output": done(), "files": {"a.txt": "A\\n"}}]
        self.script["review"] = [review_pass()]
        self.assertEqual(self.run_loop(), 0, self.output)
        # 始めるとき、機械のチェック、push の直前。どれも main から取り出した道具で実行する
        self.assertEqual(self.read_lines(calls), ["--repo REPO --base SHA --check-version @main-tools", "--repo REPO --base SHA @main-tools", "--repo REPO --base SHA @main-tools"])
        self.assertEqual(self.main_file("a.txt"), "A\\n")

    def test_secret_found_stops_without_push_or_fix_round(self) -> None:
        self.write_secrets_stub()
        self.task(1)
        self.task(2)
        self.script["implement"] = [{"structured_output": done(), "files": {"a.txt": "LEAK\\n"}}]
        self.script["review"] = [review_pass()]
        self.assertEqual(self.run_loop(), 2, self.output)
        self.assertIn("仕組みの失敗: 秘密の情報の検査", self.output)
        self.assertIn("push せずに止めます", self.output)
        self.assertEqual(len(self.claude_calls("implement")), 1)  # 直しに回さない
        self.assertEqual(self.claude_calls("review"), [])
        self.assertEqual(self.origin_branches(), ["main"])
        self.assertEqual(self.state["prs"], {})
        self.assertEqual(self.issue(1)["labels"], ["実装タスク", "難易度:小"])
        self.assertEqual(self.issue(1)["comments"], [])
        self.assertEqual(self.sh(["git", "status", "--porcelain"], cwd=self.worktree), "")

    def test_secret_in_stuck_work_is_not_pushed(self) -> None:
        # 衝突した作業は、機械のチェックを通らずに行き詰まりになる。push の前に確かめる
        self.write_secrets_stub()
        self.task(1)
        self.script["implement"] = [
            {"structured_output": done(), "files": {"shared.txt": "LEAK\\n"}, "push_main": {"shared.txt": "ほかの変更 1\\n"}},
            {"structured_output": done(), "files": {"shared.txt": "LEAK\\n"}, "push_main": {"shared.txt": "ほかの変更 2\\n"}},
        ]
        self.assertEqual(self.run_loop(), 2, self.output)
        self.assertIn("push せずに止めます", self.output)
        self.assertEqual(self.origin_branches(), ["main"])
        self.assertNotIn("行き詰まり", self.issue(1)["labels"])
        self.assertEqual(self.issue(1)["comments"], [])

    def test_gitleaks_version_mismatch_stops_before_any_call(self) -> None:
        calls = self.write_secrets_stub()
        os.environ["FAKE_GITLEAKS_VERSION"] = "8.29.0"
        self.task(1)
        self.add_issue(5, ["AI起票", "未判断"])
        self.assertEqual(self.run_loop(), 2)
        self.assertIn("tools/check-secrets.py --check-version`（`origin/main` のもの）が失敗したので、始めません", self.output)
        self.assertIn("固定した版 8.30.1 と違います", self.output)
        self.assertEqual(self.claude_calls(), [])
        self.assertEqual(self.read_lines(calls), ["--repo REPO --base SHA --check-version @main-tools"])

    def test_stuck_push_uses_main_tool_even_if_task_rewrote_it(self) -> None:
        # (A) 2 回衝突した作業が、検査の道具と設定を、何もせずに通るものに書き換えていた
        self.write_secrets_stub()
        self.task(1)
        files = {"shared.txt": "LEAK\n", "tools/check-secrets.py": "import sys\nsys.exit(0)\n", ".gitleaks.toml": ""}
        self.script["implement"] = [
            {"structured_output": done(), "files": files, "push_main": {"shared.txt": "ほかの変更 1\n"}},
            {"structured_output": done(), "files": files, "push_main": {"shared.txt": "ほかの変更 2\n"}},
        ]
        self.assertEqual(self.run_loop(), 2, self.output)
        self.assertIn("push せずに止めます", self.output)
        self.assertEqual(self.origin_branches(), ["main"])
        self.assertNotIn("行き詰まり", self.issue(1)["labels"])

    def test_blocked_push_uses_main_tool_even_if_task_deleted_it(self) -> None:
        # (B) 進められないと答えた作業が、検査の道具と設定を消していた
        self.write_secrets_stub()
        self.task(1, difficulty="大")
        self.script["implement"] = [
            {"structured_output": done(status="blocked", blocked_reason="分からない。"),
             "files": {"a.txt": "LEAK\n"}, "delete": ["tools/check-secrets.py", ".gitleaks.toml"]},
        ]
        self.assertEqual(self.run_loop(), 2, self.output)
        self.assertIn("push せずに止めます", self.output)
        self.assertEqual(self.origin_branches(), ["main"])
        self.assertEqual(self.state["prs"], {})

    def test_blocked_work_without_secret_is_pushed_after_check(self) -> None:
        calls = self.write_secrets_stub()
        self.task(1, difficulty="大")
        self.script["implement"] = [
            {"structured_output": done(status="blocked", blocked_reason="分からない。"),
             "files": {"a.txt": "A\n"}, "delete": ["tools/check-secrets.py"]},
        ]
        self.assertEqual(self.run_loop(), 0, self.output)
        self.assertIn("task/1", self.origin_branches())
        self.assertEqual(self.read_lines(calls)[-1], "--repo REPO --base SHA @main-tools")

    def test_replaced_main_tool_is_not_used(self) -> None:
        # (R1) claude が、origin/main の検査の道具の blob を、何もせず 0 で終わるものに置き換えた（refs/replace）
        self.write_secrets_stub()
        self.task(1)
        fake = os.path.join(self.tmp, "fake-tool.py")
        with open(fake, "w", encoding="utf-8") as handle:
            handle.write("import sys\nsys.exit(0)\n")
        replace = f'git replace "$(git rev-parse origin/main:tools/check-secrets.py)" "$(git hash-object -w {fake})"'
        self.script["implement"] = [{"structured_output": done(), "files": {"a.txt": "LEAK\n"}, "shell": [replace]}]
        self.script["review"] = [review_pass()]
        self.assertEqual(self.run_loop(), 2, self.output)
        self.assertIn("push せずに止めます", self.output)
        self.assertEqual(self.origin_branches(), ["main"])
        self.assertEqual(self.state["prs"], {})

    def test_moved_remote_tracking_main_is_not_used(self) -> None:
        # (T1) claude が、手元の origin/main を、検査の道具がまだ無いコミットに向けた（進められないと答えた作業）
        self.write_secrets_stub()
        self.task(1, difficulty="大")
        move = "git fetch --quiet . +$(git rev-list --max-parents=0 HEAD):refs/remotes/origin/main"
        self.script["implement"] = [
            {"structured_output": done(status="blocked", blocked_reason="分からない。"), "files": {"a.txt": "LEAK\n"}, "shell": [move]},
        ]
        self.assertEqual(self.run_loop(), 2, self.output)
        self.assertIn("push せずに止めます", self.output)
        self.assertEqual(self.origin_branches(), ["main"])

    def test_hooks_do_not_run_and_cannot_hide_commits(self) -> None:
        # (H1) claude が、post-commit・post-merge のフックで、追跡ブランチを HEAD に向けるようにした
        self.write_secrets_stub()
        self.task(1)
        marker = os.path.join(self.tmp, "hook-ran")
        hook = f'#!/bin/sh\ntouch {marker}\ngit update-ref refs/remotes/origin/zz HEAD\n'
        hooks = os.path.join(self.tmp, "hook-body")
        with open(hooks, "w", encoding="utf-8") as handle:
            handle.write(hook)
        install = (
            'd="$(git rev-parse --git-common-dir)/hooks"; mkdir -p "$d"; '
            f'for h in post-commit post-merge pre-push; do cp {hooks} "$d/$h"; chmod +x "$d/$h"; done'
        )
        self.script["implement"] = [{"structured_output": done(), "files": {"a.txt": "LEAK\n"}, "shell": [install]}]
        self.script["review"] = [review_pass()]
        self.assertEqual(self.run_loop(), 2, self.output)
        self.assertIn("push せずに止めます", self.output)
        self.assertEqual(self.origin_branches(), ["main"])
        self.assertFalse(os.path.exists(marker))  # ループのコミット・マージで、フックが動いていない

    def test_tool_removed_from_main_during_run_stops(self) -> None:
        # 始めるときに main にあった道具が、途中で無くなったら、飛ばさずに止まる
        self.write_secrets_stub()
        self.task(1)
        self.script["implement"] = [
            {"structured_output": done(), "files": {"a.txt": "A\n"}, "push_main": {"tools/check-secrets.py": None}},
        ]
        self.script["review"] = [review_pass()]
        code = self.run_loop()
        self.assertEqual(code, 2, self.output)
        self.assertIn("見つからなくなりました", self.output)
        self.assertEqual(self.origin_branches(), ["main"])

    def test_worktree_is_reused_and_cleaned(self) -> None:
        self.task(1)
        self.script["implement"] = [{"structured_output": done(), "files": {"a.txt": "A\n"}}]
        self.script["review"] = [review_pass()]
        self.assertEqual(self.run_loop(), 0, self.output)
        with open(os.path.join(self.worktree, "leftover.txt"), "w", encoding="utf-8") as handle:
            handle.write("前の実行の残り\n")
        with open(os.path.join(self.worktree, ".claude", "settings.local.json"), "w", encoding="utf-8") as handle:
            handle.write("{}\n")
        self.task(2)
        self.script["implement"] = [{"structured_output": done(), "files": {"b.txt": "B\n"}}]
        self.script["review"] = [review_pass()]
        self.assertEqual(self.run_loop(), 0, self.output)
        self.assertEqual(self.main_file("b.txt"), "B\n")
        self.assertIsNone(self.main_file("leftover.txt"))
        self.assertFalse(os.path.exists(os.path.join(self.worktree, ".claude", "settings.local.json")))


if __name__ == "__main__":
    unittest.main()
