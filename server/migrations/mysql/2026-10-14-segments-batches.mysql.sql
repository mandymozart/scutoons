-- Segments, a campaign's segment, code batches (agents/research/GIFT-INVITES.md 1b; owner 2026-10-14). MariaDB,
-- idempotent; two new tables, two new columns, nothing else changes. Run after 2026-10-14-campaigns.
-- segments: named groups of accounts the admin assigns by hand (Users & roles → Add to segment).
-- segment_members: who is in a segment.
-- campaigns.segment_id: inviters = segment → the members of this segment may invite.
-- qr.batch: the batch a code was made in (the manifest file BATCH-….json) – product batches of an edition and promo
--   batches (owner: "promo batches like editions"); NULL = made on its own. Older batch codes: `php bin/console
--   backfill-edition-numbers` fills it from the manifests.
CREATE TABLE IF NOT EXISTS `segments` (
  `id` CHAR(36) NOT NULL,
  `name` VARCHAR(100) NOT NULL,
  `created_by` CHAR(36) NULL DEFAULT NULL,
  `created_at` DATETIME NOT NULL COMMENT 'UTC',
  PRIMARY KEY (`id`),
  UNIQUE KEY `segments_name` (`name`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_general_ci;

CREATE TABLE IF NOT EXISTS `segment_members` (
  `segment_id` CHAR(36) NOT NULL,
  `user_id` CHAR(36) NOT NULL,
  `added_by` CHAR(36) NULL DEFAULT NULL,
  `added_at` DATETIME NOT NULL COMMENT 'UTC',
  PRIMARY KEY (`segment_id`, `user_id`),
  KEY `segment_members_user` (`user_id`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_general_ci;

ALTER TABLE `campaigns` ADD COLUMN IF NOT EXISTS `segment_id` CHAR(36) NULL DEFAULT NULL COMMENT 'inviters = segment' AFTER `inviters`;

ALTER TABLE `qr` ADD COLUMN IF NOT EXISTS `batch` VARCHAR(100) NULL DEFAULT NULL COMMENT 'the batch manifest (BATCH-….json), NULL = made on its own';
ALTER TABLE `qr` ADD INDEX IF NOT EXISTS `qr_batch` (`batch`);
