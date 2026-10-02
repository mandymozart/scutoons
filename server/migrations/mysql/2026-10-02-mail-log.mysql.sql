-- Sent-mail log of the email module (Domain\Mail\MailLog, agents/PLATFORM.md "Emails"; admin app → Emails, admin API
-- GET /api/admin/emails/log): one row per mail the API tried to send. Never the subject, text or variables (a login
-- mail carries a one-time code); the recipient address is kept MAIL_LOG_DAYS days (default 180, pruned on write).
-- Without this table the API still sends, it just logs nothing. MariaDB, idempotent; needs nothing else.
CREATE TABLE IF NOT EXISTS `mail_log` (
  `id` BIGINT UNSIGNED NOT NULL AUTO_INCREMENT,
  `template` VARCHAR(64) NOT NULL COMMENT 'MailTemplates id, e.g. login_code',
  `recipient` VARCHAR(254) NOT NULL,
  `locale` VARCHAR(8) NOT NULL,
  `status` VARCHAR(16) NOT NULL COMMENT 'sent | failed',
  `transport` VARCHAR(16) NOT NULL COMMENT 'smtp | mail | log',
  `error` VARCHAR(500) NULL DEFAULT NULL,
  `created_at` DATETIME NOT NULL COMMENT 'UTC',
  PRIMARY KEY (`id`),
  KEY `mail_log_created` (`created_at`),
  KEY `mail_log_template` (`template`, `created_at`),
  KEY `mail_log_recipient` (`recipient`, `created_at`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_general_ci;
