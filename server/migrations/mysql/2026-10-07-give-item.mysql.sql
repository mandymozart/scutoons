-- Items are not variables (agents/BACKLOG.md 10.38, owner 2026-10-06): a script gives a catalog item with the VN event
-- `giveItem` / a choice's action `give`; `set` / `add` of a story variable named like a catalog item no longer grants
-- the item (the old "VN item bridge" is gone from the viewer). So every stored `add` of an item-named variable becomes
-- a give – history included (undo / restore must not bring back an add that would grant nothing now):
--   a choice's action  {"add":{"variable":"cucumber","value":1}}                       → {"give":{"item":"cucumber","qty":1}}
--   a setVariable      {"type":"setVariable","params":{"variable":"cucumber","value":1,"op":"add"}}
--                                                                                      → {"type":"giveItem","params":{"item":"cucumber","qty":1}}
-- An item-named variable is a catalog item id with or without a domain (`global.cucumber`, `catpope.cucumber` –
-- @scutoons/content variables.ts); qty 1 … 10 (the most one give hands out). Anything else stays as it is and the
-- creator warns about it ("a catalog item, not a story variable"): a `set`, a condition on such a name, a qty over 10.
-- Analysis 2026-10-07 on the dev DB (= the production seed + today's work): the latest versions have 4 – choice adds in
-- scene:sandbox:1/2/5 (cucumber ×3, testo ×1); older versions also catpope's repair_coupon / broken_record_player (52
-- versions with any `add`); no setVariable on an item name, no condition on one. Stored JSON is compact.
-- Idempotent (a second run matches nothing). Catalog ids are [a-z0-9_] (checked) – safe inside the pattern.
-- phpMyAdmin: select the database (d0487143), SQL tab. Check first (the versions to change):
--   SET @items = (SELECT GROUP_CONCAT(`id` SEPARATOR '|') FROM `catalog_items`);
--   SELECT `entity_key`, COUNT(*) FROM `entity_versions` WHERE `json` REGEXP CONCAT('"add":\\{"variable":"([a-z_][a-z0-9_]*\\.)?(', @items, ')"') GROUP BY `entity_key`;
-- then run the file as it is; the same SELECT answers nothing afterwards.
SET @items = (SELECT GROUP_CONCAT(`id` SEPARATOR '|') FROM `catalog_items`);
SET @choice = CONCAT('\\{"add":\\{"variable":"(?:[a-z_][a-z0-9_]*\\.)?(', @items, ')","value":([1-9]|10)\\}\\}');
SET @event = CONCAT('\\{"type":"setVariable","params":\\{"variable":"(?:[a-z_][a-z0-9_]*\\.)?(', @items, ')","value":([1-9]|10),"op":"add"\\}');
UPDATE `entity_versions` SET `json` = REGEXP_REPLACE(REGEXP_REPLACE(`json`, @choice, '{"give":{"item":"\\1","qty":\\2}}'), @event, '{"type":"giveItem","params":{"item":"\\1","qty":\\2}') WHERE @items IS NOT NULL AND (`json` REGEXP @choice OR `json` REGEXP @event);
UPDATE `content_versions` SET `json` = REGEXP_REPLACE(REGEXP_REPLACE(`json`, @choice, '{"give":{"item":"\\1","qty":\\2}}'), @event, '{"type":"giveItem","params":{"item":"\\1","qty":\\2}') WHERE @items IS NOT NULL AND (`json` REGEXP @choice OR `json` REGEXP @event);
