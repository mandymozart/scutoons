Status 2026-10-19: concept – owner to choose (§6 questions; nothing built). Issue #61.

# GENERATORS – image vs video generation in the storyboard studio (concept, 2026-10-19)

Owner 2026-10-19: "luma can still be used through the api. it is one of the generators. we need to differentiate between
image generation and video generation. devise a concept." Builds on: research/STORYBOARD.md §3 / §7 "Budget" / §9
"Generators as built", the studio README "Generators", research/HANDHELD-CAMERA.md (the passes), research/STUDIO-LAYOUT.md
(stage 1), research/VARIANTS-CANVAS.md (the board, the lineage record – drafted in the studio-layout session, not on
main yet), research/TEXT-ACROSS-SLICES.md (scene · shot · slice: a slice's `shot` reference, being planned there).

## 0. What exists (2026-10-19)
- Registry `workflows/storyboard-studio/generators/` (jobs.mjs): `{ id, label, kinds, paid, models[{ id, kind, mode,
  inputs[], durations, ratios, price }], status, estimate, run }`; async jobs, cancel, a confirm ≥ the estimate for paid
  jobs, never a retry, a spend counter per studio session. Results: `_out/<ch>/generated/<shot>--g<n>.<ext>` +
  `generated.jsonl`; "Approve as chosen" (approve.mjs) keeps `chosen.png` AND `chosen.mp4` per shot.
- Generators: **comfy-local** (image: comfy.mjs, Qwen-Image-Edit, free), **runway** (video: gen4_turbo / gen4.5 image →
  video, aleph2 video → video), **comfyui-remote** (stub), **fake**. In the studio-layout session (uncommitted):
  **luma-manual** (image + video "by hand": copy the built prompt + render, drop the result – ManualGenerate.svelte).
- Outside the registry: comfy.mjs `--engine luma` (uni-1 text → image with `image_ref`, the Agents API, key
  `LUMA_AGENT(S)_API_KEY` already in `.env`), scripts/storyboard/gemini.mjs (Gemini image, `GEMINI_API_KEY`).
- Model entries already carry a `kind` – but the UI mixes them in one list and `inputs[]` can't say "≤ 3 refs" or
  "10 s not with a start frame".

## 1. Two kinds

| | IMAGE generation | VIDEO generation |
|---|---|---|
| Makes | a **still per shot** (the shot's aspect, also tall 1:4 – 1:7 via `gen.plan` segments) | a **clip per shot** (viewport shots, loops, 5 – 10 s; 9:16 / 16:9 …) |
| Main input | the layout render without texts (clean) or a handheld frame – as the **source** (edit) or as a **reference** | a **still** (first / last frame, keyframes) and / or a **control video** (a handheld take's pass: grey · beauty · depth · normals) |
| + | character refs (the figures' look images), style refs (storyboards/style/), the shot's prompt | the same refs where the model takes them; the prompt (motion, light); duration, loop |
| Out | n picture candidates (png / webp) | n clip candidates (mp4) + a poster (the first frame) |
| Cost unit | per image (+ per ref) | per second (× resolution), minimums |
| Belongs to | the **shot** (take = the handheld frame's take, if any) | the **shot + take** (the control video's take; none for pure i2v) |

**Typical chains** (cheap first, video only from an approved picture):
1. **Image first**: render → image candidates (comfy-local free / uni-1 ≈ 0.05 USD) → approve = the shot's picture.
2. **Image → video**: the approved picture as the start frame (+ end frame where it exists) + a motion prompt →
   gen4_turbo (0.25 USD / 5 s) or ray-3.2 720p (0.30 / 5 s) → approve = the shot's clip.
3. **Handheld → video**: a take's pass as the control video + the approved picture as the keyframe at 0 → aleph2
   (1.40 / 5 s) or Luma video_edit (`depth` / `normals` / `pose` controls, 1.08 / 5 s at 720p). The camera move is the
   phone's; the look is the picture's.
4. **Improve**: any candidate back in as source / start frame / keyframe (VARIANTS-CANVAS §3.6); a video's first /
   last frame → a still. A last frame → i2v makes a new clip from where the old one ends (Luma: a real `extend`).

## 2. The interface: one registry, split by kind, capabilities per model
- **One registry**, but every **model** has exactly one `kind` and a `caps` object (replacing `inputs[]`); a generator
  that does both (luma, runway, comfyui-remote) simply lists models of both kinds. `GET /api/generators?kind=image|video`
  returns only fitting models; `transport: 'api' | 'local' | 'manual'` (manual = luma-manual: never `run`, the panel
  shows Copy prompt; its result is a dropped file recorded as that generator's candidate).
- `caps` (all optional; the UI shows a field only if its cap exists, pre-fills it from the shot, greys what's over a max):
  ```
  source:     'edit' | null             // the still IS edited (Qwen-Edit, uni-1 image_edit, Gemini)
  refs:       { max, roles?: ['character','style','layout'], tagged? }   // gen4_image: 3, tagged @name; uni-1: 9 (edit 7)
  startFrame, endFrame: boolean;  keyframes: { max, at: 'index' | 'fraction' }   // ray-3.2 edit: 64; aleph2: 5
  control:    { passes: ['grey','beauty','depth','normals'], maxSeconds, signals?: ['depth','normals','pose','trajectory'] }
  durations:  number[] | { min, max };  ratios: string[];  resolutions?: string[];  loop?: boolean;  seed?: boolean
  rules:      [{ when: { endFrame: true }, durations: [5] }]   // the model's exclusions, checked by the server too
  price:      { perImage?, perRef?, perSecond?: { [res]: usd }, min?, gpuSecondsEstimate? }  // estimate() computes from it
  ```
- `estimate(job)` stays per generator but reads `price`; the server re-checks `caps` + `rules` before `run` (a 422 at
  the provider = money-free, but a confusing error).
- **Jobs**: unchanged (queued → running → succeeded / failed / cancelled), plus `kind` and `take` on the job.
- **Cost gate** (exists): the estimate in the panel, the confirm dialog with the cost, `confirm ≥ estimate` on the server.
- **Spend counter** (exists, per session, in memory) → becomes a **ledger** (below) so it survives restarts.
- **Budget gate – propose (STORYBOARD phase 3a, not built), in two steps:**
  - **3a-local (now, ~0.5 day)**: `storyboards/_out/ledger.jsonl` (one line per paid job: date, who = studio / agent,
    toon, ch, shot, take, generator, model, estimate, cost, status) + `STUDIO_BUDGET_USD` (and an optional
    `STUDIO_BUDGET_MONTH`) in `.env`. Before a job: `spent (this month) + running estimates + estimate ≤ budget`, else
    `402 budget_exhausted { spent, budget }` and one notice ("Budget used up: 48.20 of 50 USD – raise STUDIO_BUDGET_USD").
    A running job finishes. Every paid script (comfy.mjs `--engine luma`, gemini.mjs, map-paint) books into the same
    file through one helper `scripts/lib/ledger.mjs`.
  - **3a-platform (later, before unattended / multi-person runs)**: STORYBOARD §7 as written – `generation_budget` in
    the admin settings, a `generation_ledger` table, `POST /api/generation/reserve` → `settle`; the studio and the
    cloud worker call it with a developer token, the local ledger is imported once. Keys stay where the job runs.

## 3. Where results go – the shot owns its candidates
- **Candidates belong to a SHOT** (and, for control-video clips, a **take**): the generated.jsonl v2 record
  (VARIANTS-CANVAS §3.7) + two fields: `kind: 'image' | 'video'`, `take: '<take id>' | null`; the ledger line points to
  it (`ledger: '<id>'`). Files stay `_out/<ch>/generated/<shot>--g<n>.<ext>`.
- **Approve an image** → the shot's **picture**: approve.mjs copies it to `dataset/files/<ch>/<shot>/chosen.<ext>` and
  records `from: '<shot>--g<n>.<ext>'`; the manifest's `files.chosen` splits into
  `chosen: { image: { file, from, date }, video: { file, from, date, take } }` (one per kind; the previous one stays a
  candidate on the board).
- **Into the creator via the shot reference**: a chapter slice carries `shot: 'sb-3-1-2'` (TEXT-ACROSS-SLICES – the
  slices-rework session defines it; a tall shot → n consecutive slices with the same `shot`). "Send to chapter"
  (Export step, per shot or for all approved): the picture → `upload_asset` (MCP, the asset manager, named by shot id),
  then a **draft** of the chapter where every slice with that `shot` gets the new image (cut into the n tiles for a tall
  shot; width / height per slice); the creator sees the draft and publishes (RULES: drafts + link). Slices without a
  `shot` are never touched.
- **Approve a video** → the shot's **clip**: (a) the studio timeline's Generated mode plays it (exists: chosen video,
  else the newest); (b) for a viewport / loop shot "Send to chapter" sets the slice's `video` (`SliceVideo`, sources per
  quality level, the shot's picture as the poster = the slice `image`); a clip for a non-video shot stays in the
  storyboard (an animatic, a trailer, the live mode later).
- **Storyboard shot JSON** (creator-only) gets one optional field: `gen.prefer: { image: 'luma:uni-1',
  video: 'runway:gen4_turbo' }` (+ `gen.fallback`, STORYBOARD §3 "parallel, not a cut-over"); the Generate panel
  pre-selects it. Nothing new goes into the reader's chapter except what slices already have (`image`, `video`, `shot`).

## 4. The UI – the Generate step (stage-1 layout)
- **Generate panel** (dock), top to bottom:
  1. **Image | Video** – a square `sc-seg` (key **G** toggles). Default: Image while the shot has no chosen picture,
     Video after. The switch is the panel's only mode; it filters everything below.
  2. **Generator · Model** – two selects, only models of the kind; "not configured" ones listed dimmed with their
     one-line how-to; the shot's `gen.prefer` pre-selected. Manual (Luma by hand) is one entry here, not a panel.
  3. **Inputs from caps**, pre-filled from the shot: Image – Source (the clean render · saved frame · a candidate) or
     References (character chips per figure + style chips, "3 of 3" against `refs.max`); Video – Start frame (the chosen
     picture), End frame, Keyframes, Control (Take · Pass, from the newest take), Duration (only the allowed values),
     Ratio (the nearest to the shot's aspect, "letterboxed" when not exact), Resolution, Loop. Prompt last (the shot's
     image prompt for Image; the action + move for Video), editable, kept with the result.
  4. **Cost line** before sending: "≈ 0.30 USD · this month 12.40 of 50 USD" (red + Generate disabled over the budget);
     free models say "free – own GPU". Then **Generate** (the one primary, last); paid → `askDialog` with the cost.
- **Results** land on the **Variants board** (VARIANTS-CANVAS: the main view's editor, Generate opens it): image and
  video cards in the same rows (lineage decides the place, not the kind); a kind chip on video cards (length), a ✓ per
  kind for the chosen picture / clip; "Improve on this" fills the panel and flips the switch to the card's target kind
  (an image card → Video offers "use as start frame"). Running jobs = placeholders on the board.
- **Bottom timeline** (Shot · Take · Rendered · Generated): Generated plays the chosen clip, else the chosen picture as a
  still for the shot's reading time, else the newest candidate (audition: the current card while the board is open –
  VARIANTS-CANVAS Q3). Take mode shows which take a clip came from (the clip's `take`).
- Phone (handheld) unchanged: it only records takes; generating stays on the PC.

## 5. Providers per kind (prices read 2026-10-19; 1 Runway credit = 0.01 USD)

**IMAGE**

| Generator · model | Price | Can | Can't |
|---|---|---|---|
| **comfy-local** · Qwen-Image-Edit 2511 | free (~10 min a shot, RTX 2070) | edit the render + 2–3 character / style images, any ratio, own LoRAs | fast; > 3 refs |
| **luma** · uni-1 / uni-1-max (Agents API) | 0.040 / 0.100 per image, + 0.003 per ref; edit 0.043 | `image` with ≤ 9 refs (roles named in the prompt), `image_edit` = the render as source + ≤ 7 refs, 2K out; ratios 3:1 … 1:3; `style: manga` (portrait only) | tall beyond 1:3 (→ segments); tagged refs; a seed |
| **runway** · gen4_image / turbo | 5 – 8 / 2 credits (0.05 – 0.08 / 0.02) | ≤ 3 **tagged** refs (`@catpope`) – a new picture after them | an edit of the render (it's a ref, not a source); tall ratios |
| runway · gemini_image3.1_flash, nano_banana_2.1, gpt_image_2_5, seedream5 | 4 – 76 credits | edits with refs (the closed models FLORA had), one key | – (per model) |
| comfyui-remote · Qwen / FLUX.2 on RunPod | GPU seconds: 4090 1.10, 5090 1.58 USD / h (≈ 0.03 – 0.05 a shot) | everything comfy-local can, faster, LoRAs per persona | not built; cold starts |

**VIDEO**

| Generator · model | Price | Can | Can't |
|---|---|---|---|
| **runway** · gen4_turbo / gen4.5 | 5 / 12 credits/s (0.25 / 0.60 per 5 s) | start frame + prompt, 2–10 s | end frame, refs, control video |
| runway · aleph2 | 28 credits/s, min 56 (1.40 per 5 s) | the handheld pass (2–30 s) restyled, ≤ 5 keyframes | refs; cheap |
| **luma** · ray-3.2 `video` | per 5 s: 0.06 (360p draft) · 0.15 · 0.30 (720p) · 1.20 (1080p); 10 s ×3 | start + end frame, multi-keyframe (10 s), loop, extend, 9:16 … 21:9; 360p drafts | 10 s with a start / end frame; refs on video; camera presets (prompt only) |
| luma · ray-3.2 `video_edit` | 1.08 per 5 s at 720p (0.54 at 360p) | a source ≤ 18 s with `depth` / `normals` / `pose` / `trajectory` / `face` controls or `auto_controls`, strength adhere · flex · reimagine, ≤ 64 keyframes | `image_ref`; `end_frame`; loop |
| runway · kling3.0 std / pro, klingO3 | 9 / 12, 13 / 17 credits/s | start + end frame, elements (refs) on O3, audio | – |
| runway · seedance2 / 2.5 | 36 / 20 – 68 credits/s, minimums | refs + reference video, multi-shot | cheap (64 – 80 credit minimums) |
| runway · hailuo3 | 10 credits/s + 2 per ref / keyframe | refs, keyframes, physics | long clips |
| runway · veo3.1 / fast | 20 / 10 credits/s (no audio) | first + last frame, refs, quality | price at length |
| comfyui-remote · Wan VACE / Fun-Control, LTX | GPU seconds | depth / pose control by construction, own LoRAs, any ratio | not built; quality below the services today |

Not taken: **Luma's legacy Dream Machine API** (ray-2, photon-1 – Luma moved it to the Agents API, Ray 2 / 3 are under a
deprecation notice; ray-3.2 + uni-1 replace them); **Comfy Cloud** (UI only; the newer Comfy API bills GPU seconds at
≈ 1.3 × RunPod – RunPod direct is cheaper with the volume `scutoons-models` already there); **direct Kling / BytePlus /
MiniMax / Google accounts** – Runway's API now sells those models with the same key, task polling and credits.

**Recommendation:**
- **Image**: comfy-local (free, default for drafts) · **luma uni-1 `image_edit`** as the paid default (the render as the
  source = the layout holds, ≤ 7 refs, 0.05) · runway gen4_image as the second opinion · comfyui-remote when LoRAs matter.
- **Video**: **runway gen4_turbo** for i2v drafts (0.25 / 5 s) · **luma ray-3.2** for start + end frame, loops and 360p
  drafts (0.06) · **luma video_edit / aleph2** for handheld takes (compare both on one take) · Kling / Seedance / Veo as
  extra models of the runway generator, added only when a shot needs what they can · open models on RunPod later.
- One adapter per **API**, not per model family: `luma.mjs` (Agents API) and `runway.mjs` (+ models) cover all but the
  own GPU.

## 6. Rollout – each step shippable
1. **Split by kind** (~1 day): `caps` + `rules` per model (runway, comfy-local, luma-manual, fake), `?kind=`, the
   Image | Video switch, inputs drawn from caps; node tests for the rules. No new provider.
2. **Lineage + kind** (~1 day, = VARIANTS-CANVAS phase 1 + `kind` / `take`): records survive, approvals say `from`.
3. **3a-local budget** (~0.5 day): the ledger file, `STUDIO_BUDGET_USD`, 402 + notice, the cost line "of budget";
   gemini.mjs / comfy.mjs `--engine luma` book into it.
4. **luma generator** (~1.5 days): Agents API `POST /v1/generations` + poll; models uni-1 / uni-1-max (`image`,
   `image_edit`), ray-3.2 (`video`: start / end frame, loop, 360p – 1080p) and ray-3.2 edit (control pass, keyframe at
   0, strength); a mock like runway-mock; keys `LUMA_AGENTS_API_KEY` / the owner's `LUMA_AGENT_API_KEY`. luma-manual
   stays for the app subscription.
5. **runway image + hub models** (~1 day): gen4_image (tagged refs = the figures' cast names), then on demand
   kling3.0 / veo3.1 / seedance2 as models with their caps; aleph2 keyframes ≤ 5.
6. **Send to chapter** (~1.5 days, after `slice.shot` lands in the slices rework): approved picture / clip → asset →
   chapter draft by shot reference; the creator publishes.
7. **comfyui-remote** on RunPod Serverless (~2 days) and **3a-platform** (~2–3 days) – before unattended or
   several-person generation.

## 7. Questions for the owner (each with my recommendation)
1. **Luma API**: the Agents API (ray-3.2 + uni-1 – your `.env` already has its key) as the `luma` generator, with
   "Luma – by hand" kept for the app – or only by hand? → **Agents API + keep by hand.**
2. **Runway as the hub** for Kling, Seedance, Hailuo, Veo (one key, one credit pool, one adapter) – or own accounts per
   provider? → **Runway hub**; a direct account only if a needed feature is missing there.
3. **Paid image default**: Luma uni-1 `image_edit` (the render stays the layout, ≤ 7 refs, ~0.05) – or Runway gen4_image
   (3 tagged refs, a new picture) – with comfy-local the free default either way? → **uni-1 edit.**
4. **Budget now**: a studio-local cap (`STUDIO_BUDGET_USD` in `.env`, a ledger file, hard cut) before the admin budget
   of phase 3a – and how much per month? → **yes, local first; you name the amount** (e.g. 50 USD).
5. **Approved pictures into the chapter**: "Send to chapter" by the slice's `shot` reference as a creator draft –
   instead of the Figma path (figma-chapter.mjs) – once the slices rework adds `shot`? → **yes; Figma stays for layout
   passes by hand.**

## Sources (read 2026-10-19)
- Luma Agents API: docs.agents.lumalabs.ai – quickstart, guides/model, guides/pricing, guides/videos/generation,
  guides/videos/editing, guides/videos/migration, guides/images/generation.
- Runway API pricing: docs.dev.runwayml.com/guides/pricing (all models incl. Kling / Seedance / Hailuo / Veo / Wan).
- RunPod Serverless prices: runpod.io/pricing. Comfy Cloud / Comfy API: blog.comfy.org (Comfy Cloud pricing),
  creativeainews.com "comfy api vs runpod modal deploy cost 2026" (third-party).
- Kling / Seedance / Hailuo direct prices (third-party, unverified, only for comparison): costbench.com (Kling API),
  atlascloud.ai (Seedance 2.0 cost per second), apiframe.ai / picassoia (Hailuo). Veo 3.1: benchlm.ai media-pricing/veo.
