-- The owner publishes physical catalog changes (owner 2026-10-12, CATALOG-IN-CREATOR D2): the review rule `physical`
-- for every toon (`*`) for tilman@porschuetz.de – nobody else. Without it nobody publishes a physical change after the
-- catalog moved into the creator (edition, product, physical items, shop – Domain\Content\Reviews).
-- Idempotent: inserted only when the account exists and has no such rule yet (he may have added it by hand in
-- Admin → Users → Reviews); a second run inserts nothing. Literals only (collation #1267).
-- Check: SELECT r.* FROM content_review_rules r JOIN users u ON u.id = r.user_id WHERE u.email = 'tilman@porschuetz.de';

INSERT INTO `content_review_rules` (`toon`, `user_id`, `rule`, `created_at`, `created_by`)
SELECT '*', u.`id`, 'physical', UTC_TIMESTAMP(), 'migration 2026-10-12-owner-physical-rule'
FROM `users` u
WHERE u.`email` = 'tilman@porschuetz.de'
  AND NOT EXISTS (SELECT 1 FROM `content_review_rules` r WHERE r.`user_id` = u.`id` AND r.`toon` = '*' AND r.`rule` = 'physical');
