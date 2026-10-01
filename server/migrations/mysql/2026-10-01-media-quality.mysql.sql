-- "Media quality" per member (scutoons agents/DESIGN.md "Media quality", owner 2026-10-01). MariaDB, idempotent.
-- users.media_quality: auto (by the measured bandwidth, the default for every account) | low (480p) | medium (720p) |
-- high (1080p); the member picks it on the account page (PUT /api/me/ { media_quality }). Guests keep it in localStorage.
ALTER TABLE `users` ADD COLUMN IF NOT EXISTS `media_quality` VARCHAR(8) NOT NULL DEFAULT 'auto';
