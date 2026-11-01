-- Instruments, presets, rigs (#258, agents/research/INSTRUMENTS.md §5; owner 2026-11-01: "factory presets and rigs and …
-- modification by individual users"). Two layers:
-- catalog_instruments / catalog_presets / catalog_rigs: the factory, catalog kinds like catalog_items (agents/CATALOG.md:
--   one row per entry, `data` = the serde form of deck/space/src/rig – Instrument, Preset, Rig –, catalog_history + undo,
--   documents `instrument:<id>`, `preset:<id>`, `rig:<subject>` in the creator). A rig's id is its subject
--   (`character.<id>[.<persona>]`, `item.<id>`); a preset's `instrument` / `parent` are columns kept in step with `data`.
-- member_presets: a member's derivatives (id `m:<ulid>`, so a lineage crosses from the factory into a member's);
--   `parent` = a factory preset or one of the member's own. member_rigs: a member's variant of a rig, one per subject.
-- The core rigs' rows come with 2026-11-01-rigs_core (data only, its own file). MariaDB 10.11, idempotent.
CREATE TABLE IF NOT EXISTS `catalog_instruments` (
  `id` VARCHAR(100) NOT NULL,
  `position` INT NOT NULL DEFAULT 0,
  `data` LONGTEXT NOT NULL COMMENT 'deck-space rig::Instrument { id, kind, name?, node, link?, out, expose?, rights }' CHECK (JSON_VALID(`data`)),
  `updated_at` DATETIME NOT NULL COMMENT 'UTC',
  `updated_by` VARCHAR(255) NULL DEFAULT NULL,
  PRIMARY KEY (`id`),
  KEY `catalog_instruments_position` (`position`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_bin;

CREATE TABLE IF NOT EXISTS `catalog_presets` (
  `id` VARCHAR(100) NOT NULL,
  `position` INT NOT NULL DEFAULT 0,
  `data` LONGTEXT NOT NULL COMMENT 'deck-space rig::Preset { id, instrument, parent?, name?, tags?, values?, unlink?, link?, mapping?, rights }' CHECK (JSON_VALID(`data`)),
  `instrument` VARCHAR(100) NULL DEFAULT NULL COMMENT 'catalog_instruments.id (= data.instrument)',
  `parent` VARCHAR(100) NULL DEFAULT NULL COMMENT 'catalog_presets.id (= data.parent): the preset it derives from',
  `updated_at` DATETIME NOT NULL COMMENT 'UTC',
  `updated_by` VARCHAR(255) NULL DEFAULT NULL,
  PRIMARY KEY (`id`),
  KEY `catalog_presets_position` (`position`),
  KEY `catalog_presets_instrument` (`instrument`),
  KEY `catalog_presets_parent` (`parent`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_bin;

CREATE TABLE IF NOT EXISTS `catalog_rigs` (
  `id` VARCHAR(100) NOT NULL COMMENT 'the subject: character.<id>[.<persona>] | item.<id>',
  `position` INT NOT NULL DEFAULT 0,
  `data` LONGTEXT NOT NULL COMMENT 'deck-space rig::Rig { subject, track, route?, personas?, moods?, rights }' CHECK (JSON_VALID(`data`)),
  `updated_at` DATETIME NOT NULL COMMENT 'UTC',
  `updated_by` VARCHAR(255) NULL DEFAULT NULL,
  PRIMARY KEY (`id`),
  KEY `catalog_rigs_position` (`position`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_bin;

CREATE TABLE IF NOT EXISTS `member_presets` (
  `id` VARCHAR(40) CHARACTER SET utf8mb4 COLLATE utf8mb4_bin NOT NULL COMMENT 'm:<ulid>',
  `user_id` CHAR(36) NOT NULL COMMENT 'users.id: whose derivative',
  `parent` VARCHAR(100) CHARACTER SET utf8mb4 COLLATE utf8mb4_bin NOT NULL COMMENT 'a catalog_presets.id or one of the member''s own member_presets.id',
  `instrument` VARCHAR(100) CHARACTER SET utf8mb4 COLLATE utf8mb4_bin NOT NULL COMMENT 'catalog_instruments.id (= data.instrument, the parent''s)',
  `data` LONGTEXT NOT NULL COMMENT 'deck-space rig::Preset with id = this id, parent = parent' CHECK (JSON_VALID(`data`)),
  `created_at` DATETIME NOT NULL COMMENT 'UTC',
  PRIMARY KEY (`id`),
  KEY `member_presets_user` (`user_id`),
  KEY `member_presets_parent` (`parent`),
  CONSTRAINT `member_presets_user_fk` FOREIGN KEY (`user_id`) REFERENCES `users` (`id`) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_general_ci;

CREATE TABLE IF NOT EXISTS `member_rigs` (
  `user_id` CHAR(36) NOT NULL COMMENT 'users.id',
  `subject` VARCHAR(100) CHARACTER SET utf8mb4 COLLATE utf8mb4_bin NOT NULL COMMENT 'the rig''s subject (a catalog_rigs.id): character.<id>[.<persona>] | item.<id>',
  `data` LONGTEXT NOT NULL COMMENT 'deck-space rig::Rig with subject = subject' CHECK (JSON_VALID(`data`)),
  `updated_at` DATETIME NOT NULL COMMENT 'UTC',
  PRIMARY KEY (`user_id`, `subject`),
  CONSTRAINT `member_rigs_user_fk` FOREIGN KEY (`user_id`) REFERENCES `users` (`id`) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_general_ci;
