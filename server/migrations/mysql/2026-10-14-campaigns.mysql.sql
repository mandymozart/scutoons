-- Campaigns: gift invites (agents/research/GIFT-INVITES.md, FEATURES 10.96; owner 2026-10-13/14). MariaDB, idempotent,
-- three new tables, nothing else changes. Literals only (collation #1267).
-- campaigns: one row per campaign. kind = gift_invite (the only kind for now; promo campaigns later). Inside starts_at …
--   ends_at (UTC) and with status = started, the members it names may invite people without an account: the gift waits
--   for them and SCUTOONS mails a personal promo code. inviters = all (every account, also later ones) | chosen
--   (campaign_inviters). gift = own (the member's item) | campaign (item_id / variant from the catalog, given by
--   SCUTOONS, `stock` in all, NULL = no limit). A physical campaign item ships to the friend: payer = scutoons | member
--   (the member pays price_cents by hand – a mail to the admins, an admin marks it paid), countries = where it ships.
--   access = keep (whoever joins keeps reading after the campaign: their invite code gets no end) | until (access_until).
-- campaign_inviters: the chosen accounts (inviters = chosen).
-- campaign_invites: every invite. status payment (on hold until an admin marks it paid) → waiting (mailed, until
--   expires_at) → joined (the friend made an account) | returned (not joined in time: the gift went back) | recalled (the
--   member called it back) | unpaid (not paid in time). invite_code = the promo code mailed (qr.code). A physical gift:
--   the friend's shipping address after joining, shipped_at when an admin sent it.
CREATE TABLE IF NOT EXISTS `campaigns` (
  `id` CHAR(36) NOT NULL,
  `kind` VARCHAR(16) NOT NULL DEFAULT 'gift_invite',
  `name` VARCHAR(200) NOT NULL,
  `status` VARCHAR(16) NOT NULL DEFAULT 'draft' COMMENT 'draft | started | stopped',
  `starts_at` DATETIME NOT NULL COMMENT 'UTC',
  `ends_at` DATETIME NOT NULL COMMENT 'UTC',
  `inviters` VARCHAR(8) NOT NULL DEFAULT 'chosen' COMMENT 'all | chosen',
  `gift` VARCHAR(16) NOT NULL DEFAULT 'own' COMMENT 'own | campaign',
  `item_id` VARCHAR(64) NULL DEFAULT NULL,
  `variant` VARCHAR(64) NULL DEFAULT NULL,
  `stock` INT NULL DEFAULT NULL COMMENT 'campaign items in all, NULL = no limit',
  `per_member` INT NOT NULL DEFAULT 5,
  `per_day` INT NOT NULL DEFAULT 5,
  `wait_days` INT NOT NULL DEFAULT 30,
  `payer` VARCHAR(16) NULL DEFAULT NULL COMMENT 'physical campaign item: scutoons | member',
  `price_cents` INT NULL DEFAULT NULL COMMENT 'payer member: incl. shipping',
  `currency` CHAR(3) NOT NULL DEFAULT 'EUR',
  `countries` VARCHAR(200) NULL DEFAULT NULL COMMENT 'physical: ISO codes, comma separated',
  `access` VARCHAR(8) NOT NULL DEFAULT 'keep' COMMENT 'keep | until',
  `access_until` DATETIME NULL DEFAULT NULL COMMENT 'UTC, access = until',
  `created_by` CHAR(36) NULL DEFAULT NULL,
  `created_at` DATETIME NOT NULL COMMENT 'UTC',
  `updated_at` DATETIME NOT NULL COMMENT 'UTC',
  PRIMARY KEY (`id`),
  KEY `campaigns_window` (`status`, `starts_at`, `ends_at`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_general_ci;

CREATE TABLE IF NOT EXISTS `campaign_inviters` (
  `campaign_id` CHAR(36) NOT NULL,
  `user_id` CHAR(36) NOT NULL,
  `added_at` DATETIME NOT NULL COMMENT 'UTC',
  PRIMARY KEY (`campaign_id`, `user_id`),
  KEY `campaign_inviters_user` (`user_id`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_general_ci;

CREATE TABLE IF NOT EXISTS `campaign_invites` (
  `id` CHAR(36) NOT NULL,
  `campaign_id` CHAR(36) NOT NULL,
  `inviter_id` CHAR(36) NOT NULL,
  `email` VARCHAR(254) NOT NULL COMMENT 'the friend, lower case',
  `locale` VARCHAR(5) NOT NULL DEFAULT 'de' COMMENT 'the invite mail: the inviter''s language',
  `gift` VARCHAR(16) NOT NULL COMMENT 'own | campaign',
  `item_id` VARCHAR(64) NOT NULL,
  `variant` VARCHAR(64) NULL DEFAULT NULL,
  `qty` INT NOT NULL DEFAULT 1,
  `physical` TINYINT(1) NOT NULL DEFAULT 0,
  `invite_code` VARCHAR(32) NULL DEFAULT NULL COMMENT 'the promo code mailed (qr.code)',
  `status` VARCHAR(16) NOT NULL COMMENT 'payment | waiting | joined | returned | recalled | unpaid',
  `price_cents` INT NULL DEFAULT NULL,
  `currency` CHAR(3) NULL DEFAULT NULL,
  `created_at` DATETIME NOT NULL COMMENT 'UTC',
  `payment_due_at` DATETIME NULL DEFAULT NULL COMMENT 'UTC',
  `paid_at` DATETIME NULL DEFAULT NULL COMMENT 'UTC',
  `paid_by` CHAR(36) NULL DEFAULT NULL COMMENT 'the admin',
  `sent_at` DATETIME NULL DEFAULT NULL COMMENT 'UTC: the invite mail',
  `expires_at` DATETIME NULL DEFAULT NULL COMMENT 'UTC: waiting until',
  `joined_at` DATETIME NULL DEFAULT NULL COMMENT 'UTC',
  `joined_user_id` CHAR(36) NULL DEFAULT NULL,
  `ended_at` DATETIME NULL DEFAULT NULL COMMENT 'UTC: returned / recalled / unpaid',
  `ship_name` VARCHAR(200) NULL DEFAULT NULL,
  `ship_address` TEXT NULL DEFAULT NULL,
  `ship_country` CHAR(2) NULL DEFAULT NULL,
  `ship_given_at` DATETIME NULL DEFAULT NULL COMMENT 'UTC',
  `shipped_at` DATETIME NULL DEFAULT NULL COMMENT 'UTC',
  `shipped_by` CHAR(36) NULL DEFAULT NULL,
  `ship_note` VARCHAR(500) NULL DEFAULT NULL,
  PRIMARY KEY (`id`),
  KEY `campaign_invites_campaign` (`campaign_id`, `status`),
  KEY `campaign_invites_email` (`email`, `status`),
  KEY `campaign_invites_inviter` (`inviter_id`, `created_at`),
  KEY `campaign_invites_code` (`invite_code`),
  KEY `campaign_invites_due` (`status`, `expires_at`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_general_ci;
