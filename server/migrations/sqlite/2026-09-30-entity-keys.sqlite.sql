-- Entity keys (agents/DATA.md §2): the content store keyed by entity instead of toon/locale/path. New table
-- entity_versions; every row of content_versions is copied with the SAME id (history, "v71" labels and drafts stay),
-- its key computed from the old path. content_versions is left untouched (rollback) and no longer used.
-- Keys: toon:<toon> · toon-settings:<toon>:<l> · season:<toon>:<n>:<l> · chapter:<toon>:<n>:<l> ·
-- scene:<toon>:<n>:<name>:<l> · locations:<toon>:<l> · routes:<toon>:<l> · cast:<toon> · credits:<toon> ·
-- characters · moods · asset-requests · people · roles · events. Idempotent (rows already copied are skipped).
CREATE TABLE IF NOT EXISTS entity_versions (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  entity_key TEXT NOT NULL,
  kind TEXT NOT NULL,
  toon TEXT NULL DEFAULT NULL,
  locale TEXT NULL DEFAULT NULL,
  json TEXT NOT NULL,
  author TEXT NULL DEFAULT NULL,
  created_at TEXT NOT NULL,
  status TEXT NOT NULL DEFAULT 'draft',
  message TEXT NULL DEFAULT NULL
);
CREATE INDEX IF NOT EXISTS entity_versions_key ON entity_versions (entity_key, id);
CREATE INDEX IF NOT EXISTS entity_versions_status ON entity_versions (entity_key, status);
CREATE INDEX IF NOT EXISTS entity_versions_toon ON entity_versions (toon, status);
CREATE INDEX IF NOT EXISTS entity_versions_kind ON entity_versions (kind, status);
INSERT INTO entity_versions (id, entity_key, kind, toon, locale, json, author, created_at, status, message)
SELECT id, k, substr(k, 1, instr(k || ':', ':') - 1), CASE WHEN toon = 'scutoons' THEN NULL ELSE toon END,
  CASE WHEN locale = '_' THEN NULL ELSE locale END, json, author, created_at, status, message
FROM (
  SELECT c.*, CASE
    WHEN locale = '_' AND path = 'index.json' THEN 'toon:' || toon
    WHEN locale = '_' AND path IN ('cast.json', 'credits.json') THEN replace(path, '.json', '') || ':' || toon
    WHEN locale = '_' AND toon = 'scutoons' AND instr(path, '/') = 0 THEN replace(path, '.json', '')
    WHEN locale <> '_' AND path = 'toon.json' THEN 'toon-settings:' || toon || ':' || locale
    WHEN locale <> '_' AND path IN ('locations.json', 'routes.json') THEN replace(path, '.json', '') || ':' || toon || ':' || locale
    WHEN locale <> '_' AND instr(path, '/') > 0 AND path GLOB 'chapter[0-9]*/*.json'
      THEN 'scene:' || toon || ':' || substr(path, 8, instr(path, '/') - 8) || ':' || substr(path, instr(path, '/') + 1, length(path) - instr(path, '/') - 5) || ':' || locale
    WHEN locale <> '_' AND instr(path, '/') = 0 AND path GLOB 'season[0-9]*.json' THEN 'season:' || toon || ':' || substr(path, 7, length(path) - 11) || ':' || locale
    WHEN locale <> '_' AND instr(path, '/') = 0 AND path GLOB 'chapter[0-9]*.json' THEN 'chapter:' || toon || ':' || substr(path, 8, length(path) - 12) || ':' || locale
    ELSE 'legacy:' || toon || ':' || locale || ':' || replace(replace(path, '/', '~'), '.json', '')
  END AS k
  FROM content_versions c
) m
WHERE NOT EXISTS (SELECT 1 FROM entity_versions e WHERE e.id = m.id);
