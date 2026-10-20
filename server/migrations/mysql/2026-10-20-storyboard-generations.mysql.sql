-- The storyboard's generations (#120, agents/research/STUDIO-IN-CREATOR.md §2.2 / §2.5): one row per generated still or
-- video of a shot – the budget ledger (estimate → cost) and the review state (mark, approved, dataset). The file itself
-- lives on a member's machine (`machine` = their label, nothing uploaded) or in the shared work storage on the API host
-- (`machine` = 'work', `work_path` = its name there; STORYBOARD_WORK_PATH, never the readers' CDN). Provider jobs (#121)
-- fill `job` / `status` queued → running → succeeded; a local or uploaded generation is stored finished. Read and
-- written through /api/storyboard/generations (role creator, docs/API.md "Storyboard"). MariaDB 10.11, idempotent.
-- Check first (0 rows before the first run):
--   SELECT TABLE_NAME FROM information_schema.TABLES WHERE TABLE_SCHEMA = DATABASE() AND TABLE_NAME = 'storyboard_generations';
CREATE TABLE IF NOT EXISTS `storyboard_generations` (
  `id` BIGINT NOT NULL AUTO_INCREMENT,
  `toon` VARCHAR(64) NOT NULL,
  `chapter` INT NOT NULL,
  `shot` VARCHAR(64) NOT NULL COMMENT 'the shot id of storyboard:<toon>:<chapter>, e.g. sb-35-0',
  `kind` VARCHAR(8) NOT NULL COMMENT 'image | video',
  `take` VARCHAR(64) NULL DEFAULT NULL COMMENT 'the take (pose sample) it was made from',
  `parents` LONGTEXT NULL DEFAULT NULL COMMENT 'JSON list of generation ids it was made from' CHECK (`parents` IS NULL OR JSON_VALID(`parents`)),
  `inputs` LONGTEXT NULL DEFAULT NULL COMMENT 'JSON list of the references with their roles' CHECK (`inputs` IS NULL OR JSON_VALID(`inputs`)),
  `generator` VARCHAR(64) NOT NULL COMMENT 'luma, runway, gemini, comfy-local, upload …',
  `model` VARCHAR(128) NULL DEFAULT NULL,
  `mode` VARCHAR(64) NULL DEFAULT NULL,
  `job` VARCHAR(191) NULL DEFAULT NULL COMMENT 'the provider''s task id (server-side jobs, #121)',
  `status` VARCHAR(16) NOT NULL DEFAULT 'succeeded' COMMENT 'queued | running | succeeded | failed | cancelled',
  `prompt` TEXT NULL DEFAULT NULL,
  `seed` BIGINT NULL DEFAULT NULL,
  `ratio` VARCHAR(16) NULL DEFAULT NULL COMMENT 'e.g. 9:16',
  `duration` DECIMAL(8,3) NULL DEFAULT NULL COMMENT 'seconds (video)',
  `ms` INT NULL DEFAULT NULL COMMENT 'how long the generation took',
  `estimate` DECIMAL(10,4) NULL DEFAULT NULL COMMENT 'reserved before the job (the budget gate, #121)',
  `cost` DECIMAL(10,4) NULL DEFAULT NULL COMMENT 'booked after the job',
  `currency` CHAR(3) NULL DEFAULT NULL COMMENT 'of estimate / cost, e.g. USD',
  `file` VARCHAR(191) NULL DEFAULT NULL COMMENT 'the file name in the member''s workspace folder',
  `file_hash` VARCHAR(128) NULL DEFAULT NULL COMMENT 'sha-256 (hex) of the file',
  `machine` VARCHAR(64) NULL DEFAULT NULL COMMENT 'where the file lives: a member''s machine label, or work = the shared work storage',
  `work_path` VARCHAR(191) NULL DEFAULT NULL COMMENT 'the file''s name in the work storage of its toon / chapter (STORYBOARD_WORK_PATH)',
  `mark` VARCHAR(16) NULL DEFAULT NULL COMMENT 'review | rejected | NULL',
  `marked_at` DATETIME NULL DEFAULT NULL COMMENT 'UTC',
  `approved` TINYINT(1) NOT NULL DEFAULT 0,
  `dataset` TINYINT(1) NOT NULL DEFAULT 0 COMMENT '1 = part of the training dataset (a flag, no copy)',
  `note` VARCHAR(1000) NULL DEFAULT NULL,
  `error` VARCHAR(1000) NULL DEFAULT NULL,
  `params` LONGTEXT NULL DEFAULT NULL COMMENT 'JSON object: the generator''s other settings' CHECK (`params` IS NULL OR JSON_VALID(`params`)),
  `created_by` CHAR(36) NULL DEFAULT NULL COMMENT 'users.id (NULL once the account is deleted)',
  `via_token` CHAR(36) NULL DEFAULT NULL COMMENT 'api_tokens.id: made through this AI connection',
  `created_at` DATETIME NOT NULL COMMENT 'UTC',
  `updated_at` DATETIME NOT NULL COMMENT 'UTC',
  PRIMARY KEY (`id`),
  KEY `storyboard_generations_shot` (`toon`, `chapter`, `shot`, `id`),
  KEY `storyboard_generations_status` (`status`),
  KEY `storyboard_generations_created` (`created_at`),
  CONSTRAINT `storyboard_generations_user_fk` FOREIGN KEY (`created_by`) REFERENCES `users` (`id`) ON DELETE SET NULL
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_general_ci;
