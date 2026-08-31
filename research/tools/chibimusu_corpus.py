#!/usr/bin/env python3
"""Discover Chibimusu mathematics worksheet catalogue metadata.

This crawler owns catalogue discovery only.  It stores lightweight HTML/source
metadata and problem-PDF URLs.  PDF acquisition, extraction, normalized SQLite
persistence, and immediate PDF disposal are owned by chibimusu_ingest.py.
"""

from __future__ import annotations

import argparse
import dataclasses
import datetime as dt
import gzip
import hashlib
import html
import json
import re
import time
import unicodedata
import urllib.error
import urllib.parse
import urllib.request
from collections import Counter, defaultdict, deque
from html.parser import HTMLParser
from pathlib import Path
from typing import Iterable, Iterator, Mapping, Sequence

BASE_URL = "https://happylilac.net/"
DEFAULT_OUTPUT = Path("research/corpora/chibimusu")
USER_AGENT = "AutoDrill-reference-corpus/1.0 (research; respectful crawl)"

# These are the mathematics catalogue pages exposed from the elementary-school
# mathematics section.  They are semantic entry points, not URL-pattern hacks:
# each owns one part of the public mathematics catalogue.
SEED_PAGES: Mapping[str, str] = {
    "numbers": "sansu-h.html",
    "number_structure": "keisan-kazu.html",
    "addition": "keisan-tasizan.html",
    "subtraction": "keisan-hikizan.html",
    "multiplication": "keisan-kakezan.html",
    "division": "keisan-warizan.html",
    "word_problems": "c-msp.html",
    "timed_calculation": "5minutes-2024.html",
    "clock": "tokei.html",
    "time": "zikan.html",
    "decimals_fractions": "syousu-bunsu.html",
    "fractions": "bunsu.html",
    "geometry": "zukei.html",
    "quantitative_relations": "keisan-suryokankei.html",
    "grid_calculation": "100masu.html",
    "calculation_chain": "keisan-siritori.html",
    "math_quiz": "keisan-rc.html",
    "junior_high_math": "p-jhs-math.html",
}

MATH_TERMS = (
    "算数", "数学", "計算", "数の構成", "数のしくみ", "数の表し", "大きな数", "大きい数",
    "たし算", "足し算", "加法", "ひき算", "引き算", "減法", "かけ算", "掛け算", "乗法",
    "わり算", "割り算", "除法", "九九", "小数", "分数", "通分", "約分", "整数", "正負",
    "正の数", "負の数", "文字式", "文字と式", "方程式", "連立", "比例", "反比例", "関数",
    "平方根", "因数分解", "展開", "多項式", "単項式", "式の計算", "図形", "三角形", "四角形",
    "円", "おうぎ形", "角", "合同", "相似", "三平方", "面積", "体積", "立体", "作図", "対称",
    "長さ", "距離", "重さ", "かさ", "単位", "時刻", "時間", "時計", "速さ", "割合", "百分率",
    "歩合", "平均", "倍数", "約数", "素数", "概数", "四捨五入", "場合の数", "確率", "資料",
    "データ", "グラフ", "表と", "文章問題", "活用", "数量", "□を使った式", "ます計算",
    "計算しりとり", "計算チャレンジ", "数直線", "比と比", "比の利用", "標本調査",
)

# A generic word such as 「文章問題」 or 「図形」 can appear on non-mathematics
# material.  Subject markers take precedence over weak math vocabulary so the
# crawler cannot drift from a mathematics seed into Japanese/English/etc.
NON_MATH_SUBJECT_TERMS = (
    "国語", "漢字", "ひらがな", "カタカナ", "作文", "読解",
    "英語", "英単語", "アルファベット", "ローマ字",
    "理科", "社会", "歴史", "地理", "音楽",
)

# Common global navigation/footer pages and non-reference material.  These are
# excluded by semantics: they are not worksheet catalogue/problem pages.
EXCLUDED_PATH_PARTS = (
    "redirect_", "riyo.html", "sy-spprint.html", "calendar.html", "kisetsu-sozai.html",
    "sy-link.html", "yw1604181448.html", "index2.html", "highschool.html", "syogaku.html",
    "english", "kanji", "kanzi", "rekishi", "science", "chizu", "tizu", "music",
)

COLLABORATION_PATH_PREFIXES = ("rd-",)
COLLABORATION_TERMS = (
    "コラボ教材", "Ｚ会", "Z会", "ドリルの王様", "天才脳ドリル", "学研", "進研ゼミ",
)
NON_PROBLEM_PAGE_TERMS = (
    "リンク集", "印刷方法", "学習ポスター", "一覧表ポスター", "テンプレート", "九九表", "九九カード",
)

ANSWER_TERMS = ("ans", "answer", "kotae", "解答", "答え")
SUMMARY_TERMS = ("matome", "まとめ")
WORD_PROBLEM_TERMS = ("文章問題", "文章題", "活用", "応用")

GRADE_KANJI = {"一": 1, "二": 2, "三": 3, "四": 4, "五": 5, "六": 6}


@dataclasses.dataclass(frozen=True)
class Anchor:
    href: str
    text: str


@dataclasses.dataclass(frozen=True)
class ParsedHtml:
    title: str
    description: str
    anchors: tuple[Anchor, ...]


class CatalogueParser(HTMLParser):
    def __init__(self) -> None:
        super().__init__(convert_charrefs=True)
        self.title_parts: list[str] = []
        self.description = ""
        self.anchors: list[Anchor] = []
        self._in_title = False
        self._anchor_href: str | None = None
        self._anchor_text: list[str] = []

    def handle_starttag(self, tag: str, attrs: list[tuple[str, str | None]]) -> None:
        values = dict(attrs)
        if tag == "title":
            self._in_title = True
        elif tag == "meta" and (values.get("name") or "").lower() == "description":
            self.description = values.get("content") or ""
        elif tag == "a":
            self._anchor_href = values.get("href")
            self._anchor_text = []

    def handle_data(self, data: str) -> None:
        if self._in_title:
            self.title_parts.append(data)
        if self._anchor_href is not None:
            self._anchor_text.append(data)

    def handle_endtag(self, tag: str) -> None:
        if tag == "title":
            self._in_title = False
        elif tag == "a" and self._anchor_href is not None:
            self.anchors.append(
                Anchor(self._anchor_href, clean_text("".join(self._anchor_text)))
            )
            self._anchor_href = None
            self._anchor_text = []

    def result(self) -> ParsedHtml:
        return ParsedHtml(
            title=clean_text("".join(self.title_parts)),
            description=clean_text(self.description),
            anchors=tuple(self.anchors),
        )


def clean_text(value: str) -> str:
    return " ".join(html.unescape(value).split())


def normalize_search_text(value: str) -> str:
    return unicodedata.normalize("NFKC", value).lower()


def sha256_bytes(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def json_line(value: object) -> str:
    return json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(",", ":"))


def normalized_url(base: str, href: str) -> str | None:
    if not href or href.startswith(("mailto:", "javascript:", "tel:")):
        return None
    absolute = urllib.parse.urljoin(base, href)
    parsed = urllib.parse.urlsplit(absolute)
    if parsed.scheme not in {"http", "https"}:
        return None
    host = (parsed.hostname or "").lower()
    if host == "www.happylilac.net":
        host = "happylilac.net"
    if host != "happylilac.net":
        return None
    path = parsed.path or "/"
    # Fragments only navigate within the same catalogue page and do not define a
    # different source document. Query parameters are preserved if present.
    return urllib.parse.urlunsplit(("https", host, path, parsed.query, ""))


def request_bytes(url: str, *, timeout: float = 30.0, retries: int = 3) -> tuple[bytes, str]:
    last_error: Exception | None = None
    for attempt in range(retries):
        try:
            req = urllib.request.Request(url, headers={"User-Agent": USER_AGENT})
            with urllib.request.urlopen(req, timeout=timeout) as response:
                return response.read(), response.headers.get("Content-Type", "")
        except (urllib.error.URLError, TimeoutError, ConnectionError) as exc:
            last_error = exc
            if attempt + 1 < retries:
                time.sleep(0.5 * (attempt + 1))
    assert last_error is not None
    raise last_error


def parse_html_bytes(data: bytes) -> ParsedHtml:
    text = data.decode("utf-8", errors="replace")
    parser = CatalogueParser()
    parser.feed(text)
    return parser.result()


def looks_math(value: str) -> bool:
    normalized = normalize_search_text(value)
    if any(normalize_search_text(term) in normalized for term in NON_MATH_SUBJECT_TERMS):
        return False
    return any(normalize_search_text(term) in normalized for term in MATH_TERMS)


def is_collaboration(path: str, label: str) -> bool:
    basename = Path(path).name.lower()
    if basename.startswith(COLLABORATION_PATH_PREFIXES):
        return True
    normalized = normalize_search_text(label)
    return any(normalize_search_text(term) in normalized for term in COLLABORATION_TERMS)


def is_non_problem_page(label: str) -> bool:
    normalized = normalize_search_text(label)
    return any(normalize_search_text(term) in normalized for term in NON_PROBLEM_PAGE_TERMS)


def is_html_candidate(url: str, anchor_text: str) -> bool:
    parsed = urllib.parse.urlsplit(url)
    path_lower = parsed.path.lower()
    basename = Path(path_lower).name
    if any(part in path_lower for part in EXCLUDED_PATH_PARTS):
        return False
    if is_collaboration(path_lower, anchor_text):
        return False
    suffix = Path(path_lower).suffix
    if suffix and suffix not in {".html", ".htm", ".htmll"}:
        return False
    if is_non_problem_page(anchor_text):
        return False
    # A link from a math catalogue is retained when either its human-facing
    # label or its path identifies it as mathematics.  Cryptic timestamp URLs
    # are still captured because their catalogue labels are descriptive.
    path_signal = any(
        token in basename
        for token in (
            "math", "sansu", "keisan", "zukei", "bunsu", "syousu", "tokei", "zikan",
            "kuku", "100masu", "siritori", "msp",
        )
    )
    return path_signal or looks_math(anchor_text)


def infer_grade(title: str) -> tuple[str | None, int | None]:
    normalized = unicodedata.normalize("NFKC", title)
    elementary = re.search(r"小学\s*([1-6一二三四五六])\s*年", normalized)
    if elementary:
        token = elementary.group(1)
        return "elementary", int(token) if token.isdigit() else GRADE_KANJI[token]
    junior = re.search(r"中学\s*([1-3一二三])\s*年", normalized)
    if junior:
        token = junior.group(1)
        return "junior_high", int(token) if token.isdigit() else GRADE_KANJI[token]
    short_elementary = re.search(r"(?:^|\s)小\s*([1-6])(?:\D|$)", normalized)
    if short_elementary:
        return "elementary", int(short_elementary.group(1))
    short_junior = re.search(r"(?:^|\s)中\s*([1-3])(?:\D|$)", normalized)
    if short_junior:
        return "junior_high", int(short_junior.group(1))
    return None, None


def classify_pdf_role(url: str, anchor_texts: Iterable[str]) -> str:
    combined = normalize_search_text(url + " " + " ".join(anchor_texts))
    if any(term in combined for term in ANSWER_TERMS):
        return "answer"
    return "problem"


def is_summary(url: str, anchor_texts: Iterable[str]) -> bool:
    combined = normalize_search_text(url + " " + " ".join(anchor_texts))
    return any(normalize_search_text(term) in combined for term in SUMMARY_TERMS)


def is_word_problem_source(*values: str) -> bool:
    combined = normalize_search_text(" ".join(values))
    return any(normalize_search_text(term) in combined for term in WORD_PROBLEM_TERMS)


def safe_html_name(url: str) -> str:
    path = urllib.parse.urlsplit(url).path.strip("/") or "index.html"
    basename = re.sub(r"[^A-Za-z0-9._-]+", "_", path.replace("/", "__"))
    digest = hashlib.sha256(url.encode("utf-8")).hexdigest()[:10]
    return f"{basename}.{digest}.html.gz"


def write_jsonl(path: Path, rows: Iterable[object]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", encoding="utf-8") as handle:
        for row in rows:
            handle.write(json_line(row))
            handle.write("\n")


def discover(output: Path, *, max_depth: int = 2) -> dict[str, object]:
    html_dir = output / "html"
    html_dir.mkdir(parents=True, exist_ok=True)

    seed_urls = {category: urllib.parse.urljoin(BASE_URL, path) for category, path in SEED_PAGES.items()}
    queue: deque[tuple[str, int, str, str]] = deque(
        (url, 0, category, category) for category, url in seed_urls.items()
    )
    seen_urls: set[str] = set()
    retained_urls: set[str] = set()
    page_rows: list[dict[str, object]] = []
    excluded_rows: list[dict[str, object]] = []
    pdf_sources: dict[str, list[dict[str, str]]] = defaultdict(list)
    url_categories: dict[str, set[str]] = defaultdict(set)

    while queue:
        url, depth, category, anchor_hint = queue.popleft()
        url_categories[url].add(category)
        if url in seen_urls:
            continue
        seen_urls.add(url)
        force_seed = url in seed_urls.values()
        try:
            data, content_type = request_bytes(url)
        except Exception as exc:  # record source failures; do not lose the crawl
            excluded_rows.append({"url": url, "reason": "fetch_error", "error": repr(exc)})
            continue
        if "html" not in content_type.lower() and not data.lstrip().lower().startswith(b"<!doctype html"):
            excluded_rows.append({"url": url, "reason": "not_html", "content_type": content_type})
            continue
        parsed = parse_html_bytes(data)
        label = " ".join((parsed.title, parsed.description, anchor_hint))
        path = urllib.parse.urlsplit(url).path
        if not force_seed and (
            not looks_math(label)
            or is_collaboration(path, label)
            or is_non_problem_page(label)
        ):
            excluded_rows.append({"url": url, "reason": "not_reference_math_page", "title": parsed.title})
            continue

        retained_urls.add(url)
        html_name = safe_html_name(url)
        with gzip.open(html_dir / html_name, "wb") as handle:
            handle.write(data)
        stage, grade = infer_grade(parsed.title + " " + parsed.description)
        categories = sorted(url_categories[url])
        row = {
            "url": url,
            "title": parsed.title,
            "description": parsed.description,
            "stage": stage,
            "grade": grade,
            "categories": categories,
            "word_problem_source": is_word_problem_source(parsed.title, parsed.description, " ".join(categories)),
            "html_path": str((Path("html") / html_name).as_posix()),
            "html_sha256": sha256_bytes(data),
            "bytes": len(data),
        }
        page_rows.append(row)

        for anchor in parsed.anchors:
            target = normalized_url(url, anchor.href)
            if target is None:
                continue
            target_path = urllib.parse.urlsplit(target).path.lower()
            if target_path.endswith(".pdf"):
                pdf_sources[target].append(
                    {
                        "source_page_url": url,
                        "source_page_title": parsed.title,
                        "anchor_text": anchor.text,
                        "category": category,
                    }
                )
                continue
            if depth < max_depth and target not in seen_urls and is_html_candidate(target, anchor.text):
                queue.append((target, depth + 1, category, anchor.text))

    # Categories can arrive from a second catalogue link after the first fetch;
    # repair them from the canonical URL->category relation before persistence.
    for row in page_rows:
        categories = sorted(url_categories[str(row["url"])])
        row["categories"] = categories
        row["word_problem_source"] = is_word_problem_source(
            str(row["title"]), str(row["description"]), " ".join(categories)
        )

    pdf_rows: list[dict[str, object]] = []
    page_by_url = {str(row["url"]): row for row in page_rows}
    for pdf_url, sources in sorted(pdf_sources.items()):
        anchor_texts = sorted({source["anchor_text"] for source in sources if source["anchor_text"]})
        source_titles = sorted({source["source_page_title"] for source in sources})
        source_categories = sorted({source["category"] for source in sources})
        source_word_problem = any(
            bool(page_by_url.get(source["source_page_url"], {}).get("word_problem_source"))
            for source in sources
        )
        pdf_rows.append(
            {
                "pdf_url": pdf_url,
                "filename": Path(urllib.parse.urlsplit(pdf_url).path).name,
                "role": classify_pdf_role(pdf_url, anchor_texts),
                "summary": is_summary(pdf_url, anchor_texts),
                "word_problem_source": source_word_problem or is_word_problem_source(*source_titles, *anchor_texts),
                "source_categories": source_categories,
                "anchor_texts": anchor_texts,
                "sources": sources,
            }
        )

    page_rows.sort(key=lambda row: str(row["url"]))
    excluded_rows.sort(key=lambda row: str(row["url"]))
    write_jsonl(output / "source_pages.jsonl", page_rows)
    write_jsonl(output / "pdf_links.discovered.jsonl", pdf_rows)
    write_jsonl(output / "excluded_pages.jsonl", excluded_rows)

    stats = {
        "generated_at": dt.datetime.now(dt.timezone.utc).isoformat(),
        "scope": "Chibimusu original elementary/junior-high mathematics catalogue reachable from semantic seed pages",
        "seed_pages": len(seed_urls),
        "html_urls_seen": len(seen_urls),
        "html_pages_retained": len(page_rows),
        "pdf_links_discovered": len(pdf_rows),
        "problem_pdf_links": sum(row["role"] == "problem" for row in pdf_rows),
        "answer_pdf_links": sum(row["role"] == "answer" for row in pdf_rows),
        "word_problem_pdf_links": sum(bool(row["word_problem_source"]) and row["role"] == "problem" for row in pdf_rows),
        "excluded_html_pages": len(excluded_rows),
        "max_depth": max_depth,
    }
    (output / "discovery_stats.json").write_text(
        json.dumps(stats, ensure_ascii=False, indent=2, sort_keys=True) + "\n", encoding="utf-8"
    )
    return stats



def main(argv: Sequence[str] | None = None) -> int:
    parser = argparse.ArgumentParser(
        description=(
            "Discover Chibimusu mathematics catalogue metadata. "
            "Use chibimusu_ingest.py for the bounded download/extraction pipeline."
        )
    )
    parser.add_argument("command", choices=("discover",))
    parser.add_argument("--output", type=Path, default=DEFAULT_OUTPUT)
    parser.add_argument("--max-depth", type=int, default=2)
    args = parser.parse_args(argv)

    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=True)
    result = discover(output, max_depth=max(0, args.max_depth))
    print(json.dumps(result, ensure_ascii=False, indent=2, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
