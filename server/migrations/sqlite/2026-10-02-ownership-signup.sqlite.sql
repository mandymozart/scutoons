-- "This is mine" without an account (same as the .mysql.sql of this date; agents/ITEMS.md "Ownership, claims, gifting").
ALTER TABLE login_requests ADD COLUMN dispute INTEGER NOT NULL DEFAULT 0;
