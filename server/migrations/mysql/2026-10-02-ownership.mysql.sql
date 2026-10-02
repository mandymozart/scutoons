-- Ownership claims, claim attachments and the transfer history (scutoons agents/ITEMS.md "Ownership, claims,
-- gifting"). MariaDB, idempotent, three new tables, nothing else changes. Run after 2026-09-30-code-owner-guard (the
-- transfer moves a code through claimed_by NULL, which that guard allows).
-- ownership_claims: "This is mine" – a reader says a code registered to another account is theirs. One open claim per
--   claimant + code (checked by the API). owner_id = the account the code belonged to when the claim was filed (NULL
--   for a code registered by the old email-only form). An admin decides: transferred | rejected; the claimant may
--   withdraw an open claim.
-- ownership_claim_files: optional evidence (receipt, photo, PDF), stored under FILES_PATH/claims/ (never served
--   directly, admin-only download).
-- item_transfers: every ownership change – a code moved by a claim or an admin, an item gifted by a reader. Append-only,
--   no foreign keys on purpose (the record stays when an account is deleted). seen_at = the recipient saw the gift.
CREATE TABLE IF NOT EXISTS `ownership_claims` (
  `id` CHAR(36) NOT NULL,
  `code` VARCHAR(32) NOT NULL,
  `claimant_id` CHAR(36) NOT NULL,
  `owner_id` CHAR(36) NULL DEFAULT NULL COMMENT 'owner when the claim was filed',
  `statement` TEXT NOT NULL,
  `status` VARCHAR(16) NOT NULL DEFAULT 'open' COMMENT 'open | transferred | rejected | withdrawn',
  `admin_id` CHAR(36) NULL DEFAULT NULL COMMENT 'who decided',
  `admin_note` TEXT NULL DEFAULT NULL,
  `created_at` DATETIME NOT NULL COMMENT 'UTC',
  `decided_at` DATETIME NULL DEFAULT NULL COMMENT 'UTC: transferred / rejected / withdrawn',
  PRIMARY KEY (`id`),
  KEY `ownership_claims_code` (`code`, `status`),
  KEY `ownership_claims_claimant` (`claimant_id`, `created_at`),
  KEY `ownership_claims_status` (`status`, `created_at`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_general_ci;

CREATE TABLE IF NOT EXISTS `ownership_claim_files` (
  `id` CHAR(36) NOT NULL,
  `claim_id` CHAR(36) NOT NULL,
  `filename` VARCHAR(200) NOT NULL COMMENT 'stored name under FILES_PATH/claims/',
  `name` VARCHAR(200) NOT NULL COMMENT 'the uploaded file name',
  `content_type` VARCHAR(64) NOT NULL,
  `size` INT UNSIGNED NOT NULL,
  `created_at` DATETIME NOT NULL COMMENT 'UTC',
  PRIMARY KEY (`id`),
  KEY `ownership_claim_files_claim` (`claim_id`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_general_ci;

CREATE TABLE IF NOT EXISTS `item_transfers` (
  `id` BIGINT UNSIGNED NOT NULL AUTO_INCREMENT,
  `kind` VARCHAR(16) NOT NULL COMMENT 'claim | gift | admin',
  `code` VARCHAR(32) NULL DEFAULT NULL COMMENT 'the code that moved (NULL = a virtual item gift)',
  `item_id` VARCHAR(64) NULL DEFAULT NULL,
  `variant` VARCHAR(64) NULL DEFAULT NULL,
  `qty` INT NOT NULL DEFAULT 1,
  `from_user_id` CHAR(36) NULL DEFAULT NULL,
  `to_user_id` CHAR(36) NOT NULL,
  `by_user_id` CHAR(36) NULL DEFAULT NULL COMMENT 'who acted: the admin (claim, admin) or the giver (gift)',
  `claim_id` CHAR(36) NULL DEFAULT NULL,
  `note` TEXT NULL DEFAULT NULL,
  `created_at` DATETIME NOT NULL COMMENT 'UTC',
  `seen_at` DATETIME NULL DEFAULT NULL COMMENT 'UTC: the recipient saw it (gift notice)',
  PRIMARY KEY (`id`),
  KEY `item_transfers_code` (`code`, `created_at`),
  KEY `item_transfers_to` (`to_user_id`, `created_at`),
  KEY `item_transfers_from` (`from_user_id`, `kind`, `created_at`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_general_ci;
