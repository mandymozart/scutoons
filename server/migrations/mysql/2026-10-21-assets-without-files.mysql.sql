-- Asset entries without a file go (issue 152; owner 2026-10-21: "die entries muessen einfach weg"). A rescan
-- (AssetImporter) drops the renditions whose file is gone from the storage but keeps the asset and its versions – an
-- active asset with no rendition left in ANY version is an empty entry: a broken preview in the admin and the picker,
-- nothing to download, nothing to delete on the storage. An asset with files only in an archived version (a rename /
-- replace) is not touched, nor are `deleted` assets (their row is the delete's history).
-- The uses (asset_usages) are not touched: they are an index of the content, keyed by path, rebuilt by every rescan /
-- save; a content path with no file stays listed under "missing" until its document changes (the issue's comment lists
-- them for the owner). Children first, one table per statement, so the migration backup keeps every removed row (the
-- FKs would cascade, unbacked). Idempotent: a second run finds no empty entry and changes 0 rows. No user variables.

DELETE FROM `asset_jobs`
WHERE `asset_id` IN (
  SELECT `a`.`id` FROM `assets` `a`
  WHERE `a`.`status` = 'active'
    AND NOT EXISTS (SELECT 1 FROM `asset_renditions` `r` WHERE `r`.`asset_id` = `a`.`id`)
);

DELETE FROM `asset_shapes`
WHERE `asset_id` IN (
  SELECT `a`.`id` FROM `assets` `a`
  WHERE `a`.`status` = 'active'
    AND NOT EXISTS (SELECT 1 FROM `asset_renditions` `r` WHERE `r`.`asset_id` = `a`.`id`)
);

DELETE FROM `asset_versions`
WHERE `asset_id` IN (
  SELECT `a`.`id` FROM `assets` `a`
  WHERE `a`.`status` = 'active'
    AND NOT EXISTS (SELECT 1 FROM `asset_renditions` `r` WHERE `r`.`asset_id` = `a`.`id`)
);

DELETE FROM `assets`
WHERE `status` = 'active'
  AND NOT EXISTS (SELECT 1 FROM `asset_renditions` `r` WHERE `r`.`asset_id` = `assets`.`id`);
