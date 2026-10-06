-- The reader's name (agents/EXPRESSIONS.md §6, step 3): what an askText with scope "reader" asked, shown in texts as
-- `{reader.name}` – on every toon, every device of the account. users.display_name: plain text, trimmed, at most 32
-- characters (NULL = none yet); PUT /api/me/ { display_name } sets it (null / "" clears it), the account page shows and
-- edits it. Guests keep it on the device (localStorage scutoons_reader_name). Never shown to other readers.
-- MariaDB, idempotent (the users table is utf8mb4). phpMyAdmin: select the database (d0487143), SQL tab, run the file.
ALTER TABLE `users` ADD COLUMN IF NOT EXISTS `display_name` VARCHAR(32) NULL DEFAULT NULL;
