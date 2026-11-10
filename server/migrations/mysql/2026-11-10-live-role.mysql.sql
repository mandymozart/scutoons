-- The `live` role (LIVE.md B.5.1, #90): who may save and run shows (`show:<id>` in entity_versions, the conductor's
-- session tokens). For all shows; creator and admin do not imply it. Granted in the admin's Users page like `creator`.
-- MariaDB 10.11, idempotent.
INSERT IGNORE INTO `roles` (`id`, `name`, `description`) VALUES
  (7, 'live', 'Builds and runs live shows: show documents, conductor session tokens');
