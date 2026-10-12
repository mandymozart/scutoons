Status 2026-10-12: vision, not built. Its "AI assistant in the creator" is superseded: AI = the member's own Claude through the SCUTOONS MCP (agents/MCP.md); the Ask panel is gone.

# AUTOMATION — from story to finished webtoon (vision, owner 2026-09-30)

Not built yet. This is the direction; it grows progressively as we have more data and refine stories.
Manual/semi-automated precursors live in `workflows/` (one folder per domain).

## The pipeline (progressive, refined per iteration)

```
story ──► script ──► scenes ──► frames / slices ──► VN events, text boxes, audio ──► chapter
            │           │            │
            │           │            └─► missing assets:   backgrounds, props, items
            │           └─► missing characters / personas / looks / moods  (workflows/characters)
            └─► characters named in the script ↔ character registry
```

- Each stage produces **proposals**, never silent changes: the author accepts, edits or rejects them in the creator.
- It is **progressive**: a rough story already yields a scene list and a "wanted" list; every refinement (better
  script, approved frames) sharpens the next stage.
- **Artist content always wins.** Anything generated can be replaced by an uploaded artist version (same slot / name),
  and refined artist uploads are never overwritten by generation.
- Everything ends in the same place: data through the API (database), media on the CDN by convention — the same
  entities the creator edits by hand.

## Pieces

| Piece | What | Depends on |
|---|---|---|
| Character base views | `workflows/characters/base-views.md` → creator action "Complete base views" | character registry, asset upload (CDN refactor) |
| Moods | portrait variations per mood → "Complete moods" | base views |
| Scenes / shots | `workflows/scenes/generate-shots.md` (FLORA) → creator "Scene → shot list → stills"; concept: [TREATMENTS.md](TREATMENTS.md) | script import |
| Script import | story/screenplay (e.g. `de/screenplay.txt`) → scenes, characters, dialogue lines | data refactor (entities in DB) |
| Wanted list | one queue of everything missing (images, looks, moods, translations) — today "Wanted images" | asset requests |
| Asset upload | upload + normalize + WebP in the creator, replacing the old manual FTP upload (repo `cdn/` removed 2026-10-04) | CDN plan (P6) |

### Creator roadmap (recorded 2026-09-30, not built)

Future creator actions from `workflows/`. Nothing is implemented. The disabled "coming soon" Actions buttons were
removed (owner 2026-10-04); meanwhile the character page ends with **Prompts** (`character-prompts.svelte`): the
base-views and missing-moods briefs, filled in for the persona + look shown, to copy for an image generator or an artist.

| Action | Where in the creator | From | Needs first |
|---|---|---|---|
| **Complete base views** | character detail → Actions | `workflows/characters/base-views.md` (any 1 of 4 views → portrait, avatar, full, A-pose; normalized, named, registered) | asset upload; the character registry through the API (data refactor) |
| **Complete moods** | character detail → Actions (per look) | moods workflow (`moods.md`, to come): portrait variations per mood of the standard set | base views; asset upload |
| **Scene → shot list → stills** | chapter editor, group (scene) level | `workflows/scenes/generate-shots.md` (FLORA) | script import; scenes / characters as entities (data refactor) |
| **Upload images** | character detail → Actions; later every image field | replaces the old FTP upload + the manual naming step (normalize, WebP, avatar) | CDN plan (P6) + an upload API |

Dependencies in order: data refactor (entities in the DB, the API instead of files) → asset upload (API/CDN plan)
→ base views → moods; script import → shot lists. Each action writes normal drafts (review → publish) like manual edits.

## AI assistant in the creator

- Helps the author **reorganize content** (move slices between groups, rename/merge characters, fill translations,
  restructure scenes, bulk edits) — by **calling the API**, inside the creator's scope and permissions (role
  `creator`), with the same draft → review → publish flow as manual edits. Every change is a normal, reviewable draft
  version with history/restore.
- It does **not** touch code, the repo, the build or the server configuration.

## Separation: dev vs exploratory work

- **Dev work** (code, schema, API, infrastructure) happens in the repo with agents, tests and the rules in PLAN.md.
- **Exploratory / creative work** (stories, scripts, generation, content reorganization) happens in the creator
  against the API (sandbox toon for experiments), never in code.
- Workflows in `workflows/` are the bridge: documented, repeatable, and step by step turned into creator actions.
