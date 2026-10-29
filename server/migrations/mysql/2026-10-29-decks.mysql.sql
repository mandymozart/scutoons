-- Cyberdecks: deck identity and access (#180, agents/research/CYBERDECKS.md §9; docs/API.md "Decks"): every deck has
-- its own Ed25519 key pair (made at first boot, the private key never leaves the deck). It asks for access with its
-- public key and shows a short code on its splash; a signed-in member (every member, readers included) approves the code
-- on the account page and the deck becomes theirs: exactly the member's rights, read-only. The approval's lifetime is
-- set at approval like the AI connections (default 182 days, renewable, one reminder mail; or shorter / idle N days).
-- Within it the deck signs a server challenge and gets a ~1 h token (only its SHA-256 stored here). Revoked or expired →
-- the deck asks again (the same row goes back to pending with a new code). One row per public key. MariaDB 10.11,
-- idempotent. Check first (0 rows before the first run):
--   SELECT TABLE_NAME FROM information_schema.TABLES WHERE TABLE_SCHEMA = DATABASE() AND TABLE_NAME = 'decks';
CREATE TABLE IF NOT EXISTS `decks` (
  `id` CHAR(36) NOT NULL,
  `public_key` CHAR(64) CHARACTER SET utf8mb4 COLLATE utf8mb4_bin NOT NULL COMMENT 'the deck''s Ed25519 public key, 32 bytes as lower-case hex',
  `status` VARCHAR(16) NOT NULL DEFAULT 'pending' COMMENT 'pending | approved | revoked (expired = expires_at / idle in the past)',
  `code` VARCHAR(8) CHARACTER SET utf8mb4 COLLATE utf8mb4_bin NULL DEFAULT NULL COMMENT 'pending: the code on the splash (6 characters, shown K7Q-42M); NULL once approved',
  `build` VARCHAR(64) NOT NULL DEFAULT '' COMMENT 'the deck''s image build (what it said)',
  `class` VARCHAR(16) NOT NULL DEFAULT '' COMMENT 'deck class (A, B, gadget …), what it said',
  `hostname` VARCHAR(64) NOT NULL DEFAULT '' COMMENT 'what it said',
  `name` VARCHAR(100) NULL DEFAULT NULL COMMENT 'what the member called it at approval (else the hostname is shown)',
  `user_id` CHAR(36) NULL DEFAULT NULL COMMENT 'users.id: the member it belongs to (approved); NULL while pending',
  `mapping` VARCHAR(16) NOT NULL DEFAULT 'member' COMMENT 'what the deck is mapped to: member (now); role / live later',
  `mapping_ref` VARCHAR(191) NULL DEFAULT NULL COMMENT 'the role or Live session of the mapping (later)',
  `lifetime_days` SMALLINT NULL DEFAULT NULL COMMENT 'the approval''s lifetime, set at approval (1–182; renew moves expires_at this far on)',
  `idle_days` SMALLINT NULL DEFAULT NULL COMMENT 'optional: the approval ends when the deck was not seen this many days',
  `expires_at` DATETIME NOT NULL COMMENT 'UTC: pending = the request ends (minutes); approved = the approval ends',
  `reminded_at` DATETIME NULL DEFAULT NULL COMMENT 'UTC: the expiry reminder of this period went out',
  `challenge` CHAR(64) CHARACTER SET utf8mb4 COLLATE utf8mb4_bin NULL DEFAULT NULL COMMENT 'the open server challenge (hex), single use',
  `challenge_expires_at` DATETIME NULL DEFAULT NULL COMMENT 'UTC',
  `token_hash` CHAR(64) CHARACTER SET utf8mb4 COLLATE utf8mb4_bin NULL DEFAULT NULL COMMENT 'SHA-256 of the deck''s current short token (scd_…)',
  `token_expires_at` DATETIME NULL DEFAULT NULL COMMENT 'UTC (~1 h)',
  `created_at` DATETIME NOT NULL COMMENT 'UTC: the (latest) request',
  `approved_at` DATETIME NULL DEFAULT NULL COMMENT 'UTC',
  `last_seen_at` DATETIME NULL DEFAULT NULL COMMENT 'UTC: the last token or use (at most once a minute)',
  `revoked_at` DATETIME NULL DEFAULT NULL COMMENT 'UTC',
  PRIMARY KEY (`id`),
  UNIQUE KEY `decks_public_key` (`public_key`),
  UNIQUE KEY `decks_code` (`code`),
  UNIQUE KEY `decks_token` (`token_hash`),
  KEY `decks_user` (`user_id`, `status`),
  KEY `decks_expires` (`status`, `expires_at`),
  CONSTRAINT `decks_user_fk` FOREIGN KEY (`user_id`) REFERENCES `users` (`id`) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_general_ci;
