-- Roles, signup rule, marketing opt-in (same as the .mysql.sql of this date).
CREATE TABLE IF NOT EXISTS roles (
  id INTEGER NOT NULL PRIMARY KEY,
  name TEXT NOT NULL UNIQUE,
  description TEXT NOT NULL DEFAULT ''
);

INSERT OR IGNORE INTO roles (id, name, description) VALUES
  (1, 'member', 'Every account'),
  (2, 'admin', 'Reserved (later: admin UI in the viewer)'),
  (3, 'creator', 'May use scutoons.com/creator and the content API');

CREATE TABLE IF NOT EXISTS user_roles (
  user_id TEXT NOT NULL REFERENCES users (id) ON DELETE CASCADE,
  role_id INTEGER NOT NULL REFERENCES roles (id) ON DELETE CASCADE,
  granted_at TEXT NOT NULL,
  granted_by TEXT NULL DEFAULT NULL,
  PRIMARY KEY (user_id, role_id)
);
CREATE INDEX IF NOT EXISTS user_roles_role ON user_roles (role_id);

ALTER TABLE users ADD COLUMN marketing_opt_in INTEGER NOT NULL DEFAULT 0;
ALTER TABLE users ADD COLUMN marketing_opt_in_at TEXT NULL DEFAULT NULL;
ALTER TABLE users ADD COLUMN signup_code TEXT NULL DEFAULT NULL;
ALTER TABLE login_requests ADD COLUMN marketing_opt_in INTEGER NOT NULL DEFAULT 0;

INSERT OR IGNORE INTO user_roles (user_id, role_id, granted_at, granted_by)
  SELECT id, 1, strftime('%Y-%m-%d %H:%M:%S', 'now'), 'migration' FROM users;
