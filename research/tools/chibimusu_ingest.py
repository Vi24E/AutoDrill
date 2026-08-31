#!/usr/bin/env python3
"""Stream Chibimusu problem worksheets into a normalized SQLite corpus.

Network download and PDF extraction are intentionally separate pipeline stages.
A bounded queue provides backpressure, so extraction can run concurrently with
fetching without allowing temporary PDFs to accumulate on disk.  A PDF is
removed only after its extracted representation has committed successfully.

The SQLite database is the canonical corpus.  PDF bytes are disposable staging
artifacts, not part of the retained dataset.
"""

from __future__ import annotations

import argparse
import dataclasses
import datetime as dt
import hashlib
import json
import os
import queue
import sqlite3
import threading
import time
import urllib.error
import urllib.request
from pathlib import Path
from typing import Iterable, Mapping, Sequence

USER_AGENT = "AutoDrill-reference-corpus/1.0 (research; respectful crawl)"
DEFAULT_OUTPUT = Path("research/corpora/chibimusu")
SCHEMA_PATH = Path(__file__).with_name("chibimusu_schema.sql")
_STOP = object()


@dataclasses.dataclass(frozen=True)
class WorksheetJob:
    worksheet_id: int
    pdf_url: str
    supplemental: bool = False


@dataclasses.dataclass(frozen=True)
class StagedPdf:
    job: WorksheetJob
    path: Path
    sha256: str
    byte_count: int


@dataclasses.dataclass(frozen=True)
class TextBlock:
    block_no: int
    block_type: int
    x0: float
    y0: float
    x1: float
    y1: float
    text: str


@dataclasses.dataclass(frozen=True)
class ExtractedPage:
    page_number: int
    width: float
    height: float
    rotation: int
    text: str
    normalized_text_sha256: str | None
    blocks: tuple[TextBlock, ...]
    raster_png: bytes | None
    raster_width: int | None
    raster_height: int | None


@dataclasses.dataclass(frozen=True)
class ExtractionResult:
    job: WorksheetJob
    staged_path: Path | None
    sha256: str | None
    byte_count: int | None
    pages: tuple[ExtractedPage, ...]
    error: str | None

    @property
    def ok(self) -> bool:
        return self.error is None and self.sha256 is not None and self.byte_count is not None


def utc_now() -> str:
    return dt.datetime.now(dt.timezone.utc).isoformat()


def sha256_bytes(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def url_cache_key(url: str) -> str:
    return hashlib.sha256(url.encode("utf-8")).hexdigest()


def normalized_text_hash(text: str) -> str | None:
    normalized = " ".join(text.split()).casefold()
    if not normalized:
        return None
    return hashlib.sha256(normalized.encode("utf-8")).hexdigest()


def load_jsonl(path: Path) -> list[dict[str, object]]:
    rows: list[dict[str, object]] = []
    with path.open("r", encoding="utf-8") as handle:
        for line in handle:
            line = line.strip()
            if line:
                rows.append(json.loads(line))
    return rows


def open_database(path: Path) -> sqlite3.Connection:
    path.parent.mkdir(parents=True, exist_ok=True)
    connection = sqlite3.connect(path)
    connection.row_factory = sqlite3.Row
    connection.execute("PRAGMA foreign_keys = ON")
    connection.executescript(SCHEMA_PATH.read_text(encoding="utf-8"))
    return connection


def get_or_create_category(connection: sqlite3.Connection, name: str) -> int:
    connection.execute("INSERT OR IGNORE INTO categories(name) VALUES (?)", (name,))
    row = connection.execute("SELECT id FROM categories WHERE name = ?", (name,)).fetchone()
    assert row is not None
    return int(row["id"])


def seed_catalogue(connection: sqlite3.Connection, output: Path) -> None:
    source_path = output / "source_pages.jsonl"
    worksheet_path = output / "pdf_links.discovered.jsonl"
    if not source_path.exists() or not worksheet_path.exists():
        raise SystemExit("run chibimusu_corpus.py discover before ingest")

    source_rows = load_jsonl(source_path)
    discovered_rows = [
        row for row in load_jsonl(worksheet_path) if str(row.get("role")) == "problem"
    ]

    with connection:
        # These temp tables define the authoritative discovery snapshot.  They
        # let us synchronize deletions without constructing huge SQL IN lists.
        connection.execute("CREATE TEMP TABLE IF NOT EXISTS current_source_urls(url TEXT PRIMARY KEY)")
        connection.execute("DELETE FROM current_source_urls")
        connection.executemany(
            "INSERT INTO current_source_urls(url) VALUES (?)",
            [(str(row["url"]),) for row in source_rows],
        )
        connection.execute("CREATE TEMP TABLE IF NOT EXISTS current_worksheet_urls(url TEXT PRIMARY KEY)")
        connection.execute("DELETE FROM current_worksheet_urls")
        connection.executemany(
            "INSERT INTO current_worksheet_urls(url) VALUES (?)",
            [(str(row["pdf_url"]),) for row in discovered_rows],
        )

        category_ids: dict[str, int] = {}

        def category_id(name: str) -> int:
            if name not in category_ids:
                category_ids[name] = get_or_create_category(connection, name)
            return category_ids[name]

        source_ids: dict[str, int] = {}
        for row in source_rows:
            url = str(row["url"])
            connection.execute(
                """
                INSERT INTO source_pages(
                    url, title, description, stage, grade, word_problem_source,
                    html_sha256, html_path
                ) VALUES (?, ?, ?, ?, ?, ?, ?, ?)
                ON CONFLICT(url) DO UPDATE SET
                    title=excluded.title,
                    description=excluded.description,
                    stage=excluded.stage,
                    grade=excluded.grade,
                    word_problem_source=excluded.word_problem_source,
                    html_sha256=excluded.html_sha256,
                    html_path=excluded.html_path
                """,
                (
                    url,
                    str(row.get("title") or ""),
                    str(row.get("description") or ""),
                    row.get("stage"),
                    row.get("grade"),
                    int(bool(row.get("word_problem_source"))),
                    row.get("html_sha256"),
                    row.get("html_path"),
                ),
            )
            source = connection.execute("SELECT id FROM source_pages WHERE url = ?", (url,)).fetchone()
            assert source is not None
            source_id = int(source["id"])
            source_ids[url] = source_id
            connection.execute(
                "DELETE FROM source_page_categories WHERE source_page_id = ?", (source_id,)
            )
            for name in sorted(set(map(str, row.get("categories", [])))):
                connection.execute(
                    "INSERT INTO source_page_categories(source_page_id, category_id) VALUES (?, ?)",
                    (source_id, category_id(name)),
                )

        for row in discovered_rows:
            pdf_url = str(row["pdf_url"])
            connection.execute(
                """
                INSERT INTO worksheets(pdf_url, filename, summary, word_problem_source)
                VALUES (?, ?, ?, ?)
                ON CONFLICT(pdf_url) DO UPDATE SET
                    filename=excluded.filename,
                    summary=excluded.summary,
                    word_problem_source=excluded.word_problem_source
                """,
                (
                    pdf_url,
                    str(row.get("filename") or Path(pdf_url).name),
                    int(bool(row.get("summary"))),
                    int(bool(row.get("word_problem_source"))),
                ),
            )
            worksheet = connection.execute(
                "SELECT id FROM worksheets WHERE pdf_url = ?", (pdf_url,)
            ).fetchone()
            assert worksheet is not None
            worksheet_id = int(worksheet["id"])
            connection.execute(
                "DELETE FROM worksheet_sources WHERE worksheet_id = ?", (worksheet_id,)
            )
            for source in row.get("sources", []):
                if not isinstance(source, Mapping):
                    continue
                source_url = str(source.get("source_page_url") or "")
                source_id = source_ids.get(source_url)
                if source_id is None:
                    existing = connection.execute(
                        "SELECT id FROM source_pages WHERE url = ?", (source_url,)
                    ).fetchone()
                    if existing is None:
                        continue
                    source_id = int(existing["id"])
                name = str(source.get("category") or "uncategorized")
                connection.execute(
                    """
                    INSERT OR IGNORE INTO worksheet_sources(
                        worksheet_id, source_page_id, category_id, anchor_text
                    ) VALUES (?, ?, ?, ?)
                    """,
                    (
                        worksheet_id,
                        source_id,
                        category_id(name),
                        str(source.get("anchor_text") or ""),
                    ),
                )

        # Discovery is a snapshot, not an append-only log.  Remove stale logical
        # worksheets/sources and then reclaim content documents no longer
        # referenced by any retained worksheet.
        connection.execute(
            "DELETE FROM worksheets WHERE pdf_url NOT IN (SELECT url FROM current_worksheet_urls)"
        )
        connection.execute(
            "DELETE FROM source_pages WHERE url NOT IN (SELECT url FROM current_source_urls)"
        )
        connection.execute(
            "DELETE FROM documents WHERE NOT EXISTS (SELECT 1 FROM worksheets WHERE worksheets.document_id = documents.id)"
        )


def pending_jobs(connection: sqlite3.Connection) -> list[WorksheetJob]:
    rows = connection.execute(
        """
        SELECT
            w.id,
            w.pdf_url,
            w.state,
            EXISTS (
                SELECT 1
                FROM pages p
                WHERE p.document_id = w.document_id
                  AND p.text_char_count = 0
                  AND NOT EXISTS (SELECT 1 FROM page_rasters r WHERE r.page_id = p.id)
            ) AS needs_raster
        FROM worksheets w
        WHERE w.state != 'extracted'
           OR EXISTS (
                SELECT 1
                FROM pages p
                WHERE p.document_id = w.document_id
                  AND p.text_char_count = 0
                  AND NOT EXISTS (SELECT 1 FROM page_rasters r WHERE r.page_id = p.id)
           )
        ORDER BY w.id
        """
    ).fetchall()
    return [
        WorksheetJob(
            int(row["id"]),
            str(row["pdf_url"]),
            supplemental=str(row["state"]) == "extracted" and bool(row["needs_raster"]),
        )
        for row in rows
    ]


def legacy_pdf_path(output: Path, pdf_url: str) -> Path:
    return output / "download" / f"{url_cache_key(pdf_url)}.pdf"


def staged_pdf_path(output: Path, pdf_url: str) -> Path:
    return output / ".staging" / f"{url_cache_key(pdf_url)}.pdf"


def request_pdf(url: str, *, timeout: float = 45.0, retries: int = 3) -> bytes:
    last_error: Exception | None = None
    for attempt in range(retries):
        try:
            request = urllib.request.Request(url, headers={"User-Agent": USER_AGENT})
            with urllib.request.urlopen(request, timeout=timeout) as response:
                data = response.read()
            if not data.startswith(b"%PDF"):
                raise ValueError(f"response is not PDF; first bytes={data[:12]!r}")
            return data
        except (urllib.error.URLError, TimeoutError, ConnectionError, ValueError) as exc:
            last_error = exc
            if attempt + 1 < retries:
                time.sleep(0.5 * (attempt + 1))
    assert last_error is not None
    raise last_error


def acquire_pdf(output: Path, job: WorksheetJob) -> StagedPdf:
    legacy = legacy_pdf_path(output, job.pdf_url)
    staging = staged_pdf_path(output, job.pdf_url)
    for candidate in (legacy, staging):
        if candidate.exists():
            data = candidate.read_bytes()
            if data.startswith(b"%PDF"):
                return StagedPdf(job, candidate, sha256_bytes(data), len(data))
            candidate.unlink(missing_ok=True)

    staging.parent.mkdir(parents=True, exist_ok=True)
    data = request_pdf(job.pdf_url)
    temp = staging.with_suffix(".tmp")
    temp.write_bytes(data)
    os.replace(temp, staging)
    return StagedPdf(job, staging, sha256_bytes(data), len(data))


def extract_pdf(staged: StagedPdf) -> ExtractionResult:
    try:
        import pymupdf

        pages: list[ExtractedPage] = []
        with pymupdf.open(staged.path) as document:
            for page_index in range(document.page_count):
                page = document.load_page(page_index)
                text = page.get_text("text", sort=True)
                blocks: list[TextBlock] = []
                for block in page.get_text("blocks", sort=True):
                    blocks.append(
                        TextBlock(
                            block_no=int(block[5]),
                            block_type=int(block[6]),
                            x0=round(float(block[0]), 3),
                            y0=round(float(block[1]), 3),
                            x1=round(float(block[2]), 3),
                            y1=round(float(block[3]), 3),
                            text=str(block[4]),
                        )
                    )
                raster_png: bytes | None = None
                raster_width: int | None = None
                raster_height: int | None = None
                if not text.strip():
                    # Textless PDFs are usually image/outline based.  Preserve
                    # only those exceptional pages as a compact grayscale raster
                    # so the problem remains recoverable without retaining PDF bytes.
                    pixmap = page.get_pixmap(
                        matrix=pymupdf.Matrix(1.5, 1.5),
                        colorspace=pymupdf.csGRAY,
                        alpha=False,
                    )
                    raster_png = pixmap.tobytes("png")
                    raster_width = int(pixmap.width)
                    raster_height = int(pixmap.height)
                pages.append(
                    ExtractedPage(
                        page_number=page_index + 1,
                        width=round(float(page.rect.width), 3),
                        height=round(float(page.rect.height), 3),
                        rotation=int(page.rotation),
                        text=text,
                        normalized_text_sha256=normalized_text_hash(text),
                        blocks=tuple(blocks),
                        raster_png=raster_png,
                        raster_width=raster_width,
                        raster_height=raster_height,
                    )
                )
        return ExtractionResult(
            job=staged.job,
            staged_path=staged.path,
            sha256=staged.sha256,
            byte_count=staged.byte_count,
            pages=tuple(pages),
            error=None,
        )
    except Exception as exc:
        return ExtractionResult(
            job=staged.job,
            staged_path=staged.path,
            sha256=staged.sha256,
            byte_count=staged.byte_count,
            pages=(),
            error=repr(exc),
        )


def failed_result(job: WorksheetJob, exc: Exception) -> ExtractionResult:
    return ExtractionResult(job, None, None, None, (), repr(exc))


def downloader_worker(
    output: Path,
    jobs: "queue.Queue[WorksheetJob | object]",
    extraction_queue: "queue.Queue[StagedPdf | object]",
    results: "queue.Queue[ExtractionResult]",
) -> None:
    while True:
        item = jobs.get()
        try:
            if item is _STOP:
                return
            assert isinstance(item, WorksheetJob)
            try:
                extraction_queue.put(acquire_pdf(output, item))
            except Exception as exc:
                results.put(failed_result(item, exc))
        finally:
            jobs.task_done()


def extractor_worker(
    extraction_queue: "queue.Queue[StagedPdf | object]",
    results: "queue.Queue[ExtractionResult]",
) -> None:
    while True:
        item = extraction_queue.get()
        try:
            if item is _STOP:
                return
            assert isinstance(item, StagedPdf)
            results.put(extract_pdf(item))
        finally:
            extraction_queue.task_done()


def store_success(connection: sqlite3.Connection, result: ExtractionResult) -> None:
    assert result.ok
    assert result.sha256 is not None
    assert result.byte_count is not None
    text_char_count = sum(len(page.text.strip()) for page in result.pages)
    textless_pages = sum(not page.text.strip() for page in result.pages)

    with connection:
        connection.execute(
            """
            INSERT OR IGNORE INTO documents(
                sha256, byte_count, page_count, text_char_count, textless_pages, extracted_at
            ) VALUES (?, ?, ?, ?, ?, ?)
            """,
            (
                result.sha256,
                result.byte_count,
                len(result.pages),
                text_char_count,
                textless_pages,
                utc_now(),
            ),
        )
        document = connection.execute(
            "SELECT id FROM documents WHERE sha256 = ?", (result.sha256,)
        ).fetchone()
        assert document is not None
        document_id = int(document["id"])

        for page in result.pages:
            connection.execute(
                """
                INSERT OR IGNORE INTO pages(
                    document_id, page_number, width, height, rotation, text,
                    text_char_count, normalized_text_sha256
                ) VALUES (?, ?, ?, ?, ?, ?, ?, ?)
                """,
                (
                    document_id,
                    page.page_number,
                    page.width,
                    page.height,
                    page.rotation,
                    page.text,
                    len(page.text.strip()),
                    page.normalized_text_sha256,
                ),
            )
            stored_page = connection.execute(
                "SELECT id FROM pages WHERE document_id = ? AND page_number = ?",
                (document_id, page.page_number),
            ).fetchone()
            assert stored_page is not None
            page_id = int(stored_page["id"])
            connection.executemany(
                """
                INSERT OR IGNORE INTO text_blocks(
                    page_id, block_no, block_type, x0, y0, x1, y1, text
                ) VALUES (?, ?, ?, ?, ?, ?, ?, ?)
                """,
                [
                    (
                        page_id,
                        block.block_no,
                        block.block_type,
                        block.x0,
                        block.y0,
                        block.x1,
                        block.y1,
                        block.text,
                    )
                    for block in page.blocks
                ],
            )
            if page.raster_png is not None:
                assert page.raster_width is not None and page.raster_height is not None
                connection.execute(
                    """
                    INSERT INTO page_rasters(page_id, mime_type, pixel_width, pixel_height, data)
                    VALUES (?, 'image/png', ?, ?, ?)
                    ON CONFLICT(page_id) DO UPDATE SET
                        mime_type=excluded.mime_type,
                        pixel_width=excluded.pixel_width,
                        pixel_height=excluded.pixel_height,
                        data=excluded.data
                    """,
                    (page_id, page.raster_width, page.raster_height, page.raster_png),
                )

        connection.execute(
            """
            UPDATE worksheets
            SET state='extracted', document_id=?, last_error=NULL, updated_at=?
            WHERE id=?
            """,
            (document_id, utc_now(), result.job.worksheet_id),
        )


def store_error(connection: sqlite3.Connection, result: ExtractionResult) -> None:
    # Failure to add a supplemental raster must not destroy an already valid
    # text extraction.  Missing rasters remain discoverable and are retried on
    # the next run.
    if result.job.supplemental:
        return
    with connection:
        connection.execute(
            """
            UPDATE worksheets
            SET state='error', document_id=NULL, last_error=?, updated_at=?
            WHERE id=?
            """,
            (result.error or "unknown error", utc_now(), result.job.worksheet_id),
        )


def remove_committed_pdf(path: Path | None) -> int:
    if path is None or not path.exists():
        return 0
    size = path.stat().st_size
    path.unlink()
    return size


def clean_empty_staging_dirs(output: Path) -> None:
    for directory in (output / ".staging", output / "download"):
        if directory.exists():
            try:
                directory.rmdir()
            except OSError:
                pass


def write_readme(output: Path, stats: Mapping[str, object]) -> None:
    text = f"""# ちびむすドリル 数学リファレンスコーパス（local research data）

AutoDrill の問題分布比較・文章題設計用のローカル参照データです。
第三者著作物を含むため公開・再配布用ではありません。

## Canonical data

- `corpus.sqlite3`: 問題PDFから抽出したテキスト・座標付きtext blockと教材メタデータ
- `source_pages.jsonl`: クロール時の教材ページメタデータ（再発見用）
- `pdf_links.discovered.jsonl`: 問題PDF URLの発見記録（再取得用）
- `html/`: 小容量の教材ページHTMLスナップショット

PDFそのものは canonical data ではありません。取得と抽出は別ワーカーでパイプライン実行し、
SQLiteへのcommit成功直後に各PDFを削除します。文字オブジェクトを持たないページだけは、
PDFの代わりに低容量のグレースケールPNGを `page_rasters` に保持します。

## Normalized schema

`categories` / `source_pages` / `source_page_categories` / `worksheets` /
`worksheet_sources` / `documents` / `pages` / `text_blocks` / `page_rasters` に分離しています。
同一PDF内容は `documents.sha256` で一意化し、複数URL・教材ページとの関係だけを
`worksheets` と `worksheet_sources` で保持します。`pages_fts` は検索用の派生indexです。

## Latest ingest

```json
{json.dumps(stats, ensure_ascii=False, indent=2, sort_keys=True)}
```

## Resume / refresh

```bash
PYTHONPATH=.tmp/pydeps python3 research/tools/chibimusu_ingest.py \\
  --output research/corpora/chibimusu
```
"""
    (output / "README.md").write_text(text, encoding="utf-8")


def database_stats(connection: sqlite3.Connection) -> dict[str, int]:
    def scalar(sql: str) -> int:
        row = connection.execute(sql).fetchone()
        assert row is not None
        return int(row[0])

    return {
        "worksheets": scalar("SELECT COUNT(*) FROM worksheets"),
        "extracted_worksheets": scalar("SELECT COUNT(*) FROM worksheets WHERE state='extracted'"),
        "error_worksheets": scalar("SELECT COUNT(*) FROM worksheets WHERE state='error'"),
        "unique_documents": scalar("SELECT COUNT(*) FROM documents"),
        "pages": scalar("SELECT COUNT(*) FROM pages"),
        "text_blocks": scalar("SELECT COUNT(*) FROM text_blocks"),
        "text_characters": scalar("SELECT COALESCE(SUM(text_char_count), 0) FROM pages"),
        "textless_pages": scalar("SELECT COUNT(*) FROM pages WHERE text_char_count=0"),
        "raster_fallback_pages": scalar("SELECT COUNT(*) FROM page_rasters"),
        "word_problem_worksheets": scalar("SELECT COUNT(*) FROM worksheets WHERE word_problem_source=1"),
    }


def ingest(
    output: Path,
    *,
    download_workers: int,
    extract_workers: int,
    queue_depth: int,
) -> dict[str, object]:
    try:
        import pymupdf  # noqa: F401
    except ImportError as exc:
        raise SystemExit("PyMuPDF is required: python3 -m pip install pymupdf") from exc

    output.mkdir(parents=True, exist_ok=True)
    database_path = output / "corpus.sqlite3"
    connection = open_database(database_path)
    seed_catalogue(connection, output)
    jobs_to_run = pending_jobs(connection)

    jobs: "queue.Queue[WorksheetJob | object]" = queue.Queue()
    extraction_queue: "queue.Queue[StagedPdf | object]" = queue.Queue(maxsize=max(1, queue_depth))
    results: "queue.Queue[ExtractionResult]" = queue.Queue(maxsize=max(1, queue_depth))

    download_threads = [
        threading.Thread(
            target=downloader_worker,
            args=(output, jobs, extraction_queue, results),
            name=f"download-{index + 1}",
            daemon=True,
        )
        for index in range(max(1, download_workers))
    ]
    extract_threads = [
        threading.Thread(
            target=extractor_worker,
            args=(extraction_queue, results),
            name=f"extract-{index + 1}",
            daemon=True,
        )
        for index in range(max(1, extract_workers))
    ]
    for thread in extract_threads + download_threads:
        thread.start()

    for job in jobs_to_run:
        jobs.put(job)
    for _ in download_threads:
        jobs.put(_STOP)

    processed = 0
    succeeded = 0
    failed = 0
    bytes_deleted = 0
    started = time.monotonic()
    total = len(jobs_to_run)

    while processed < total:
        result = results.get()
        try:
            processed += 1
            if result.ok:
                try:
                    store_success(connection, result)
                except Exception as exc:
                    failed += 1
                    store_error(
                        connection,
                        dataclasses.replace(result, error=f"database commit failed: {exc!r}"),
                    )
                else:
                    succeeded += 1
                    bytes_deleted += remove_committed_pdf(result.staged_path)
            else:
                failed += 1
                store_error(connection, result)

            if processed % 50 == 0 or processed == total:
                elapsed = max(time.monotonic() - started, 0.001)
                print(
                    json.dumps(
                        {
                            "processed": processed,
                            "total": total,
                            "ok": succeeded,
                            "errors": failed,
                            "rate_per_second": round(processed / elapsed, 2),
                            "deleted_mb": round(bytes_deleted / (1024 * 1024), 1),
                            "download_to_extract_queue": extraction_queue.qsize(),
                            "results_queue": results.qsize(),
                        },
                        ensure_ascii=False,
                    ),
                    flush=True,
                )
        finally:
            results.task_done()

    jobs.join()
    for _ in extract_threads:
        extraction_queue.put(_STOP)
    extraction_queue.join()
    for thread in download_threads + extract_threads:
        thread.join(timeout=5)

    clean_empty_staging_dirs(output)
    # Materialize the WAL before reporting retained corpus size.  This also makes
    # a completed corpus self-contained if it is copied while no writer is open.
    connection.execute("PRAGMA wal_checkpoint(TRUNCATE)")
    stats: dict[str, object] = database_stats(connection)
    stats.update(
        {
            "run_processed": processed,
            "run_succeeded": succeeded,
            "run_failed": failed,
            "run_deleted_bytes": bytes_deleted,
            "database_bytes": database_path.stat().st_size,
            "generated_at": utc_now(),
        }
    )
    (output / "ingest_stats.json").write_text(
        json.dumps(stats, ensure_ascii=False, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )
    write_readme(output, stats)
    connection.close()
    return stats


def main(argv: Sequence[str] | None = None) -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--output", type=Path, default=DEFAULT_OUTPUT)
    parser.add_argument("--download-workers", type=int, default=6)
    parser.add_argument("--extract-workers", type=int, default=3)
    parser.add_argument(
        "--queue-depth",
        type=int,
        default=12,
        help="maximum staged PDFs waiting between pipeline stages",
    )
    args = parser.parse_args(argv)

    result = ingest(
        args.output.resolve(),
        download_workers=max(1, args.download_workers),
        extract_workers=max(1, args.extract_workers),
        queue_depth=max(1, args.queue_depth),
    )
    print(json.dumps(result, ensure_ascii=False, indent=2, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
