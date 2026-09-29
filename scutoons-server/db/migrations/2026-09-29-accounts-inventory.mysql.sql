-- Accounts, items and inventory (scutoons agents/ITEMS.md). MariaDB 10.3+, utf8mb4.
-- Idempotent: CREATE TABLE IF NOT EXISTS / ADD COLUMN IF NOT EXISTS (MariaDB) - safe to run again, e.g. in phpMyAdmin.
-- Times are UTC DATETIMEs written by PHP (no NOW()), so the same queries run on SQLite locally.
-- Collation utf8mb4_general_ci like the existing `qr` table (joins on `code` must not mix collations).

-- Which edition number (n of edition_count) a code is, and which user claimed it (NULL = unclaimed or legacy email claim)
ALTER TABLE `qr` ADD COLUMN IF NOT EXISTS `edition_number` INT NULL DEFAULT NULL COMMENT 'n of the edition (batch manifest)';
ALTER TABLE `qr` ADD COLUMN IF NOT EXISTS `claimed_by` CHAR(36) NULL DEFAULT NULL COMMENT 'users.id of the claimer';

CREATE TABLE IF NOT EXISTS `users` (
  `id` CHAR(36) NOT NULL,
  `email` VARCHAR(254) NOT NULL,
  `locale` VARCHAR(8) NOT NULL DEFAULT 'en',
  `created_at` DATETIME NOT NULL,
  `confirmed_at` DATETIME NOT NULL,
  `updated_at` DATETIME NOT NULL,
  PRIMARY KEY (`id`),
  UNIQUE KEY `users_email` (`email`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_general_ci;

-- One per "send me a login email": magic-link token and 6-digit code, stored as hashes only.
-- claim_* = a code (and the chosen item/variant) to claim when the login is confirmed.
CREATE TABLE IF NOT EXISTS `login_requests` (
  `id` CHAR(36) NOT NULL,
  `email` VARCHAR(254) NOT NULL,
  `token_hash` CHAR(64) NOT NULL,
  `code_hash` CHAR(64) NOT NULL,
  `locale` VARCHAR(8) NOT NULL DEFAULT 'en',
  `claim_code` VARCHAR(8) NULL DEFAULT NULL,
  `claim_item_id` VARCHAR(64) NULL DEFAULT NULL,
  `claim_variant` VARCHAR(64) NULL DEFAULT NULL,
  `ip_hash` CHAR(64) NOT NULL,
  `attempts` INT NOT NULL DEFAULT 0,
  `created_at` DATETIME NOT NULL,
  `expires_at` DATETIME NOT NULL,
  `used_at` DATETIME NULL DEFAULT NULL,
  PRIMARY KEY (`id`),
  UNIQUE KEY `login_requests_token` (`token_hash`),
  KEY `login_requests_email` (`email`, `created_at`),
  KEY `login_requests_ip` (`ip_hash`, `created_at`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_general_ci;

-- A signed-in device: the client keeps the token (Bearer), the server only its hash
CREATE TABLE IF NOT EXISTS `sessions` (
  `id` CHAR(36) NOT NULL,
  `user_id` CHAR(36) NOT NULL,
  `token_hash` CHAR(64) NOT NULL,
  `created_at` DATETIME NOT NULL,
  `last_used_at` DATETIME NOT NULL,
  PRIMARY KEY (`id`),
  UNIQUE KEY `sessions_token` (`token_hash`),
  KEY `sessions_user` (`user_id`),
  CONSTRAINT `sessions_user_fk` FOREIGN KEY (`user_id`) REFERENCES `users` (`id`) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_general_ci;

-- What a code contains. No rows + qr.edition_id set = the edition's catalog items are options (picked at claim time)
CREATE TABLE IF NOT EXISTS `code_items` (
  `id` INT NOT NULL AUTO_INCREMENT,
  `code` VARCHAR(8) NOT NULL,
  `item_id` VARCHAR(64) NOT NULL,
  `variant` VARCHAR(64) NULL DEFAULT NULL,
  `qty` INT NOT NULL DEFAULT 1,
  PRIMARY KEY (`id`),
  KEY `code_items_code` (`code`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_general_ci;

-- Inventory entries (ITEMS.md `Entry`). Stackable items: one row per (user, item, variant) with qty;
-- others: one row per piece. source: qr | story | scan | craft | starter | import
CREATE TABLE IF NOT EXISTS `inventory` (
  `id` CHAR(36) NOT NULL,
  `user_id` CHAR(36) NOT NULL,
  `item_id` VARCHAR(64) NOT NULL,
  `variant` VARCHAR(64) NULL DEFAULT NULL,
  `qty` INT NOT NULL DEFAULT 1,
  `source` VARCHAR(16) NOT NULL,
  `code` VARCHAR(8) NULL DEFAULT NULL,
  `edition_number` INT NULL DEFAULT NULL,
  `acquired_at` DATETIME NOT NULL,
  `updated_at` DATETIME NOT NULL,
  PRIMARY KEY (`id`),
  KEY `inventory_user` (`user_id`, `item_id`),
  KEY `inventory_code` (`code`),
  CONSTRAINT `inventory_user_fk` FOREIGN KEY (`user_id`) REFERENCES `users` (`id`) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_general_ci;

-- The avatar: one inventory entry per body-part slot
CREATE TABLE IF NOT EXISTS `avatar_slots` (
  `user_id` CHAR(36) NOT NULL,
  `slot` VARCHAR(32) NOT NULL,
  `entry_id` CHAR(36) NOT NULL,
  `updated_at` DATETIME NOT NULL,
  PRIMARY KEY (`user_id`, `slot`),
  CONSTRAINT `avatar_slots_user_fk` FOREIGN KEY (`user_id`) REFERENCES `users` (`id`) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_general_ci;

-- Viewer game state per toon (JSON), last write wins by updated_at (ms)
CREATE TABLE IF NOT EXISTS `progress` (
  `user_id` CHAR(36) NOT NULL,
  `toon_id` VARCHAR(64) NOT NULL,
  `state` MEDIUMTEXT NOT NULL,
  `updated_at` BIGINT NOT NULL COMMENT 'milliseconds since 1970 (UTC), sent by the client',
  PRIMARY KEY (`user_id`, `toon_id`),
  CONSTRAINT `progress_user_fk` FOREIGN KEY (`user_id`) REFERENCES `users` (`id`) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_general_ci;

-- Rate-limit events (grants, imports): count per bucket in a time window; ref de-duplicates (e.g. a story grant)
CREATE TABLE IF NOT EXISTS `rate_limits` (
  `id` CHAR(36) NOT NULL,
  `bucket` VARCHAR(191) NOT NULL,
  `ref` VARCHAR(191) NULL DEFAULT NULL,
  `created_at` DATETIME NOT NULL,
  PRIMARY KEY (`id`),
  KEY `rate_limits_bucket` (`bucket`, `created_at`),
  KEY `rate_limits_ref` (`bucket`, `ref`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_general_ci;
