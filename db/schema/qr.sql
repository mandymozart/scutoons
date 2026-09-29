-- phpMyAdmin SQL Dump
-- version 5.2.1
-- https://www.phpmyadmin.net/
--
-- Host: localhost
-- Generation Time: Sep 23, 2025 at 07:21 PM
-- Server version: 10.11.13-MariaDB-0ubuntu0.24.04.1-log
-- PHP Version: 7.4.33-nmm7

SET SQL_MODE = "NO_AUTO_VALUE_ON_ZERO";
START TRANSACTION;
SET time_zone = "+00:00";


/*!40101 SET @OLD_CHARACTER_SET_CLIENT=@@CHARACTER_SET_CLIENT */;
/*!40101 SET @OLD_CHARACTER_SET_RESULTS=@@CHARACTER_SET_RESULTS */;
/*!40101 SET @OLD_COLLATION_CONNECTION=@@COLLATION_CONNECTION */;
/*!40101 SET NAMES utf8mb4 */;

--
-- Database: `d044d724`
--

-- --------------------------------------------------------

--
-- Table structure for table `qr`
--

CREATE TABLE `qr` (
  `id` uuid NOT NULL DEFAULT uuid(),
  `code` varchar(8) NOT NULL COMMENT 'qr code',
  `created_at` timestamp NOT NULL DEFAULT current_timestamp(),
  `expires_at` timestamp NULL DEFAULT NULL,
  `claimed_at` timestamp NOT NULL DEFAULT current_timestamp(),
  `redirect_to` varchar(255) NOT NULL COMMENT 'to any app',
  `type` varchar(255) NOT NULL COMMENT 'membership, event, xr, promo',
  `email` varchar(255) NOT NULL COMMENT 'claimed by',
  `edition_id` varchar(100) DEFAULT NULL COMMENT 'links to editions.json id',
  `signature` longtext CHARACTER SET utf8mb4 COLLATE utf8mb4_bin DEFAULT NULL CHECK (json_valid(`signature`)),
  `template_data` longtext CHARACTER SET utf8mb4 COLLATE utf8mb4_bin DEFAULT NULL CHECK (json_valid(`template_data`)),
  `edition_number` int(11) DEFAULT NULL COMMENT 'n of the edition (batch manifest)',
  `claimed_by` char(36) DEFAULT NULL COMMENT 'users.id of the claimer'
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_general_ci;

-- Accounts, items and inventory: see db/migrations/2026-09-29-accounts-inventory.mysql.sql (tables appended
-- at the end of this file).

--
-- Indexes for dumped tables
--

--
-- Indexes for table `qr`
--
ALTER TABLE `qr`
  ADD PRIMARY KEY (`id`),
  ADD UNIQUE KEY `code` (`code`);
COMMIT;

/*!40101 SET CHARACTER_SET_CLIENT=@OLD_CHARACTER_SET_CLIENT */;
/*!40101 SET CHARACTER_SET_RESULTS=@OLD_CHARACTER_SET_RESULTS */;
/*!40101 SET COLLATION_CONNECTION=@OLD_COLLATION_CONNECTION */;

-- --------------------------------------------------------
-- Accounts, items and inventory (2026-09-29, same as the migration of that date)
-- --------------------------------------------------------



CREATE TABLE IF NOT EXISTS `users` (
  `id` CHAR(36) NOT NULL,
  `email` VARCHAR(254) NULL DEFAULT NULL COMMENT 'NULL = anonymous account (code only)',
  `locale` VARCHAR(8) NOT NULL DEFAULT 'en',
  `created_at` DATETIME NOT NULL,
  `confirmed_at` DATETIME NULL DEFAULT NULL,
  `updated_at` DATETIME NOT NULL,
  PRIMARY KEY (`id`),
  UNIQUE KEY `users_email` (`email`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_general_ci;

-- One per "send me a login email": magic-link token and 6-digit code, stored as hashes only.
-- claim_* = a code (and the chosen item/variant) to claim when the login is confirmed.
CREATE TABLE IF NOT EXISTS `login_requests` (
  `id` CHAR(36) NOT NULL,
  `email` VARCHAR(254) NOT NULL,
  `token_hash` CHAR(64) NOT NULL,
  `code_hash` CHAR(64) NOT NULL,
  `locale` VARCHAR(8) NOT NULL DEFAULT 'en',
  `claim_code` VARCHAR(8) NULL DEFAULT NULL,
  `claim_item_id` VARCHAR(64) NULL DEFAULT NULL,
  `claim_variant` VARCHAR(64) NULL DEFAULT NULL,
  `ip_hash` CHAR(64) NOT NULL,
  `attempts` INT NOT NULL DEFAULT 0,
  `created_at` DATETIME NOT NULL,
  `expires_at` DATETIME NOT NULL,
  `used_at` DATETIME NULL DEFAULT NULL,
  `anon_user_id` CHAR(36) NULL DEFAULT NULL,
  PRIMARY KEY (`id`),
  UNIQUE KEY `login_requests_token` (`token_hash`),
  KEY `login_requests_email` (`email`, `created_at`),
  KEY `login_requests_ip` (`ip_hash`, `created_at`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_general_ci;

-- A signed-in device: the client keeps the token (Bearer), the server only its hash
CREATE TABLE IF NOT EXISTS `sessions` (
  `id` CHAR(36) NOT NULL,
  `user_id` CHAR(36) NOT NULL,
  `token_hash` CHAR(64) NOT NULL,
  `created_at` DATETIME NOT NULL,
  `last_used_at` DATETIME NOT NULL,
  PRIMARY KEY (`id`),
  UNIQUE KEY `sessions_token` (`token_hash`),
  KEY `sessions_user` (`user_id`),
  CONSTRAINT `sessions_user_fk` FOREIGN KEY (`user_id`) REFERENCES `users` (`id`) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_general_ci;

-- What a code contains. No rows + qr.edition_id set = the edition's catalog items are options (picked at claim time)
CREATE TABLE IF NOT EXISTS `code_items` (
  `id` INT NOT NULL AUTO_INCREMENT,
  `code` VARCHAR(8) NOT NULL,
  `item_id` VARCHAR(64) NOT NULL,
  `variant` VARCHAR(64) NULL DEFAULT NULL,
  `qty` INT NOT NULL DEFAULT 1,
  PRIMARY KEY (`id`),
  KEY `code_items_code` (`code`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_general_ci;

-- Inventory entries (ITEMS.md `Entry`). Stackable items: one row per (user, item, variant) with qty;
-- others: one row per piece. source: qr | story | scan | craft | starter | import
CREATE TABLE IF NOT EXISTS `inventory` (
  `id` CHAR(36) NOT NULL,
  `user_id` CHAR(36) NOT NULL,
  `item_id` VARCHAR(64) NOT NULL,
  `variant` VARCHAR(64) NULL DEFAULT NULL,
  `qty` INT NOT NULL DEFAULT 1,
  `source` VARCHAR(16) NOT NULL,
  `code` VARCHAR(8) NULL DEFAULT NULL,
  `edition_number` INT NULL DEFAULT NULL,
  `acquired_at` DATETIME NOT NULL,
  `updated_at` DATETIME NOT NULL,
  PRIMARY KEY (`id`),
  KEY `inventory_user` (`user_id`, `item_id`),
  KEY `inventory_code` (`code`),
  CONSTRAINT `inventory_user_fk` FOREIGN KEY (`user_id`) REFERENCES `users` (`id`) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_general_ci;

-- The avatar: one inventory entry per body-part slot
CREATE TABLE IF NOT EXISTS `avatar_slots` (
  `user_id` CHAR(36) NOT NULL,
  `slot` VARCHAR(32) NOT NULL,
  `entry_id` CHAR(36) NOT NULL,
  `updated_at` DATETIME NOT NULL,
  PRIMARY KEY (`user_id`, `slot`),
  CONSTRAINT `avatar_slots_user_fk` FOREIGN KEY (`user_id`) REFERENCES `users` (`id`) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_general_ci;

-- Viewer game state per toon (JSON), last write wins by updated_at (ms)
CREATE TABLE IF NOT EXISTS `progress` (
  `user_id` CHAR(36) NOT NULL,
  `toon_id` VARCHAR(64) NOT NULL,
  `state` MEDIUMTEXT NOT NULL,
  `updated_at` BIGINT NOT NULL COMMENT 'milliseconds since 1970 (UTC), sent by the client',
  PRIMARY KEY (`user_id`, `toon_id`),
  CONSTRAINT `progress_user_fk` FOREIGN KEY (`user_id`) REFERENCES `users` (`id`) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_general_ci;

-- Rate-limit events (grants, imports): count per bucket in a time window; ref de-duplicates (e.g. a story grant)
CREATE TABLE IF NOT EXISTS `rate_limits` (
  `id` CHAR(36) NOT NULL,
  `bucket` VARCHAR(191) NOT NULL,
  `ref` VARCHAR(191) NULL DEFAULT NULL,
  `created_at` DATETIME NOT NULL,
  PRIMARY KEY (`id`),
  KEY `rate_limits_bucket` (`bucket`, `created_at`),
  KEY `rate_limits_ref` (`bucket`, `ref`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_general_ci;

-- One owner per QR code (db/migrations/2026-09-30-code-owner-guard.mysql.sql); phpMyAdmin: use delimiter $$
CREATE TRIGGER IF NOT EXISTS `qr_owner_immutable` BEFORE UPDATE ON `qr` FOR EACH ROW BEGIN IF OLD.claimed_by IS NOT NULL AND NEW.claimed_by IS NOT NULL AND NEW.claimed_by <> OLD.claimed_by THEN SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'qr code already belongs to another account'; END IF; END;
CREATE TRIGGER IF NOT EXISTS `inventory_code_owner_insert` BEFORE INSERT ON `inventory` FOR EACH ROW BEGIN IF NEW.code IS NOT NULL AND NOT (NEW.user_id <=> (SELECT claimed_by FROM qr WHERE code = NEW.code)) THEN SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'inventory code does not belong to this account'; END IF; END;
CREATE TRIGGER IF NOT EXISTS `inventory_code_owner_update` BEFORE UPDATE ON `inventory` FOR EACH ROW BEGIN IF NEW.code IS NOT NULL AND NOT (NEW.user_id <=> (SELECT claimed_by FROM qr WHERE code = NEW.code)) THEN SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'inventory code does not belong to this account'; END IF; END;
