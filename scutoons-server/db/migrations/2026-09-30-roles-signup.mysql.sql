-- Roles, signup rule, marketing opt-in (scutoons agents/ITEMS.md "Roles, signup rule, promo codes"). MariaDB, idempotent.
-- roles: seeded member / admin / creator (fixed ids). user_roles: which account has which role.
-- Every account gets `member` (new accounts at signup; existing ones by the backfill below).
-- users.signup_code = the code used to create the account (membership or promo; permissions.json "signup": true).
-- users.marketing_opt_in (+ _at) = "Send me news about new releases" (unchecked by default, toggle on the account page).
-- login_requests.marketing_opt_in = the checkbox value sent with the login request (used when the account is created).
CREATE TABLE IF NOT EXISTS `roles` (
  `id` INT NOT NULL,
  `name` VARCHAR(32) NOT NULL,
  `description` VARCHAR(255) NOT NULL DEFAULT '',
  PRIMARY KEY (`id`),
  UNIQUE KEY `roles_name` (`name`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_general_ci;

INSERT IGNORE INTO `roles` (`id`, `name`, `description`) VALUES
  (1, 'member', 'Every account'),
  (2, 'admin', 'Reserved (later: admin UI in the viewer)'),
  (3, 'creator', 'May use scutoons.com/creator and the content API');

CREATE TABLE IF NOT EXISTS `user_roles` (
  `user_id` CHAR(36) NOT NULL,
  `role_id` INT NOT NULL,
  `granted_at` DATETIME NOT NULL COMMENT 'UTC',
  `granted_by` VARCHAR(64) NULL DEFAULT NULL COMMENT 'signup | migration | admin | cli',
  PRIMARY KEY (`user_id`, `role_id`),
  KEY `user_roles_role` (`role_id`),
  CONSTRAINT `user_roles_user` FOREIGN KEY (`user_id`) REFERENCES `users` (`id`) ON DELETE CASCADE,
  CONSTRAINT `user_roles_role_fk` FOREIGN KEY (`role_id`) REFERENCES `roles` (`id`) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_general_ci;

ALTER TABLE `users` ADD COLUMN IF NOT EXISTS `marketing_opt_in` TINYINT(1) NOT NULL DEFAULT 0;
ALTER TABLE `users` ADD COLUMN IF NOT EXISTS `marketing_opt_in_at` DATETIME NULL DEFAULT NULL COMMENT 'UTC, last change';
ALTER TABLE `users` ADD COLUMN IF NOT EXISTS `signup_code` VARCHAR(32) NULL DEFAULT NULL COMMENT 'code used to create the account';
ALTER TABLE `login_requests` ADD COLUMN IF NOT EXISTS `marketing_opt_in` TINYINT(1) NOT NULL DEFAULT 0;

-- Backfill: every existing account is a member
INSERT IGNORE INTO `user_roles` (`user_id`, `role_id`, `granted_at`, `granted_by`)
  SELECT `id`, 1, UTC_TIMESTAMP(), 'migration' FROM `users`;
