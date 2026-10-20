Status 2026-10-20: plan agreed (owner: Q2, Q3, Q7 answered; the other questions take the recommendation until the owner says otherwise); nothing built. Milestone "Top Toy workshop" (due 2027-02-17): #80–#91 + #28, #56, #62, #74, #75, #79. Building starts after chapter 3's release (#64, 2026-11-15).

# STORY-BIBLE — season / chapter scripts, character / persona / look / place texts (plan, 2026-10-20)

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
| a character | bible | `bible:character:<id>:<l>` | Backstory · Who · Look · Rules |
| a persona | bible | `bible:character:<id>:<persona>:<l>` | What · Why different · Look · Rules |
| a look | bible | `bible:character:<id>:<persona>:<look>:<l>` | Look · When |
| a map place | bible | `bible:place:<id>:<l>` | Backstory · Scenery · Rules |
| the toon (§9 Q5) | bible | `bible:toon:<toon>:<l>` | Premise · Tone · World rules |

- Season and chapter scripts may both exist: a tool that needs chapter n takes the **chapter script**, else chapter n's
  part of the season script (`# n. Kapitel`). Cat Pope stays season-written, Top Toy is chapter-written.
- **Publish = canon.** Generators, briefs and the MCP's answers use the published version; drafts are work in
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
  in `storyboards/style/`), printable, "Download" as PDF / zip for external artists (§9 Q6).
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
| Jan 4 – 22 | artist brief + references export; #61's generators on the texts | LIVE phases 3–4 (audience, stage builder, rehearsal) | publishes the texts as canon |
| Jan 25 – Feb 5 | fixes, #56 / #74 parts the owner keeps in | LIVE phase 5 (full run-through) | – |
| Feb 8 – 12 | **test week**: generators + briefs on the real texts | the stage experience on the real Top Toy chapter | freeze Feb 12 |

Issues (new): the text model + editor · season / chapter scripts + migration 1 · character / persona / look texts ·
place texts · generators read the texts (+ migration 2) · artist brief + references · MCP · generator test week ·
the stage experience (performance mode, LIVE.md – as one issue or one per phase).
Created 2026-10-20: #80 model + editor · #81 scripts + toon text · #82 character texts · #83 place texts · #84 MCP ·
#85 generators · #86 briefs + references · #87 test week · #88 owner writes · #89–#91 LIVE phases 0–1, 2–3, 4–5.
Moved in (owner: "add the character, location, scene, references to this milestone"): #56, #75, #74, #28, #62.

## 9. Owner questions (recommendation first)
Answered 2026-10-20: Q2 own text each · Q3 per language · Q7 the full LIVE Must list, the workshop instead of the
first show. Q8: no preference – the recommended five went in. Q1, Q4, Q5, Q6 (#86), Q9 follow the recommendation.

1. **Cat Pope's script = season 1's script?** Move it (migration 1) – yes.
2. **Personas and looks: their own text each** (rec.: they are edited on their own cards, the generator picks exactly
   the persona / look of a figure) – or sections inside the character's one text?
3. **Languages:** per language like the script, generators read en with the source as fallback (rec.) – or the
   bible texts in English only?
4. **In production** (creator only): yes (rec.; needed by the MCP, generators, artists) – the 2026-10-01 default goes.
5. **A toon text** (premise, tone, world rules – Top Toy's for the workshop): yes (rec.). A universe text (the art style
   for every generator) stays in `storyboards/style/` for now.
6. **External artists:** a brief page + PDF / zip download from the creator, sent by a member (rec.) – or a shareable
   link for people without an account?
7. **Performance mode in this milestone:** the stage experience needs it on Feb 17 – the full LIVE B.10 Must list
   from Nov 16 (tight: ~11 weeks as planned before, now beside lane 1) or a cut to what the workshop's stage needs
   (which: projection + conductor? audience phones? MIDI / DMX?). And: the workshop replaces the "first show Feb 1–14"?
8. **Existing issues into the milestone:** character #56 (cast 3D models), #75 (pronouns); location #74 (3D rooms),
   #28 (places ↔ story); scene #62 (studio stage 2; #55 stays in chapter 3); references – a new issue (§7). #61 stays in
   chapter 3 for the gap prompts, its test on the texts is a new issue here. Which of these?
9. **Dates:** code complete Feb 5, test week Feb 8–12, freeze Feb 12 – fine?
