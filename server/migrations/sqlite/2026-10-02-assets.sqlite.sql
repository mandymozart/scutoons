-- The media asset registry (same as the .mysql.sql of this date; agents/ASSETS.md).
CREATE TABLE IF NOT EXISTS assets (
  id TEXT PRIMARY KEY,
  slug TEXT NOT NULL,
  kind TEXT NOT NULL,
  folder TEXT NULL DEFAULT NULL,
  title TEXT NOT NULL DEFAULT '',
  tags TEXT NOT NULL DEFAULT '[]',
  status TEXT NOT NULL DEFAULT 'active',
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  created_by TEXT NULL DEFAULT NULL
);
CREATE UNIQUE INDEX IF NOT EXISTS assets_slug ON assets (slug);
CREATE INDEX IF NOT EXISTS assets_folder ON assets (folder, kind);

CREATE TABLE IF NOT EXISTS asset_versions (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  asset_id TEXT NOT NULL REFERENCES assets (id) ON DELETE CASCADE,
  number INTEGER NOT NULL,
  path TEXT NOT NULL,
  storage TEXT NOT NULL,
  size INTEGER NOT NULL DEFAULT 0,
  sha256 TEXT NULL DEFAULT NULL,
  mime TEXT NULL DEFAULT NULL,
  width INTEGER NULL DEFAULT NULL,
  height INTEGER NULL DEFAULT NULL,
  duration REAL NULL DEFAULT NULL,
  status TEXT NOT NULL DEFAULT 'current',
  archive_path TEXT NULL DEFAULT NULL,
  note TEXT NULL DEFAULT NULL,
  created_at TEXT NOT NULL,
  created_by TEXT NULL DEFAULT NULL
);
CREATE UNIQUE INDEX IF NOT EXISTS asset_versions_number ON asset_versions (asset_id, number);
CREATE INDEX IF NOT EXISTS asset_versions_path ON asset_versions (path);

CREATE TABLE IF NOT EXISTS asset_renditions (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  asset_id TEXT NOT NULL REFERENCES assets (id) ON DELETE CASCADE,
  version_id INTEGER NOT NULL REFERENCES asset_versions (id) ON DELETE CASCADE,
  label TEXT NOT NULL,
  quality TEXT NULL DEFAULT NULL,
  path TEXT NOT NULL,
  size INTEGER NOT NULL DEFAULT 0,
  sha256 TEXT NULL DEFAULT NULL,
  mime TEXT NULL DEFAULT NULL,
  width INTEGER NULL DEFAULT NULL,
  height INTEGER NULL DEFAULT NULL,
  duration REAL NULL DEFAULT NULL,
  bitrate INTEGER NULL DEFAULT NULL,
  origin TEXT NOT NULL DEFAULT 'imported',
  modified_at TEXT NULL DEFAULT NULL,
  created_at TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS asset_renditions_path ON asset_renditions (path);
CREATE INDEX IF NOT EXISTS asset_renditions_version ON asset_renditions (version_id);
CREATE INDEX IF NOT EXISTS asset_renditions_asset ON asset_renditions (asset_id);

CREATE TABLE IF NOT EXISTS asset_usages (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  path TEXT NOT NULL,
  source TEXT NOT NULL,
  ref TEXT NOT NULL,
  field TEXT NOT NULL,
  state TEXT NOT NULL,
  version_id INTEGER NULL DEFAULT NULL,
  toon TEXT NULL DEFAULT NULL,
  raw TEXT NOT NULL,
  scanned_at TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS asset_usages_path ON asset_usages (path);
CREATE INDEX IF NOT EXISTS asset_usages_ref ON asset_usages (source, ref);

CREATE TABLE IF NOT EXISTS asset_jobs (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  asset_id TEXT NULL DEFAULT NULL,
  kind TEXT NOT NULL,
  status TEXT NOT NULL DEFAULT 'queued',
  params TEXT NOT NULL DEFAULT '{}',
  log TEXT NULL DEFAULT NULL,
  created_at TEXT NOT NULL,
  started_at TEXT NULL DEFAULT NULL,
  finished_at TEXT NULL DEFAULT NULL,
  created_by TEXT NULL DEFAULT NULL
);
CREATE INDEX IF NOT EXISTS asset_jobs_status ON asset_jobs (status, id);
CREATE INDEX IF NOT EXISTS asset_jobs_asset ON asset_jobs (asset_id, id);
