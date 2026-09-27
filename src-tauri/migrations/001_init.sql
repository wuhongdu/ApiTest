-- ApiTest phase 0 schema

CREATE TABLE IF NOT EXISTS meta (
  key   TEXT PRIMARY KEY NOT NULL,
  value TEXT NOT NULL
);

INSERT OR IGNORE INTO meta (key, value) VALUES ('schema_version', '1');

CREATE TABLE IF NOT EXISTS collections (
  id         INTEGER PRIMARY KEY AUTOINCREMENT,
  parent_id  INTEGER,
  name       TEXT NOT NULL,
  sort_order INTEGER NOT NULL DEFAULT 0,
  created_at TEXT NOT NULL DEFAULT (datetime('now')),
  updated_at TEXT NOT NULL DEFAULT (datetime('now')),
  FOREIGN KEY (parent_id) REFERENCES collections(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS requests (
  id            INTEGER PRIMARY KEY AUTOINCREMENT,
  collection_id INTEGER,
  name          TEXT NOT NULL,
  method        TEXT NOT NULL DEFAULT 'GET',
  url           TEXT NOT NULL DEFAULT '',
  params_json   TEXT NOT NULL DEFAULT '[]',
  headers_json  TEXT NOT NULL DEFAULT '[]',
  body_type     TEXT NOT NULL DEFAULT 'none',
  body_content  TEXT NOT NULL DEFAULT '',
  sort_order    INTEGER NOT NULL DEFAULT 0,
  created_at    TEXT NOT NULL DEFAULT (datetime('now')),
  updated_at    TEXT NOT NULL DEFAULT (datetime('now')),
  FOREIGN KEY (collection_id) REFERENCES collections(id) ON DELETE SET NULL
);

CREATE TABLE IF NOT EXISTS environments (
  id         INTEGER PRIMARY KEY AUTOINCREMENT,
  name       TEXT NOT NULL UNIQUE,
  is_active  INTEGER NOT NULL DEFAULT 0,
  created_at TEXT NOT NULL DEFAULT (datetime('now')),
  updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS environment_vars (
  id             INTEGER PRIMARY KEY AUTOINCREMENT,
  environment_id INTEGER NOT NULL,
  key            TEXT NOT NULL,
  value          TEXT NOT NULL DEFAULT '',
  enabled        INTEGER NOT NULL DEFAULT 1,
  FOREIGN KEY (environment_id) REFERENCES environments(id) ON DELETE CASCADE,
  UNIQUE (environment_id, key)
);

CREATE TABLE IF NOT EXISTS history (
  id              INTEGER PRIMARY KEY AUTOINCREMENT,
  request_id      INTEGER,
  method          TEXT NOT NULL,
  url             TEXT NOT NULL,
  status_code     INTEGER,
  duration_ms     INTEGER,
  request_snapshot  TEXT,
  response_snapshot TEXT,
  created_at      TEXT NOT NULL DEFAULT (datetime('now')),
  FOREIGN KEY (request_id) REFERENCES requests(id) ON DELETE SET NULL
);

INSERT OR IGNORE INTO environments (id, name, is_active) VALUES (1, 'Default', 1);
