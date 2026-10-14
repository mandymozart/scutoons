-- The avatar's home (agents/research/CAVE.md, FEATURES 10.98). MariaDB 10.11, idempotent.
-- catalog_home_scenes: what a home looks like (`cave`, `tatami_room` …; data = @scutoons/content HomeScene: a still +
--   an optional video loop, 1080p `src` / 4K `variants.high`) – edited in the catalog like albums; the first two rows
--   come without pictures (the admin uploads them; the reader draws a plain room meanwhile).
-- inventory.home_id / home_x / home_y / home_flip: a furniture entry standing in a home – its foot point in % of the
--   room, mirrored to face left; set once when placed and never cleared (furniture can't leave its home; only the mover
--   moves it into another home).
-- homes: a reader's homes – one per place (a character's `home`); its scene comes from the character it was made for.
-- home_residents: which of the reader's avatars (characters) lives in which home – at most 5 per home (the WG); a
--   character without a row lives in its own home.
-- home_visits: a friend's visit link (a token, 7 days, revocable) – opens the home read-only, the workbench usable.
CREATE TABLE IF NOT EXISTS `catalog_home_scenes` (
  `id` VARCHAR(100) NOT NULL,
  `position` INT NOT NULL DEFAULT 0,
  `data` LONGTEXT NOT NULL COMMENT '{ id, name, image: { src, variants? }, video?: { src, variants? } }' CHECK (JSON_VALID(`data`)),
  `updated_at` DATETIME NOT NULL COMMENT 'UTC',
  `updated_by` VARCHAR(255) NULL DEFAULT NULL,
  PRIMARY KEY (`id`),
  KEY `catalog_home_scenes_position` (`position`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_bin;

INSERT IGNORE INTO `catalog_home_scenes` (`id`, `position`, `data`, `updated_at`, `updated_by`) VALUES
  ('cave', 0, '{"id":"cave","name":{"de":"Höhle","en":"Cave"},"image":{"src":""}}', UTC_TIMESTAMP(), 'migration:2026-10-14-homes'),
  ('tatami_room', 1, '{"id":"tatami_room","name":{"de":"Tatami-Zimmer","en":"Tatami room"},"image":{"src":""}}', UTC_TIMESTAMP(), 'migration:2026-10-14-homes');

ALTER TABLE `inventory` ADD COLUMN IF NOT EXISTS `home_id` CHAR(36) NULL DEFAULT NULL COMMENT 'furniture: the home it stands in (same user)' AFTER `aged_since`;
ALTER TABLE `inventory` ADD COLUMN IF NOT EXISTS `home_x` DECIMAL(5,2) NULL DEFAULT NULL COMMENT 'furniture: its foot, % of the room width' AFTER `home_id`;
ALTER TABLE `inventory` ADD COLUMN IF NOT EXISTS `home_y` DECIMAL(5,2) NULL DEFAULT NULL COMMENT 'furniture: its foot, % of the room height' AFTER `home_x`;
ALTER TABLE `inventory` ADD COLUMN IF NOT EXISTS `home_flip` TINYINT(1) NOT NULL DEFAULT 0 COMMENT 'furniture: mirrored (faces left)' AFTER `home_y`;
ALTER TABLE `inventory` ADD INDEX IF NOT EXISTS `inventory_home` (`home_id`);

CREATE TABLE IF NOT EXISTS `homes` (
  `id` CHAR(36) NOT NULL,
  `user_id` CHAR(36) NOT NULL,
  `place` VARCHAR(100) NOT NULL COMMENT 'the map place id',
  `scene` VARCHAR(100) NOT NULL COMMENT 'catalog_home_scenes.id',
  `created_at` DATETIME NOT NULL COMMENT 'UTC',
  `updated_at` DATETIME NOT NULL COMMENT 'UTC',
  PRIMARY KEY (`id`),
  UNIQUE KEY `homes_user_place` (`user_id`, `place`),
  CONSTRAINT `homes_user_fk` FOREIGN KEY (`user_id`) REFERENCES `users` (`id`) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_general_ci;

CREATE TABLE IF NOT EXISTS `home_residents` (
  `user_id` CHAR(36) NOT NULL,
  `character_id` VARCHAR(64) NOT NULL,
  `home_id` CHAR(36) NOT NULL,
  `moved_at` DATETIME NOT NULL COMMENT 'UTC',
  PRIMARY KEY (`user_id`, `character_id`),
  KEY `home_residents_home` (`home_id`),
  CONSTRAINT `home_residents_user_fk` FOREIGN KEY (`user_id`) REFERENCES `users` (`id`) ON DELETE CASCADE,
  CONSTRAINT `home_residents_home_fk` FOREIGN KEY (`home_id`) REFERENCES `homes` (`id`) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_general_ci;

CREATE TABLE IF NOT EXISTS `home_visits` (
  `token` VARCHAR(64) NOT NULL,
  `user_id` CHAR(36) NOT NULL,
  `home_id` CHAR(36) NOT NULL,
  `created_at` DATETIME NOT NULL COMMENT 'UTC',
  `expires_at` DATETIME NOT NULL COMMENT 'UTC',
  `revoked_at` DATETIME NULL DEFAULT NULL COMMENT 'UTC',
  PRIMARY KEY (`token`),
  KEY `home_visits_home` (`home_id`),
  CONSTRAINT `home_visits_user_fk` FOREIGN KEY (`user_id`) REFERENCES `users` (`id`) ON DELETE CASCADE,
  CONSTRAINT `home_visits_home_fk` FOREIGN KEY (`home_id`) REFERENCES `homes` (`id`) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_general_ci;
