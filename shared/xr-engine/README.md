# @scutoons/xr-engine

OSCT's XR engine and its content contract, for the reader's XR mode (#482): three.js, image tracking with the 8th Wall
engine, the experience's configuration (scenes, surfaces, entries, targets, entities), its guards and the plugin runtime.

**`osct/` is an unchanged copy. Never edit a file in it.** A change belongs in OSCT; here it arrives with the next copy.

| | |
|---|---|
| source | the OSCT repository (the owner's), branch `develop` |
| commit | `e7852303` (2026-11-23): app 6.3.2, content schema 6.0, plugin API 4.0 – OSCT 6.0 says listen instead of hear itself; no stored experience needed a migration (the test copy's poster was imported again at 6.0, #790) |
| copied | `engine/src` without its tests (58 files) and the files of `shared/` it imports, followed through (25 files) |
| copied for the stage | of OSCT's editor (`editor/src/lib`): the stage's twelve files (`stage/mock-tracker.ts`, `mock-stage.ts`, `preview.ts`, `placement.ts`, `surfaces.ts`, `origin.ts`, `snap.ts`, `shading.ts`, `sky.ts`, `plywood.ts`, `oak.ts`, `linoleum.ts`), `storage.ts`, `library-place.ts`, and what those reach only through types (`api.ts`, `publish-plugins.ts`, `plugins/compile.ts`, four files of `shared/api`); of `editor/static`: the desk, its two textures and one sky, with their READMEs (sources and licences) |
| tests | OSCT's own (`engine/src/**/__tests__`, vitest) stay there; `npm run check` here is the type check against SCUTOONS's three.js |
| copied for the logic | OSCT's logic package, `editor/src/lib/logic/` without its tests (12 files: projection, ops, layout, navigation, palette, signals …) and `behaviour-edit.ts`: the XR scope of the one node view (#763), not used by any page yet |
| three.js | SCUTOONS's one version (0.186.1); OSCT builds against 0.174. The copy type-checks unchanged, but for one file: `editor/…/stage/mock-stage.ts` reads a texture's `image`, which the types of 0.186 call `unknown`. It is left out of the check (`tsconfig.json` `exclude`) and declared by `src/osct-mock-stage.d.ts`; the code that runs is OSCT's |

Once OSCT publishes the engine and the contract as packages with a tag, this copy is replaced by that pin.

## Using it
- **An app imports `@scutoons/xr-engine/reader`** (`src/reader.ts`, SCUTOONS's own): `openXr(container, configuration,
  scene, options)` reads a configuration and builds one scene. Its types come from `src/reader-api.ts`, which imports
  none of OSCT's files: an app's own TypeScript settings (the viewer's NodeNext) refuse them, so they are only checked
  here. `reader.ts` must fit those declarations; `npm run check` says when it does not.
- **The editor's stage is `@scutoons/xr-engine/stage`** (`src/stage.ts`, types `src/stage-api.ts`): `openStage(container,
  configuration, plugins, scene, options)` plays a scene on OSCT's desk without a camera (its simulated tracker), with
  picking, the selection's outline, OSCT's gizmo and live entity overrides. It runs the experience's plugins in the app's
  own page (owner 2026-11-21: no sandbox – the member is trusted, as with the API). The app serves the files of
  `osct/editor/static` (`options.stageBase`). `src/load.ts` reads a configuration and loads the plugins' code for both entries.
- **The XR scope of the one network is `@scutoons/xr-engine/network`** (`src/network.ts`, types `src/network-api.ts`):
  `projectXr(document, context)` gives one level of an experience's logic as the spore network's weave (`@scutoons/api`
  `Spores`, #763): OSCT's projection (`osct/editor/src/lib/logic/project.ts`) translated – OSCT's signals into the deck's
  table, rule homes into JSON pointers into the experience document, stubs as far nodes. `npm run test` here runs it against
  `test/network-fixture.ts` and the contract's shared vectors (`test/write-xr-vector.ts` writes the `xr` view anew).
- The copied files import each other through `@shared/*`, the editor's also through `@engine/*` and `#lib/*`. An app
  that imports this package maps `@shared` to `shared/xr-engine/osct/shared` and, for the stage, `@engine` to
  `shared/xr-engine/osct/engine/src` (SvelteKit: `kit.alias`), as this package's `tsconfig.json` does; `#lib/*` is this
  package's own `imports` (`package.json`).
- `@scutoons/xr-engine/scene` is the scene (`ArScene`), `…/tracker` the 8th Wall image tracker (kept apart so only
  the page that scans loads it), `…/content` the configuration loader, `…/contract` the configuration's types.
- The tracker loads the 8th Wall engine's files (`xr.js`, `xr-tracking.js`, npm `@8thwall/engine` 0.1.0, MIT) from
  the app's own origin; the app serves them with their LICENSE.

## Taking a newer OSCT
Export `engine`, `shared` and `editor` at the new commit (`git archive <commit> engine shared editor/src/lib
editor/static`), copy `engine/src` without `__tests__`, the editor's stage files named above and every file they import
(followed through) into `osct/`, write the commit into the table above, run
`npm run check -w @scutoons/xr-engine`. A content schema MAJOR needs the stored experiences migrated first.
