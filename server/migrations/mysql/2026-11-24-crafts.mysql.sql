-- Crafts (#826; owner 2026-11-23 "each craft has workflows hence needs workspaces that make sense to them"): what a member
-- does (illustrator, writer, translator …) and so which creator workspaces they start with. NOT the catalog recipes'
-- "crafting" (items made from items) – a separate word in code: Domain\Crafts, `crafts`, `member_crafts`.
-- `crafts.workspaces` = the creator's workspace ids (creator/src/lib/shell/address.ts WorkspaceKind), in the order a new
-- member sees them. The rows come in 2026-11-24-seed-crafts (data apart from DDL).
-- `member_crafts.source`: invite | admin | member. `users.crafts_asked_at`: the member answered the first-login question.
-- `member_invites`: an admin's invite of a team member by address with the crafts they start with, applied when that
-- address's account comes (normal signup with a code) or signs in. It signs no one up.
-- What it touches: three new tables (empty), one new nullable column. Dry run: SHOW TABLES LIKE '%craft%'; → member_crafts, crafts.
-- MariaDB 10.11, idempotent.
CREATE TABLE IF NOT EXISTS `crafts` (
  `id` VARCHAR(32) NOT NULL,
  `position` INT NOT NULL DEFAULT 0,
  `name` LONGTEXT NOT NULL COMMENT 'locale map { en, de, … }' CHECK (JSON_VALID(`name`)),
  `icon` VARCHAR(32) NOT NULL DEFAULT 'square',
  `workspaces` LONGTEXT NOT NULL COMMENT 'the creator workspace ids it starts with, in order' CHECK (JSON_VALID(`workspaces`)),
  PRIMARY KEY (`id`),
  KEY `crafts_position` (`position`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_general_ci;

CREATE TABLE IF NOT EXISTS `member_crafts` (
  `user_id` CHAR(36) NOT NULL,
  `craft_id` VARCHAR(32) NOT NULL,
  `source` VARCHAR(16) NOT NULL DEFAULT 'member' COMMENT 'invite | admin | member',
  `assigned_by` VARCHAR(255) NULL DEFAULT NULL,
  `created_at` DATETIME NOT NULL COMMENT 'UTC',
  PRIMARY KEY (`user_id`, `craft_id`),
  KEY `member_crafts_craft` (`craft_id`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_general_ci;

CREATE TABLE IF NOT EXISTS `member_invites` (
  `id` CHAR(36) NOT NULL,
  `email` VARCHAR(254) NOT NULL,
  `crafts` LONGTEXT NOT NULL COMMENT 'craft ids' CHECK (JSON_VALID(`crafts`)),
  `invited_by` VARCHAR(255) NULL DEFAULT NULL,
  `created_at` DATETIME NOT NULL COMMENT 'UTC',
  `accepted_at` DATETIME NULL DEFAULT NULL COMMENT 'UTC',
  `user_id` CHAR(36) NULL DEFAULT NULL,
  PRIMARY KEY (`id`),
  UNIQUE KEY `member_invites_email` (`email`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_general_ci;

ALTER TABLE `users` ADD COLUMN `crafts_asked_at` DATETIME NULL DEFAULT NULL COMMENT 'UTC: answered the first-login crafts question';
