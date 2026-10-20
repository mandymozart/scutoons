-- Provider jobs of the storyboard's generations (#121, agents/research/STUDIO-IN-CREATOR.md §2.5; docs/API.md
-- "Storyboard" → "Provider jobs"): the server starts a paid job at Luma / Runway, stores the provider's task id in `job`
-- and asks the provider about it when the job is read (no worker) – `polled_at` = when it last asked (at most once per
-- GENERATION_POLL_SECONDS), `progress` = the provider's 0–1, `result_url` = the provider's (short-lived) link to the
-- result, streamed through GET /api/storyboard/generations/{id}/result (the server stores no file). MariaDB, idempotent.
-- Check first (0 rows before the first run):
--   SELECT COLUMN_NAME FROM information_schema.COLUMNS WHERE TABLE_SCHEMA = DATABASE() AND TABLE_NAME = 'storyboard_generations' AND COLUMN_NAME IN ('progress', 'result_url', 'polled_at');
ALTER TABLE `storyboard_generations` ADD COLUMN IF NOT EXISTS `progress` DECIMAL(5,4) NULL DEFAULT NULL COMMENT 'a running provider job: 0–1, as the provider says' AFTER `status`;
ALTER TABLE `storyboard_generations` ADD COLUMN IF NOT EXISTS `result_url` TEXT NULL DEFAULT NULL COMMENT 'the provider''s link to the result (expires: Luma 1 h, Runway 24–48 h)' AFTER `work_path`;
ALTER TABLE `storyboard_generations` ADD COLUMN IF NOT EXISTS `polled_at` DATETIME NULL DEFAULT NULL COMMENT 'UTC: the provider was last asked about the job' AFTER `result_url`;
