-- The media asset registry (Domain\Assets, agents/ASSETS.md): logical assets, their file versions, the renditions of a
-- version (480p / 720p / 1080p, widths, WebP …), the usage index (where content / the catalog references a path) and
-- the optimization jobs. Content keeps referencing stable CDN paths – the reader never needs these tables; without
-- them only the asset manager (admin Assets page, the creator's asset picker) answers 503. MariaDB, idempotent.
CREATE TABLE IF NOT EXISTS `assets` (
  `id` CHAR(36) NOT NULL,
  `slug` VARCHAR(400) NOT NULL COMMENT 'the logical path without rendition marker + extension, e.g. catpope/videos/s1-trailer-loop',
  `kind` VARCHAR(16) NOT NULL COMMENT 'image | video | audio | archive | document | other',
  `folder` VARCHAR(64) NULL DEFAULT NULL COMMENT 'top-level CDN folder: a toon id, characters, scutoons …',
  `title` VARCHAR(200) NOT NULL DEFAULT '',
  `tags` TEXT NOT NULL COMMENT 'JSON array of strings',
  `status` VARCHAR(16) NOT NULL DEFAULT 'active' COMMENT 'active | deleted',
  `created_at` DATETIME NOT NULL COMMENT 'UTC',
  `updated_at` DATETIME NOT NULL COMMENT 'UTC',
  `created_by` CHAR(36) NULL DEFAULT NULL,
  PRIMARY KEY (`id`),
  UNIQUE KEY `assets_slug` (`slug`(255)),
  KEY `assets_folder` (`folder`, `kind`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_general_ci;

CREATE TABLE IF NOT EXISTS `asset_versions` (
  `id` BIGINT UNSIGNED NOT NULL AUTO_INCREMENT,
  `asset_id` CHAR(36) NOT NULL,
  `number` INT NOT NULL,
  `path` VARCHAR(500) NOT NULL COMMENT 'the source file of this version (CDN-root path)',
  `storage` VARCHAR(16) NOT NULL COMMENT 'local | bunny',
  `size` BIGINT NOT NULL DEFAULT 0,
  `sha256` CHAR(64) NULL DEFAULT NULL,
  `mime` VARCHAR(100) NULL DEFAULT NULL,
  `width` INT NULL DEFAULT NULL,
  `height` INT NULL DEFAULT NULL,
  `duration` DOUBLE NULL DEFAULT NULL COMMENT 'seconds',
  `status` VARCHAR(16) NOT NULL DEFAULT 'current' COMMENT 'current | archived | purged',
  `archive_path` VARCHAR(500) NULL DEFAULT NULL COMMENT 'where the bytes went when a newer version replaced the file in place',
  `note` VARCHAR(500) NULL DEFAULT NULL,
  `created_at` DATETIME NOT NULL,
  `created_by` CHAR(36) NULL DEFAULT NULL,
  PRIMARY KEY (`id`),
  UNIQUE KEY `asset_versions_number` (`asset_id`, `number`),
  KEY `asset_versions_path` (`path`(255)),
  CONSTRAINT `asset_versions_asset` FOREIGN KEY (`asset_id`) REFERENCES `assets` (`id`) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_general_ci;

CREATE TABLE IF NOT EXISTS `asset_renditions` (
  `id` BIGINT UNSIGNED NOT NULL AUTO_INCREMENT,
  `asset_id` CHAR(36) NOT NULL,
  `version_id` BIGINT UNSIGNED NOT NULL,
  `label` VARCHAR(32) NOT NULL COMMENT 'source | 480p | 720p | 1080p | 1280w | 2x | webp | 128k …',
  `quality` VARCHAR(8) NULL DEFAULT NULL COMMENT 'MediaQuality: low | medium | high',
  `path` VARCHAR(500) NOT NULL,
  `size` BIGINT NOT NULL DEFAULT 0,
  `sha256` CHAR(64) NULL DEFAULT NULL,
  `mime` VARCHAR(100) NULL DEFAULT NULL,
  `width` INT NULL DEFAULT NULL,
  `height` INT NULL DEFAULT NULL,
  `duration` DOUBLE NULL DEFAULT NULL,
  `bitrate` INT NULL DEFAULT NULL,
  `origin` VARCHAR(16) NOT NULL DEFAULT 'imported' COMMENT 'imported | uploaded | generated',
  `modified_at` DATETIME NULL DEFAULT NULL COMMENT 'the storage''s last change',
  `created_at` DATETIME NOT NULL,
  PRIMARY KEY (`id`),
  KEY `asset_renditions_path` (`path`(255)),
  KEY `asset_renditions_version` (`version_id`),
  KEY `asset_renditions_asset` (`asset_id`),
  CONSTRAINT `asset_renditions_asset_fk` FOREIGN KEY (`asset_id`) REFERENCES `assets` (`id`) ON DELETE CASCADE,
  CONSTRAINT `asset_renditions_version_fk` FOREIGN KEY (`version_id`) REFERENCES `asset_versions` (`id`) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_general_ci;

CREATE TABLE IF NOT EXISTS `asset_usages` (
  `id` BIGINT UNSIGNED NOT NULL AUTO_INCREMENT,
  `path` VARCHAR(500) NOT NULL COMMENT 'the CDN-root path the reference resolves to',
  `source` VARCHAR(16) NOT NULL COMMENT 'content | catalog',
  `ref` VARCHAR(255) NOT NULL COMMENT 'entity key | catalog file (items.json …)',
  `field` VARCHAR(500) NOT NULL COMMENT 'JSON pointer inside the document',
  `state` VARCHAR(16) NOT NULL COMMENT 'published | draft | config',
  `version_id` BIGINT UNSIGNED NULL DEFAULT NULL COMMENT 'entity_versions.id',
  `toon` VARCHAR(64) NULL DEFAULT NULL,
  `raw` VARCHAR(500) NOT NULL COMMENT 'the string as written in the document',
  `scanned_at` DATETIME NOT NULL,
  PRIMARY KEY (`id`),
  KEY `asset_usages_path` (`path`(255)),
  KEY `asset_usages_ref` (`source`, `ref`(191))
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_general_ci;

CREATE TABLE IF NOT EXISTS `asset_jobs` (
  `id` BIGINT UNSIGNED NOT NULL AUTO_INCREMENT,
  `asset_id` CHAR(36) NULL DEFAULT NULL,
  `kind` VARCHAR(32) NOT NULL COMMENT 'video-renditions | image-webp | poster',
  `status` VARCHAR(16) NOT NULL DEFAULT 'queued' COMMENT 'queued | running | done | failed',
  `params` TEXT NOT NULL,
  `log` MEDIUMTEXT NULL DEFAULT NULL,
  `created_at` DATETIME NOT NULL,
  `started_at` DATETIME NULL DEFAULT NULL,
  `finished_at` DATETIME NULL DEFAULT NULL,
  `created_by` CHAR(36) NULL DEFAULT NULL,
  PRIMARY KEY (`id`),
  KEY `asset_jobs_status` (`status`, `id`),
  KEY `asset_jobs_asset` (`asset_id`, `id`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_general_ci;
