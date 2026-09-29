-- Anonymous accounts (ITEMS.md "Access model 2026-09-30"): users.email and users.confirmed_at become nullable
-- (email IS NULL = anonymous account created by POST /auth/code/). The UNIQUE key still holds for non-null emails.
-- login_requests.anon_user_id = the anonymous account that asked to attach this email (merged on verify).
-- Idempotent (MODIFY / ADD COLUMN IF NOT EXISTS) – safe to run again in phpMyAdmin.
ALTER TABLE `users` MODIFY `email` VARCHAR(254) NULL DEFAULT NULL;
ALTER TABLE `users` MODIFY `confirmed_at` DATETIME NULL DEFAULT NULL;
ALTER TABLE `login_requests` ADD COLUMN IF NOT EXISTS `anon_user_id` CHAR(36) NULL DEFAULT NULL;
