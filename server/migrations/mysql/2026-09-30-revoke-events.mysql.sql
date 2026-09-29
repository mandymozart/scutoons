-- Revoke single codes + activity statistics (ITEMS.md "Revoke + activity statistics"). MariaDB, idempotent.
-- No device limits: a code can be shared; the owner revokes one code when it spreads too far.
-- Privacy: code_events never stores a raw IP, email or user agent. device_hash = HMAC of the browser's random
-- X-Device-Id; ip_hash = HMAC of the IP with a daily salt (derived from AUTH_SECRET + UTC date, no table).
ALTER TABLE `qr` ADD COLUMN IF NOT EXISTS `revoked_at` DATETIME NULL DEFAULT NULL COMMENT 'set = the code grants nothing';
ALTER TABLE `qr` ADD COLUMN IF NOT EXISTS `revoked_reason` VARCHAR(255) NULL DEFAULT NULL;

CREATE TABLE IF NOT EXISTS `code_events` (
  `id` BIGINT NOT NULL AUTO_INCREMENT,
  `code` VARCHAR(8) NOT NULL,
  `event` VARCHAR(16) NOT NULL COMMENT 'scan | access | claim | claim_refused | login | revoke | unrevoke',
  `device_hash` CHAR(64) NULL DEFAULT NULL,
  `user_id` CHAR(36) NULL DEFAULT NULL,
  `ip_hash` CHAR(64) NULL DEFAULT NULL,
  `toon` VARCHAR(64) NULL DEFAULT NULL,
  `created_at` DATETIME NOT NULL COMMENT 'UTC',
  PRIMARY KEY (`id`),
  KEY `code_events_code` (`code`, `created_at`),
  KEY `code_events_event` (`event`, `created_at`),
  KEY `code_events_dedupe` (`code`, `event`, `device_hash`, `created_at`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_general_ci;
