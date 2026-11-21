# @scutoons/xr-engine

OSCT's XR engine and its content contract, for the reader's XR mode (#482): three.js, image tracking with the 8th Wall
engine, the experience's configuration (scenes, surfaces, entries, targets, entities), its guards and the plugin runtime.

**`osct/` is an unchanged copy. Never edit a file in it.** A change belongs in OSCT; here it arrives with the next copy.

| | |
|---|---|
| source | the OSCT repository (the owner's), branch `develop` |
| commit | `8525607b` (2026-11-21): app 4.1.0, content schema 5.1, plugin API 3.1 |
| copied | `engine/src` without its tests (58 files) and the files of `shared/` it imports, followed through (25 files) |
| tests | OSCT's own (`engine/src/**/__tests__`, vitest) stay there; `npm run check` here is the type check against SCUTOONS's three.js |
| three.js | SCUTOONS's one version (0.186.1); OSCT builds against 0.174. The copy type-checks unchanged |

Once OSCT publishes the engine and the contract as packages with a tag, this copy is replaced by that pin.

## Using it
- **An app imports `@scutoons/xr-engine/reader`** (`src/reader.ts`, SCUTOONS's own): `openXr(container, configuration,
  scene, options)` reads a configuration and builds one scene. Its types come from `src/reader-api.ts`, which imports
  none of OSCT's files: an app's own TypeScript settings (the viewer's NodeNext) refuse them, so they are only checked
  here. `reader.ts` must fit those declarations; `npm run check` says when it does not.
- The copied files import each other through `@shared/*`. An app that imports this package maps `@shared` to
  `shared/xr-engine/osct/shared` (SvelteKit: `kit.alias`), as this package's `tsconfig.json` does.
- `@scutoons/xr-engine/scene` is the scene (`ArScene`), `…/tracker` the 8th Wall image tracker (kept apart so only
  the page that scans loads it), `…/content` the configuration loader, `…/contract` the configuration's types.
- The tracker loads the 8th Wall engine's files (`xr.js`, `xr-tracking.js`, npm `@8thwall/engine` 0.1.0, MIT) from
  the app's own origin; the app serves them with their LICENSE.

## Taking a newer OSCT
Export `engine` and `shared` at the new commit (`git archive <commit> engine shared`), copy `engine/src` without
`__tests__` and every `shared/` file it imports into `osct/`, write the commit into the table above, run
`npm run check -w @scutoons/xr-engine`. A content schema MAJOR needs the stored experiences migrated first.
