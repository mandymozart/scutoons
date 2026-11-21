Status 2026-11-21: concept for the owner (#773, epic #626) – nothing decided, nothing built. Pictures: characters.html (`?hook=closed|open|running|landed`).

# AI canvas hooks – where external generative tools plug into the creator

Owner 2026-11-21 (#773), thinking aloud: *"a good point to connect to some external gen ai tools later … we actually will try to
figure out a concept of where we can add these ai canvas hooks"*. To be presented at the Screen Institute "AI for filmmakers"
conference around 2026-11-30.

**In one sentence:** a hook is a small, visible socket on the canvas, at the thing it works on. It says what goes out, which
kind of tool does the work, and what comes back. The result always lands as a **candidate** next to its source, and only a
person's Accept puts it into the Library.

## 1. One shape

Every hook in the creator has the same four parts and the same four states. Only the words change.

| Part | What it is | Example (the character sheet) |
|---|---|---|
| **Out** | what leaves SCUTOONS, ticked in the hook | the front A-pose, the side view, the Kanon "Look" text |
| **Tool** | a neutral tool category, then one configured tool | image → 3D: local GPU (e.g. TripoSR), a paid service (e.g. Tripo), or by hand |
| **Asks** | the prompt / settings it sends, pre-filled from the document | "Low poly, A-pose, faces +z, feet on the floor, metres." |
| **Back** | what returns and where it lands | a GLB → a candidate beside the sheet, `from:` the front A-pose |

The states, each drawn in the draft:
1. **Closed** – a pill with a cyan socket: `image → 3D → model`. Quiet, always in the same place on its object. It isn't a
   button bar or a side panel (RULES H10).
2. **Open** – a card at the socket: Sends (chips), Tool (three rows: local · service · by hand), Asks (the text field),
   Comes back. One primary: **Send**. "Copy inputs" sits next to it for tools without an API.
3. **Running** – the pill becomes progress: tool · where it runs · time · cancel. You can keep working. The job runs on the
   server or on the member's machine, never in the tab.
4. **Landed** – the pill counts candidates. The candidate sits next to its source with a dashed line back to it, plus
   **Accept into the Library** · Again · Discard.

The shape is the same on every surface. Learn it once on a character, and you can read it on a storyboard shot, a place or a
sound.

## 2. Where hooks sit

| Area | Hook (out → tool → back) | Sits on | Lands as |
|---|---|---|---|
| **Characters – sheet** | image → 3D → model | the front A-pose | a GLB candidate beside the sheet |
| | image → variants → images | the sheet's title | new looks in a row under the sheet |
| **Characters – expressions** | text → expression sheet → images | the empty mood squares | images in those squares (`<character>_<view>-<mood>`) |
| **3D place** | model → rig / retarget → rigged model | the model on the stage | a candidate version (v4 next to v3) |
| | video → motion → clip | the clip strip | a clip candidate on the strip |
| | model → lighter model (retopology / fewer bones) | the "too many bones for the deck" words | a candidate version |
| **Storyboard** (exists: GENERATORS.md) | render → image; image → video; take → restyled video | the shot | the shot's candidates (`kind`, `take`, `from`) |
| **Places / sets** | image or text → 3D set / backdrop | the empty scene stage | a set candidate in the place's Library folder |
| **Items** | image → icon / cut-out | the item's slot | an image candidate in the slot |
| **Sound** | text → voice line; voice → mouth shapes; text → music stem | the line, the rig's face, the track | an audio candidate on the line / track |
| **Modules** (#419) | images → 3D object with anchors | the module's object slot | a GLB candidate the anchor check reads |

A rule for placing them: a hook sits where a person would notice something is missing, such as an empty slot, a "no face
rig" note or a mood without a picture. It never sits in a separate "AI" page or menu.

## 3. How results come back

- **The candidate is the generation record we already have.** The storyboard's ledger (GENERATORS.md §3,
  STUDIO-IN-CREATOR.md §2.2) keeps one row per generation with `kind`, `take`, the parents, the generator, the model, cost and
  status. Approval records `from`. Hooks keep that record and widen what it belongs to: today a **shot**, then **any
  document** (character, persona / look, model asset, place, item, line). `kind` grows from image | video to image | video |
  model | clip | audio.
- **Accept** puts the file into the Library as a **new version of what it came from**: the v4 rig stacks on v3, and a mood
  picture fills its slot. Nothing is overwritten. The previous version stays in the stack (the Library's version stacks,
  #653).
- **Provenance travels with it:** `from` (the source file or version), the tool category, the tool and model, the take, the
  prompt sent, the date and who pressed Send. The Library and History show it. A public credit line can come from it later.
- **Discard** removes the candidate from view, and History keeps it. **Again** reopens the hook with the same inputs.
- **Checks still apply.** A model candidate goes through the same checks as an upload: joint names (figures.js), the bone
  budget for the deck and spots (#724 / #725). A candidate can fail a check and still be shown. It just can't be accepted
  while it has an error (RULES H7).

## 4. What stays ours, what is the tool's

| Ours (SCUTOONS) | The external tool's |
|---|---|
| The hook's place and shape, and what it may send (ticked by the person) | The model, the GPU, the generation itself |
| The candidate, Accept, the version stack, provenance, History | Its own account, pricing, terms and licence |
| The checks (joints, bones, spots, file names, sizes) | Its output quality |
| The generator registry, keys on the server, the budget gate (STUDIO-IN-CREATOR §2.5) | Its API, which can change: one adapter per API, not per model |
| The "by hand" path: copy the inputs out, drop the file back, recorded the same way | – |

- **No assistant in the app.** Hooks are plugs for tools. The member's own Claude keeps working through the MCP (RULES G2): it
  can open the same hooks (`list_generators`, `start_generation`, `record_generation` exist for the storyboard) and saves
  drafts, while the member accepts in the browser.
- **The person decides what leaves.** Each hook shows its outputs as chips. Nothing is sent that isn't ticked, and a Kanon text
  goes out only when ticked.
- **Own GPU first.** A local option (the member's machine, through the MCP, as `generate_local` does today) is listed first
  where one exists. Paid services sit behind the budget gate.
- **Licences:** note the model's licence on the record, don't block on it (RULES G3).

## 5. Open questions for the owner

1. **Does a hook fit "AI = the member's own Claude, no in-app AI" (G2)?** My reading is yes: a hook calls a tool, not an
   assistant, and the storyboard generators set the precedent. *Recommendation:* record it as a decision, so later sessions
   don't read G2 as "no generators in the creator".
2. **Which three hooks first?** *Recommendation:* image → 3D on the sheet, text → expression sheet on the empty moods, and
   model → rig / retarget on the 3D place. Together they make the character story for the conference.
3. **Where do keys live: per member, or the studio's (the server registry today)?** *Recommendation:* the server for now,
   with a member key as an option later.
4. **Does Accept always make a new version, or can a candidate replace in place?** *Recommendation:* always a version, since
   it's cheap and keeps provenance.
5. **Should the conference version show real tools or "by hand" only?** *Recommendation:* run one live local tool and show the
   rest as by hand. It's honest about what's built, and it shows the shape.
6. **A public credit for generated parts:** show it in the reader, or only in the creator? *Recommendation:* only in the
   creator for now, and decide before the first generated asset is published.
