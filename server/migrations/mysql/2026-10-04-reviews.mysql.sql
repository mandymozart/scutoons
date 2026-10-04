-- Review before publish (agents/CUTOVER.md 10.28 "Review levels"). MariaDB, idempotent, two new tables, nothing else
-- changes (no backfill: without rules every creator publishes directly, as before).
-- content_review_rules: per toon (`*` = every toon) an admin marks accounts as `reviewer` (gets the mails, publishes or
--   sends back) or `needs_review` (may save drafts and submit them, but not publish / unpublish that toon). Only
--   toon-level entities (every kind with a toon in its key) are reviewed; platform-wide ones publish directly.
-- content_reviews: one row per submit, pinned to the submitted version. A newer submit for the same key annuls the
--   open one. state: pending | published | returned | annulled.
CREATE TABLE IF NOT EXISTS `content_review_rules` (
  `toon` VARCHAR(64) NOT NULL COMMENT 'toon id | * = every toon',
  `user_id` CHAR(36) NOT NULL,
  `rule` VARCHAR(16) NOT NULL COMMENT 'reviewer | needs_review',
  `created_at` DATETIME NOT NULL COMMENT 'UTC',
  `created_by` VARCHAR(255) NULL DEFAULT NULL COMMENT 'admin:<email> | cli',
  PRIMARY KEY (`toon`, `user_id`, `rule`),
  KEY `content_review_rules_user` (`user_id`),
  CONSTRAINT `content_review_rules_user_fk` FOREIGN KEY (`user_id`) REFERENCES `users` (`id`) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_general_ci;

CREATE TABLE IF NOT EXISTS `content_reviews` (
  `id` BIGINT NOT NULL AUTO_INCREMENT,
  `entity_key` VARCHAR(191) NOT NULL,
  `toon` VARCHAR(64) NOT NULL,
  `version_id` BIGINT NOT NULL COMMENT 'entity_versions.id – what the reviewer publishes',
  `author` CHAR(36) NULL DEFAULT NULL COMMENT 'users.id of who submitted',
  `note` VARCHAR(1000) NULL DEFAULT NULL,
  `state` VARCHAR(16) NOT NULL DEFAULT 'pending' COMMENT 'pending | published | returned | annulled',
  `created_at` DATETIME NOT NULL COMMENT 'UTC',
  `decided_by` CHAR(36) NULL DEFAULT NULL,
  `decided_at` DATETIME NULL DEFAULT NULL COMMENT 'UTC',
  `decision_note` VARCHAR(1000) NULL DEFAULT NULL,
  PRIMARY KEY (`id`),
  KEY `content_reviews_key` (`entity_key`, `state`),
  KEY `content_reviews_toon` (`toon`, `state`),
  KEY `content_reviews_author` (`author`, `id`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_general_ci;
