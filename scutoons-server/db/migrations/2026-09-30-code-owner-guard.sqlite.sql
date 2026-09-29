-- One owner per QR code, enforced by the database (not only by the API):
-- 1. once qr.claimed_by is set it can't be changed to another account (setting it back to NULL = admin release)
-- 2. an inventory row that references a code must belong to the account that claimed that code
-- Each trigger stays on one line: scripts/migrate.php splits statements on ";" + newline.
CREATE TRIGGER IF NOT EXISTS qr_owner_immutable BEFORE UPDATE OF claimed_by ON qr WHEN OLD.claimed_by IS NOT NULL AND NEW.claimed_by IS NOT NULL AND NEW.claimed_by <> OLD.claimed_by BEGIN SELECT RAISE(ABORT, 'qr code already belongs to another account'); END;
CREATE TRIGGER IF NOT EXISTS inventory_code_owner_insert BEFORE INSERT ON inventory WHEN NEW.code IS NOT NULL AND NEW.user_id IS NOT (SELECT claimed_by FROM qr WHERE code = NEW.code) BEGIN SELECT RAISE(ABORT, 'inventory code does not belong to this account'); END;
CREATE TRIGGER IF NOT EXISTS inventory_code_owner_update BEFORE UPDATE OF user_id, code ON inventory WHEN NEW.code IS NOT NULL AND NEW.user_id IS NOT (SELECT claimed_by FROM qr WHERE code = NEW.code) BEGIN SELECT RAISE(ABORT, 'inventory code does not belong to this account'); END;
