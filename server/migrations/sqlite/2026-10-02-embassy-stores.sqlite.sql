-- Embassy stores: the stockists of the public "Where to buy" page (same as the .mysql.sql of this date, agents/EMBASSY.md "Stores").
ALTER TABLE embassy_orgs ADD COLUMN listed INTEGER NOT NULL DEFAULT 0;
ALTER TABLE embassy_orgs ADD COLUMN independent INTEGER NOT NULL DEFAULT 0;
ALTER TABLE embassy_orgs ADD COLUMN address TEXT NOT NULL DEFAULT '';
ALTER TABLE embassy_orgs ADD COLUMN opening TEXT NOT NULL DEFAULT '';
ALTER TABLE embassy_orgs ADD COLUMN carries TEXT NOT NULL DEFAULT '[]';
