-- "Show hints" per member (owner 2026-10-07, BACKLOG M.1): the reader outlines what can be tapped / hovered (VN click and
-- hover areas, sound areas) only when this is on – off by default for every account, existing ones included; the member
-- turns it on on the account page (PUT /api/me/ { show_hints }). MariaDB, idempotent.
ALTER TABLE `users` ADD COLUMN IF NOT EXISTS `show_hints` TINYINT(1) NOT NULL DEFAULT 0;
