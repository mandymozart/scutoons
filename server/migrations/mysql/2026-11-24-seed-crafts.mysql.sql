-- The crafts (#826): the owner's first list (workspace-thoughts.md point 18) with the creator workspaces each starts with
-- (ids of creator/src/lib/shell/address.ts WorkspaceKind). Edited later in the DB like other configuration; the creator reads
-- them from GET /api/crafts. Needs 2026-11-24-crafts. Dry run: 8 rows inserted on a fresh database, 0 when they exist.
-- MariaDB 10.11, idempotent.
INSERT IGNORE INTO `crafts` (`id`, `position`, `name`, `icon`, `workspaces`) VALUES ('show_runner', 10, '{"en":"Show runner","de":"Showrunning"}', 'monitor', '["toon","chapter","flow","scene"]');
INSERT IGNORE INTO `crafts` (`id`, `position`, `name`, `icon`, `workspaces`) VALUES ('illustrator', 20, '{"en":"Illustrator","de":"Illustration"}', 'pen-tool', '["character","storyboard","chapter"]');
INSERT IGNORE INTO `crafts` (`id`, `position`, `name`, `icon`, `workspaces`) VALUES ('video_editor', 30, '{"en":"Video editor","de":"Videoschnitt"}', 'film', '["storyboard","scene","sound"]');
INSERT IGNORE INTO `crafts` (`id`, `position`, `name`, `icon`, `workspaces`) VALUES ('writer', 40, '{"en":"Writer","de":"Schreiben"}', 'edit-3', '["text","chapter","storyboard"]');
INSERT IGNORE INTO `crafts` (`id`, `position`, `name`, `icon`, `workspaces`) VALUES ('translator', 50, '{"en":"Translator","de":"Übersetzung"}', 'globe', '["text","chapter"]');
INSERT IGNORE INTO `crafts` (`id`, `position`, `name`, `icon`, `workspaces`) VALUES ('animator', 60, '{"en":"Animator","de":"Animation"}', 'box', '["character","scene","storyboard"]');
INSERT IGNORE INTO `crafts` (`id`, `position`, `name`, `icon`, `workspaces`) VALUES ('sound_designer', 70, '{"en":"Sound designer","de":"Sounddesign"}', 'activity', '["sound","scene","character"]');
INSERT IGNORE INTO `crafts` (`id`, `position`, `name`, `icon`, `workspaces`) VALUES ('developer', 80, '{"en":"Developer","de":"Entwicklung"}', 'code', '["catalog","map","flow","scene"]');
