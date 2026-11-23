-- The storm cloud in production (#805; owner 2026-11-23: "we have to move it into the sandbox on our production server. so i
-- can access it. i want to be able to simply replace the asset in the library myself for the cloud low poly bubble"):
-- - the catalog rig `rig:item.cloud` (the cloud-storm rig, deck/native/rigs/cloud_storm.toml in deck-space serde's JSON form)
--   and its one instrument `cloud_storm`: the cloud (its puffs the model in its `puff` slot, scutoons/3d/cloud-puff.glb), the
--   thunder and kick samplers (scutoons/sounds/cloud-thunder.wav, cloud-kick.wav) and their mixer. The rows are
--   server/tests/fixtures/rigs/cloud-storm.json, which deck-native's tests/cloud_storm.rs keeps equal to the TOML;
-- - the published scene `scene:cloud-light` of the toon `sandbox`: the rig placed (a holder with `rig` only), held and tinted
--   storm-blue by its holder's values, its `outputs` the wash (Thunder Wash 600 RGBW, DMX 1) in the cloud's colour at every
--   repeat and the strobe (DMX 10) on the cloud's gate – server/tests/fixtures/scenes/cloud-light.json, which deck-space's
--   tests/cloud_light.rs resolves with this rig into the sandbox deck/native/scenes/sandbox/cloud-light.toml's graph and light
--   (what the build service's /validate asks on a publish). Its `version` is 1, as a first save stamps it.
-- The files the rows name are library assets, not rows: scutoons/3d/cloud-puff.glb (the low-poly bubble, `cargo run -p
-- deck-space --bin cloud-puff -- cloud-puff.glb`) and the two stand-in sounds (deck-native tests/stand_ins,
-- `SAMPLER_OUT_DIR=<dir>` on its tests) are uploaded under those paths once; one replaced in the library (a new version
-- under the same path) is what a deck loads from then on – a deck that downloaded the old file keeps its cached copy until
-- that cache is cleared (deck-native library.rs `file`: downloaded once). A file that is not there leaves the cloud its
-- placeholder puffs, a sampler silent.
-- The modules: module:cloud (2026-11-22-factory-cloud, its slot by 2026-11-24-update-cloud-puff-slot), module:sampler
-- (2026-11-22-factory-sampler) and module:mixer (2026-11-21-factory-audio-modules) – compiled by an admin (Compile).
-- What it touches: catalog_instruments, catalog_rigs: one row each, INSERT IGNORE – an existing `cloud_storm` / `item.cloud`
--   (a creator's own) is left as it is. entity_versions: `scene:cloud-light` as a published version, only while the key has
--   no version at all (a creator's own scene of that name stays as it is).
-- Dry run (before): SELECT id FROM catalog_rigs WHERE id = 'item.cloud'; SELECT id FROM catalog_instruments WHERE id =
--   'cloud_storm'; SELECT id, status FROM entity_versions WHERE entity_key = 'scene:cloud-light'; -> expected: no rows each.
-- Test (after): the same three -> one row each, the scene's `published`.
-- Plan's row counts on a database without them: 1 . 1 . 1. MariaDB 10.11, idempotent: a second run changes nothing.
-- Escaping: plain string literals (no NO_BACKSLASH_ESCAPES): `\\` is one backslash of the JSON, `\'` a quote; a row is one line.

INSERT IGNORE INTO `catalog_instruments` (`id`, `position`, `data`, `updated_at`, `updated_by`) VALUES
  ('cloud_storm', 6, '{"id":"cloud_storm","kind":"drum","name":{"de":"Gewitterwolke","en":"Storm cloud"},"node":[{"id":"cloud","kind":"module:cloud","assets":{"puff":"scutoons/3d/cloud-puff.glb"},"drift":0.6,"flash":0.8,"height":6.0,"light":45.0,"puffs":18.0,"rain":0.5,"rate":0.6,"size":6.0,"sync":1.0},{"id":"thunder","kind":"module:sampler","assets":{"sample":"scutoons/sounds/cloud-thunder.wav"},"mode":1.0,"pitch":1.0,"release":0.8,"velocity":50.0},{"id":"kick","kind":"module:sampler","assets":{"sample":"scutoons/sounds/cloud-kick.wav"},"pitch":1.0,"velocity":40.0,"voices":2.0},{"id":"mix","kind":"module:mixer","gain1":-3.0,"gain2":0.0}],"link":[{"from":"cloud.thunder","to":"thunder.notes","scale":1.0,"offset":0.0},{"from":"cloud.kick","to":"kick.notes","scale":1.0,"offset":0.0},{"from":"thunder.out","to":"mix.in1","scale":1.0,"offset":0.0},{"from":"kick.out","to":"mix.in2","scale":1.0,"offset":0.0}],"out":"mix.out","expose":[{"path":"cloud.hold","label":{"de":"Halten","en":"hold"}},{"path":"cloud.rate","label":{"de":"Tempo","en":"rate"}},{"path":"cloud.sync","label":{"de":"Takt","en":"sync"}},{"path":"cloud.rumble","label":{"de":"Donner","en":"thunder"}},{"path":"cloud.thump","label":{"de":"Kick","en":"kick"}},{"path":"cloud.flash","label":{"de":"Blitz","en":"flash"}},{"path":"cloud.rain","label":{"de":"Regen","en":"rain"}},{"path":"cloud.split","label":{"de":"Teilen","en":"split"}},{"path":"cloud.size","label":{"de":"Größe","en":"size"}}],"rights":{"lent":["play","tweak"],"owned":["play","tweak","derive","copy"],"core":["play","tweak","derive"]}}', UTC_TIMESTAMP(), 'migration:2026-11-24-cloud-light-sandbox');

INSERT IGNORE INTO `catalog_rigs` (`id`, `position`, `data`, `updated_at`, `updated_by`) VALUES
  ('item.cloud', 4, '{"subject":"item.cloud","track":[{"id":"storm","instrument":"cloud_storm","gain":1.0,"pan":0.0,"mute":false}],"rights":{"lent":["play","tweak"],"owned":["play","tweak","derive","copy"],"core":["play","tweak","derive"]},"sticks":[["storm:cloud.rate","storm:cloud.hold"],["storm:cloud.rain","storm:cloud.split"]]}', UTC_TIMESTAMP(), 'migration:2026-11-24-cloud-light-sandbox');

INSERT INTO `entity_versions` (`entity_key`, `kind`, `toon`, `locale`, `json`, `author`, `created_at`, `status`, `message`)
SELECT 'scene:cloud-light', 'scene', NULL, NULL, '{"format":2,"id":"cloud-light","toons":["sandbox"],"holder":[{"id":"cloud","rig":{"ref":"rig:item.cloud"},"values":{"storm:cloud.hold":1.0,"storm:cloud.hue":0.62,"storm:cloud.tint":60.0}}],"stage":{"name":"Cloud light","render_scale":1.0,"transition":"fade","transition_secs":0.4},"node":[{"id":"space","kind":"space","bloom":0.6,"glow":0.6,"glow_light":1.0,"glow_range":7.0,"hue":0.52,"join":"deck","light":0.85,"post":["bloom","curves","pixelate","fxaa"],"quality":"low","threshold":0.55}],"link":[],"outputs":{"dmx":"auto","fixture":[{"address":1,"id":"wash","profile":"thunderwash-600-rgbw"},{"address":10,"id":"strobe","profile":"strobe"}],"map":[{"from":"cloud_storm_cloud.red","gain":[{"from":"cloud_storm_cloud.pulse","in":[0.25,1]}],"to":[{"dmx":"wash.red"}]},{"from":"cloud_storm_cloud.green","gain":[{"from":"cloud_storm_cloud.pulse","in":[0.25,1]}],"to":[{"dmx":"wash.green"}]},{"from":"cloud_storm_cloud.blue","gain":[{"from":"cloud_storm_cloud.pulse","in":[0.25,1]}],"to":[{"dmx":"wash.blue"}]}],"strobe":[{"duration":30,"fixture":"strobe","trigger":"cloud_storm_cloud.gate"}]},"version":1}', NULL, UTC_TIMESTAMP(), 'published', 'scene (migration 2026-11-24-cloud-light-sandbox)'
  FROM DUAL
 WHERE NOT EXISTS (SELECT 1 FROM `entity_versions` e WHERE e.`entity_key` = 'scene:cloud-light');
