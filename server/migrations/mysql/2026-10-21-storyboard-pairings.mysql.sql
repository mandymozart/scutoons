-- The handheld camera's pairings (#126, agents/research/STUDIO-IN-CREATOR.md §2.6; docs/API.md "Storyboard" →
-- "Handheld pairing"): the creator's Handheld panel asks for a one-time code (shown as a QR code), the phone's barebone
-- page reads the PC's WebRTC offer with it and writes its answer; the poses then run peer to peer over the data channel
-- (nothing per frame through PHP). A row lives at most 10 minutes and is deleted when the PC has read the answer (paired)
-- or when it ran out (every new pairing sweeps the expired ones). The code is the only right the phone has: it reads and
-- writes this one row, nothing else. Codes are case-sensitive (utf8mb4_bin). MariaDB 10.11, idempotent.
-- Check first (0 rows before the first run):
--   SELECT TABLE_NAME FROM information_schema.TABLES WHERE TABLE_SCHEMA = DATABASE() AND TABLE_NAME = 'storyboard_pairings';
CREATE TABLE IF NOT EXISTS `storyboard_pairings` (
  `code` VARCHAR(32) CHARACTER SET utf8mb4 COLLATE utf8mb4_bin NOT NULL COMMENT 'the one-time code (22 characters base64url, 128 bits)',
  `toon` VARCHAR(64) NOT NULL,
  `chapter` INT NOT NULL,
  `shot` VARCHAR(64) NULL DEFAULT NULL COMMENT 'the shot the phone films (shown on the phone)',
  `created_by` CHAR(36) NOT NULL COMMENT 'users.id: the member whose PC asked for the code (only they read the answer)',
  `offer` MEDIUMTEXT NULL DEFAULT NULL COMMENT 'the PC''s WebRTC offer (SDP, ICE candidates gathered)',
  `answer` MEDIUMTEXT NULL DEFAULT NULL COMMENT 'the phone''s WebRTC answer (SDP)',
  `expires_at` DATETIME NOT NULL COMMENT 'UTC',
  `created_at` DATETIME NOT NULL COMMENT 'UTC',
  PRIMARY KEY (`code`),
  KEY `storyboard_pairings_expires` (`expires_at`),
  KEY `storyboard_pairings_user` (`created_by`),
  CONSTRAINT `storyboard_pairings_user_fk` FOREIGN KEY (`created_by`) REFERENCES `users` (`id`) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_general_ci;
