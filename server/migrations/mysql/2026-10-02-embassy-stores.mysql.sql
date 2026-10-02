-- Embassy stores (agents/EMBASSY.md "Stores"): an organisation of type `store` or `record_shop` is a stockist – it
-- carries SCUTOONS products (ids of config/products.json) and, when `listed` and carrying at least one product, shows up
-- on the public "Where to buy" page (scutoons.com/stores, GET /api/stockists/) with its public fields only: name, type,
-- city, country, address, website, opening, independent, carries – never people, notes, leads or contact data.
-- Additive, MariaDB, idempotent (IF NOT EXISTS). Until it ran, the Embassy works as before and /api/stockists/ is empty.
ALTER TABLE `embassy_orgs` ADD COLUMN IF NOT EXISTS `listed` TINYINT(1) NOT NULL DEFAULT 0 COMMENT 'stores: 1 = on the public Where to buy page';
ALTER TABLE `embassy_orgs` ADD COLUMN IF NOT EXISTS `independent` TINYINT(1) NOT NULL DEFAULT 0 COMMENT 'stores: 1 = small / independent vendor (listed first)';
ALTER TABLE `embassy_orgs` ADD COLUMN IF NOT EXISTS `address` VARCHAR(255) NOT NULL DEFAULT '' COMMENT 'stores: street + postcode (public)';
ALTER TABLE `embassy_orgs` ADD COLUMN IF NOT EXISTS `opening` VARCHAR(255) NOT NULL DEFAULT '' COMMENT 'stores: opening info as written by the store (public)';
ALTER TABLE `embassy_orgs` ADD COLUMN IF NOT EXISTS `carries` VARCHAR(2048) NOT NULL DEFAULT '[]' COMMENT 'stores: JSON list of product ids (config/products.json)';
