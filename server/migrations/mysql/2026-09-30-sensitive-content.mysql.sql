-- "Hide sensitive content" per member (scutoons agents/ITEMS.md "Sensitive content"). MariaDB, idempotent.
-- users.hide_sensitive: 1 = content marked `sensitive: true` (e.g. the `naked` A-pose looks) is hidden – the default for
-- every account, existing ones included; the member turns it off on the account page (PUT /api/me/ { hide_sensitive }).
ALTER TABLE `users` ADD COLUMN IF NOT EXISTS `hide_sensitive` TINYINT(1) NOT NULL DEFAULT 1;
