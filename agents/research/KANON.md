Status 2026-10-20: plan agreed – every question answered by the owner (§9); nothing built. Milestone "Top Toy workshop" (due 2027-02-17): #80–#91 + #28, #56, #62, #74, #75. Building starts after chapter 3's release (#64, 2026-11-15).

# KANON — season / chapter scripts and the Kanon texts of characters, personas, looks, places (plan, 2026-10-20)

The word is **Kanon** (owner 2026-10-20: "i hate bibel as a work use KANON") – never "bible" in UI, code or docs.

## 1. What the owner asked
Owner 2026-10-20: "currently only toon has a script" · "character backstories in each character" · "a script for each
seasons, because that is how cat pope was written" · "chapter scripts, since this is how top toy will be written" ·
"locations have backstories and scenic description" · personas / looks: "some explanation what they are and why they
differ from the character" · "any generator or external artist will need these to create their art according to the
universe" · "all of this is always one large plain text element with some rules attached to one document".
Deadline: the **Top Toy workshop starts Wed 2027-02-17** (3 days) – "need no more code work" then; "the story should be
available by then and generators thoroughly tested"; the texts generate the Top Toy toon **and the stage experience**
(performance mode, research/LIVE.md).

## 2. What exists
- `script:<toon>:<l>` – one screenplay per toon (DECISIONS 2026-10-01), creator only, page `/<toon>/script` (textarea,
  outline, Format rules, meta, history; DESIGN "Creator: script page"). Only catpope has one: "Katzenpabst – Das
  Erwachen" (de complete, en partial) = in fact **season 1's** script. Production has it (version 90).
- Characters (`characters`): a short `description` on character and persona – reader-facing (cast page, VN). Looks: a
  label only. No backstory anywhere.
- Places (`map`): a name only. Regions have `description` + `vegetation`.
- **Generator knowledge lives in code:** persona descriptions in `scripts/storyboard/luma.js` (`PERSONA`), place
  descriptions in `shared/content/src/map.ts` (`PLACE_DESCRIPTIONS`, read by `placeImageBrief`). The storyboard studio
  reads `script:<toon>:de` (api.mjs); reference packs per shot (refpack.mjs, refs.mjs) attach character images.
- Wanted images (`asset-requests`) have a `brief` – derived for map places only.
- Top Toy today: toon, season 1, chapter 1, cast, credits – no script, no texts.

## 3. The model: one text per document
The owner's sentence is the model. A **text** = one large plain text, versioned (draft / publish / history like every
document), per language (the source language is the master, translations optional – as the script), **creator only**
(never served to readers: spoilers, working material), attached to **one document**, with a **kind** whose **rules**
say how it is written and what tools read from it.

| Text of | Kind | Entity (proposal) | Rules (§4) |
|---|---|---|---|
| a season | script | `script:<toon>:season:<n>:<l>` (replaces `script:<toon>:<l>`) | screenplay, chapters + scenes + songs |
| a chapter | script | `script:<toon>:chapter:<n>:<l>` | screenplay, scenes + songs (the chapter is the document) |
| a character | kanon | `kanon:character:<id>:<l>` | Backstory · Who · Look · Rules |
| a persona | kanon | `kanon:character:<id>:<persona>:<l>` | What · Why different · Look · Rules |
| a look | kanon | `kanon:character:<id>:<persona>:<look>:<l>` | Look · When |
| a map place | kanon | `kanon:place:<id>:<l>` | Backstory · Scenery · Rules |
| every toon (catpope, toptoy, yellowmouse) | kanon | `kanon:toon:<toon>:<l>` | Premise · Tone · World rules |

- Season and chapter scripts may both exist: a tool that needs chapter n takes the **chapter script**, else chapter n's
  part of the season script (`# n. Kapitel`). Cat Pope stays season-written, Top Toy is chapter-written.
- **Publish = Kanon.** Generators, briefs and the MCP's answers use the published version; drafts are work in
  progress (the member's Claude only saves drafts – RULES "Working with the owner").
- The short reader-facing `description` stays as it is; the text is the long, internal one behind it.
- Stored in production like every document (the only source of content); the 2026-10-01 default "scripts are not in
  production" goes (§9 Q4).

## 4. The rules per kind ("some rules attached")
Each kind = a writing guide (the editor's Format rules panel, the MCP's rules resource) + a parser (outline, checks,
what tools extract). Prose under `## ` headings; every section optional, the order fixed; unknown headings → a warning.
- **Script** – the screenplay editor's format, as today (`scriptOutline`); a chapter script has no chapter markers.
- **Character** – `## Backstory` (where from, what happened before the story) · `## Who` (personality, wants, fears,
  how they speak) · `## Look` (what an artist must always draw: body, colours, marks, what never changes – generators
  read this) · `## Rules` (do / don't: "no male / female", pronouns (#75), what never happens to them).
- **Persona** – `## What` (what this persona is) · `## Why different` (how and why it differs from the character, when
  and why it appears) · `## Look` · `## Rules`.
- **Look** – `## Look` (outfit, gear, what changes against the persona) · `## When` (when / why it is worn).
- **Place** – `## Backstory` (history, who lives there, what happened there) · `## Scenery` (what it looks like:
  architecture, materials, light, weather, season, sounds – generators, dioramas and the 3D rooms (#74) read this) ·
  `## Rules`.
- **Generators read English**: the `en` text, else the source language (the brief says so – the MCP's Claude can draft
  the translation).

## 5. The editor
- **One editor for every text**: the script page's editor generalised (textarea, outline from the kind's headings,
  Format rules, the language switch in the header, Save draft / Publish / History). The script page becomes one use of it.
- Opened where the document is edited, no new navigation: season page → "Script"; chapter page → "Script"; the
  toon sidebar's "Script" → the toon's scripts (seasons, chapters); character page → "Text" per character, per persona,
  per look; map builder's place inspector → "Text". A small marker shows which texts exist / are published.
- The toon's script page today moves to season 1 (§6); `/<toon>/script` stays as the list (creator URLs aren't printed).

## 6. Migrations (RULES "Stored format changes")
1. `script:<toon>:<l>` → `script:<toon>:season:1:<l>` with every version (catpope de / en); the old key form leaves
   the code (`EntityKey`, `parseEntityKey`, MCP, studio api.mjs), no alias.
2. The hard-coded `PERSONA` (luma.js) and `PLACE_DESCRIPTIONS` (map.ts) become **draft** texts (`## Look` /
   `## Scenery`, en) by a migration; the code then reads the texts and the constants go. The owner publishes them.

## 7. Who reads the texts
- **Generators** – the storyboard prompt (luma.js) and reference packs take each figure's character + persona + look
  `## Look`; the place diorama brief and the 3D rooms take `## Scenery`; the studio reads the chapter's script (§3).
  Whatever #61 decides (image vs video, Luma …) gets the same inputs.
- **Wanted images** – every request gets a `brief` built from the texts (today only places): what to draw + the
  references.
- **Artist brief + references** – per character / persona / look / place one page in the creator: the texts (Look /
  Scenery, Backstory, Rules) + the reference images (portraits per mood, A-pose, 3D model, place image, the style refs
  in `storyboards/style/`). External artists get a **shareable link** to it (read-only, no account; revocable, the
  creator lists the links it gave out); an artist who needs more gets a creator account and works inside the creator
  (§9 Q6).
- **The MCP** – reads and drafts every text (read_entity / save_draft know the kinds; the rules resource carries §4),
  so the member's own Claude can write backstories, translate and build briefs – also during the workshop.
- **Performance mode** – the chapter script is where performers' lines and cues come from (LIVE B.2); optional.

## 8. The milestone "Top Toy workshop" (due Wed 2027-02-17)
Code complete **Fri 2027-02-05**; test week Feb 8–12 (every generator on the real Top Toy texts); **deploy freeze Fri
2027-02-12**. Two lanes at most (RULES "Git"); holiday break Dec 21 – Jan 1 (as LIVE B.10). Chapters 4 (Dec 15) and
5 (Jan 15) release in between – the owner's art, little agent time.

| Weeks | Lane 1 – texts → generators → references | Lane 2 – performance mode | Owner |
|---|---|---|---|
| Nov 16 – Dec 4 | the text model, entities, validation, the editor everywhere, migration 1, the MCP | LIVE phase 0–1 (bus, spikes, relay, MIDI / DMX) | – |
| Dec 7 – 18 | migration 2, generators read the texts, Wanted-image briefs | LIVE phase 2 (protocol, outputs, hosted relay) | **writes**: Top Toy toon, characters, places, season / chapter scripts (until Feb 5) |
| Jan 4 – 22 | artist brief + references export; #61's generators on the texts | LIVE phases 3–4 (audience, stage builder, rehearsal) | publishes the texts (= Kanon) |
| Jan 25 – Feb 5 | fixes, #56 / #74 parts the owner keeps in | LIVE phase 5 (full run-through) | – |
| Feb 8 – 12 | **test week**: generators + briefs on the real texts | the stage experience on the real Top Toy chapter | freeze Feb 12 |

Issues (new): the text model + editor · season / chapter scripts + migration 1 · character / persona / look texts ·
place texts · generators read the texts (+ migration 2) · artist brief + references · MCP · generator test week ·
the stage experience (performance mode, LIVE.md – as one issue or one per phase).
Created 2026-10-20: #80 model + editor · #81 scripts + toon texts · #82 character texts · #83 place texts · #84 MCP ·
#85 generators · #86 briefs + references · #87 test week · #88 owner writes · #89–#91 LIVE phases 0–1, 2–3, 4–5.
Moved in (owner: "add the character, location, scene, references to this milestone"): #56, #75, #74, #28, #62.

## 9. Owner answers (2026-10-20)
1. Cat Pope's toon script becomes season 1's script (migration 1) – "yes".
2. Personas and looks: their own text each – "Own text each".
3. Languages: per language like the script; generators read en, the source language as fallback.
4. The texts live in production (creator only); the 2026-10-01 default "scripts are not in production" goes – "yes".
5. A toon text for **every toon** – "so does cat pope and yellow mouse, each toon". A universe text (the art style for
   every generator) stays in `storyboards/style/` for now.
6. External artists: a **shareable link**, not a PDF / zip – "shareable link, we can try to get them to work inside the
   creator if they need more info" (#86).
7. Performance mode: the full LIVE Must list from Nov 16; the workshop replaces the first show Feb 1–14.
8. The milestone takes #56, #75, #74, #28, #62 (no preference – the recommendation); #55 and #61 stay in chapter 3.
9. Dates: code complete Fri Feb 5, test week Feb 8–12, freeze Fri Feb 12 – "we push through".
