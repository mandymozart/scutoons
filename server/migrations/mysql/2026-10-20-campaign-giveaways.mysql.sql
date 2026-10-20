-- Giveaway campaigns (#66, agents/research/CAMPAIGN-GIFTS.md; owner 2026-10-20): a campaign of kind `giveaway`
-- ("Verschenken") gives its item to the recipients of a newsletter – they claim it ("Abholen") while it runs.
--   campaigns.kind            gets the value `giveaway` (VARCHAR already; nothing to change in existing rows)
--   campaign_claims           one row per recipient who claimed (UNIQUE campaign + email = once per address):
--                             virtual → `claimed` (in the inventory); physical → `address` (waits for the account's
--                             shipping address) → `ship` (an admin sends it) → `shipped`; `lapsed` = no address until
--                             30 days after the campaign's end
--   campaign_codes            the personal promo codes a giveaway made for recipients who gave nothing but their email
--                             (one per campaign + address; the code ends with the campaign)
--   mail_campaigns.gift_campaign_id   the newsletter's gift: a giveaway campaign (NULL = none)
--   users.ship_*              the account's shipping address (account page → Stammdaten)
-- Nothing to convert: the new columns are NULL, the tables start empty. Idempotent: IF NOT EXISTS everywhere.
CREATE TABLE IF NOT EXISTS `campaign_claims` (
  `id` BIGINT UNSIGNED NOT NULL AUTO_INCREMENT,
  `campaign_id` CHAR(36) NOT NULL,
  `email` VARCHAR(254) NOT NULL COMMENT 'the address the gift was sent to (lower-case)',
  `user_id` CHAR(36) NOT NULL,
  `item_id` VARCHAR(64) NOT NULL,
  `variant` VARCHAR(64) NULL DEFAULT NULL,
  `status` VARCHAR(16) NOT NULL COMMENT 'claimed | address | ship | shipped | lapsed',
  `ship_name` VARCHAR(200) NULL DEFAULT NULL,
  `ship_address` VARCHAR(1000) NULL DEFAULT NULL,
  `ship_country` CHAR(2) NULL DEFAULT NULL,
  `created_at` DATETIME NOT NULL COMMENT 'UTC: claimed',
  `shipped_at` DATETIME NULL DEFAULT NULL COMMENT 'UTC',
  PRIMARY KEY (`id`),
  UNIQUE KEY `campaign_claims_once` (`campaign_id`, `email`),
  KEY `campaign_claims_user` (`user_id`, `status`),
  KEY `campaign_claims_status` (`campaign_id`, `status`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_general_ci;

CREATE TABLE IF NOT EXISTS `campaign_codes` (
  `campaign_id` CHAR(36) NOT NULL,
  `email` VARCHAR(254) NOT NULL COMMENT 'lower-case',
  `code` VARCHAR(8) CHARACTER SET utf8mb4 COLLATE utf8mb4_bin NOT NULL COMMENT 'qr.code, type promo',
  `created_at` DATETIME NOT NULL COMMENT 'UTC',
  PRIMARY KEY (`campaign_id`, `email`),
  KEY `campaign_codes_code` (`code`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_general_ci;

ALTER TABLE `mail_campaigns` ADD COLUMN IF NOT EXISTS `gift_campaign_id` CHAR(36) NULL DEFAULT NULL COMMENT 'the gift: a giveaway campaign (campaigns.id); NULL = none';

ALTER TABLE `users` ADD COLUMN IF NOT EXISTS `ship_name` VARCHAR(200) NULL DEFAULT NULL COMMENT 'shipping address: the name on the parcel';
ALTER TABLE `users` ADD COLUMN IF NOT EXISTS `ship_address` VARCHAR(1000) NULL DEFAULT NULL COMMENT 'shipping address: street, postcode, city (lines)';
ALTER TABLE `users` ADD COLUMN IF NOT EXISTS `ship_country` CHAR(2) NULL DEFAULT NULL COMMENT 'shipping address: ISO country code';
