-- Embassy pace + reminders (owner 2026-10-05; docs/API.md "Embassy" → "Pace"): the Embassy
-- work is done by part-time liaisons at their own pace. Each person sets how many contacts a week they take on
-- (`per_week`, preset 10, 1–100) and a reminder mail to themselves (`weekly` = Monday, `daily` = Mon–Fri, `off`) in the
-- onboarding, and is asked every 4 weeks whether the pace is right (less / fine / more = −5 / 0 / +5).
-- 1. `embassy_pace`: one row per person (none = not onboarded yet – the app shows the onboarding).
-- 2. `embassy_checkins`: every answer to the 4-weekly question, with the pace before and after (ambassadors see them).
-- 3. an index for "contacts this person made this week" (embassy_activities by user + time).
-- Until it ran: GET /embassy/me answers `pace: null`, the pace, portion, team pace endpoints answer 503 not_available,
-- the cron URL only runs the inbox sync; nothing else changes. Reminders only ever go to the team member – never to
-- a contact.
--
-- phpMyAdmin (production): select the database first (d0487143), SQL tab, run this file. Additive, MariaDB, idempotent
-- (IF NOT EXISTS); needs 2026-10-01-embassy.
CREATE TABLE IF NOT EXISTS `embassy_pace` (
  `user_id` CHAR(36) NOT NULL COMMENT 'users.id: the team member',
  `per_week` INT NOT NULL COMMENT 'contacts per week they take on (1–100)',
  `reminder` VARCHAR(8) NOT NULL DEFAULT 'weekly' COMMENT 'weekly (Monday) | daily (Mon–Fri) | off',
  `onboarded_at` DATETIME NOT NULL COMMENT 'UTC: the onboarding',
  `asked_at` DATETIME NULL DEFAULT NULL COMMENT 'UTC: the last check-in answered (or the onboarding)',
  `reminded_at` DATETIME NULL DEFAULT NULL COMMENT 'UTC: the last reminder mail',
  `updated_at` DATETIME NOT NULL COMMENT 'UTC',
  PRIMARY KEY (`user_id`),
  CONSTRAINT `embassy_pace_user_fk` FOREIGN KEY (`user_id`) REFERENCES `users` (`id`) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_general_ci;

CREATE TABLE IF NOT EXISTS `embassy_checkins` (
  `id` BIGINT NOT NULL AUTO_INCREMENT,
  `user_id` CHAR(36) NOT NULL COMMENT 'users.id: the team member',
  `at` DATETIME NOT NULL COMMENT 'UTC',
  `answer` VARCHAR(8) NOT NULL COMMENT 'less | fine | more',
  `per_week_before` INT NOT NULL,
  `per_week_after` INT NOT NULL,
  PRIMARY KEY (`id`),
  KEY `embassy_checkins_user` (`user_id`, `at`),
  CONSTRAINT `embassy_checkins_user_fk` FOREIGN KEY (`user_id`) REFERENCES `users` (`id`) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_general_ci;

CREATE INDEX IF NOT EXISTS `embassy_activities_by_user` ON `embassy_activities` (`by_user_id`, `at`);
