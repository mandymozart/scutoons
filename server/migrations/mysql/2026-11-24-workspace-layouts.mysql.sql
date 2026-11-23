-- Saved workspace layouts (#827; owner 2026-11-23 "yes in database … so reset to the last saved state of that workspace").
-- `member_layouts`: a member's saved state of a creator workspace – the layout JSON the creator's floating panels keep
-- ({ places, tabs, stage }; the server checks only that shape and its size, the creator reads the inside). A row with a
-- `name` is an own workspace made from another one (`kind`: the workspace id it was made from).
-- `craft_layouts`: a craft's default layout per workspace (configuration, edited in the DB; empty for now) – what a member
-- of that craft starts from before they save. Needs nothing; reads `crafts` / `member_crafts` (2026-11-24-crafts) when there.
-- What it touches: two new tables (empty). Dry run: SHOW TABLES LIKE '%_layouts'; → craft_layouts, member_layouts.
-- MariaDB 10.11, idempotent.
CREATE TABLE IF NOT EXISTS `member_layouts` (
  `user_id` CHAR(36) NOT NULL,
  `workspace` VARCHAR(48) NOT NULL COMMENT 'the creator workspace id, or an own workspace id',
  `kind` VARCHAR(48) NULL DEFAULT NULL COMMENT 'an own workspace: the workspace id it is made from',
  `name` VARCHAR(80) NULL DEFAULT NULL COMMENT 'an own workspace: its name; NULL for the creator workspaces',
  `layout` LONGTEXT NOT NULL COMMENT '{ places, tabs, stage }' CHECK (JSON_VALID(`layout`)),
  `saved_at` DATETIME NOT NULL COMMENT 'UTC',
  PRIMARY KEY (`user_id`, `workspace`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_general_ci;

CREATE TABLE IF NOT EXISTS `craft_layouts` (
  `craft_id` VARCHAR(32) NOT NULL,
  `workspace` VARCHAR(48) NOT NULL,
  `layout` LONGTEXT NOT NULL COMMENT '{ places, tabs, stage }' CHECK (JSON_VALID(`layout`)),
  `updated_at` DATETIME NOT NULL COMMENT 'UTC',
  PRIMARY KEY (`craft_id`, `workspace`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_general_ci;
