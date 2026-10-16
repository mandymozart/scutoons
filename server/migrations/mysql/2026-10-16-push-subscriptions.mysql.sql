-- Web Push (agents/research/PWA.md §2b, FEATURES 10.74 phase 4): the browsers that opted in on the account page
-- (POST /api/push/subscribe) and the "new chapter" sends from the admin app (POST /api/admin/push/new-chapter). Nothing
-- is sent automatically. A subscription belongs to a browser (endpoint), optionally to the signed-in account (deleted
-- with it). MariaDB 10.11, idempotent.
CREATE TABLE IF NOT EXISTS `push_subscriptions` (
  `id` CHAR(64) NOT NULL COMMENT 'sha256 of the endpoint',
  `endpoint` VARCHAR(1024) NOT NULL COMMENT 'the push service URL (https, a known push service)',
  `p256dh` VARCHAR(128) NOT NULL COMMENT 'the browser''s public key, base64url',
  `auth` VARCHAR(32) NOT NULL COMMENT 'the browser''s auth secret, base64url',
  `user_id` CHAR(36) NULL DEFAULT NULL,
  `locale` VARCHAR(8) NOT NULL DEFAULT 'de',
  `failures` INT NOT NULL DEFAULT 0 COMMENT 'sends in a row the push service refused (not 404 / 410: those delete the row)',
  `created_at` DATETIME NOT NULL COMMENT 'UTC',
  `updated_at` DATETIME NOT NULL COMMENT 'UTC',
  PRIMARY KEY (`id`),
  KEY `push_subscriptions_user` (`user_id`),
  CONSTRAINT `push_subscriptions_user_fk` FOREIGN KEY (`user_id`) REFERENCES `users` (`id`) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_general_ci;

CREATE TABLE IF NOT EXISTS `push_sends` (
  `id` INT NOT NULL AUTO_INCREMENT,
  `kind` VARCHAR(32) NOT NULL DEFAULT 'new_chapter',
  `toon` VARCHAR(64) NOT NULL,
  `chapter` INT NOT NULL,
  `test` TINYINT(1) NOT NULL DEFAULT 0 COMMENT '1 = only to the sending admin''s own browsers',
  `recipients` INT NOT NULL DEFAULT 0,
  `delivered` INT NOT NULL DEFAULT 0,
  `gone` INT NOT NULL DEFAULT 0,
  `failed` INT NOT NULL DEFAULT 0,
  `sent_by` CHAR(36) NULL DEFAULT NULL,
  `sent_at` DATETIME NOT NULL COMMENT 'UTC',
  PRIMARY KEY (`id`),
  KEY `push_sends_chapter` (`toon`, `chapter`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_general_ci;
