-- Team push (agents/research/PWA-APPS.md §5, owner 2026-10-18): creator, admin and embassy get their own push messages.
-- 1. push_subscriptions.app – which app a browser subscribed in (viewer = the reader's "new chapter"; creator / admin /
--    embassy = the team's kinds); every existing row is the reader's.
-- 2. push_prefs – a team member's own choice per kind (no row = the kind's default: on); `embassy.mail` = the Embassy
--    reminder by mail (the push and the mail share the reminder's rhythm, the person picks the channels).
-- 3. push_notified – what a person was told about once (a review, a proposal, a draft version, the Embassy day), so a
--    run never repeats it.
-- 4. push_sends.toon / chapter may be empty (a team send has no chapter).
-- MariaDB 10.11, idempotent (IF NOT EXISTS / MODIFY to the same type).
ALTER TABLE `push_subscriptions`
  ADD COLUMN IF NOT EXISTS `app` VARCHAR(16) NOT NULL DEFAULT 'viewer' COMMENT 'viewer | creator | admin | embassy – the app the browser subscribed in' AFTER `locale`;
CREATE INDEX IF NOT EXISTS `push_subscriptions_app_user` ON `push_subscriptions` (`app`, `user_id`);

CREATE TABLE IF NOT EXISTS `push_prefs` (
  `user_id` CHAR(36) NOT NULL,
  `kind` VARCHAR(32) NOT NULL COMMENT 'embassy.due | embassy.mail | admin.proposals | creator.reviews | creator.drafts',
  `on` TINYINT(1) NOT NULL,
  `updated_at` DATETIME NOT NULL COMMENT 'UTC',
  PRIMARY KEY (`user_id`, `kind`),
  CONSTRAINT `push_prefs_user_fk` FOREIGN KEY (`user_id`) REFERENCES `users` (`id`) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_general_ci;

CREATE TABLE IF NOT EXISTS `push_notified` (
  `user_id` CHAR(36) NOT NULL,
  `kind` VARCHAR(32) NOT NULL,
  `ref` VARCHAR(191) NOT NULL COMMENT 'what it was about: a review / proposal id, entity key + version, the Embassy day',
  `sent_at` DATETIME NOT NULL COMMENT 'UTC',
  PRIMARY KEY (`user_id`, `kind`, `ref`),
  KEY `push_notified_sent` (`sent_at`),
  CONSTRAINT `push_notified_user_fk` FOREIGN KEY (`user_id`) REFERENCES `users` (`id`) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_general_ci;

ALTER TABLE `push_sends`
  MODIFY COLUMN `toon` VARCHAR(64) NULL DEFAULT NULL,
  MODIFY COLUMN `chapter` INT NULL DEFAULT NULL;
