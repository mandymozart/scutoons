-- Embassy: partner network of Top Toy / SCUTOONS (same as the .mysql.sql of this date, agents/EMBASSY.md).
INSERT OR IGNORE INTO roles (id, name, description) VALUES
  (4, 'liaison', 'Embassy (part time): works the leads, calls, mails, notes'),
  (5, 'ambassador', 'Embassy (full time): everything a liaison does, plus assigning, importing, deleting');

CREATE TABLE IF NOT EXISTS embassy_orgs (
  id TEXT NOT NULL PRIMARY KEY,
  name TEXT NOT NULL,
  type TEXT NOT NULL DEFAULT 'other',
  city TEXT NOT NULL DEFAULT '',
  country TEXT NOT NULL DEFAULT '',
  website TEXT NOT NULL DEFAULT '',
  email TEXT NOT NULL DEFAULT '',
  phone TEXT NOT NULL DEFAULT '',
  links TEXT NOT NULL DEFAULT '[]',
  tags TEXT NOT NULL DEFAULT '[]',
  territories TEXT NOT NULL DEFAULT '[]',
  tier TEXT NULL DEFAULT NULL,
  source TEXT NOT NULL DEFAULT '',
  notes TEXT NOT NULL DEFAULT '',
  do_not_contact INTEGER NOT NULL DEFAULT 0,
  name_key TEXT NOT NULL,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  created_by TEXT NULL DEFAULT NULL
);
CREATE INDEX IF NOT EXISTS embassy_orgs_name_key ON embassy_orgs (name_key);
CREATE INDEX IF NOT EXISTS embassy_orgs_country ON embassy_orgs (country);

CREATE TABLE IF NOT EXISTS embassy_people (
  id TEXT NOT NULL PRIMARY KEY,
  org_id TEXT NOT NULL REFERENCES embassy_orgs (id) ON DELETE CASCADE,
  name TEXT NOT NULL DEFAULT '',
  role TEXT NOT NULL DEFAULT '',
  email TEXT NOT NULL DEFAULT '',
  phone TEXT NOT NULL DEFAULT '',
  link TEXT NOT NULL DEFAULT '',
  language TEXT NOT NULL DEFAULT '',
  territories TEXT NOT NULL DEFAULT '[]',
  notes TEXT NOT NULL DEFAULT '',
  do_not_contact INTEGER NOT NULL DEFAULT 0,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS embassy_people_org ON embassy_people (org_id);

CREATE TABLE IF NOT EXISTS embassy_leads (
  id TEXT NOT NULL PRIMARY KEY,
  org_id TEXT NOT NULL REFERENCES embassy_orgs (id) ON DELETE CASCADE,
  track TEXT NOT NULL,
  stage TEXT NOT NULL DEFAULT 'research',
  owner_id TEXT NULL DEFAULT NULL REFERENCES users (id) ON DELETE SET NULL,
  next_at TEXT NULL DEFAULT NULL,
  last_contact_at TEXT NULL DEFAULT NULL,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  UNIQUE (org_id, track)
);
CREATE INDEX IF NOT EXISTS embassy_leads_next ON embassy_leads (next_at);
CREATE INDEX IF NOT EXISTS embassy_leads_owner ON embassy_leads (owner_id);

CREATE TABLE IF NOT EXISTS embassy_activities (
  id TEXT NOT NULL PRIMARY KEY,
  org_id TEXT NOT NULL REFERENCES embassy_orgs (id) ON DELETE CASCADE,
  lead_id TEXT NULL DEFAULT NULL REFERENCES embassy_leads (id) ON DELETE SET NULL,
  person_id TEXT NULL DEFAULT NULL REFERENCES embassy_people (id) ON DELETE SET NULL,
  kind TEXT NOT NULL,
  outcome TEXT NULL DEFAULT NULL,
  note TEXT NOT NULL DEFAULT '',
  by_user_id TEXT NULL DEFAULT NULL REFERENCES users (id) ON DELETE SET NULL,
  by_email TEXT NOT NULL DEFAULT '',
  at TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS embassy_activities_org ON embassy_activities (org_id, at);
