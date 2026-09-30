-- Reader avatar bodies & character unlocks (scutoons agents/ITEMS.md "Reader avatar bodies & character unlocks").
-- MariaDB, idempotent. Run after 2026-09-30-settings.
-- member_characters: characters a member has unlocked as avatar body (with the character's default A-pose look).
-- member_character_looks: A-pose looks beyond the default (e.g. `naked`).
-- The default unlocks of every member (owner 2026-09-30: catpope with swim + naked) are NOT rows: they come from the
-- setting AVATAR_DEFAULT_UNLOCKS (config/settings.php), so the list can change without touching accounts.
-- source: default | code | item | story | admin; ref: what granted it (code, item id, story point, admin:<email>).
-- avatar_body: the member's chosen body { character, look } (avatar.body of GET /me/, PUT /avatar/ { body }).
CREATE TABLE IF NOT EXISTS `member_characters` (
  `user_id` CHAR(36) NOT NULL,
  `character_id` VARCHAR(64) NOT NULL,
  `source` VARCHAR(16) NOT NULL DEFAULT 'admin' COMMENT 'default | code | item | story | admin',
  `ref` VARCHAR(255) NULL DEFAULT NULL,
  `unlocked_at` DATETIME NOT NULL COMMENT 'UTC',
  PRIMARY KEY (`user_id`, `character_id`),
  CONSTRAINT `member_characters_user_fk` FOREIGN KEY (`user_id`) REFERENCES `users` (`id`) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_general_ci;

CREATE TABLE IF NOT EXISTS `member_character_looks` (
  `user_id` CHAR(36) NOT NULL,
  `character_id` VARCHAR(64) NOT NULL,
  `look_id` VARCHAR(64) NOT NULL,
  `source` VARCHAR(16) NOT NULL DEFAULT 'admin' COMMENT 'default | code | item | story | admin',
  `ref` VARCHAR(255) NULL DEFAULT NULL,
  `unlocked_at` DATETIME NOT NULL COMMENT 'UTC',
  PRIMARY KEY (`user_id`, `character_id`, `look_id`),
  CONSTRAINT `member_character_looks_user_fk` FOREIGN KEY (`user_id`) REFERENCES `users` (`id`) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_general_ci;

CREATE TABLE IF NOT EXISTS `avatar_body` (
  `user_id` CHAR(36) NOT NULL,
  `character_id` VARCHAR(64) NOT NULL,
  `look_id` VARCHAR(64) NOT NULL,
  `updated_at` DATETIME NOT NULL COMMENT 'UTC',
  PRIMARY KEY (`user_id`),
  CONSTRAINT `avatar_body_user_fk` FOREIGN KEY (`user_id`) REFERENCES `users` (`id`) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_general_ci;
