-- The catalog moves from server/config/*.json into the database (agents/CATALOG.md, CUTOVER 10.29): one row per entry,
-- its definition as JSON in the shape of the old files (`data`), its order (`position`) and the columns lookups need.
-- Every change made in the admin is recorded in catalog_history (before / after, by, at – an admin can undo it).
-- The files stay in the repository as the seed of empty / local databases (php bin/console catalog:seed).
--
-- phpMyAdmin (production): select the database first (d0487143), SQL tab, run this whole file at once. Idempotent:
-- CREATE TABLE IF NOT EXISTS, each table is filled only while it is empty (a second run changes nothing, never brings
-- back a deleted entry), the folds run once (they delete their setting). Data = the files of 2026-10-04 (generated:
-- php bin/console catalog:export-sql --write).
-- Order with the deploy: either. Without the tables the API reads the files (read-only, as before the migration).
--
-- Also folded in and retired: the runtime settings `item_respawn` (admin respawn overrides → the items' `respawn`) and
-- `featured_products` (the landing page's list → catalog_featured).
--
-- `data` is LONGTEXT + JSON_VALID, not the JSON type: MySQL's JSON type would reorder object keys (MariaDB keeps
-- them either way); ids are case-sensitive (utf8mb4_bin, like the codes – 2026-10-03-code-case).

CREATE TABLE IF NOT EXISTS `catalog_items` (
  `id` VARCHAR(100) NOT NULL,
  `position` INT NOT NULL DEFAULT 0,
  `toon` VARCHAR(64) NULL DEFAULT NULL,
  `edition_id` VARCHAR(100) NULL DEFAULT NULL COMMENT 'catalog_editions.id (the claim options of an edition''s codes)',
  `data` LONGTEXT NOT NULL COMMENT 'the item definition (agents/ITEMS.md "Item definition")' CHECK (JSON_VALID(`data`)),
  `updated_at` DATETIME NOT NULL COMMENT 'UTC',
  `updated_by` VARCHAR(255) NULL DEFAULT NULL COMMENT 'admin:<email> | seed | cli',
  PRIMARY KEY (`id`),
  KEY `catalog_items_position` (`position`),
  KEY `catalog_items_toon` (`toon`),
  KEY `catalog_items_edition` (`edition_id`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_bin;

CREATE TABLE IF NOT EXISTS `catalog_recipes` (
  `id` VARCHAR(100) NOT NULL,
  `position` INT NOT NULL DEFAULT 0,
  `data` LONGTEXT NOT NULL COMMENT '{ id, inputs: [{ item, qty }], output: { item, qty } }' CHECK (JSON_VALID(`data`)),
  `updated_at` DATETIME NOT NULL COMMENT 'UTC',
  `updated_by` VARCHAR(255) NULL DEFAULT NULL,
  PRIMARY KEY (`id`),
  KEY `catalog_recipes_position` (`position`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_bin;

CREATE TABLE IF NOT EXISTS `catalog_albums` (
  `id` VARCHAR(100) NOT NULL,
  `position` INT NOT NULL DEFAULT 0,
  `toon` VARCHAR(64) NULL DEFAULT NULL,
  `data` LONGTEXT NOT NULL COMMENT '{ id, toon, title, tracks: [{ id, title, src }], download?, cover?, access? }' CHECK (JSON_VALID(`data`)),
  `updated_at` DATETIME NOT NULL COMMENT 'UTC',
  `updated_by` VARCHAR(255) NULL DEFAULT NULL,
  PRIMARY KEY (`id`),
  KEY `catalog_albums_position` (`position`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_bin;

CREATE TABLE IF NOT EXISTS `catalog_products` (
  `id` VARCHAR(100) NOT NULL,
  `position` INT NOT NULL DEFAULT 0,
  `toon` VARCHAR(64) NULL DEFAULT NULL,
  `item_id` VARCHAR(100) NULL DEFAULT NULL COMMENT 'catalog_items.id: the digital item of the product',
  `edition_id` VARCHAR(100) NULL DEFAULT NULL COMMENT 'catalog_editions.id',
  `data` LONGTEXT NOT NULL COMMENT 'the shop product (@scutoons/content ProductCatalog products[])' CHECK (JSON_VALID(`data`)),
  `updated_at` DATETIME NOT NULL COMMENT 'UTC',
  `updated_by` VARCHAR(255) NULL DEFAULT NULL,
  PRIMARY KEY (`id`),
  KEY `catalog_products_position` (`position`),
  KEY `catalog_products_item` (`item_id`),
  KEY `catalog_products_edition` (`edition_id`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_bin;

CREATE TABLE IF NOT EXISTS `catalog_editions` (
  `id` VARCHAR(100) NOT NULL,
  `position` INT NOT NULL DEFAULT 0,
  `toon` VARCHAR(64) NULL DEFAULT NULL,
  `data` LONGTEXT NOT NULL COMMENT 'the limited edition (the physical products QR codes are printed for)' CHECK (JSON_VALID(`data`)),
  `updated_at` DATETIME NOT NULL COMMENT 'UTC',
  `updated_by` VARCHAR(255) NULL DEFAULT NULL,
  PRIMARY KEY (`id`),
  KEY `catalog_editions_position` (`position`),
  KEY `catalog_editions_toon` (`toon`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_bin;

CREATE TABLE IF NOT EXISTS `catalog_featured` (
  `id` VARCHAR(191) NOT NULL COMMENT '<product> or <product>:<variant>',
  `position` INT NOT NULL DEFAULT 0,
  `product_id` VARCHAR(100) NULL DEFAULT NULL COMMENT 'catalog_products.id',
  `variant_id` VARCHAR(100) NULL DEFAULT NULL,
  `data` LONGTEXT NOT NULL COMMENT '{ product, variant? }' CHECK (JSON_VALID(`data`)),
  `updated_at` DATETIME NOT NULL COMMENT 'UTC',
  `updated_by` VARCHAR(255) NULL DEFAULT NULL,
  PRIMARY KEY (`id`),
  KEY `catalog_featured_position` (`position`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_bin;

-- server-only (security): what code types grant – never served to readers
CREATE TABLE IF NOT EXISTS `catalog_permissions` (
  `id` VARCHAR(100) NOT NULL,
  `position` INT NOT NULL DEFAULT 0,
  `data` LONGTEXT NOT NULL COMMENT 'the description (a JSON string)' CHECK (JSON_VALID(`data`)),
  `updated_at` DATETIME NOT NULL COMMENT 'UTC',
  `updated_by` VARCHAR(255) NULL DEFAULT NULL,
  PRIMARY KEY (`id`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_bin;

CREATE TABLE IF NOT EXISTS `catalog_code_types` (
  `id` VARCHAR(100) NOT NULL COMMENT 'qr.type; `default` = types not listed',
  `position` INT NOT NULL DEFAULT 0,
  `data` LONGTEXT NOT NULL COMMENT '{ claimable, signup, grants: [permission ids] }' CHECK (JSON_VALID(`data`)),
  `updated_at` DATETIME NOT NULL COMMENT 'UTC',
  `updated_by` VARCHAR(255) NULL DEFAULT NULL,
  PRIMARY KEY (`id`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_bin;

-- the toons QR codes point to (GET /api/toons/); created by admins, the creator fills the content (`toon:<id>`)
CREATE TABLE IF NOT EXISTS `toons` (
  `id` VARCHAR(64) NOT NULL,
  `position` INT NOT NULL DEFAULT 0,
  `data` LONGTEXT NOT NULL COMMENT '{ id, name, redirectUrl }' CHECK (JSON_VALID(`data`)),
  `updated_at` DATETIME NOT NULL COMMENT 'UTC',
  `updated_by` VARCHAR(255) NULL DEFAULT NULL,
  PRIMARY KEY (`id`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_bin;

-- the files' top-level fields: items { version }, products { version, currency, checked_at, source }, permissions { version }
CREATE TABLE IF NOT EXISTS `catalog_meta` (
  `id` VARCHAR(32) NOT NULL,
  `position` INT NOT NULL DEFAULT 0,
  `data` LONGTEXT NOT NULL CHECK (JSON_VALID(`data`)),
  `updated_at` DATETIME NOT NULL COMMENT 'UTC',
  `updated_by` VARCHAR(255) NULL DEFAULT NULL,
  PRIMARY KEY (`id`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_bin;

CREATE TABLE IF NOT EXISTS `catalog_history` (
  `id` BIGINT UNSIGNED NOT NULL AUTO_INCREMENT,
  `kind` VARCHAR(32) NOT NULL COMMENT 'items | recipes | albums | products | editions | featured | permissions | code-types | toons | meta',
  `entry_id` VARCHAR(191) NOT NULL DEFAULT '' COMMENT '\'\' = the whole kind (reorder / replace)',
  `action` VARCHAR(16) NOT NULL COMMENT 'create | update | delete | reorder | replace',
  `before_json` LONGTEXT NULL DEFAULT NULL COMMENT 'data before (NULL = created); reorder: the ids' CHECK (JSON_VALID(`before_json`)),
  `after_json` LONGTEXT NULL DEFAULT NULL COMMENT 'data after (NULL = deleted)' CHECK (JSON_VALID(`after_json`)),
  `position` INT NULL DEFAULT NULL COMMENT 'the entry''s position before the change',
  `undo_of` BIGINT UNSIGNED NULL DEFAULT NULL COMMENT 'the change this one undid',
  `note` VARCHAR(255) NULL DEFAULT NULL,
  `changed_by` VARCHAR(255) NULL DEFAULT NULL COMMENT 'admin:<email> | cli',
  `changed_at` DATETIME NOT NULL COMMENT 'UTC',
  PRIMARY KEY (`id`),
  KEY `catalog_history_entry` (`kind`, `entry_id`, `id`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_bin;

-- The landing page's list chosen in the admin (setting featured_products) – before the seed, which fills empty tables only
INSERT INTO `catalog_featured` (`id`, `position`, `product_id`, `variant_id`, `data`, `updated_at`, `updated_by`)
SELECT CONCAT(f.`product`, IF(COALESCE(f.`variant`, '') = '', '', CONCAT(':', f.`variant`))), f.`n` - 1, f.`product`, NULLIF(f.`variant`, ''),
  IF(COALESCE(f.`variant`, '') = '', JSON_OBJECT('product', f.`product`), JSON_OBJECT('product', f.`product`, 'variant', f.`variant`)), s.`updated_at`, s.`updated_by`
FROM `settings` s, JSON_TABLE(s.`value`, '$[*]' COLUMNS (`n` FOR ORDINALITY, `product` VARCHAR(100) PATH '$.product', `variant` VARCHAR(100) PATH '$.variant')) f
WHERE s.`key` = 'featured_products' AND NOT EXISTS (SELECT 1 FROM `catalog_featured`);
DELETE FROM `settings` WHERE `key` = 'featured_products';

-- Today's catalog: one INSERT per table, each only while the table is empty (a second run changes nothing)
-- BEGIN catalog seed (generated by php bin/console catalog:export-sql --write – do not edit by hand)
INSERT INTO `catalog_items` (`id`, `position`, `data`, `updated_at`, `updated_by`, `toon`, `edition_id`) SELECT * FROM (
  SELECT 'camera' AS `id`, 0 AS `position`, '{"id":"camera","toon":"catpope","name":{"de":"Kamera","en":"Camera"},"description":{"de":"Deine erste Kamera. Sie kann nur eines: die QR-Codes von SCUTOONS-Sachen scannen. Vielleicht lässt sie sich ja verbessern …","en":"Your very first camera. It does one thing: scan the QR codes on SCUTOONS things. Maybe it can be improved …"},"kind":"virtual","category":"tool","icon":"catpope/items/camera_photo.png","stackable":false,"starter":true,"grantable":[],"slot":null,"skills":[{"type":"scan_qr"}],"edition_id":null,"product":null,"variants":null}' AS `data`, '2026-10-04 00:00:00' AS `updated_at`, 'seed' AS `updated_by`, 'catpope' AS `toon`, NULL AS `edition_id`
  UNION ALL SELECT 'scutoons_tshirt', 1, '{"id":"scutoons_tshirt","toon":null,"name":{"de":"SCUTOONS T-Shirt","en":"SCUTOONS T-shirt"},"description":{"de":"Das T-Shirt, mit dem bei SCUTOONS alle anfangen. Zieh es an, zieh es aus, tausch es gegen etwas Besseres.","en":"The T-shirt everyone at SCUTOONS starts with. Put it on, take it off, swap it for something better."},"kind":"virtual","category":"wearable","icon":"scutoons/items/scutoons_tshirt.png","stackable":false,"starter":true,"default_worn":true,"grantable":[],"slot":"torso","skills":[],"edition_id":null,"product":null,"variants":null,"wear":{"catpope":"characters/catpope/wear/scutoons_tshirt.webp"}}', '2026-10-04 00:00:00', 'seed', NULL, NULL
  UNION ALL SELECT 'scutoons_swim_pants', 2, '{"id":"scutoons_swim_pants","toon":null,"name":{"de":"SCUTOONS Badehose","en":"SCUTOONS swim pants"},"description":{"de":"Die Badehose für alle Figuren. Solange „Sensible Inhalte ausblenden“ an ist, kommt sie wieder an, wenn nichts anderes die Beine bedeckt.","en":"The swim pants for every character. While \\"Hide sensitive content\\" is on, they go back on when nothing else covers the legs."},"kind":"virtual","category":"wearable","icon":"scutoons/items/scutoons_swim_pants.png","stackable":false,"starter":true,"grantable":[],"slot":"legs","skills":[],"edition_id":null,"product":null,"variants":null,"wear":{"catpope":"characters/catpope/wear/scutoons_swim_pants.webp","yellowmouse":"characters/yellowmouse/wear/scutoons_swim_pants.webp","toptoy/legs-left":"characters/toptoy/wear/scutoons_swim_pants-legs-left.webp","toptoy/legs-right":"characters/toptoy/wear/scutoons_swim_pants-legs-right.webp"}}', '2026-10-04 00:00:00', 'seed', NULL, NULL
  UNION ALL SELECT 'scutoons_shorts', 3, '{"id":"scutoons_shorts","toon":null,"name":{"de":"SCUTOONS Shorts","en":"SCUTOONS shorts"},"description":{"de":"Kurze Jeans-Shorts – Teil der Grundausstattung, zusammen mit T-Shirt und Sandalen.","en":"Short denim shorts – part of the basic dress, with the T-shirt and the sandals."},"kind":"virtual","category":"wearable","icon":"scutoons/items/scutoons_shorts.png","stackable":false,"starter":true,"default_worn":true,"grantable":[],"slot":"legs","skills":[],"edition_id":null,"product":null,"variants":null,"wear":{"catpope":"characters/catpope/wear/scutoons_shorts.webp"}}', '2026-10-04 00:00:00', 'seed', NULL, NULL
  UNION ALL SELECT 'scutoons_sandals', 4, '{"id":"scutoons_sandals","toon":null,"name":{"de":"SCUTOONS Sandalen","en":"SCUTOONS sandals"},"description":{"de":"Sandalen mit zwei Riemen – Teil der Grundausstattung, zusammen mit T-Shirt und Shorts.","en":"Two-strap sandals – part of the basic dress, with the T-shirt and the shorts."},"kind":"virtual","category":"wearable","icon":"scutoons/items/scutoons_sandals.png","stackable":false,"starter":true,"default_worn":true,"grantable":[],"slot":"feet","skills":[],"edition_id":null,"product":null,"variants":null,"wear":{"catpope":"characters/catpope/wear/scutoons_sandals.webp"}}', '2026-10-04 00:00:00', 'seed', NULL, NULL
  UNION ALL SELECT 'cucumber', 5, '{"id":"cucumber","toon":"catpope","name":{"de":"Gurke","en":"Cucumber"},"description":{"de":"Eine knackige grüne Gurke. Cat Pope schwört darauf. Essen senkt deinen Appetit-Wert ein wenig. Lässt sich mit anderen Dingen kombinieren.","en":"A crunchy green cucumber. Cat Pope swears by it. Eating it lowers your appetite level a little. Can be combined with other things."},"kind":"virtual","category":"food","icon":"catpope/items/cucumber_slice.png","stackable":true,"starter":false,"grantable":["story","scan"],"slot":null,"skills":[{"type":"apply"}],"edition_id":null,"product":null,"variants":null,"effects":[{"stat":"appetite","delta":-5}],"respawn":{"default":{"hours":6},"min":{"hours":1},"max":{"days":7}}}', '2026-10-04 00:00:00', 'seed', 'catpope', NULL
  UNION ALL SELECT 'cucumber_camera', 6, '{"id":"cucumber_camera","toon":"catpope","name":{"de":"Gurkenkamera","en":"Cucumber camera"},"description":{"de":"Eine Kamera mit Gurke. Sie sieht nur noch Gurken: QR-Codes kann sie nicht mehr lesen, dafür erkennt sie echte Gurken in der echten Welt. Jede gefundene Gurke landet in deinem Inventar.","en":"A camera with a cucumber. It only sees cucumbers now: it can no longer read QR codes, but it recognises real cucumbers in the real world. Every cucumber it finds ends up in your inventory."},"kind":"virtual","category":"tool","icon":"catpope/items/cucumber_camera_photo.png","stackable":false,"starter":false,"grantable":[],"slot":null,"skills":[{"type":"scan_object","labels":["cucumber","cuke","zucchini","courgette"],"grants":"cucumber","cooldown_s":60,"per_day":3}],"edition_id":null,"product":null,"variants":null}', '2026-10-04 00:00:00', 'seed', 'catpope', NULL
  UNION ALL SELECT 'cucumber_cross', 7, '{"id":"cucumber_cross","toon":"catpope","name":{"de":"Gurkenkreuz","en":"Cucumber cross"},"description":{"de":"Zwei Gurken, über Kreuz gebunden. Taugt für einen Exorzismus.","en":"Two cucumbers, bound into a cross. Good for an exorcism."},"kind":"virtual","category":"tool","icon":"catpope/items/cucumber_cross.png","stackable":false,"starter":false,"grantable":[],"slot":null,"skills":[],"edition_id":null,"product":null,"variants":null}', '2026-10-04 00:00:00', 'seed', 'catpope', NULL
  UNION ALL SELECT 'testo', 8, '{"id":"testo","toon":null,"name":{"de":"Testo","en":"Testo"},"description":{"de":"Eine Tube Testo – ein Spielgegenstand. Benutzen hebt deinen Testo-Wert, der mit der Zeit wieder zur Mitte sinkt. Lässt sich mit der Kamera kombinieren.","en":"A tube of testo – a game item. Using it raises your testo level, which drifts back to the middle over time. Can be combined with the camera."},"kind":"virtual","category":"substance","icon":"scutoons/items/testo.png","stackable":true,"starter":false,"grantable":["story","scan"],"respawn":{"default":{"days":1},"min":{"hours":6},"max":{"days":7}},"slot":null,"skills":[{"type":"apply"}],"edition_id":null,"product":null,"variants":null,"effects":[{"stat":"testo","delta":20}]}', '2026-10-04 00:00:00', 'seed', NULL, NULL
  UNION ALL SELECT 'testo_camera', 9, '{"id":"testo_camera","toon":null,"name":{"de":"Testo-Kamera","en":"Testo camera"},"description":{"de":"Eine Kamera mit Testo. Sie liest keine QR-Codes mehr, dafür erkennt sie Tuben und Fläschchen in der echten Welt – jeder Fund ist eine Tube Testo im Spiel.","en":"A camera with testo. It no longer reads QR codes, but it spots tubes and little bottles in the real world – every find is a tube of testo in the game."},"kind":"virtual","category":"tool","icon":"scutoons/items/testo_camera.png","stackable":false,"starter":false,"grantable":[],"slot":null,"skills":[{"type":"scan_object","labels":["lotion","sunscreen","pill bottle"],"grants":"testo","cooldown_s":10}],"edition_id":null,"product":null,"variants":null}', '2026-10-04 00:00:00', 'seed', NULL, NULL
  UNION ALL SELECT 'checkit_camera', 10, '{"id":"checkit_camera","toon":null,"name":{"de":"Check It Kamera","en":"Check It camera"},"description":{"de":"Eine Spiel-Kamera aus Kamera und Testo, die Dinge aus der echten Welt in Spielgegenstände verwandelt: Pillendosen und Tuben, Spritzen, Pilze, kleine Briefchen. Sie analysiert keine echten Substanzen.","en":"A game camera made of a camera and testo that turns things from the real world into game items: pill jars and tubes, syringes, mushrooms, little envelopes. It does not analyse real substances."},"kind":"virtual","category":"tool","icon":"scutoons/items/checkit_camera.png","stackable":false,"starter":false,"grantable":["story"],"slot":null,"skills":[{"type":"scan_object","labels":["pill bottle","lotion","sunscreen"],"grants":"testo","name":{"de":"Pillendose / Tube","en":"pill jar / tube"},"map":[{"labels":["syringe"],"grants":"semaglutide","name":{"de":"Spritze","en":"syringe"}},{"labels":["mushroom","agaric"],"grants":"psilocybin","name":{"de":"Pilz","en":"mushroom"}},{"labels":["envelope"],"grants":"lsd","name":{"de":"Briefchen","en":"little envelope"}}],"cooldown_s":15}],"edition_id":null,"product":null,"variants":null}', '2026-10-04 00:00:00', 'seed', NULL, NULL
  UNION ALL SELECT 'psilocybin', 11, '{"id":"psilocybin","toon":null,"name":{"de":"Zauberpilz","en":"Magic mushroom"},"description":{"de":"Ein Pilz aus dem Spiel. Benutzen lässt die Seiten eine Weile in Farben schwimmen – jederzeit beendbar.","en":"A mushroom from the game. Using it makes the pages swim in colours for a while – you can end it any time."},"kind":"virtual","category":"substance","icon":"scutoons/items/psilocybin.png","stackable":true,"starter":false,"grantable":["story","scan"],"respawn":{"default":{"days":3},"min":{"days":1},"max":{"days":14}},"slot":null,"skills":[{"type":"apply"}],"edition_id":null,"product":null,"variants":null,"effect":{"kind":"psychedelic","duration_min":20}}', '2026-10-04 00:00:00', 'seed', NULL, NULL
  UNION ALL SELECT 'lsd', 12, '{"id":"lsd","toon":null,"name":{"de":"Bunte Pappe","en":"Blotter"},"description":{"de":"Ein buntes Stück Pappe aus dem Spiel. Benutzen lässt die Seiten länger in Farben schwimmen – jederzeit beendbar.","en":"A colourful piece of blotter from the game. Using it makes the pages swim in colours for longer – you can end it any time."},"kind":"virtual","category":"substance","icon":"scutoons/items/lsd.png","stackable":true,"starter":false,"grantable":["story","scan"],"respawn":{"default":{"days":3},"min":{"days":1},"max":{"days":14}},"slot":null,"skills":[{"type":"apply"}],"edition_id":null,"product":null,"variants":null,"effect":{"kind":"psychedelic","duration_min":45}}', '2026-10-04 00:00:00', 'seed', NULL, NULL
  UNION ALL SELECT 'semaglutide', 13, '{"id":"semaglutide","toon":null,"name":{"de":"Semaglutid-Pen","en":"Semaglutide pen"},"description":{"de":"Ein Pen aus dem Spiel. Benutzen senkt deinen Appetit-Wert, der mit der Zeit wieder zur Mitte steigt. Noch ohne Folgen im Spiel.","en":"A pen from the game. Using it lowers your appetite level, which drifts back to the middle over time. No consequences in the game yet."},"kind":"virtual","category":"substance","icon":"scutoons/items/semaglutide.png","stackable":true,"starter":false,"grantable":["story","scan"],"respawn":{"default":{"days":7},"min":{"days":1},"max":{"days":30}},"slot":null,"skills":[{"type":"apply"}],"edition_id":null,"product":null,"variants":null,"effects":[{"stat":"appetite","delta":-20}]}', '2026-10-04 00:00:00', 'seed', NULL, NULL
  UNION ALL SELECT 'weed', 14, '{"id":"weed","toon":null,"name":{"de":"Gras","en":"Weed"},"description":{"de":"Ein Tütchen Gras aus dem Spiel. Benutzen hebt deinen Appetit-Wert, der mit der Zeit wieder zur Mitte sinkt.","en":"A little bag of weed from the game. Using it raises your appetite level, which drifts back to the middle over time."},"kind":"virtual","category":"substance","icon":"scutoons/items/weed.png","stackable":true,"starter":false,"grantable":["story","scan"],"respawn":{"default":{"hours":12},"min":{"hours":1},"max":{"days":7}},"slot":null,"skills":[{"type":"apply"}],"edition_id":null,"product":null,"variants":null,"effects":[{"stat":"appetite","delta":25}]}', '2026-10-04 00:00:00', 'seed', NULL, NULL
  UNION ALL SELECT 'tuna_can', 15, '{"id":"tuna_can","toon":null,"name":{"de":"Thunfischdose","en":"Can of tuna"},"description":{"de":"Eine Dose Thunfisch. Essen senkt deinen Appetit-Wert deutlich – viel mehr als eine Gurke.","en":"A can of tuna. Eating it lowers your appetite level a lot – much more than a cucumber."},"kind":"virtual","category":"food","icon":"scutoons/items/tuna_can.png","stackable":true,"starter":false,"grantable":["story","scan"],"slot":null,"skills":[{"type":"apply"}],"edition_id":null,"product":null,"variants":null,"effects":[{"stat":"appetite","delta":-30}],"respawn":{"default":{"hours":12},"min":{"hours":1},"max":{"days":7}}}', '2026-10-04 00:00:00', 'seed', NULL, NULL
  UNION ALL SELECT 'dago_hoodie', 16, '{"id":"dago_hoodie","toon":"catpope","name":{"de":"Cat Pope Dago Hoodie","en":"Cat Pope Dago Hoodie"},"description":{"de":"Oversized Acid-Wash-Hoodie mit dem Dago-Vektormotiv, von Hand siebgedruckt. Limitierte, nummerierte Edition.","en":"Oversized acid wash hoodie with the Dago vector motif, hand silk-screen printed. Limited, numbered edition."},"kind":"physical","category":"wearable","icon":"catpope/items/dago_hoodie.png","stackable":false,"starter":false,"grantable":[],"slot":"torso","skills":[],"edition_id":"1_catpope_dago","product":"dago_hoodie","variants":[{"id":"black","name":{"de":"Schwarz","en":"Black"},"icon":"catpope/items/dago_hoodie_black.png","product":"dago_hoodie"},{"id":"beige","name":{"de":"Beige","en":"Beige"},"icon":"catpope/items/dago_hoodie_beige.png","product":"dago_hoodie"},{"id":"grey","name":{"de":"Grau","en":"Grey"},"icon":"catpope/items/dago_hoodie_grey.png","product":"dago_hoodie"}]}', '2026-10-04 00:00:00', 'seed', 'catpope', '1_catpope_dago'
  UNION ALL SELECT 'dago_heavy_t', 17, '{"id":"dago_heavy_t","toon":"catpope","name":{"de":"Cat Pope Dago Heavy T","en":"Cat Pope Dago Heavy T"},"description":{"de":"Oversized Heavy-T-Shirt mit dem Dago-Vektormotiv, von Hand siebgedruckt. Limitierte, nummerierte Edition.","en":"Oversized heavy tee with the Dago vector motif, hand silk-screen printed. Limited, numbered edition."},"kind":"physical","category":"wearable","icon":"catpope/items/dago_heavy_t.png","stackable":false,"starter":false,"grantable":[],"slot":"torso","skills":[],"edition_id":"1_catpope_dago","product":"dago_heavy_t","variants":[{"id":"black","name":{"de":"Schwarz","en":"Black"},"icon":"catpope/items/dago_heavy_t_black.png","product":"dago_heavy_t"},{"id":"white","name":{"de":"Weiß","en":"White"},"icon":"catpope/items/dago_heavy_t_white.png","product":"dago_heavy_t"}]}', '2026-10-04 00:00:00', 'seed', 'catpope', '1_catpope_dago'
  UNION ALL SELECT 'scorpion_amulet', 18, '{"id":"scorpion_amulet","toon":"catpope","name":{"de":"Skorpion-Amulett","en":"Scorpion Amulet"},"description":{"de":"3D-gedrucktes Amulett nach den Käfer- und Skorpion-Vektoren aus Cat Pope. Nummerierte Edition von 20.","en":"3D printed amulet based on the beetle and scorpion vectors from Cat Pope. Numbered edition of 20."},"kind":"physical","category":"wearable","icon":"catpope/items/scorpion_amulet.png","stackable":false,"starter":false,"grantable":[],"slot":"neck","skills":[],"edition_id":"2_catpope_beetles","product":"scorpion_amulet","variants":[{"id":"purple","name":{"de":"Lila","en":"Purple"},"icon":"catpope/items/scorpion_amulet_purple.png","product":"scorpion_amulet"}]}', '2026-10-04 00:00:00', 'seed', 'catpope', '2_catpope_beetles'
  UNION ALL SELECT 'awakening_vinyl', 19, '{"id":"awakening_vinyl","toon":"catpope","name":{"de":"Awakening – LE 12\\" Vinyl","en":"Awakening – LE 12\\" vinyl"},"description":{"de":"Die limitierte 12\\"-Schallplatte zum Cat-Pope-Album „Awakening“. Mit einem Plattenspieler im Inventar spielt sie die Musik ab.","en":"The limited edition 12\\" record of the Cat Pope album \\"Awakening\\". With a record player in the inventory it plays the music."},"kind":"physical","category":"media","icon":"catpope/items/awakening_vinyl.png","stackable":false,"starter":false,"grantable":[],"slot":null,"skills":[],"album":"awakening","format":"vinyl","edition_id":"3_catpope_vinyl","product":"awakening_vinyl","variants":null}', '2026-10-04 00:00:00', 'seed', 'catpope', '3_catpope_vinyl'
  UNION ALL SELECT 'awakening_test_press', 20, '{"id":"awakening_test_press","toon":"catpope","name":{"de":"Awakening – Testpressung","en":"Awakening – test press"},"description":{"de":"Eine von nur zwei Testpressungen der „Awakening“-Platte, handnummeriert.","en":"One of only two test pressings of the \\"Awakening\\" record, hand numbered."},"kind":"physical","category":"media","icon":"catpope/items/awakening_test_press.png","stackable":false,"starter":false,"grantable":[],"slot":null,"skills":[],"album":"awakening","format":"vinyl","edition_id":null,"product":"awakening_test_press","variants":[{"id":"1_of_2","name":{"de":"Testpressung 1/2","en":"Test press 1/2"},"icon":"catpope/items/awakening_test_press_1_of_2.png","product":"awakening_test_press"},{"id":"2_of_2","name":{"de":"Testpressung 2/2","en":"Test press 2/2"},"icon":"catpope/items/awakening_test_press_2_of_2.png","product":"awakening_test_press"}]}', '2026-10-04 00:00:00', 'seed', 'catpope', NULL
  UNION ALL SELECT 'kangaroo_cat_hoodie', 21, '{"id":"kangaroo_cat_hoodie","toon":"catpope","name":{"de":"Love For Women – Kangaroo Cat Hoodie","en":"Love For Women – Kangaroo Cat Hoodie"},"description":{"de":"Hoodie mit der Känguru-Katze aus „Love For Women“.","en":"Hoodie featuring the kangaroo cat from \\"Love For Women\\"."},"kind":"physical","category":"wearable","icon":"catpope/items/kangaroo_cat_hoodie.png","stackable":false,"starter":false,"grantable":[],"slot":"torso","skills":[],"edition_id":null,"product":"kangaroo_cat_hoodie","variants":null}', '2026-10-04 00:00:00', 'seed', 'catpope', NULL
  UNION ALL SELECT 'record_player', 22, '{"id":"record_player","toon":null,"name":{"de":"Plattenspieler","en":"Record player"},"description":{"de":"Ein Plattenspieler. Er spielt die Schallplatten in deinem Inventar ab. Du bekommst ihn, wenn du einen kaputten Plattenspieler mit einem Reparatur-Gutschein reparieren lässt.","en":"A record player. It plays the records in your inventory. You get it by having a broken record player repaired with a repair coupon."},"kind":"virtual","category":"tool","icon":"scutoons/items/record_player.png","stackable":false,"starter":false,"grantable":[],"slot":null,"skills":[],"plays":["vinyl"],"edition_id":null,"product":null,"variants":null}', '2026-10-04 00:00:00', 'seed', NULL, NULL
  UNION ALL SELECT 'tape_player', 23, '{"id":"tape_player","toon":null,"name":{"de":"Kassettenspieler","en":"Tape player"},"description":{"de":"Ein Kassettenspieler. Er spielt Kassetten aus deinem Inventar ab.","en":"A tape player. It plays the tapes in your inventory."},"kind":"virtual","category":"tool","icon":"scutoons/items/tape_player.png","stackable":false,"starter":false,"grantable":[],"slot":null,"skills":[],"plays":["tape"],"edition_id":null,"product":null,"variants":null}', '2026-10-04 00:00:00', 'seed', NULL, NULL
  UNION ALL SELECT 'cd_player', 24, '{"id":"cd_player","toon":null,"name":{"de":"CD-Spieler","en":"CD player"},"description":{"de":"Ein CD-Spieler. Er spielt CDs aus deinem Inventar ab.","en":"A CD player. It plays the CDs in your inventory."},"kind":"virtual","category":"tool","icon":"scutoons/items/cd_player.png","stackable":false,"starter":false,"grantable":[],"slot":null,"skills":[],"plays":["cd"],"edition_id":null,"product":null,"variants":null}', '2026-10-04 00:00:00', 'seed', NULL, NULL
  UNION ALL SELECT 'broken_record_player', 25, '{"id":"broken_record_player","toon":null,"name":{"de":"Kaputter Plattenspieler","en":"Broken record player"},"description":{"de":"Ein alter Plattenspieler – kaputt, der Tonarm hängt schief. So spielt er nichts ab. Mit einem Reparatur-Gutschein lässt er sich reparieren.","en":"An old record player – broken, the tonearm hangs crooked. It won''t play anything like this. A repair coupon gets it fixed."},"kind":"virtual","category":"tool","icon":"scutoons/items/broken_record_player.png","stackable":false,"starter":false,"grantable":["story"],"slot":null,"skills":[],"edition_id":null,"product":null,"variants":null}', '2026-10-04 00:00:00', 'seed', NULL, NULL
  UNION ALL SELECT 'repair_coupon', 26, '{"id":"repair_coupon","toon":null,"name":{"de":"Reparatur-Gutschein","en":"Repair coupon"},"description":{"de":"Ein Gutschein der Stadt Wien: Damit lässt du etwas Kaputtes reparieren, statt es wegzuwerfen. Kombiniere ihn mit dem, was repariert werden soll. Er gilt einmal.","en":"A coupon from the city of Vienna: it gets something broken repaired instead of thrown away. Combine it with the thing to repair. It is good for one repair."},"kind":"virtual","category":"collectible","icon":"scutoons/items/repair_coupon.png","stackable":false,"starter":false,"grantable":["story"],"slot":null,"skills":[],"edition_id":null,"product":null,"variants":null}', '2026-10-04 00:00:00', 'seed', NULL, NULL
) AS seed WHERE NOT EXISTS (SELECT 1 FROM `catalog_items`);
INSERT INTO `catalog_recipes` (`id`, `position`, `data`, `updated_at`, `updated_by`) SELECT * FROM (
  SELECT 'cucumber_camera' AS `id`, 0 AS `position`, '{"id":"cucumber_camera","inputs":[{"item":"camera","qty":1},{"item":"cucumber","qty":1}],"output":{"item":"cucumber_camera","qty":1}}' AS `data`, '2026-10-04 00:00:00' AS `updated_at`, 'seed' AS `updated_by`
  UNION ALL SELECT 'cucumber_cross', 1, '{"id":"cucumber_cross","inputs":[{"item":"cucumber","qty":2}],"output":{"item":"cucumber_cross","qty":1}}', '2026-10-04 00:00:00', 'seed'
  UNION ALL SELECT 'testo_camera', 2, '{"id":"testo_camera","inputs":[{"item":"camera","qty":1},{"item":"testo","qty":1}],"output":{"item":"testo_camera","qty":1}}', '2026-10-04 00:00:00', 'seed'
  UNION ALL SELECT 'checkit_camera', 3, '{"id":"checkit_camera","inputs":[{"item":"camera","qty":1},{"item":"testo","qty":1}],"output":{"item":"checkit_camera","qty":1}}', '2026-10-04 00:00:00', 'seed'
  UNION ALL SELECT 'record_player_repair', 4, '{"id":"record_player_repair","inputs":[{"item":"broken_record_player","qty":1},{"item":"repair_coupon","qty":1}],"output":{"item":"record_player","qty":1}}', '2026-10-04 00:00:00', 'seed'
) AS seed WHERE NOT EXISTS (SELECT 1 FROM `catalog_recipes`);
INSERT INTO `catalog_albums` (`id`, `position`, `data`, `updated_at`, `updated_by`, `toon`) SELECT * FROM (
  SELECT 'awakening' AS `id`, 0 AS `position`, '{"id":"awakening","toon":"catpope","title":{"de":"Awakening","en":"Awakening"},"tracks":[{"id":"brass_morton","title":"Instrumental (Brass Morton)","src":"catpope/s1/c1/s1c1-instrumental-brass-morton.128.mp3"}],"download":"catpope/s1/CatPope-Awakening-Music.zip","cover":"catpope/s1/s1-music-vinyl.webp","access":{"download":"members","listen":"owners"}}' AS `data`, '2026-10-04 00:00:00' AS `updated_at`, 'seed' AS `updated_by`, 'catpope' AS `toon`
) AS seed WHERE NOT EXISTS (SELECT 1 FROM `catalog_albums`);
INSERT INTO `catalog_products` (`id`, `position`, `data`, `updated_at`, `updated_by`, `toon`, `item_id`, `edition_id`) SELECT * FROM (
  SELECT 'dago_hoodie' AS `id`, 0 AS `position`, '{"id":"dago_hoodie","toon":"catpope","item_id":"dago_hoodie","edition_id":"1_catpope_dago","edition_size":37,"name":"Cat Pope Dago Hoodie","category":"apparel","link":"https://mandymozart.bandcamp.com/merch/cat-pope-dago-hoodie","price":99.9,"limited":12,"remaining":3,"sizes":["M","L","XL","XXL"],"soldOutSizes":["XXL"],"credits":["Design: Tatjana Stürmer","Print: Louise Rath"],"icon":"catpope/items/dago_hoodie.png","defaultVariant":"beige","variants":[{"id":"beige","name":{"de":"Beige","en":"Beige"},"soldOut":false,"icon":"catpope/items/dago_hoodie_beige.png","images":["catpope/merch/1_catpope_dago_hoodie_beige_web-13.jpg","catpope/merch/1_catpope_dago_hoodie_beige_web-14.jpg","catpope/merch/1_catpope_dago_hoodie_beige_web-15.jpg","catpope/merch/1_catpope_dago_hoodie_beige_web-16.jpg","catpope/merch/1_catpope_dago_hoodie_beige_web-17.jpg","catpope/merch/1_catpope_dago_hoodie_beige_web-18.jpg","catpope/merch/1_catpope_dago_hoodie_beige_web-19.jpg","catpope/merch/1_catpope_dago_hoodie_beige_web-20.jpg","catpope/merch/1_catpope_dago_hoodie_beige_web-21.jpg","catpope/merch/1_catpope_dago_hoodie_beige_web-47.jpg","catpope/merch/1_catpope_dago_hoodie_beige_web-49.jpg","catpope/merch/1_catpope_dago_hoodie_beige_web-50.jpg"]},{"id":"black","name":{"de":"Schwarz","en":"Black"},"soldOut":true,"icon":"catpope/items/dago_hoodie_black.png","images":["catpope/merch/1_catpope_dago_hoodie_black_web-31.jpg","catpope/merch/1_catpope_dago_hoodie_black_web-32.jpg","catpope/merch/1_catpope_dago_hoodie_black_web-33.jpg","catpope/merch/1_catpope_dago_hoodie_black_web-34.jpg","catpope/merch/1_catpope_dago_hoodie_black_web-35.jpg","catpope/merch/1_catpope_dago_hoodie_black_web-36.jpg","catpope/merch/1_catpope_dago_hoodie_black_web-38.jpg","catpope/merch/1_catpope_dago_hoodie_black_web-39.jpg","catpope/merch/1_catpope_dago_hoodie_black_web-56.jpg","catpope/merch/1_catpope_dago_hoodie_black_web-57.jpg","catpope/merch/1_catpope_dago_hoodie_black_web-58.jpg","catpope/merch/1_catpope_dago_hoodie_black_web-59.jpg"]},{"id":"grey","name":{"de":"Grau","en":"Grey"},"soldOut":true,"icon":"catpope/items/dago_hoodie_grey.png","images":["catpope/merch/1_catpope_dago_hoodie_grey_web-22.jpg","catpope/merch/1_catpope_dago_hoodie_grey_web-23.jpg","catpope/merch/1_catpope_dago_hoodie_grey_web-24.jpg","catpope/merch/1_catpope_dago_hoodie_grey_web-25.jpg","catpope/merch/1_catpope_dago_hoodie_grey_web-26.jpg","catpope/merch/1_catpope_dago_hoodie_grey_web-27.jpg","catpope/merch/1_catpope_dago_hoodie_grey_web-28.jpg","catpope/merch/1_catpope_dago_hoodie_grey_web-29.jpg","catpope/merch/1_catpope_dago_hoodie_grey_web-30.jpg","catpope/merch/1_catpope_dago_hoodie_grey_web-51.jpg","catpope/merch/1_catpope_dago_hoodie_grey_web-53.jpg","catpope/merch/1_catpope_dago_hoodie_grey_web-54.jpg","catpope/merch/1_catpope_dago_hoodie_grey_web-55.jpg"]}]}' AS `data`, '2026-10-04 00:00:00' AS `updated_at`, 'seed' AS `updated_by`, 'catpope' AS `toon`, 'dago_hoodie' AS `item_id`, '1_catpope_dago' AS `edition_id`
  UNION ALL SELECT 'dago_heavy_t', 1, '{"id":"dago_heavy_t","toon":"catpope","item_id":"dago_heavy_t","edition_id":"1_catpope_dago","edition_size":37,"name":"Cat Pope Dago Heavy T","category":"apparel","link":"https://mandymozart.bandcamp.com/merch/cat-pope-dago-heavy-t","price":59.9,"limited":25,"remaining":null,"sizes":["S","M","L","XL"],"soldOutSizes":[],"credits":["Design: Tatjana Stürmer","Print: Louise Rath"],"icon":"catpope/items/dago_heavy_t.png","defaultVariant":"white","variants":[{"id":"white","name":{"de":"Weiß","en":"White"},"soldOut":false,"icon":"catpope/items/dago_heavy_t_white.png","images":["catpope/merch/1_catpope_dago_heavy_t_white_web-1.jpg","catpope/merch/1_catpope_dago_heavy_t_white_web-2.jpg","catpope/merch/1_catpope_dago_heavy_t_white_web-3.jpg","catpope/merch/1_catpope_dago_heavy_t_white_web-4.jpg","catpope/merch/1_catpope_dago_heavy_t_white_web-5.jpg","catpope/merch/1_catpope_dago_heavy_t_white_web-6.jpg","catpope/merch/1_catpope_dago_heavy_t_white_web-40.jpg","catpope/merch/1_catpope_dago_heavy_t_white_web-41.jpg","catpope/merch/1_catpope_dago_heavy_t_white_web-42.jpg"]},{"id":"black","name":{"de":"Schwarz","en":"Black"},"soldOut":true,"icon":"catpope/items/dago_heavy_t_black.png","images":["catpope/merch/1_catpope_dago_heavy_t_black_web-7.jpg","catpope/merch/1_catpope_dago_heavy_t_black_web-8.jpg","catpope/merch/1_catpope_dago_heavy_t_black_web-9.jpg","catpope/merch/1_catpope_dago_heavy_t_black_web-10.jpg","catpope/merch/1_catpope_dago_heavy_t_black_web-11.jpg","catpope/merch/1_catpope_dago_heavy_t_black_web-12.jpg","catpope/merch/1_catpope_dago_heavy_t_black_web-43.jpg","catpope/merch/1_catpope_dago_heavy_t_black_web-44.jpg","catpope/merch/1_catpope_dago_heavy_t_black_web-45.jpg"]}]}', '2026-10-04 00:00:00', 'seed', 'catpope', 'dago_heavy_t', '1_catpope_dago'
  UNION ALL SELECT 'scorpion_amulet', 2, '{"id":"scorpion_amulet","toon":"catpope","item_id":"scorpion_amulet","edition_id":"2_catpope_beetles","edition_size":20,"name":"Scorpion Amulet","category":"jewellery","link":"https://mandymozart.bandcamp.com/merch/scropion-amulet","price":50.0,"limited":null,"remaining":null,"sizes":[],"soldOutSizes":[],"credits":["Design: Tatjana Stürmer","3D: Mandy Mozart"],"icon":"catpope/items/scorpion_amulet.png","defaultVariant":"purple","variants":[{"id":"purple","name":{"de":"Lila","en":"Purple"},"soldOut":false,"icon":"catpope/items/scorpion_amulet_purple.png","images":["catpope/merch/bandcamp/web/scorpion_amulet_0046916043.jpg"]}]}', '2026-10-04 00:00:00', 'seed', 'catpope', 'scorpion_amulet', '2_catpope_beetles'
  UNION ALL SELECT 'awakening_vinyl', 3, '{"id":"awakening_vinyl","toon":"catpope","item_id":"awakening_vinyl","edition_id":"3_catpope_vinyl","edition_size":300,"name":"Cat Pope – Awakening · Limited Edition 12\\" Vinyl","category":"vinyl","link":"https://mandymozart.bandcamp.com/album/cat-pope-awakening","price":27.5,"limited":300,"remaining":null,"sizes":[],"soldOutSizes":[],"credits":["Design: Tatjana Stürmer","Photo: Thomas Dalby"],"icon":"catpope/items/awakening_vinyl.png","defaultVariant":null,"variants":[{"id":"default","name":{"de":"12\\" Vinyl","en":"12\\" vinyl"},"soldOut":false,"icon":"catpope/items/awakening_vinyl.png","images":["catpope/merch/bandcamp/web/awakening_vinyl_0046915732.jpg","catpope/merch/bandcamp/web/awakening_vinyl_0046915789.jpg"]}]}', '2026-10-04 00:00:00', 'seed', 'catpope', 'awakening_vinyl', '3_catpope_vinyl'
  UNION ALL SELECT 'awakening_test_press', 4, '{"id":"awakening_test_press","toon":"catpope","item_id":"awakening_test_press","edition_id":null,"edition_size":null,"name":"Cat Pope – Awakening · Test Press","category":"vinyl","link":"https://mandymozart.bandcamp.com/album/cat-pope-awakening","price":75.0,"limited":2,"remaining":null,"sizes":[],"soldOutSizes":[],"credits":["Pressed by Rand Muzik Leipzig"],"icon":"catpope/items/awakening_test_press.png","defaultVariant":"1_of_2","variants":[{"id":"1_of_2","name":{"de":"Test Press 1/2","en":"Test press 1/2"},"soldOut":false,"icon":"catpope/items/awakening_test_press_1_of_2.png","images":["catpope/merch/bandcamp/web/awakening_test_press_1_of_2_0046916239.jpg"],"price":100.0},{"id":"2_of_2","name":{"de":"Test Press 2/2","en":"Test press 2/2"},"soldOut":false,"icon":"catpope/items/awakening_test_press_2_of_2.png","images":["catpope/merch/bandcamp/web/awakening_test_press_2_of_2_0046916279.jpg"],"price":75.0}]}', '2026-10-04 00:00:00', 'seed', 'catpope', 'awakening_test_press', NULL
  UNION ALL SELECT 'kangaroo_cat_hoodie', 5, '{"id":"kangaroo_cat_hoodie","toon":"catpope","item_id":"kangaroo_cat_hoodie","edition_id":null,"edition_size":null,"name":"Love For Women – Kangaroo Cat Hoodie","category":"apparel","link":"https://mandymozart.bandcamp.com/merch/love-for-women-kangaroo-cat-hoodie","price":59.99,"limited":4,"remaining":3,"sizes":[],"soldOutSizes":[],"credits":[],"icon":"catpope/items/kangaroo_cat_hoodie.png","defaultVariant":null,"variants":[{"id":"default","name":{"de":"Flieder","en":"Lilac"},"soldOut":false,"icon":"catpope/items/kangaroo_cat_hoodie.png","images":["catpope/merch/bandcamp/web/kangaroo_cat_hoodie_0044221026.jpg","catpope/merch/bandcamp/web/kangaroo_cat_hoodie_0044221024.jpg","catpope/merch/bandcamp/web/kangaroo_cat_hoodie_0044221055.jpg","catpope/merch/bandcamp/web/kangaroo_cat_hoodie_0044221058.jpg"]}]}', '2026-10-04 00:00:00', 'seed', 'catpope', 'kangaroo_cat_hoodie', NULL
) AS seed WHERE NOT EXISTS (SELECT 1 FROM `catalog_products`);
INSERT INTO `catalog_editions` (`id`, `position`, `data`, `updated_at`, `updated_by`, `toon`) SELECT * FROM (
  SELECT '1_catpope_dago' AS `id`, 0 AS `position`, '{"id":"1_catpope_dago","toon":"catpope","name":"Cat Pope Dago","description":"Oversized Hoodies and Shirts with Dago Vector Motiv","edition_number":1,"edition_count":37,"excluded_number":3,"created_at":"2025-10-04T20:00:00.000Z","creators":"Tatjana Stuermer Vectors\\nLouise Rath Silk Screen Print \\nMandy Mozart Pre-Vector AI renderings, Concept","variants":[{"id":"oversized_acid_wash_hoodie","name":"Oversized Acid Wash Hoodie","sizes":["M","L","XL","XXL"],"colors":["grey","black","beige"]},{"id":"oversized_heavy_t","name":"Oversized Heavy T","sizes":["S","M","L","XL"],"colors":["black","white"]}]}' AS `data`, '2026-10-04 00:00:00' AS `updated_at`, 'seed' AS `updated_by`, 'catpope' AS `toon`
  UNION ALL SELECT '2_catpope_beetles', 1, '{"id":"2_catpope_beetles","toon":"catpope","name":"Cat Pope Beetles","description":"Jewlery made from Vector Motives","edition_number":1,"edition_count":20,"excluded_number":0,"created_at":"2026-05-15T20:00:00.000Z","creators":"Tatjana Stuermer Vectors\\n\\nMandy Mozart 3D, Concept","variants":[{"id":"scorpion_amulet_3d","name":"Scorpion Amulet 3D","sizes":["regular"],"colors":["purple"]}]}', '2026-10-04 00:00:00', 'seed', 'catpope'
  UNION ALL SELECT '3_catpope_vinyl', 2, '{"id":"3_catpope_vinyl","toon":"catpope","name":"Cat Pope Vinyl","description":"Record of the Album","edition_number":1,"edition_count":300,"created_at":"2026-06-30T20:00:00.000Z","creators":"Mandy Mozart & various","variants":[{"id":"first_edition_vinyl","name":"First Edition Vinyl","sizes":["regular"]}]}', '2026-10-04 00:00:00', 'seed', 'catpope'
) AS seed WHERE NOT EXISTS (SELECT 1 FROM `catalog_editions`);
INSERT INTO `catalog_featured` (`id`, `position`, `data`, `updated_at`, `updated_by`, `product_id`, `variant_id`) SELECT * FROM (
  SELECT 'dago_hoodie:beige' AS `id`, 0 AS `position`, '{"product":"dago_hoodie","variant":"beige"}' AS `data`, '2026-10-04 00:00:00' AS `updated_at`, 'seed' AS `updated_by`, 'dago_hoodie' AS `product_id`, 'beige' AS `variant_id`
  UNION ALL SELECT 'dago_heavy_t:white', 1, '{"product":"dago_heavy_t","variant":"white"}', '2026-10-04 00:00:00', 'seed', 'dago_heavy_t', 'white'
) AS seed WHERE NOT EXISTS (SELECT 1 FROM `catalog_featured`);
INSERT INTO `catalog_permissions` (`id`, `position`, `data`, `updated_at`, `updated_by`) SELECT * FROM (
  SELECT 'catalog' AS `id`, 0 AS `position`, '"Read every chapter of every toon"' AS `data`, '2026-10-04 00:00:00' AS `updated_at`, 'seed' AS `updated_by`
) AS seed WHERE NOT EXISTS (SELECT 1 FROM `catalog_permissions`);
INSERT INTO `catalog_code_types` (`id`, `position`, `data`, `updated_at`, `updated_by`) SELECT * FROM (
  SELECT 'membership' AS `id`, 0 AS `position`, '{"claimable":true,"signup":true,"grants":["catalog"]}' AS `data`, '2026-10-04 00:00:00' AS `updated_at`, 'seed' AS `updated_by`
  UNION ALL SELECT 'promo', 1, '{"claimable":false,"signup":true,"grants":["catalog"]}', '2026-10-04 00:00:00', 'seed'
  UNION ALL SELECT 'page', 2, '{"claimable":false,"signup":false,"grants":[]}', '2026-10-04 00:00:00', 'seed'
  UNION ALL SELECT 'event', 3, '{"claimable":false,"signup":false,"grants":[]}', '2026-10-04 00:00:00', 'seed'
  UNION ALL SELECT 'xr', 4, '{"claimable":false,"signup":false,"grants":[]}', '2026-10-04 00:00:00', 'seed'
  UNION ALL SELECT 'default', 5, '{"claimable":false,"signup":false,"grants":[]}', '2026-10-04 00:00:00', 'seed'
) AS seed WHERE NOT EXISTS (SELECT 1 FROM `catalog_code_types`);
INSERT INTO `toons` (`id`, `position`, `data`, `updated_at`, `updated_by`) SELECT * FROM (
  SELECT 'catpope' AS `id`, 0 AS `position`, '{"id":"catpope","name":"Cat Pope","redirectUrl":"https://scutoons.com/catpope"}' AS `data`, '2026-10-04 00:00:00' AS `updated_at`, 'seed' AS `updated_by`
  UNION ALL SELECT 'toptoy', 1, '{"id":"toptoy","name":"Top Toy","redirectUrl":"https://scutoons.com/toptoy"}', '2026-10-04 00:00:00', 'seed'
  UNION ALL SELECT 'yellowmouse', 2, '{"id":"yellowmouse","name":"Yellow Mouse","redirectUrl":"https://scutoons.com/yellowmouse"}', '2026-10-04 00:00:00', 'seed'
) AS seed WHERE NOT EXISTS (SELECT 1 FROM `toons`);
INSERT INTO `catalog_meta` (`id`, `position`, `data`, `updated_at`, `updated_by`) SELECT * FROM (
  SELECT 'items' AS `id`, 0 AS `position`, '{"version":1}' AS `data`, '2026-10-04 00:00:00' AS `updated_at`, 'seed' AS `updated_by`
  UNION ALL SELECT 'products', 1, '{"version":2,"currency":"EUR","checked_at":"2026-09-29","source":"https://mandymozart.bandcamp.com/merch"}', '2026-10-04 00:00:00', 'seed'
  UNION ALL SELECT 'permissions', 2, '{"version":1}', '2026-10-04 00:00:00', 'seed'
) AS seed WHERE NOT EXISTS (SELECT 1 FROM `catalog_meta`);
-- END catalog seed

-- The admin's respawn overrides (setting item_respawn: { "<item>": rule | null }) become the items' own `respawn`
UPDATE `catalog_items` i JOIN `settings` s ON s.`key` = 'item_respawn'
SET i.`data` = JSON_SET(i.`data`, '$.respawn', JSON_EXTRACT(s.`value`, CONCAT('$."', i.`id`, '"'))), i.`updated_at` = s.`updated_at`, i.`updated_by` = s.`updated_by`
WHERE JSON_CONTAINS_PATH(s.`value`, 'one', CONCAT('$."', i.`id`, '"'));
DELETE FROM `settings` WHERE `key` = 'item_respawn';
