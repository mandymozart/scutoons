-- Runtime settings the admin app changes without a deploy (Domain\Settings\RuntimeSettings, admin API
-- GET/PUT /api/admin/settings): e.g. scan_limit_per_hour, lookup_limit_per_hour. A missing row = the default from the
-- config (config/settings.php). `value` is JSON (a number today). MariaDB, idempotent.
CREATE TABLE IF NOT EXISTS `settings` (
  `key` VARCHAR(64) NOT NULL,
  `value` JSON NOT NULL,
  `updated_at` DATETIME NOT NULL COMMENT 'UTC',
  `updated_by` VARCHAR(255) NULL DEFAULT NULL COMMENT 'admin:<email> | cli',
  PRIMARY KEY (`key`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_general_ci;
