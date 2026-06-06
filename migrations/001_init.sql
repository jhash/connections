-- Puzzle sources. Stored as TEXT in all tables.
-- Valid values: 'nytimes' | 'connections_plus'

CREATE TABLE IF NOT EXISTS puzzles (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    source      TEXT NOT NULL CHECK (source IN ('nytimes', 'connections_plus')),
    external_id TEXT NOT NULL,  -- NYT: numeric id as text; connections+: short game id e.g. "tBZCr6"
    author      TEXT,           -- NYT: editor name (nullable); connections+: username
    date        TEXT,           -- NYT: "YYYY-MM-DD"; connections+: ISO timestamp
    name        TEXT,           -- NULL for NYT; emoji/title for connections+
    UNIQUE (source, external_id)
);

-- 0=yellow (easiest) … 3=purple (hardest), matching NYT difficulty bands
CREATE TABLE IF NOT EXISTS categories (
    id        INTEGER PRIMARY KEY AUTOINCREMENT,
    puzzle_id INTEGER NOT NULL REFERENCES puzzles(id) ON DELETE CASCADE,
    title     TEXT NOT NULL,
    position  INTEGER NOT NULL CHECK (position BETWEEN 0 AND 3)
);

CREATE TABLE IF NOT EXISTS cards (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    category_id INTEGER NOT NULL REFERENCES categories(id) ON DELETE CASCADE,
    content     TEXT,       -- NULL for image puzzles
    image_url   TEXT,
    image_alt   TEXT,
    position    INTEGER NOT NULL CHECK (position BETWEEN 0 AND 15)
);

CREATE INDEX IF NOT EXISTS idx_puzzles_source_date ON puzzles(source, date);
CREATE INDEX IF NOT EXISTS idx_categories_puzzle   ON categories(puzzle_id);
CREATE INDEX IF NOT EXISTS idx_cards_category      ON cards(category_id);

-- ── Sessions ─────────────────────────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS sessions (
    id         TEXT PRIMARY KEY,  -- UUID v4
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    last_seen  TEXT NOT NULL DEFAULT (datetime('now'))
);

-- ── Game state ────────────────────────────────────────────────────────────────

-- One row per (session × puzzle). Created on first guess.
CREATE TABLE IF NOT EXISTS game_states (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id TEXT    NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
    puzzle_id  INTEGER NOT NULL REFERENCES puzzles(id),
    lives      INTEGER NOT NULL DEFAULT 4,
    created_at TEXT    NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT    NOT NULL DEFAULT (datetime('now')),
    UNIQUE (session_id, puzzle_id)
);

-- Each guess attempt. Cards referenced by real DB id, not position index.
CREATE TABLE IF NOT EXISTS guesses (
    id            INTEGER PRIMARY KEY AUTOINCREMENT,
    game_state_id INTEGER NOT NULL REFERENCES game_states(id) ON DELETE CASCADE,
    turn          INTEGER NOT NULL,
    card_id_1     INTEGER NOT NULL REFERENCES cards(id),
    card_id_2     INTEGER NOT NULL REFERENCES cards(id),
    card_id_3     INTEGER NOT NULL REFERENCES cards(id),
    card_id_4     INTEGER NOT NULL REFERENCES cards(id),
    result        TEXT NOT NULL CHECK (result IN ('correct', 'one_away', 'wrong'))
);

-- Categories solved within a game (subset of the puzzle's 4 categories).
CREATE TABLE IF NOT EXISTS solved_categories (
    game_state_id INTEGER NOT NULL REFERENCES game_states(id) ON DELETE CASCADE,
    category_id   INTEGER NOT NULL REFERENCES categories(id),
    turn          INTEGER NOT NULL,
    PRIMARY KEY (game_state_id, category_id)
);

CREATE INDEX IF NOT EXISTS idx_game_states_session ON game_states(session_id);
CREATE INDEX IF NOT EXISTS idx_guesses_game_state  ON guesses(game_state_id);
