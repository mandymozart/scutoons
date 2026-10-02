-- "This is mine" without an account (agents/ITEMS.md "Ownership, claims, gifting"): a login request may ask to create an
-- account with a membership code that is already registered to someone else, only to file an ownership claim (nothing
-- is claimed). MariaDB, idempotent (IF NOT EXISTS). Until it ran, such a request answers 503 not_available; every other
-- sign-in works as before.
ALTER TABLE `login_requests` ADD COLUMN IF NOT EXISTS `dispute` TINYINT(1) NOT NULL DEFAULT 0 COMMENT '1 = signup to file an ownership claim';
