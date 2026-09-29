-- One owner per QR code, enforced by the database (not only by the API):
-- 1. once qr.claimed_by is set it can't be changed to another account (setting it back to NULL = admin release)
-- 2. an inventory row that references a code must belong to the account that claimed that code
-- MariaDB 10.11. Each trigger is one statement on one line. phpMyAdmin: SQL tab, set "Delimiter" to $$ and
-- replace the trailing ";" of each CREATE TRIGGER line with $$ (or run: php scripts/migrate.php).
CREATE TRIGGER IF NOT EXISTS `qr_owner_immutable` BEFORE UPDATE ON `qr` FOR EACH ROW BEGIN IF OLD.claimed_by IS NOT NULL AND NEW.claimed_by IS NOT NULL AND NEW.claimed_by <> OLD.claimed_by THEN SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'qr code already belongs to another account'; END IF; END;
CREATE TRIGGER IF NOT EXISTS `inventory_code_owner_insert` BEFORE INSERT ON `inventory` FOR EACH ROW BEGIN IF NEW.code IS NOT NULL AND NOT (NEW.user_id <=> (SELECT claimed_by FROM qr WHERE code = NEW.code)) THEN SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'inventory code does not belong to this account'; END IF; END;
CREATE TRIGGER IF NOT EXISTS `inventory_code_owner_update` BEFORE UPDATE ON `inventory` FOR EACH ROW BEGIN IF NEW.code IS NOT NULL AND NOT (NEW.user_id <=> (SELECT claimed_by FROM qr WHERE code = NEW.code)) THEN SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'inventory code does not belong to this account'; END IF; END;
