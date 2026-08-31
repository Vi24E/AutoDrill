PRAGMA foreign_keys = ON;
PRAGMA journal_mode = WAL;
PRAGMA synchronous = NORMAL;

CREATE TABLE IF NOT EXISTS categories (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL UNIQUE
);

CREATE TABLE IF NOT EXISTS source_pages (
    id INTEGER PRIMARY KEY,
    url TEXT NOT NULL UNIQUE,
    title TEXT NOT NULL,
    description TEXT NOT NULL,
    stage TEXT,
    grade INTEGER,
    word_problem_source INTEGER NOT NULL CHECK (word_problem_source IN (0, 1)),
    html_sha256 TEXT,
    html_path TEXT
);

CREATE TABLE IF NOT EXISTS source_page_categories (
    source_page_id INTEGER NOT NULL REFERENCES source_pages(id) ON DELETE CASCADE,
    category_id INTEGER NOT NULL REFERENCES categories(id) ON DELETE RESTRICT,
    PRIMARY KEY (source_page_id, category_id)
) WITHOUT ROWID;

CREATE TABLE IF NOT EXISTS documents (
    id INTEGER PRIMARY KEY,
    sha256 TEXT NOT NULL UNIQUE,
    byte_count INTEGER NOT NULL CHECK (byte_count >= 0),
    page_count INTEGER NOT NULL CHECK (page_count >= 0),
    text_char_count INTEGER NOT NULL CHECK (text_char_count >= 0),
    textless_pages INTEGER NOT NULL CHECK (textless_pages >= 0),
    extracted_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS worksheets (
    id INTEGER PRIMARY KEY,
    pdf_url TEXT NOT NULL UNIQUE,
    filename TEXT NOT NULL,
    summary INTEGER NOT NULL CHECK (summary IN (0, 1)),
    word_problem_source INTEGER NOT NULL CHECK (word_problem_source IN (0, 1)),
    state TEXT NOT NULL DEFAULT 'pending' CHECK (state IN ('pending', 'extracted', 'error')),
    document_id INTEGER REFERENCES documents(id) ON DELETE RESTRICT,
    last_error TEXT,
    updated_at TEXT,
    CHECK ((state = 'extracted' AND document_id IS NOT NULL AND last_error IS NULL)
        OR (state = 'error' AND document_id IS NULL AND last_error IS NOT NULL)
        OR (state = 'pending' AND document_id IS NULL))
);

CREATE TABLE IF NOT EXISTS worksheet_sources (
    worksheet_id INTEGER NOT NULL REFERENCES worksheets(id) ON DELETE CASCADE,
    source_page_id INTEGER NOT NULL REFERENCES source_pages(id) ON DELETE CASCADE,
    category_id INTEGER NOT NULL REFERENCES categories(id) ON DELETE RESTRICT,
    anchor_text TEXT NOT NULL DEFAULT '',
    PRIMARY KEY (worksheet_id, source_page_id, category_id, anchor_text)
) WITHOUT ROWID;

CREATE TABLE IF NOT EXISTS pages (
    id INTEGER PRIMARY KEY,
    document_id INTEGER NOT NULL REFERENCES documents(id) ON DELETE CASCADE,
    page_number INTEGER NOT NULL CHECK (page_number >= 1),
    width REAL NOT NULL CHECK (width > 0),
    height REAL NOT NULL CHECK (height > 0),
    rotation INTEGER NOT NULL,
    text TEXT NOT NULL,
    text_char_count INTEGER NOT NULL CHECK (text_char_count >= 0),
    normalized_text_sha256 TEXT,
    UNIQUE (document_id, page_number)
);

CREATE TABLE IF NOT EXISTS text_blocks (
    page_id INTEGER NOT NULL REFERENCES pages(id) ON DELETE CASCADE,
    block_no INTEGER NOT NULL,
    block_type INTEGER NOT NULL,
    x0 REAL NOT NULL,
    y0 REAL NOT NULL,
    x1 REAL NOT NULL,
    y1 REAL NOT NULL,
    text TEXT NOT NULL,
    PRIMARY KEY (page_id, block_no)
) WITHOUT ROWID;

CREATE TABLE IF NOT EXISTS page_rasters (
    page_id INTEGER PRIMARY KEY REFERENCES pages(id) ON DELETE CASCADE,
    mime_type TEXT NOT NULL CHECK (mime_type = 'image/png'),
    pixel_width INTEGER NOT NULL CHECK (pixel_width > 0),
    pixel_height INTEGER NOT NULL CHECK (pixel_height > 0),
    data BLOB NOT NULL
);

CREATE INDEX IF NOT EXISTS worksheets_state_idx ON worksheets(state);
CREATE INDEX IF NOT EXISTS worksheets_document_idx ON worksheets(document_id);
CREATE INDEX IF NOT EXISTS worksheet_sources_source_idx ON worksheet_sources(source_page_id);
CREATE INDEX IF NOT EXISTS worksheet_sources_category_idx ON worksheet_sources(category_id);
CREATE INDEX IF NOT EXISTS pages_document_idx ON pages(document_id, page_number);
CREATE INDEX IF NOT EXISTS pages_text_hash_idx ON pages(normalized_text_sha256);

CREATE VIRTUAL TABLE IF NOT EXISTS pages_fts USING fts5(
    text,
    content='pages',
    content_rowid='id',
    tokenize='unicode61'
);

CREATE TRIGGER IF NOT EXISTS pages_ai AFTER INSERT ON pages BEGIN
    INSERT INTO pages_fts(rowid, text) VALUES (new.id, new.text);
END;
CREATE TRIGGER IF NOT EXISTS pages_ad AFTER DELETE ON pages BEGIN
    INSERT INTO pages_fts(pages_fts, rowid, text) VALUES ('delete', old.id, old.text);
END;
CREATE TRIGGER IF NOT EXISTS pages_au AFTER UPDATE OF text ON pages BEGIN
    INSERT INTO pages_fts(pages_fts, rowid, text) VALUES ('delete', old.id, old.text);
    INSERT INTO pages_fts(rowid, text) VALUES (new.id, new.text);
END;
