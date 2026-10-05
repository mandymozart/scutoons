# TREATMENTS — script scene → shot treatment, automated in the creator (concept, 2026-10-05)

Concept only, nothing built. Precursor: `workflows/scenes/generate-shots.md` (manual, FLORA). Pipeline context:
AUTOMATION.md ("Scene → shot list → stills"). Direction (owner): the platform stays the single source of truth; image
tools (FLORA, Seedream, GPT Image, Luma, Midjourney) come and go. So the product is a **treatment** the creator
exports, not an integration with one generator.

## What a treatment is

Per scene, one tool-independent document the author can hand to any generator, artist or Figma:

```
scene      toon, chapter, scene number, heading, script line range (de = master)
style      the toon's style block (one text, reused verbatim)
shots[]    id, type (establishing / long-scroll / medium / close-up …), aspect (intended), frame (what the reader
           sees), camera, characters [{ id, persona, look, mood }], location (map place), notes, status
           + assembled prompt, reference list (role → asset), wanted (what is missing)
```

Consistency is the hard part, so it is built in, not hoped for:

- **Canonical blocks, never re-described.** One character block per persona + look (from the registry, like
  `characterBaseViewsBrief`), one location block per place, one style block per toon. The prompt is **assembled by
  code** from them + the shot's action line. The AI never writes the character description twice.
- **References by role**, resolved from the asset folder convention (ASSETS.md): character base views / mood portraits,
  the place's background, and optionally the previous approved shot of the same scene.
- **The look is part of the shot** (KATZENPABST in robe vs. full cat form in a transformation scene).

## Pipeline (proposals, never silent changes — AUTOMATION.md)

1. **Extract (code, free).** `scriptOutline(text)` gives scenes and line ranges → scene text. Speaker names and
   all-caps mentions → character registry ids via names/aliases (KATZENPABST, KATZE, PLF → one character); unmatched
   names → "wanted". The scene heading → map place (MAP.md §8.3 already maps every screenplay location to a place).
2. **Propose shots (AI, via the Ask panel).** The agent reads the scene with the existing MCP tools (`read_entity` /
   `search_content` / `read_characters` / `read_map_places`) and returns structured shots (schema-validated), plus
   **interpretation notes** where the script is open ("VERSCHIEDENE ORTE" montage). Saved as a normal draft.
   Gate: the author edits / accepts the shot list (per scene).
3. **Resolve (code).** Assemble prompts, attach references, list what is missing into the existing **Wanted images**
   queue (asset-requests). Gate: every shot has its references, or the author waives one.
4. **Export (code).** Copy one shot, a scene, or the whole chapter as: Markdown treatment (prompt cards), JSON, and
   a reference list / zip. A **generator profile** only maps aspect and reference syntax (e.g. FLORA's default model
   tops out at 9:16, Midjourney takes `--ar`); the text blocks stay identical.
5. **Return (later).** Approved stills are uploaded through the asset manager (`<toon>/s<n>/c<n>/…`) and linked to
   the shot id; status per shot: proposed → approved → generated → still approved. Figma collage stays manual.

## Where it lives

Recommendation: new creator-only entity `treatment:<toon>:<scene>` (language-neutral `_`, prompts in English), versioned
draft → review → publish like `script` (DATA.md §2; `CREATOR_ONLY_KINDS`), never served to readers. The toon's style
block is a field on the toon entry. Alternative: stored inside the chapter group — rejected for now (treatments exist
before slices do, and are not reader content).

## Facts found while testing by hand (FLORA, Scene 1 of Cat Pope)

- The default FLORA model accepts only `auto, 21:9, 16:9, 5:4, 4:3, 3:2, 1:1, 2:3, 3:4, 4:5, 9:16`. The webtoon's
  `1:4` / `9:21` long-scroll panels are not available there. So a shot keeps its **intended** aspect and the profile
  decides: cap to 9:16, stitch two generations, or use a generator that supports it.
- The English script is incomplete (DATA.md): extraction always reads `de`, prompts are English.
- Scene 1 is a montage ("VERSCHIEDENE ORTE"): the heading gives no single location — the AI step must propose
  locations from the body text, and the author confirms them.

## Build order (smallest useful first)

1. **No AI:** scene extraction, character + place matching, wanted list, prompt assembly from hand-entered shots,
   copy / export. This alone replaces the manual copy-paste of briefs.
2. **AI proposals:** the Ask panel proposes the shot list into the draft; the same schema.
3. **Stills return:** upload, link by shot id, status; optionally a free consistency check against the references.

Test fixture: catpope de, Scene 1 (10 hand-made shots, see `workflows/scenes/generate-shots.md`) — the proposal can be
compared against it.

## Open decisions (owner)

- The **style block**: who writes it, and per toon or per season?
- Storage: the creator-only `treatment:` entity as above, or elsewhere?
- Should the Ask agent be allowed to `save_draft` treatments directly (reviewable like any draft), or only propose in
  chat?
- Prompts English only, or also a German version for artists?
- Export formats beyond Markdown / JSON (a Figma-friendly one)?
