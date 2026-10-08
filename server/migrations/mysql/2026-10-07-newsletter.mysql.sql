-- Newsletters (agents/NEWSLETTER.md; Domain\Newsletter; admin app → Emails → Newsletters / Contacts): mass mails to
-- members and to contacts who are no members, sent in small batches through the API's SMTP account.
--   mail_contacts                 addresses that are no account: the old QR form's (source `qr-claim`, copied below)
--                                 and the event sign-up lists typed in by hand (source e.g. `event:Leipzig 2026-09`).
--                                 locale NULL = not set yet – a newsletter can't start while a recipient has none.
--   mail_suppressions             never mail this address again: unsubscribed (link / one-click), bounced (a permanent
--                                 5xx at RCPT TO), or blocked by hand. Every newsletter checks it.
--   mail_campaigns                one newsletter: subject + body per locale, audiences, status
--                                 draft → sending ⇄ paused → done
--   mail_campaign_recipients      the recipients, snapshot at "Start" (UNIQUE campaign + email: nobody twice);
--                                 status queued → sent | failed | skipped (suppressed after the start)
-- Old QR form (the email-only claim of the old platform – it stated that a claim brings updates about the platform):
-- every address in qr.email whose code is in no account, and that has no account itself, becomes a contact.
-- Analysis 2026-10-07 on the dev DB: the prod seed carries no accounts or codes (0 rows) – production's numbers come from
-- the SELECTs below. Idempotent: CREATE TABLE IF NOT EXISTS; INSERT IGNORE (UNIQUE email).
-- phpMyAdmin: select the database (d0487143), SQL tab. Check first – how many addresses each audience will have:
--   SELECT COUNT(*) AS members, SUM(marketing_opt_in) AS opted_in FROM `users`;
--   SELECT COUNT(DISTINCT LOWER(TRIM(q.`email`))) AS old_qr_form FROM `qr` q LEFT JOIN `users` u ON u.`email` = LOWER(TRIM(q.`email`)) WHERE q.`claimed_by` IS NULL AND TRIM(q.`email`) <> '' AND u.`id` IS NULL;
--   SELECT `locale`, COUNT(*) FROM `users` GROUP BY `locale`;
CREATE TABLE IF NOT EXISTS `mail_contacts` (
  `id` BIGINT UNSIGNED NOT NULL AUTO_INCREMENT,
  `email` VARCHAR(254) NOT NULL COMMENT 'lower-case, trimmed',
  `name` VARCHAR(100) NULL DEFAULT NULL,
  `locale` VARCHAR(8) NULL DEFAULT NULL COMMENT 'de | en; NULL = not set yet',
  `source` VARCHAR(100) NOT NULL COMMENT 'qr-claim | event:<where, when> | …',
  `consented_at` DATE NULL DEFAULT NULL COMMENT 'when the person gave the address (event day, claim day)',
  `created_at` DATETIME NOT NULL COMMENT 'UTC',
  `created_by` VARCHAR(255) NULL DEFAULT NULL COMMENT 'admin:<email> | migration',
  PRIMARY KEY (`id`),
  UNIQUE KEY `mail_contacts_email` (`email`),
  KEY `mail_contacts_source` (`source`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_general_ci;

CREATE TABLE IF NOT EXISTS `mail_suppressions` (
  `email` VARCHAR(254) NOT NULL COMMENT 'lower-case, trimmed',
  `reason` VARCHAR(16) NOT NULL COMMENT 'unsubscribe | bounce | manual',
  `detail` VARCHAR(500) NULL DEFAULT NULL COMMENT 'the SMTP reply of a bounce, the newsletter of an unsubscribe, …',
  `created_at` DATETIME NOT NULL COMMENT 'UTC',
  PRIMARY KEY (`email`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_general_ci;

CREATE TABLE IF NOT EXISTS `mail_campaigns` (
  `id` BIGINT UNSIGNED NOT NULL AUTO_INCREMENT,
  `title` VARCHAR(200) NOT NULL COMMENT 'admin only, never sent',
  `audiences` VARCHAR(1000) NOT NULL DEFAULT '' COMMENT 'JSON list: members_opted_in | members_all | contacts | contacts:<source>',
  `subject_de` VARCHAR(200) NOT NULL DEFAULT '',
  `subject_en` VARCHAR(200) NOT NULL DEFAULT '',
  `body_de` MEDIUMTEXT NOT NULL,
  `body_en` MEDIUMTEXT NOT NULL,
  `status` VARCHAR(16) NOT NULL DEFAULT 'draft' COMMENT 'draft | sending | paused | done',
  `last_error` VARCHAR(500) NULL DEFAULT NULL COMMENT 'why the last batch stopped (SMTP login, connection, hourly limit is no error)',
  `created_at` DATETIME NOT NULL COMMENT 'UTC',
  `created_by` VARCHAR(255) NULL DEFAULT NULL,
  `updated_at` DATETIME NOT NULL COMMENT 'UTC',
  `updated_by` VARCHAR(255) NULL DEFAULT NULL,
  `started_at` DATETIME NULL DEFAULT NULL COMMENT 'UTC',
  `finished_at` DATETIME NULL DEFAULT NULL COMMENT 'UTC',
  PRIMARY KEY (`id`),
  KEY `mail_campaigns_status` (`status`, `started_at`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_general_ci;

CREATE TABLE IF NOT EXISTS `mail_campaign_recipients` (
  `id` BIGINT UNSIGNED NOT NULL AUTO_INCREMENT,
  `campaign_id` BIGINT UNSIGNED NOT NULL,
  `email` VARCHAR(254) NOT NULL,
  `locale` VARCHAR(8) NOT NULL,
  `name` VARCHAR(100) NULL DEFAULT NULL,
  `source` VARCHAR(16) NOT NULL COMMENT 'member | contact',
  `status` VARCHAR(16) NOT NULL DEFAULT 'queued' COMMENT 'queued | sent | failed | skipped',
  `attempts` TINYINT UNSIGNED NOT NULL DEFAULT 0,
  `next_attempt_at` DATETIME NULL DEFAULT NULL COMMENT 'UTC; a temporary failure waits until then',
  `sent_at` DATETIME NULL DEFAULT NULL COMMENT 'UTC',
  `error` VARCHAR(500) NULL DEFAULT NULL,
  PRIMARY KEY (`id`),
  UNIQUE KEY `mail_campaign_recipients_email` (`campaign_id`, `email`),
  KEY `mail_campaign_recipients_queue` (`campaign_id`, `status`, `next_attempt_at`),
  CONSTRAINT `mail_campaign_recipients_campaign` FOREIGN KEY (`campaign_id`) REFERENCES `mail_campaigns` (`id`) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_general_ci;

INSERT IGNORE INTO `mail_contacts` (`email`, `source`, `consented_at`, `created_at`, `created_by`)
  SELECT LOWER(TRIM(q.`email`)), 'qr-claim', DATE(MIN(q.`claimed_at`)), UTC_TIMESTAMP(), 'migration'
  FROM `qr` q LEFT JOIN `users` u ON u.`email` = LOWER(TRIM(q.`email`))
  WHERE q.`claimed_by` IS NULL AND TRIM(q.`email`) <> '' AND u.`id` IS NULL
  GROUP BY LOWER(TRIM(q.`email`));
