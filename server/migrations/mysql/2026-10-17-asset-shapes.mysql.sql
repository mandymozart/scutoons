-- The vector shape library (agents/research/VECTORS.md, owner 2026-10-17): a shape is an ordinary asset (an SVG in
-- `<toon>/vectors/`, its tags carry the role words) plus one row here with the few facts only shapes have – how it
-- repeats (`tile`), whether it can be tinted as a CSS mask (`mask`, measured), its colours (measured), where it came
-- from (`source`, provenance only – no Figma sync) and the shape it was turned from (`derived`). The list API's
-- `kind=shape` = the assets with a row here. Colour is not stored as a property to edit: a colouration is a preset
-- applied in a look / the creator. MariaDB, idempotent.
CREATE TABLE IF NOT EXISTS `asset_shapes` (
  `asset_id` CHAR(36) NOT NULL,
  `tile` CHAR(1) NULL DEFAULT NULL COMMENT 'x | y – repeats along that axis (a band); NULL = a single shape',
  `mask` TINYINT(1) NOT NULL DEFAULT 0 COMMENT '1 = one colour: usable as a CSS mask, the page gives the colour',
  `colors` TEXT NOT NULL COMMENT 'JSON array of #RRGGBB the file uses (measured)',
  `source` TEXT NULL DEFAULT NULL COMMENT 'JSON { file, page, node, frame, copies?, part_of? } – provenance (Figma)',
  `derived` TEXT NULL DEFAULT NULL COMMENT 'JSON { from: <slug>, rotate: 90 } – a turned copy of another shape',
  `created_at` DATETIME NOT NULL COMMENT 'UTC',
  `updated_at` DATETIME NOT NULL COMMENT 'UTC',
  PRIMARY KEY (`asset_id`),
  CONSTRAINT `asset_shapes_asset_fk` FOREIGN KEY (`asset_id`) REFERENCES `assets` (`id`) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_general_ci;
