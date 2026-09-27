-- ApiTest phase 3 schema upgrades

CREATE TABLE IF NOT EXISTS workspaces (
  id         INTEGER PRIMARY KEY AUTOINCREMENT,
  name       TEXT NOT NULL UNIQUE,
  is_active  INTEGER NOT NULL DEFAULT 0,
  created_at TEXT NOT NULL DEFAULT (datetime('now')),
  updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

INSERT OR IGNORE INTO workspaces (id, name, is_active) VALUES (1, 'Default', 1);

-- collections / environments scoped by workspace (SQLite ignores ADD if we guard in app; use IF NOT EXISTS via pragma in code)
-- request script & mock columns added in Rust migrator for compatibility

UPDATE meta SET value = '3' WHERE key = 'schema_version';
INSERT OR IGNORE INTO meta (key, value) VALUES ('theme', 'light');
