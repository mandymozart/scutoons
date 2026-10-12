# ISSUES — open bugs (something works wrong; open items only; done = the item leaves)

Features, changes, owner steps: agents/FEATURES.md (same number line – the next free number is in its header).
Template and rules: agents/RULES.md "Docs". Older notes of an item: `grep "10.N" agents/archive/BACKLOG.md`.

## Production, platform
- [ ] 10.17 **Deploys are not atomic** – FTP replaces files one by one, the API runs half old / half new for seconds
  (2026-10-02: "No entry or class found for Transport"). Upload to a fresh folder and switch, or a maintenance flag.
- [ ] 10.23 **Asset jobs stay queued on production** (owner 2026-10-03: "Optimize" stays queued) – `assets:jobs` reads
  its own DB; the runner on the owner's machine (ffmpeg) should take jobs through the API (list → download → transcode
  → upload → done). Job #1 (video-renditions) can be dropped.

## Content
- [ ] 10.35 **scene-14 has an unknown `end` event** – the published `scene:catpope:2:scene-14` still holds
  `{"type":"end",…}` (the reader skips it, `content:check` reports it). `decide`: publish the scene's newest version
  (no `end`) or delete the event – or close it: owner 2026-10-07 (B.2): "only configuration – a test scene".

## Reader, media
- [ ] M.15 **Some slices look poor even at 2x / high** – findings 2026-10-07: (1) the files aren't there: no slice has
  3x, ch1 has no 2x – [owner] re-export ch1 (+ ch2 scenes 15 / 16) at 1170 px and "Import slices"; `decide` (2)
  Automatic picks at least the screen density, (3) WebP quality 0.92 or libwebp `-sharp_yuv`.
- [ ] 10.80 **QR camera never decoded on the owner's webcam** (blurry image) – check focus / resolution constraints and
  a sharper decode path; test on a phone.

## Creator, admin, embassy
- [ ] 10.81 **The Reader panel's frame takes the keyboard focus when it loads** – creator shortcuts go to the viewer
  until the page is clicked (found 2026-10-07).
- [ ] 10.82 **Map builder on the phone: the size slider shows no label / value** (the `field` span is missing above it;
  owner 2026-10-11, was in 10.65).

## Checks owed or broken
- [ ] 10.39 **Copy / paste: checks still owed** – `check:tools-dock` and the admin's item-limit fields with the admin
  running (the feature is built).
- [ ] 10.84 **`check:testo` fails from its first step** – its guest half reads the sandbox toon, which is creators-only
  since the production seed. Make the sandbox public in dev, run the guest half on a published toon, or drop it.
- [ ] 10.85 **`check-next-bar.mjs` fails 7 checks** – it expects chapter 2 readable (coming soon in the seed); a replay
  label renders "↺ undefined" – update the check and look at that label.
- [ ] 10.86 **`scripts/check-map.mjs` still checks the old progress unlocks** (places unlock only by events since
  2026-10-11).
- [ ] 10.87 **MCP: four browser checks not run after the last change** (MCP.md, end) – run them.
