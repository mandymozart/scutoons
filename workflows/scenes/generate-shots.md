# Generate Shots

Turns a webtoon script into an approved, character-consistent set of shot stills,
ready to hand off to Figma for collaging. Runs in FLORA (workspace
`ws_qd79ybqq6qkv36r3ycjpgw1aw98ejvek`, project `prj_ns7bwpxbsa2zs3154ve5gmrpb58ej4ez`).

Fits between two steps that stay outside FLORA:

```
GitHub script  ---->  [ this workflow ]  ---->  Figma collage
(cat-pope-awakening)                            (manual, outside FLORA)
```

**Script source of truth: `de/screenplay.txt`**, not `en/`. The English
translation under `en/screenplay.txt` is known incomplete — always read the
scene from German
([mandymozart/cat-pope-awakening](https://github.com/mandymozart/cat-pope-awakening),
private repo) and translate on the fly for the shot list. Character names in
German: KATZENPABST (= CAT POPE), KATZE (= CAT), PLF (= PLF, Pabst-ähnliche
Figur / Pope-like Figure) — keep the German names in mind when matching
character references, since asset filenames or notes in the project may use
them.

## The law

> **Never batch-generate a scene before its references are locked and its
> shot list is approved.** A wrong reference or a misread shot doesn't
> cost one bad image — it costs every shot in that scene that inherited it.

## The four stages

```
1  SCRIPT      -> shot list        free           gate: approve the list, per scene
2  SHOT LIST   -> refs locked      free/uploads    gate: every shot has its refs
3  REFS+SHOTS  -> stills           ~$0.004/still   gate: approve the SET, per scene
4  STILLS      -> handoff          -              deliver URLs, keyed by shot id
```

Work **one scene at a time**, not the whole script at once. That's the
checkpoint the user asked for: small enough batches to catch a wrong or
missing shot before spending on the next scene, and small enough that
re-shooting one shot doesn't mean re-approving forty.

---

## Stage 1 — script to shot list

Read the scene from the script (pasted in, or fetched from
[cat-pope-awakening](https://github.com/mandymozart/cat-pope-awakening) on
GitHub). Break it into discrete shots the way a storyboard artist would —
one shot per distinct camera setup or beat, not one shot per sentence.

For each shot, capture:

```
id          sNN-scene-shot, e.g. s03-01   stable across regeneration
scene       which script scene it's from
type        establishing / long-scroll / medium / close-up / reaction / ...
frame       what's in it — subject, action, setting, mood
characters  which named characters appear
location    which location it's set in
aspect      9:16 establishing, 9:21 or 1:4 long-scroll, etc. — vertical by default
```

Default aspect ratios (override per shot when the beat calls for it). The
format is a vertical scroll, so these are all portrait — there's no
landscape default:

- **Establishing shot** — `9:16`, sets the location before the scene's
  action starts.
- **Long-scroll panel** — `9:21` or `1:4`, the webtoon-scroll equivalent of
  a tracking or reveal shot; reach for `1:4` on the more extreme reveals.
- Everything else — pick the aspect that matches how it'll sit in the
  scroll; don't default silently, and don't default to landscape.

Show the **full shot list for the scene** and stop. Nothing gets generated
until the user has read it and can argue about shot 3 while it's still free.

---

## Stage 2 — lock references

Nothing in the image model carries character identity between calls —
identity has to be carried in, by a reference image passed into every
generation for that character.

1. **Characters.** Check the FLORA canvas/assets for an existing reference
   per named character in this scene's shot list (`flora_list_assets` /
   `flora_list_canvas_nodes`). If one exists, reuse it. If not, either the
   user supplies one, or the first shot that character appears in is
   generated alone, approved, and *that becomes their reference* for every
   later shot.
2. **Locations.** Same check. If no location reference exists for a
   location this scene needs, **ask the user for one** (a photo, a mood
   image, a prior establishing shot) before generating anything set there.
   Don't invent a location from the script text alone if the user has a
   real reference in mind — ask first.
3. Record the locked reference URL against every shot that needs it. A
   reference is locked before its batch fires, never swapped mid-batch —
   shots on either side of a swap will visibly disagree.

Gate: every shot in the scene has its character and location references
resolved (or the user has explicitly said to proceed without one) before
Stage 3 fires.

---

## Stage 3 — generate the stills

Fire the scene's shots as one batch, then poll once — not one call per shot.

- Reference images go in as `params.image_url`, **singular string**. The
  plural array form is silently ignored (and still billed) — the render
  comes back as a plain text-to-image with the reference nowhere in it.
- Use `Promise.allSettled`-style handling (or the hosted batch tool), not a
  form that lets one failed shot lose the run ids of the ones that fired.
- Poll `flora_list_generations` / `generations.list` with `limit: 100`
  explicit — the default page size silently truncates a long scene.
- Key completion on `status === "completed"` with outputs present, not on
  `progress: 100` alone — a failed run reports that too.

State the batch cost before firing (e.g. "7 shots ≈ $0.03, approve?").

### Review — every shot, not a sample

Once the batch settles, look at each still against its locked reference.
A free image-to-text judge model can do the first pass (`i2t-*`, 0
credits): pass `params.image_urls` as a **plural array** `[reference,
still]` — this is the opposite convention from generation — and ask it to
describe both before ruling on consistency, so it judges the pixels and not
just your prompt text.

Present the scene's full set — shot id, image, any drift the judge or you
flagged — and get **one approval on the whole scene**. Re-shoot only the
flagged shots and re-gate before moving to the next scene.

---

## Stage 4 — handoff

This workflow ends at approved stills. It does **not** lay them out,
collage them, or touch Figma — that step stays manual, on purpose, since
that's where the actual page composition judgment happens.

Deliver in shot order, keyed by id, so they drop into Figma without
cross-referencing anything:

```
SHOT       TYPE           IMAGE
s03-01     establishing   https://media.flora.ai/....png
s03-02     long-scroll    https://media.flora.ai/....png
s03-03     close-up       https://media.flora.ai/....png  (re-shot — v1 drifted on lighting)
```

Never reorder or silently drop a shot — a gap is a bug the collage step
will find at the worst time.

---

## Gotchas (carried over from FLORA's script-to-video pattern — same engine, same traps)

```
image_url        singular string, for generation inputs. Plural array = silently
                 ignored, still billed, reference nowhere in the result.
image_urls       plural array, for judging (i2t). Singular = judge sees nothing
                 and rubber-stamps everything.
describe first   force the judge to describe both images before it rules, or it
                 answers from your prompt text instead of the pixels.
same url twice   de-duplicates to one image — reference and still must differ.
limit: 100       explicit on every generations list call, or a long scene's tail
                 silently reports as still-running.
progress != done failed runs also report progress:100. Check status + outputs.
lock before fire never swap a reference mid-batch.
one gate per     stage — shot list (free), then reference lock, then the full
                 generated set. Never skip ahead on a partial look.
```
