-- Rig modules (#281, agents/research/RIG-EDITING.md §7; owner 2026-11-02): an instrument written as Rust, compiled to wasm.
-- catalog_modules: the factory, a catalog kind like catalog_instruments (documents `module:<id>` in the creator – drafts,
--   versions, review, publish). `data` = { id, name?, files: { "lib.rs": "…" }, descriptor?, wasm?: { hash, bytes, toolchain? } }:
--   the sources, and – written by a successful compile – the descriptor (the wasm's `deck.module` section) and the content
--   hash of the .wasm on the CDN (`modules/<hash>.wasm`).
-- member_modules: a member's own module (id `m:<ulid>`, like member_presets), `name` for the list, `data` = { files,
--   descriptor?, wasm? } – the descriptor and the hash only from a compile (the API), never from the member.
-- MariaDB 10.11, idempotent.
CREATE TABLE IF NOT EXISTS `catalog_modules` (
  `id` VARCHAR(100) NOT NULL,
  `position` INT NOT NULL DEFAULT 0,
  `data` LONGTEXT NOT NULL COMMENT 'rig module { id, name?, files, descriptor?, wasm? }' CHECK (JSON_VALID(`data`)),
  `updated_at` DATETIME NOT NULL COMMENT 'UTC',
  `updated_by` VARCHAR(255) NULL DEFAULT NULL,
  PRIMARY KEY (`id`),
  KEY `catalog_modules_position` (`position`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_bin;

CREATE TABLE IF NOT EXISTS `member_modules` (
  `id` VARCHAR(40) CHARACTER SET utf8mb4 COLLATE utf8mb4_bin NOT NULL COMMENT 'm:<ulid>',
  `user_id` CHAR(36) NOT NULL COMMENT 'users.id: whose module',
  `name` VARCHAR(64) NOT NULL COMMENT 'the member''s label for the list',
  `data` LONGTEXT NOT NULL COMMENT '{ files, descriptor?, wasm? }' CHECK (JSON_VALID(`data`)),
  `created_at` DATETIME NOT NULL COMMENT 'UTC',
  `updated_at` DATETIME NOT NULL COMMENT 'UTC',
  PRIMARY KEY (`id`),
  KEY `member_modules_user` (`user_id`),
  CONSTRAINT `member_modules_user_fk` FOREIGN KEY (`user_id`) REFERENCES `users` (`id`) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_general_ci;
