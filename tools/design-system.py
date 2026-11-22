#!/usr/bin/env python3
"""design-system/ 配下のデザインシステムを検査・生成・照合する。

使い方:
    python3 tools/design-system.py [--root <design-system のディレクトリ>] <サブコマンド> ...

    check [<名前> ...]
        規則どおりに置かれているかを検査する。名前を省くと、design-system/ の直下で
        design-system.json を持つディレクトリをすべて検査する。
        README などから仕様への参照（`screens/appearance.md`「見出し」の形）は、
        --spec-root（既定: リポジトリの documents/spec/）の文書と照らす。
    tokens-css <名前> [--write]
        tokens.json から tokens.css の内容を作り、標準出力に出す。--write で tokens.css に書く。
    files <名前>
        アーティファクトへ送るファイルの一覧を JSON で標準出力に出す
        （Artifact ツールの root・file_path・files にそのまま渡せる形）。
    compare <名前> <取得したディレクトリ> [--prefix <接頭辞>] [--partial] [--skip-assets]
        アーティファクトから取得したファイルと、リポジトリのファイルをバイト単位で比べる。
        --partial は、<接頭辞>/ の下にあるファイルだけを比べる（別のアーティファクトが持つ写し用）。
        --skip-assets は、アセット（アイコン）を比べない。

<名前> は design-system/ の直下のディレクトリ名（progrust、progrust-video）。
--root を省くと、このスクリプトから見たリポジトリの design-system/ を使う（テスト用に差し替えられる）。
指摘は「パス: 規則名: 説明」の形で1行ずつ標準出力に出し、最後に件数を出す。

終了コード:
    0  指摘なし・違いなし（tokens-css・files は成功）
    1  指摘あり・違いあり（tokens-css は tokens.json から作れなかったとき）
    2  引数の誤り、名前のディレクトリが無い、など

デザインシステムの正はリポジトリのファイルで、claude.ai のアーティファクトは表示用である。
アーティファクトとのやり取りはこのスクリプトでは行わない（Claude が Artifact ツールで行う）。

標準ライブラリだけを使う。Python 3.9 以降で動く。
"""

from __future__ import annotations

import argparse
import json
import os
import re
import sys
from typing import Dict, List, Optional, Tuple

REPO_ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
DEFAULT_SPEC_ROOT = os.path.join(REPO_ROOT, "documents", "spec")

INDEX_NAME = "design-system.json"
# アーティファクトの仕組み上、assets/<Group>/ の下でもテキストは普通の公開ファイルで、
# それ以外（SVG など）だけがアセットストアに入る。
TEXT_EXTS = (".md", ".json", ".txt", ".csv")
BLOB_RE = re.compile(r"^[0-9a-f]{32}$")
NAME_RE = re.compile(r"^[a-z0-9-]+$")


# --- tokens.css の生成 ---
# 規則は、アーティファクトのページが tokens.json から tokens.css を作っていたときの出力に合わせてある
# （移す前の 2 つの tokens.css を、1 バイトも違わずに作り直せることを確かめた）。
# 知らない形の tokens.json は、黙って何かを出さずに Unsupported で止める。

TOKENS_NAME_RE = re.compile(r"^[A-Za-z0-9][A-Za-z0-9_.-]{0,63}$")
ALIAS_RE = re.compile(r"^\{([^{}]+)\}$")
# color・type のほかに tokens.json が持ってよい最上位のキー
SKIP_KEYS = {"name", "version", "color", "type"}


class Unsupported(Exception):
    """tokens.json が、生成の規則の知っている形ではない。"""


def css_name(name):
    if not TOKENS_NAME_RE.match(name):
        raise Unsupported("bad token name: %r" % name)
    return "--" + name.replace(".", "\\.")


def theme_value(value, theme, first):
    """テーマごとの値を引く。文字列だけなら、どのテーマでも同じ値。"""
    if isinstance(value, str):
        return value
    if isinstance(value, dict):
        if theme in value:
            return value[theme]
        if first in value:
            return value[first]
    raise Unsupported("unreadable value: %r" % (value,))


def render_value(v):
    if not isinstance(v, str):
        raise Unsupported("non-string value: %r" % (v,))
    m = ALIAS_RE.match(v)
    if m:
        return "var(%s)" % css_name(m.group(1))
    return v


def decl(token, theme, first):
    return "  %s: %s;" % (css_name(token["name"]), render_value(theme_value(token["value"], theme, first)))


def length(v):
    if isinstance(v, (int, float)) and not isinstance(v, bool):
        return "0" if v == 0 else "%gpx" % v
    if isinstance(v, str):
        return v
    raise Unsupported("bad length: %r" % (v,))


def style_rule(style, group_family):
    fam = style.get("family", group_family)
    if not fam:
        raise Unsupported("style %s has no family" % style["name"])
    known = {"name", "family", "fontSize", "lineHeight", "fontWeight", "letterSpacing", "usage", "sample"}
    extra = set(style) - known
    if extra:  # fontStyle・opticalSize は、実際のファイルに出力の形の例が無い
        raise Unsupported("style %s: unsupported keys %s" % (style["name"], sorted(extra)))
    parts = ["font-family: var(--font-%s)" % fam]
    if "fontSize" in style:
        parts.append("font-size: %s" % length(style["fontSize"]))
    if "lineHeight" in style:
        lh = style["lineHeight"]
        parts.append("line-height: %s" % (lh if isinstance(lh, str) else "%g" % lh))
    if "fontWeight" in style:
        parts.append("font-weight: %s" % style["fontWeight"])
    if "letterSpacing" in style:
        parts.append("letter-spacing: %s" % length(style["letterSpacing"]))
    return ".%s { %s; }" % (style["name"].replace(".", "\\."), "; ".join(parts))


def build_tokens_css(d, title):
    color = d["color"]
    themes = [t["id"] for t in color["themes"]]
    first = themes[0]
    ctoks = color["tokens"]
    shadows = d.get("shadow", {}).get("tokens", [])
    out = ["/* %s — generated from tokens.json */" % title]
    for i, th in enumerate(themes):
        out.append(':root, [data-theme="%s"] {' % th if i == 0 else '[data-theme="%s"] {' % th)
        for t in ctoks + shadows:
            out.append(decl(t, th, first))
        out.append("}")
    # :root のブロックは、tokens を持つ残りの族をファイルの順に並べ、最後にフォントを置く
    root = []
    for key, fam in d.items():
        if key in SKIP_KEYS or key == "shadow":
            continue
        if not (isinstance(fam, dict) and isinstance(fam.get("tokens"), list)):
            raise Unsupported("unknown top-level key: %s" % key)
        for t in fam["tokens"]:
            if not isinstance(t["value"], str):
                raise Unsupported("per-theme value in family %s" % key)
            root.append(decl(t, first, first))
    typ = d["type"]
    if typ.get("fonts"):
        raise Unsupported("type.fonts (@font-face) not verified against a real file")
    for k, stack in typ["families"].items():
        root.append("  --font-%s: %s;" % (k, stack))
    if root:
        out.append(":root {")
        out.extend(root)
        out.append("}")
    for g in typ["groups"]:
        for s in g["styles"]:
            out.append(style_rule(s, g.get("family")))
    return "\n".join(out) + "\n"


def generate_tokens_css(ds_dir: str) -> str:
    """tokens.json から tokens.css の内容を作る。作れないときは Unsupported・KeyError など。"""
    with open(os.path.join(ds_dir, "tokens.json"), encoding="utf-8") as f:
        d = json.load(f)
    title = d.get("name")
    if not title:
        with open(os.path.join(ds_dir, INDEX_NAME), encoding="utf-8") as f:
            title = json.load(f)["title"]
    return build_tokens_css(d, title)


# --- ファイルの一覧 ---


def is_text(name: str) -> bool:
    return name.lower().endswith(TEXT_EXTS)


def walk(base: str) -> List[str]:
    """base の下のファイルを、base からの相対パス（/ 区切り）で、名前が . で始まるものを除いて返す。"""
    found = []
    for dirpath, dirnames, filenames in os.walk(base):
        dirnames[:] = [d for d in dirnames if not d.startswith(".")]
        for fn in filenames:
            if fn.startswith("."):
                continue
            rel = os.path.relpath(os.path.join(dirpath, fn), base)
            found.append(rel.replace(os.sep, "/"))
    return sorted(found)


def asset_group_of(rel: str) -> Optional[Tuple[str, str]]:
    """assets/<Group>/<名前> の、テキスト以外のファイルなら (Group, 名前)。"""
    parts = rel.split("/")
    if len(parts) >= 3 and parts[0] == "assets" and not is_text(parts[-1]):
        return parts[1], "/".join(parts[2:])
    return None


def public_files(ds_dir: str) -> List[str]:
    return [r for r in walk(ds_dir) if asset_group_of(r) is None]


def asset_records(index: dict) -> List[Tuple[str, str, dict]]:
    """索引のアセットの記録を (Group, キー, 記録) の列にする。形が違うものは読み飛ばす（check が指摘する）。"""
    out = []
    groups = index.get("assetGroups")
    if not isinstance(groups, dict):
        return out
    for group, g in groups.items():
        files = g.get("files") if isinstance(g, dict) else None
        if not isinstance(files, dict):
            continue
        for key, rec in files.items():
            if isinstance(rec, dict):
                out.append((group, key, rec))
    return out


def display(path: str) -> str:
    ap = os.path.abspath(path)
    if ap.startswith(REPO_ROOT + os.sep):
        return os.path.relpath(ap, REPO_ROOT)
    return ap


def read_bytes(path: str) -> bytes:
    with open(path, "rb") as f:
        return f.read()


# --- check ---


class Finding:
    def __init__(self, path: str, rule: str, message: str) -> None:
        self.path = path
        self.rule = rule
        self.message = message

    def format(self) -> str:
        return "%s: %s: %s" % (self.path, self.rule, self.message)


# --- spec-ref: README などから仕様の文書への参照 ---
# 参照は、インラインコードのパスに「小見出し」を続けた形で書く（例: 仕様の `screens/appearance.md`「書体のファイル」）。
# `/` を含むパスは documents/spec/ からのパスとする。ただし、デザインシステムの中のディレクトリ
# （LOCAL_DIRS）で始まるパスと、`/` を含まないパス（`chapter-colors.md`）は、デザインシステムの中のファイルとする。

SPEC_REF_RE = re.compile(r"`([A-Za-z0-9_./-]+\.md)`((?:「[^」]+」)*)")
SPEC_HEADING_RE = re.compile(r"「([^」]+)」")
LOCAL_DIRS = ("components/", "assets/")
FENCE_RE = re.compile(r"^\s*(```|~~~)")
# 仕様の小見出しは、`## 見出し` か、1 行だけの太字（`**見出し**`）で書かれている
MD_HEADING_RE = re.compile(r"^#{1,6}\s+(.+?)\s*#*\s*$")
BOLD_HEADING_RE = re.compile(r"^\*\*(.+?)\*\*\s*$")


def headings_of(path: str) -> set:
    found = set()
    in_fence = False
    with open(path, encoding="utf-8") as f:
        for line in f:
            if FENCE_RE.match(line):
                in_fence = not in_fence
                continue
            if in_fence:
                continue
            m = MD_HEADING_RE.match(line) or BOLD_HEADING_RE.match(line)
            if m:
                found.add(m.group(1).strip())
    return found


def check_spec_refs(ds_dir: str, spec_root: str, add) -> None:
    """規則 spec-ref: 仕様とデザインシステムの文書への参照が、実在する文書と小見出しを指すこと。"""
    cache: Dict[str, set] = {}
    for rel in walk(ds_dir):
        if not rel.endswith(".md"):
            continue
        in_fence = False
        with open(os.path.join(ds_dir, rel), encoding="utf-8") as f:
            lines = f.read().split("\n")
        for number, line in enumerate(lines, 1):
            if FENCE_RE.match(line):
                in_fence = not in_fence
                continue
            if in_fence:
                continue
            for m in SPEC_REF_RE.finditer(line):
                target = m.group(1)
                where = "%s:%d" % (rel, number)
                if ".." in target.split("/") or target.startswith("/"):
                    add(where, "spec-ref", "参照 `%s` に `..` か先頭の `/` があります。仕様は `documents/spec/` からのパス、デザインシステムの中のファイルはこのディレクトリからのパスで書いてください" % target)
                    continue
                local = "/" not in target or target.startswith(LOCAL_DIRS)
                base = ds_dir if local else spec_root
                path = os.path.join(base, *target.split("/"))
                if not os.path.isfile(path):
                    if local:
                        add(where, "spec-ref", "参照 `%s` のファイルが、このデザインシステムの中にありません。実在するファイルを指すように直してください" % target)
                    else:
                        add(where, "spec-ref", "参照 `%s` の仕様の文書が `documents/spec/` にありません。仕様のファイルを移したり名前を変えたりしたときは、参照も直してください" % target)
                    continue
                if path not in cache:
                    cache[path] = headings_of(path)
                for heading in SPEC_HEADING_RE.findall(m.group(2)):
                    if heading not in cache[path]:
                        add(where, "spec-ref", "参照 `%s`「%s」の小見出しが、その文書にありません。小見出しを変えたときは、参照も直してください" % (target, heading))


# --- copied-token: 動画のデザインシステムが持つ、画面のデザインシステムの値の写し ---
# 写しを持つデザインシステムの名前 -> (写しの元の名前, 写しでないトークンの名前の接頭辞)
# 写しの元を正とし、写しの側は、接頭辞の無いトークンと書体の族（type.families）を、元と同じ名前・同じ値で持つ。
COPIES = {"progrust-video": ("progrust", "video-")}


def token_values(d: dict) -> Dict[str, object]:
    """tokens.json の、type 以外の族のトークンを {名前: 値} にする。"""
    out: Dict[str, object] = {}
    for key, fam in d.items():
        if key in ("name", "version", "type"):
            continue
        if isinstance(fam, dict) and isinstance(fam.get("tokens"), list):
            for t in fam["tokens"]:
                if isinstance(t, dict) and isinstance(t.get("name"), str):
                    out[t["name"]] = t.get("value")
    return out


def check_copied_tokens(root: str, name: str, add) -> None:
    """規則 copied-token: 写しのトークンと書体の族が、写しの元と同じ値であること。"""
    if name not in COPIES:
        return
    source, own_prefix = COPIES[name]
    try:
        with open(os.path.join(root, name, "tokens.json"), encoding="utf-8") as f:
            copy = json.load(f)
    except (OSError, ValueError):
        return  # tokens-css の規則が指摘する
    if not isinstance(copy, dict):
        return  # tokens-css の規則が指摘する
    try:
        with open(os.path.join(root, source, "tokens.json"), encoding="utf-8") as f:
            orig = json.load(f)
        if not isinstance(orig, dict):
            raise ValueError("最上位が JSON のオブジェクトではありません")
    except (OSError, ValueError) as e:
        add("tokens.json", "copied-token", "写しの元の「%s」の tokens.json が読めないので、写しを確かめられません（%s）。写しの元を直してください" % (source, e))
        return
    src_tokens = token_values(orig)
    for tname, value in sorted(token_values(copy).items()):
        if tname.startswith(own_prefix):
            continue
        if tname not in src_tokens:
            add("tokens.json", "copied-token", "トークン `%s` は「%s」の写しのはずですが、「%s」にありません。このシステムだけで決める値なら名前を `%s` で始め、写しなら「%s」に足してから合わせてください" % (tname, source, source, own_prefix, source))
        elif value != src_tokens[tname]:
            add("tokens.json", "copied-token", "トークン `%s` の値が、写しの元の「%s」と違います（ここ: %s、元: %s）。「%s」を正として、ここの値を合わせてください" % (tname, source, json.dumps(value, ensure_ascii=False), json.dumps(src_tokens[tname], ensure_ascii=False), source))
    src_fam = (orig.get("type") or {}).get("families") or {}
    for fam, stack in sorted(((copy.get("type") or {}).get("families") or {}).items()):
        if fam not in src_fam:
            add("tokens.json", "copied-token", "書体の族 `%s` が、写しの元の「%s」にありません。書体は「%s」の写しです。「%s」に足してから合わせてください" % (fam, source, source, source))
        elif stack != src_fam[fam]:
            add("tokens.json", "copied-token", "書体の族 `%s` の値が、写しの元の「%s」と違います（ここ: %s、元: %s）。「%s」を正として、ここの値を合わせてください" % (fam, source, json.dumps(stack, ensure_ascii=False), json.dumps(src_fam[fam], ensure_ascii=False), source))


def check_system(root: str, name: str, spec_root: str = "") -> List[Finding]:
    ds_dir = os.path.join(root, name)
    findings: List[Finding] = []
    spec_root = spec_root or DEFAULT_SPEC_ROOT

    def add(rel: str, rule: str, message: str) -> None:
        findings.append(Finding(display(os.path.join(ds_dir, rel)), rule, message))

    check_spec_refs(ds_dir, spec_root, add)
    check_copied_tokens(root, name, add)

    # tokens-css
    css_path = os.path.join(ds_dir, "tokens.css")
    regen = "`python3 tools/design-system.py tokens-css %s --write` で作り直してください" % name
    try:
        expected = generate_tokens_css(ds_dir)
    except FileNotFoundError as e:
        add("tokens.css", "tokens-css", "tokens.css を作る元のファイル `%s` がありません。置いてください" % os.path.basename(e.filename or ""))
    except (Unsupported, KeyError, TypeError, AttributeError, IndexError, ValueError) as e:
        add("tokens.json", "tokens-css", "tokens.json から tokens.css を作れません（%s: %s）。tokens.json を、生成の規則が扱える形に直してください" % (type(e).__name__, e))
    else:
        if not os.path.isfile(css_path):
            add("tokens.css", "tokens-css", "tokens.css がありません。%s" % regen)
        elif read_bytes(css_path) != expected.encode("utf-8"):
            add("tokens.css", "tokens-css", "tokens.css が tokens.json から作った内容と違います。tokens.css は手で編集せず、tokens.json を直したうえで%s" % regen)

    # index-json
    index = None
    index_path = os.path.join(ds_dir, INDEX_NAME)
    try:
        with open(index_path, encoding="utf-8") as f:
            index = json.load(f)
        if not isinstance(index, dict):
            raise ValueError("最上位が JSON のオブジェクトではありません")
    except (OSError, ValueError) as e:
        add(INDEX_NAME, "index-json", "%s が JSON として読めません（%s）。直してください" % (INDEX_NAME, e))
        index = None
    if index is not None:
        if "v" not in index:
            add(INDEX_NAME, "index-json", '"v"（索引の版）がありません。足してください')
        if index.get("layout") != "files":
            add(INDEX_NAME, "index-json", '"layout" が "files" ではありません。"files" にしてください')
        if "createdOnFiles" not in index and "convertedFrom" not in index:
            add(INDEX_NAME, "index-json", '"createdOnFiles" も "convertedFrom" もありません。ファイル形式の索引であることを示すために、どちらかを足してください')
        # index-editing
        if index.get("editing") != "code":
            add(
                INDEX_NAME,
                "index-editing",
                '"editing" が "code" ではありません。アーティファクトのページを読み取り専用にする設定です（正はリポジトリのファイルなので、ページでは編集させません）。"layout" の直後に "editing": "code" を足してください',
            )

    # asset-record
    recorded = {}  # Group -> 記録のある名前の集合
    groups = index.get("assetGroups") if index is not None else None
    if isinstance(groups, dict):
        for group, g in groups.items():
            gpath = "assetGroups.%s" % group
            if not isinstance(g, dict) or not isinstance(g.get("files"), dict):
                add(INDEX_NAME, "asset-record", "%s に files のオブジェクトがありません。直してください" % gpath)
                continue
            names = recorded.setdefault(group, set())
            for key, rec in g["files"].items():
                where = "%s.files.%s" % (gpath, key)
                if not isinstance(rec, dict):
                    add(INDEX_NAME, "asset-record", "%s の記録がオブジェクトではありません。直してください" % where)
                    continue
                rname = rec.get("name")
                if rname != key:
                    add(INDEX_NAME, "asset-record", "%s の name（%r）がキーと違います。キーと name を同じにしてください" % (where, rname))
                if not isinstance(rname, str) or not rname or "/" in rname or rname.startswith("."):
                    add(INDEX_NAME, "asset-record", "%s の name が使えない形です。ファイル名だけ（/ を含まない）にしてください" % where)
                    continue
                names.add(rname)
                blob = rec.get("blob")
                if not isinstance(blob, str) or not BLOB_RE.match(blob):
                    add(INDEX_NAME, "asset-record", "%s の blob が 32 桁の 16 進ではありません。アーティファクトのアセットストアの id を入れてください" % where)
                rel = "assets/%s/%s" % (group, rname)
                fpath = os.path.join(ds_dir, rel)
                if not os.path.isfile(fpath):
                    add(INDEX_NAME, "asset-record", "%s の記録に対するファイル `%s` がありません。ファイルを置くか、記録を消してください" % (where, rel))
                else:
                    size = rec.get("size")
                    actual = os.path.getsize(fpath)
                    if size != actual:
                        add(INDEX_NAME, "asset-record", "%s の size（%r）が `%s` の実際のバイト数（%d）と違います。size を直してください" % (where, size, rel, actual))
            order = g.get("order")
            if not isinstance(order, list):
                add(INDEX_NAME, "asset-record", "%s.order が配列ではありません。files の名前を並べた配列にしてください" % gpath)
            else:
                keys = set(g["files"].keys())
                missing = sorted(keys - set(order))
                extra = sorted(set(order) - keys)
                dup = len(order) != len(set(order))
                if missing or extra or dup:
                    detail = []
                    if missing:
                        detail.append("order に無い: %s" % ", ".join(missing))
                    if extra:
                        detail.append("files に無い: %s" % ", ".join(extra))
                    if dup:
                        detail.append("重複がある")
                    add(INDEX_NAME, "asset-record", "%s.order が files の名前と過不足なく合いません（%s）。直してください" % (gpath, "。".join(detail)))

    # asset-file
    for rel in walk(ds_dir):
        ag = asset_group_of(rel)
        if ag is None:
            continue
        group, fname = ag
        if fname not in recorded.get(group, set()):
            add(rel, "asset-file", "索引 %s に記録がありません。assetGroups.%s.files と order に記録を足すか（blob はアーティファクトへ送ったあとの id）、ファイルを消してください" % (INDEX_NAME, group))

    # stray-file
    for rel in walk(ds_dir):
        if rel.split("/")[0] == "api" and "/" in rel:
            add(rel, "stray-file", "api/ の下はアーティファクトのページが作るファイルなので、置かないでください。消してください")
        elif rel == "manifest.json":
            add(rel, "stray-file", "manifest.json はアーティファクトのページが作るファイルなので、置かないでください。消してください")
    return findings


def find_systems(root: str) -> List[str]:
    return sorted(
        n for n in os.listdir(root) if os.path.isfile(os.path.join(root, n, INDEX_NAME))
    )


def cmd_check(root: str, names: List[str], spec_root: str = "") -> int:
    if not names:
        names = find_systems(root)
    spec_root = spec_root or DEFAULT_SPEC_ROOT
    if not os.path.isdir(spec_root):
        raise UsageError("仕様のディレクトリ `%s` がありません。" % spec_root)
    all_findings: List[Finding] = []
    for name in names:
        require_system(root, name)
        all_findings.extend(check_system(root, name, spec_root))
    all_findings.sort(key=lambda f: (f.path, f.rule, f.message))
    for f in all_findings:
        print(f.format())
    print("%d 個のデザインシステムを検査し、指摘は %d 件でした。" % (len(names), len(all_findings)))
    return 1 if all_findings else 0


# --- tokens-css ---


def cmd_tokens_css(root: str, name: str, write: bool) -> int:
    ds_dir = require_system(root, name)
    try:
        text = generate_tokens_css(ds_dir)
    except (Unsupported, KeyError, TypeError, AttributeError, IndexError, ValueError, OSError) as e:
        print("%s: tokens-css: tokens.json から tokens.css を作れません（%s: %s）。" % (display(ds_dir), type(e).__name__, e), file=sys.stderr)
        return 1
    if write:
        with open(os.path.join(ds_dir, "tokens.css"), "w", encoding="utf-8", newline="") as f:
            f.write(text)
    else:
        sys.stdout.write(text)
    return 0


# --- files ---


def load_index(ds_dir: str) -> dict:
    try:
        with open(os.path.join(ds_dir, INDEX_NAME), encoding="utf-8") as f:
            index = json.load(f)
        if not isinstance(index, dict):
            raise ValueError("最上位が JSON のオブジェクトではありません")
        return index
    except (OSError, ValueError) as e:
        raise UsageError("%s が読めません（%s）。先に check で確かめてください。" % (display(os.path.join(ds_dir, INDEX_NAME)), e))


def cmd_files(root: str, name: str) -> int:
    ds_dir = require_system(root, name)
    index = load_index(ds_dir)
    files = {"project/" + rel: rel for rel in public_files(ds_dir)}
    assets = []
    for group, key, rec in asset_records(index):
        assets.append(
            {
                "group": group,
                "name": rec.get("name"),
                "path": "assets/%s/%s" % (group, rec.get("name")),
                "blob": rec.get("blob"),
                "size": rec.get("size"),
            }
        )
    assets.sort(key=lambda a: (a["group"], str(a["name"])))
    out = {
        "root": os.path.abspath(ds_dir),
        "file_path": os.path.abspath(os.path.join(ds_dir, INDEX_NAME)),
        "files": files,
        "assets": assets,
    }
    print(json.dumps(out, indent=2, ensure_ascii=False))
    return 0


# --- compare ---


def cmd_compare(root: str, name: str, fetched: str, prefix: str, partial: bool, skip_assets: bool = False) -> int:
    ds_dir = require_system(root, name)
    if not os.path.isdir(fetched):
        raise UsageError("取得したディレクトリ `%s` がありません。" % fetched)
    diffs: List[str] = []
    compared = 0

    def same(repo_rel: str, other: str) -> None:
        """リポジトリの repo_rel と、取得した側の other を比べる。"""
        nonlocal compared
        compared += 1
        if not os.path.isfile(other):
            diffs.append("無い: %s（取得した側に %s がありません）" % (repo_rel, os.path.relpath(other, fetched)))
        elif read_bytes(other) != read_bytes(os.path.join(ds_dir, repo_rel)):
            diffs.append("中身が違う: %s" % repo_rel)

    if partial:
        base = os.path.join(fetched, *prefix.split("/"))
        if not os.path.isdir(base):
            raise UsageError("取得したディレクトリの接頭辞 `%s` のディレクトリがありません。" % prefix)
        for rel in walk(base):
            compared += 1
            repo_file = os.path.join(ds_dir, rel)
            if not os.path.isfile(repo_file):
                diffs.append("余分: %s（リポジトリに無いファイルです）" % rel)
            elif read_bytes(repo_file) != read_bytes(os.path.join(base, rel)):
                diffs.append("中身が違う: %s" % rel)
    else:
        index = load_index(ds_dir)
        repo_public = public_files(ds_dir)
        base = os.path.join(fetched, *prefix.split("/"))
        for rel in repo_public:
            same(rel, os.path.join(base, rel))
        # アセットは id ごとに 1 つずつ取得する必要があるので、変えていないときは省けるようにする。
        for group, key, rec in ([] if skip_assets else asset_records(index)):
            aname = str(rec.get("name"))
            rel = "assets/%s/%s" % (group, aname)
            ext = os.path.splitext(aname)[1]
            if not os.path.isfile(os.path.join(ds_dir, rel)):
                diffs.append("無い: %s（リポジトリにファイルがありません）" % rel)
                compared += 1
                continue
            same(rel, os.path.join(fetched, "%s%s" % (rec.get("blob"), ext)))
        known = set(repo_public)
        if os.path.isdir(base):
            for rel in walk(base):
                if rel not in known:
                    diffs.append("余分: %s（リポジトリに無いファイルです）" % rel)
        else:
            diffs.append("無い: %s/（取得したディレクトリに公開ファイルの置き場がありません）" % prefix)
    for line in diffs:
        print(line)
    print("%d 件を比べ、違いは %d 件でした。" % (compared, len(diffs)))
    return 1 if diffs else 0


# --- 引数 ---


class UsageError(Exception):
    pass


def require_system(root: str, name: str) -> str:
    if not NAME_RE.match(name):
        raise UsageError("名前 `%s` が使えません（英小文字・数字・ハイフンだけ）。" % name)
    ds_dir = os.path.join(root, name)
    if not os.path.isdir(ds_dir):
        raise UsageError("デザインシステム `%s` のディレクトリ `%s` がありません。" % (name, display(ds_dir)))
    return ds_dir


class Parser(argparse.ArgumentParser):
    def error(self, message):  # argparse の既定の終了コードも 2 だが、メッセージを日本語にする
        self.print_usage(sys.stderr)
        print("引数の誤り: %s" % message, file=sys.stderr)
        sys.exit(2)


def build_parser() -> argparse.ArgumentParser:
    p = Parser(
        prog="design-system.py",
        description="design-system/ 配下のデザインシステムを検査・生成・照合する。終了コード: 0 指摘なし、1 指摘あり、2 引数の誤り。",
    )
    p.add_argument("--root", default=os.path.join(REPO_ROOT, "design-system"), metavar="DIR", help="design-system のディレクトリ（既定: リポジトリの design-system/）")
    sub = p.add_subparsers(dest="cmd", metavar="サブコマンド", parser_class=Parser)
    sub.required = True
    c = sub.add_parser("check", help="規則どおりに置かれているかを検査する（名前を省くと、索引を持つ全部）")
    c.add_argument("names", nargs="*", metavar="名前")
    c.add_argument("--spec-root", default=DEFAULT_SPEC_ROOT, metavar="DIR", help="仕様への参照を照らす仕様のディレクトリ（既定: リポジトリの documents/spec/）")
    t = sub.add_parser("tokens-css", help="tokens.json から tokens.css の内容を作って標準出力に出す")
    t.add_argument("name", metavar="名前")
    t.add_argument("--write", action="store_true", help="標準出力ではなく tokens.css に書く")
    f = sub.add_parser("files", help="アーティファクトへ送るファイルの一覧を JSON で出す")
    f.add_argument("name", metavar="名前")
    m = sub.add_parser("compare", help="アーティファクトから取得したファイルと、リポジトリのファイルをバイト単位で比べる")
    m.add_argument("name", metavar="名前")
    m.add_argument("fetched", metavar="取得したディレクトリ")
    m.add_argument("--prefix", default="project", metavar="接頭辞", help="公開ファイルが入っているディレクトリ（既定: project）")
    m.add_argument("--partial", action="store_true", help="<接頭辞>/ の下にあるファイルだけを、リポジトリの同じパスのファイルと比べる（アセットは比べない）")
    m.add_argument("--skip-assets", action="store_true", help="アセット（アイコン）を比べない。アセットを変えていないときに、取得を省くために使う")
    return p


def main(argv: List[str]) -> int:
    args = build_parser().parse_args(argv)
    root = args.root
    try:
        if not os.path.isdir(root):
            raise UsageError("design-system のディレクトリ `%s` がありません。" % root)
        if args.cmd == "check":
            return cmd_check(root, args.names, args.spec_root)
        if args.cmd == "tokens-css":
            return cmd_tokens_css(root, args.name, args.write)
        if args.cmd == "files":
            return cmd_files(root, args.name)
        prefix = args.prefix.strip("/")
        if not prefix and not args.partial:
            raise UsageError("--prefix が空です。")
        return cmd_compare(root, args.name, args.fetched, prefix, args.partial, args.skip_assets)
    except UsageError as e:
        print(str(e), file=sys.stderr)
        return 2


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
