# DESIGN — Scutoons design system + chapter text boxes

Part 1 is the design system extracted from the viewer on 2026-09-17
(artifact: https://claude.ai/artifact/9iweNqSed2pAjMhQcEE3bo, sources `viewer/static/css/{styles,fonts,animations}.css`
and the component `.svelte` files). Part 2 specifies the new text-box-on-slice feature for chapter 2.

---

## Part 1 — Design system

Scutoons is the reading platform for CatPope and its sibling webtoons: a square, high-contrast, mono-typeset shell
built to disappear behind full-bleed panel art, visual-novel dialogue and audio-reactive sequences. The UI reads as
a technical overlay on the artwork: black rules, no shadows, no rounded chrome, one loud accent used sparingly.

### Color

| Token | Hex | Usage |
|---|---|---|
| `ink` | `#000000` | Text, headings, primary button bg, 2px structural borders (panels, VN dialogue box, menu drawer). 21:1 on paper. |
| `paper` | `#ffffff` | Page bg, text on ink, secondary/choice button bg, loader overlay. |
| `primary-cyan` | `#00d1ff` | Signature accent: icon-toggle hover fill, active map markers, inventory-frame glow. ~1.8:1 on paper — **never text or small icons**, fills/glows only. |
| `acid-purple` | `#b70aff` | Acid VFX trio — onomatopoeia / interaction highlights in the VN layer only. |
| `acid-green` | `#3fff0a` | Acid VFX trio. |
| `acid-red` | `#ff210a` | Acid VFX trio. |
| `border-muted` | `#cccccc` | 2px border secondary buttons, nav chips (→ `ink` on hover/active). |
| `border-subtle` | `#dddddd` | 1px form-field border. |
| `border-hover` | `#333333` | Hover border of VN choice buttons. |
| `surface-hover` | `#f0f0f0` | Chapter-row hover, inactive chapter badge, table first column. |
| `surface-panel` | `#f5f5f5` | Quiet info panels (account technical details). |
| `surface-tint-cyan` | `#f0fcff` | Inactive map-marker halo. |
| `text-muted` | `#6b6b6b` | Only non-black text: dates, chapter numbers, edition lines. 5.3:1 on paper. |

Border escalation: `border-subtle` → `border-muted` → `border-hover` → `ink`.

### Type

One family: **IBM Plex Mono** 400 / 700 (`viewer/static/fonts/-F63fjptAgt5VM-kVkqdyU8n1i8q1w.woff2`,
`-F6qfjptAgt5VM-kVkqdyU8n3pQPwlBFgg.woff2`), stack `'IBM Plex Mono', monospace`. No italics except the VN narrator
line. No heading scale — root font-size is fluid: **14px < 480px, 16px baseline, 24px ≥ 768px**; never hardcode px
overrides that break it.

| Style | Size | Line height | Usage |
|---|---|---|---|
| `body` | 1rem | 1.4 | Paragraph copy |
| `button` | 1rem | 1 | Button labels, sentence case, never uppercase |
| `label-uppercase` | 1rem | 1.2 | VN speaker plate, onomatopoeia (`text-transform: uppercase`) |
| `caption` | 0.75rem | 1.3 | Meta text in `text-muted` |
| `nav-micro` | 0.625rem | 1 | Label under 16px icons in the main menu |

### Spacing, radius, borders

- Spacing: `space-025` 0.25rem · `space-05` 0.5rem · `space-075` 0.75rem · `space-1` 1rem · `space-15` 1.5rem · `space-3` 3rem.
- Radius: `radius-none` 0 (**default, brand signature**) · `radius-sm` 0.5rem (avatars, inventory count badge) ·
  `radius-full` 50% (circular icon toggles, map markers only).
- Border width: `hairline` 1px (form fields, badge) · `regular` 2px (structural).
- **Borders over shadows** — no elevation scale. `filter: drop-shadow` only as a VFX glow on acid colors / inventory frame.

### Motion

Utilitarian: 0.1–0.3s `ease`/`linear`. Keyframes: `wiggle-*` (idle VN character motion) and `pulse` (dialogue
"continue" affordance only). No spring/bounce, nothing > ~0.3s for UI state.

### Content voice

Short, literal, sentence case ("Coming Soon", "Buy access", "Claim membership"). Uppercase only for VN speaker names.
Wordmark always "SCUTOONS" and always the logo file, never re-set in type.

### Components

- **Button** — primary: `ink` bg, `paper` text, no border, `space-1 space-3` padding, square. Secondary
  (`.btn-secondary`): `paper` bg, `ink` text, 2px `border-muted` → `ink` on hover. `.disabled`: opacity 0.5,
  `not-allowed`. `.btn-clear`: no chrome. No third variant, no radius, no shadow.
- **IconButton** — fixed 2rem circle, `paper` bg, 2px `ink` border, hover fill `primary-cyan`, one 16px black icon,
  `touch-action: manipulation` (inventory / main-menu toggles).
- **ChapterItem** — grid `6rem 1fr auto` (`5rem …` < 768px; the number column at least 3rem, wider only for the
  prologue's label, set in `caption` size; < 768px the row keeps 0.5rem off the right edge), hover `surface-hover`, meta in `caption` + `text-muted`, coming-soon row at 0.5 opacity. Chapter 0
  (the season's prologue) shows "Prolog" / "Prologue" (paraglide `chapter.prologue`) instead of "#0"; prev/next in the
  chapter navigation and the coming-soon page name it the same way (`chapterName`, viewer `lib/utils/chapter-name.ts`).
  Season labels (BACKLOG 10.41): a chapter is named by its position in its season – "Kapitel 2" / "Chapter 2", with
  two or more seasons (that list chapters) "Staffel 2 · Kapitel 1" / "Season 2 · Chapter 1" (`common.season_chapter`),
  never with the absolute #N next to it: the chapter page's tab title ("<toon> · <label>: <title>"), the coming-soon
  heading, the end actions in the box, the start button's "S2 K1", the list's "#1". The toon page's list is grouped
  by season, each under its heading `#season-S` (one season: as before). No "featuring" in the reader (owner 2026-09-30: the reader must stay super logical
  and simple; the data – `Chapter.featuring`, the creator's featuring field, `/cast?chapter=` – stays for later).
- **Coming-soon page** (owner 2026-10-05): with a key visual the page is a fullscreen teaser (`chapter-teaser.svelte`):
  the key visual video fills the window, muted, on a loop, autoplay (reduced motion: the poster until "Ton an"); the
  text ("Demnächst", chapter name, release date) sits over its lower part on a dark scrim, "Ton an" bottom right.
  Portrait windows crop to fill, landscape windows show the whole key visual on black. No key visual: the text page.
- **Toon page, top to bottom** (owner 2026-09-30): cover, toon title + description, "Staffel 1: <title>", the season
  text, **the start button**, the chapter list, **the season's music** (a secondary button "Musik herunterladen" /
  "Download the music" → `/<toon>/download`, only when `season.download.music` is set; with `season.download.image`
  the record's cut-out sits left of it, decorative (`alt=""`), lazy, ≈ 77 px on a phone / ≤ 96 px on a tablet, the pair
  centred; owner 2026-10-01), **the credits colophon**.
  Chapters are named only in the chapter list (never in the music or the colophon).
  **The season's related items** (2026-10-02, `season.related_items`, agents/ITEMS.md "Players, albums, related items")
  sit right after the music: "Zur Staffel" / "With this season" (1rem heading), a list under a 2px ink rule, rows split by
  1px `border-subtle` – the item icon (4rem, desktop 3rem), the name, a caption line (price, or "Hast du schon"), right
  the secondary "Kaufen" (the product's Bandcamp link) or "Ausverkauft"; desktop: in the season column below the music
  button, sizes on the 24 px root like it.
- **Cover lettering** (owner 2026-10-01, everything language-independent; `CoverArt` in `@scutoons/reader`): with
  `cover.lettering` the cover art is text-free and the UI sets the toon's name (small) over the season's title (big, in
  quotes) in the reader's language – comic lettering, not UI chrome: Bangers (`--font-lettering`), paper fill, heavy ink
  outline + hard ink shadow, tilted −5°, sizes in `cqw` of the cover, upper part of the art. Without the flag the image is
  shown as delivered (catpope's cover still has its German title baked in). DATA.md "Language-independent media".
  The cover is the season's (`season.cover`, owner 2026-10-02): the toon page shows the cover of the season it presents.
  Since 2026-10-02 the lettering can be **an own layout** from the creator's cover editor: lines with a text source (toon
  title, season title, own text per language), position / size / box width / angle / skew / alignment, palette colours;
  each line shrinks to its box in the reader's language (DATA.md "Cover lettering layout").
- **Toon page desktop** (≥ 1024 px, owner 2026-10-01, reference: artboard "A · Vollbild" of
  https://claude.ai/artifact/AYHvMG81k7CQc1JrVf4n5R; phones and tablets keep the single column above):
  - **Hero** (`toon/hero.svelte`) = the first screen, `min(100vh, 900px)`, full-bleed: the season's trailer loop
    (`Season.trailer`: muted on every load, looping, `playsinline`; plays only while the hero is on screen and the tab
    visible, coming back resumes in the same mute state; with reduced motion the poster `<img>` only, no video download)
    under a flat `rgb(0 0 0 / 0.45)` scrim, no gradient. The video and the scrim are not clickable (no link, no handler,
    no pointer cursor). The loop carries its own soundtrack: one continuous 24 s excerpt of the song (no cut-up music
    under the 12 × 2 s montage), 0.5 s fade in / 1 s fade out for a soft seam, AAC 128k, ≈ −16 LUFS.
    Bottom left in paper: the season line in capitals ("STAFFEL 1 · DAS ERWACHEN", `hero.season`, 0.5833rem, 0.08em
    tracking), the toon title (3.3333rem ≈ 80 px), the tagline (toon description, 0.8333rem), then the start button
    **inverted** (paper bg, ink text, every state + its progress rule in paper) and "Trailer ansehen ▶" / "Watch the
    trailer ▶" (2px paper outline) → the trailer chapter (`trailer.chapter`, default 0 = the prologue); besides the start
    button the only link on the hero.
    Bottom right: the sound toggle (2px outline at 60 % white, small, a `<button>` with `aria-pressed`):
    "Ton an" / "Sound on" + speaker icon unmutes the playing loop in place (the click is the user gesture), "Ton aus" /
    "Sound off" + speaker-muted icon (outline full white) mutes it again; never navigates. Reduced motion: "Ton an" is
    the explicit request – the loop then plays with sound, "Ton aus" stops it again.
    Without a trailer the cover is the background and the trailer buttons are left out.
  - While the hero is under the fixed top bar `html.on-hero` turns the logo and the round icon toggles white
    (transparent, paper border, inverted icon; viewer `static/css/styles.css`), ink again below it.
  - **Below:** a 12-column grid (max 75rem, gutters 1.3333rem, 2rem sides): cols 1–5 the season heading (1.3333rem),
    the season text, "Musik herunterladen" (secondary; the record's cut-out `season.download.image` 6.25rem ≈ 150 px
    wide left of it, both on the column's left edge, vertically centred), then the **credits colophon** (its own centred fine print, ≈ 13 px,
    filling the column: no 36em measure, heading bars, text and the logo box span it; phones keep the measure); cols 7–12 the chapter list under a 2px ink rule: rows with 1px `border-subtle`, thumbnail
    112 × 72 (cover), title 0.75rem (+ the progress rule 5.3333rem / the coming-soon line in caption), "#n" / "Prolog"
    right-aligned, coming soon at 50 %. Published rows show no date.
- **Top-right icon group (all widths, all pages):** the menu button (round `--control-height` = 52 px at the desktop root, 44 px on touch – "One control height"; 3-line icon
  `icons/menu.svg`, `aria-label` "Menü" / "Menu") is the right-most icon and the **only** way to open the slide-out; on
  toon pages inventory, **avatar**, map and chapter list follow to its left (each `--control-height` + 0.5rem further
  left; five buttons + the logo fit at 320 px). Avatar has its own button (owner 2026-10-03: "inventory should be
  inventory"; `icons/avatar.svg`, overlay `avatar` in the navigation context) but opens the same inventory slide-out on
  its Avatar tab; the slide-out's two tabs carry the same icons (+ label; icon-only under 480 px, the label stays for
  screen readers) and switch the overlay, so the top buttons and tabs always agree. The menu slide-out's
  entries all use the `button` type (`--type-button-size` / `--type-button-line`, rem-based; the toon names and
  "Anmelden" were hard 16 px before).
- **The logo** (top left, all pages) is a link to the localized landing page (`/` · `/en/`, alt "SCUTOONS – Startseite" /
  "SCUTOONS – Home"; owner 2026-10-01); it never opens the menu. On the landing page it just stays there.
- **Slide-out order** (owner 2026-10-01): **the toons first** (avatar + name), then the account link + language
  buttons, then "Zugang kaufen" (non-members), "Exklusive Editionen kaufen", "Veranstaltungen". No "SCUTOONS" entry (the
  logo is the way home). Entries that go somewhere are links (`<a>`, the shop is its own app), the languages are
  buttons (`aria-pressed`). Keyboard: the menu button has `aria-expanded` + `aria-controls="main-menu"`; the closed
  slide-out is `inert` (off screen, not focusable); opening it focuses the first toon; Escape closes it and returns the
  focus to the menu button; click outside closes it.
- **Start button** (`toon/start-button.svelte`, like Prime Video's "▶ S1 F1 wiedergeben" but reading): the primary
  button (ink / paper, square, `button` type) as a link to the chapter, a 16 px forward arrow (2px stroke, square caps,
  `currentColor`; not a play triangle – it navigates) + the label from paraglide `start.*`: de "S1 K1 lesen" /
  "S1 K4 weiterlesen" / "S1 K1 erneut lesen", en "Read S1 C1" / "Continue S1 C4" / "Read S1 C1 again"; `title` = the
  long form ("Staffel 1, Kapitel 4: <title>"). Only in "weiterlesen": a 2px bar under the label (read part `paper`, the
  rest `text-muted`) = the stored position. State rule (`readingState` in `@scutoons/content` progress.ts, tests in
  `test/progress.test.ts`), over the season's readable chapters (published; drafts in dev like the list; coming soon
  never): nothing read → the first; the **last opened** chapter (newest checkpoint) not read to its end → continue it;
  read to its end → the next readable chapter ("lesen"); after the last one → the first never-finished one; all
  finished → the first chapter "erneut lesen". Opening a finished chapter again starts a new read-through (position
  from 0, `isComplete` stays).
  **Prologue (chapter 0, owner 2026-10-01):** a visitor who has read nothing of the other chapters and hasn't watched the
  prologue to its end gets "Prolog ansehen →" / "Watch the prologue →" (`start.prologue`, state `prologue`, title
  "Staffel 1, Prolog: <title>") → `/<toon>/chapter/0`; once it was watched (the video ended or reached 95 %) the rules
  above apply to the other chapters ("S1 K1 lesen"). Chapter 0 is never "the first chapter" of "erneut lesen" and never
  the "last opened" one for continue/next (`readingState`, `PROLOGUE` / `isPrologue` in progress.ts).
- **Reading progress** (`reading/track-reading.ts` on the chapter page): a slice counts as read when ≥ 50 % of it is on
  screen (or, taller than the screen, its bottom edge); position = (slices of the earlier groups + furthest slice + 1)
  / the chapter's slices, < 1 until the **last slice of the last group** is reached (= finished). Stored as a
  `Checkpoint { uri: "<toon>/chapter/<n>", isComplete, position }` in the existing game state (localStorage
  `scutoons_game_<toon>`, a member's `/api/progress/<toon>` via the existing last-write-wins sync). A player video slice
  is read by watching it, not by seeing it: its share of the position grows with the video's progress, and it counts as
  read once the video ended or reached 95 % (`PLAYER_WATCHED`); the chapter list's bar shows the same.
- **Player video slice** (`SliceVideo.player`, `@scutoons/reader` `slice-video.svelte`, owner 2026-10-01 for the music
  video "Roar Little Kitty"): instead of the muted in-view loop a video to watch – sound, our own controls (the
  **player overlay** below; no native controls), no loop, `playsinline`, `preload="metadata"`, the slice image as poster. **Autoplay** (owner 2026-10-01,
  the reader only: `autoplayPlayer`, the creator's preview stays still): once half of it is in view it starts by itself,
  with sound when the browser allows it (a click earlier in the same document: the in-app way from the hero's "Trailer
  ansehen ▶", the start button or the chapter list), otherwise muted with a "Ton an" / "Sound on" button (primary style, `player.sound_on`)
  in the mute button's place that unmutes and keeps playing (the mute button unmuting hides it too); the `play()` rejection is handled
  (`NotAllowedError` → muted, anything else → nothing; not even muted → the poster + the overlay's ▶). Reduced motion: no
  autoplay. The slice's
  `width` / `height` are the video's ratio at 390 px (390 × 206 for 1280 × 676 / 2048 × 1080), `object-fit: contain` on
  ink. While it plays the chapter's music pauses and comes back after a pause or the end (bubbling `sliceplayer` event →
  the viewer's audio action → `holdForPlayer`). Which file plays is the reader's **media quality** (below), not the
  viewport: `SliceVideo.variants { low, medium, high }` + `src` (chapter 0: 480p / 720p / 1080p, `src` = 720p); the
  old `sources [{ src, media }]` is gone (a validation error). No adaptive streaming: HLS (e.g. Bunny Stream) may come
  later. `SliceVideo.controls` (owner 2026-10-01, creator flag "controls" next to player / loop / autoplay): default on
  for a player and off for a loop; a player: on = the overlay's whole bar, off = only ▶ / ↺ (owner 2026-10-04); a loop:
  the browser's basic controls.
- **Player overlay** (`@scutoons/reader` `player-overlay.svelte`, owner 2026-10-04: "like the Netflix / Prime players",
  replaces the native controls; loops never get it – they are art, chapter 1's key visual stays bare). House look on the
  video: ink buttons with a 2px paper outline, IBM Plex Mono, no gradient; the top corners stay free (the app's fixed
  logo and toggles).
  - **▶** (4.5rem round, centred, a flat `rgb(0 0 0 / 0.35)` scrim): before the first play and while paused. **↺** in
    the same place at the end ("Nochmal ansehen", the video from 0) – the only replay for a group ending with a player
    video (the follow-up bar has none then).
  - **Bar** (controls on), at the bottom, 1rem from the edges: a 2px seek line (watched part paper, rest 40 % white,
    round 0.75rem thumb; a range input), below it play/pause (round) + "1:35 / 7:54" left, mute (round, `aria-pressed`)
    and full screen (round) right. While it plays the bar **fades after 3 s** without activity (and the mouse cursor
    over the video); a mouse move, a tap, a key or the keyboard focus in it bring it back; it never fades while paused.
    Not before the first play, not at the end.
  - **"Überspringen" / "Skip"** (fixed label, `player.skip`; the primary square style with ▶▶) bottom right above the bar
    while it runs or is paused; it fades with the bar (owner 2026-10-04: also in full screen). It pauses the video and sends a bubbling `sliceskip` event; the viewer
    (follow-up bar) goes to the next slice of the group, at the group's end to the next group / chapter (the bar's
    primary, no countdown), else (members / coming soon / season's end) it shows the bar and scrolls the end clear of
    it. **Skipping is not watching**: the progress stays where it was (owner 2026-10-04).
  - Clicks: a mouse click on the video plays / pauses; a tap shows / hides the bar (before the first play and without the
    bar a tap plays / pauses). Keys (focus in the overlay): Space / K play-pause, M mute, F full screen, ← / → 5 s.
  - **Full screen**: the slice's box (`requestFullscreen`, the controls come along; the whole video, contain); iPhone
    Safari has no element full screen → `webkitEnterFullscreen` (the system player); **the end leaves full screen** (owner
    2026-10-04: the follow-up bar and its countdown are outside the box – the next chapter's music started behind the
    still full-screen video).
  - "Ton an" (sound blocked) sits in the mute button's place and stays while the bar fades; without the bar bottom right.
- **Media quality** (owner 2026-10-01; PLAN.md "RULE: one file per quality level"): a reader setting, not a responsive
  question. Account page section **"Medienqualität" / "Media quality"** (`components/account/media-quality-setting.svelte`,
  for guests and members, between the account block and the inventory): one row of four square radio buttons
  "Automatisch · Niedrig · Mittel · Hoch" / "Automatic · Low · Medium · High" (2px `border-muted`, the chosen one ink
  with paper text, `caption` size), under it in `text-muted` caption "Bilder und Videos gibt es in drei Größen.
  Automatisch wählt nach deiner Verbindung." + "Automatisch gerade: Mittel." (once known) + the data of a chapter at the
  active level, not resolutions (owner 2026-10-03): "Ein Kapitel mit rund 100 Bildern und zwei kurzen Videos lädt bei
  Mittel etwa 25 MB." (≈ 10 / 25 / 50 MB, measured on catpope s1: 108 slices, two ~30 s videos; Automatic not yet
  resolved → "etwa 10 MB (Niedrig) bis 50 MB (Hoch)") + where it is stored ("In
  deinem Konto gespeichert." / "Auf diesem Gerät gespeichert."). Members: `users.media_quality` via `PUT /me/` (the
  account wins over the device; a stored session is awaited before the first video loads), also mirrored to the
  device; guests: localStorage `scutoons_media_quality` (other tabs follow through the `storage` event). Default
  **Automatic**: the Network Information API's slow signals (data saver, 2G/3G, < 1 Mbit/s) → low at once; otherwise –
  and in Safari / Firefox, which have no such API – a measured download of the first video's low file (512 KB, ≤ 4 s,
  `cache: no-store`): ≥ 7 Mbit/s high, ≥ 5 medium, else low; a failed / blocked probe → low. Decided once per page load,
  never switched by itself mid-video. A changed setting while a video plays loads the other file at the same position,
  playing / muted as before (check-media-quality "switch"). Model: `@scutoons/content` media-quality.ts
  (`pickQualityVariant`: the level, else the nearest lower, else `src`) – the same levels and the same `{ src, variants }`
  shape are meant for audio and slice images (1x / 2x / 3x) later; the season trailer's hero loop keeps its
  `sources` + media query for now (desktop-only decoration, a later candidate).
- **Video placement** (owner 2026-10-01; there was no "full viewport" slice option before – the VN "full" view is the
  only other full-height placement). A video sits in a **box** and is placed in it with a **fit** + a **position**:
  - **The box:** in the column the media's own ratio ("Original", the default) or a **frame** `SliceVideo.frame { ratio,
    fit?, position? }`: "Normaler Slice" 5:8 (390 × 625), "Keyvisual" 6:13 (the key visuals are 390 × 845, measured),
    "HD Hochformat" 9:16, "Quadrat" 1:1, "Eigenes" w:h (clamped 4:1 … 1:4); stored as `"9:16"` or `[w, h]`, the slice
    stays 390 wide and its `height` follows (390 × 693 for 9:16), so the space is reserved before anything loads. Or
    **full viewport** `SliceVideo.viewport { position, fit }`: the slice leaves the 500 px column and fills the viewport –
    its whole width (measured `clientWidth`, no sideways scroll) and the screen's height (`100svh`), ink behind; a frame
    doesn't apply there (the creator hides the ratio; validation warns when both are set).
  - **Four fits** (`ViewportFit`, the same for frames and the viewport): **"Ganz sichtbar" / "Show all"** (`contain`,
    default: the whole video, ink bars where needed) · **"An Breite einpassen" / "Fit width"** (`width`: as wide as the
    box, a taller video cropped at the top/bottom, a flatter one with bars above/below) · **"An Höhe einpassen" / "Fit
    height"** (`height`: as high as the box, a wider video cropped at the sides, a narrower one with bars left/right) ·
    **"Füllen (beschnitten)" / "Fill (cropped)"** (`cover`: always fills, the overflow cropped). Width/height resolve to
    contain or cover per box from the two ratios (`viewportObjectFit`; the media's ratio from the video's metadata, before
    that from the poster).
  - **Position** top / centre / bottom = which part stays when cropped at the top/bottom, or where the video sits with
    bars above/below; **cropping at the sides always keeps the centre**.
  - Drawing: full viewport + contain → the frame is the video's own box (`min(100%, 100svh × ratio)`) placed by the
    position, so the overlay's controls stay on the video; cover → the video fills the box with
    `object-position` 50 % 0 / 50 % / 50 % 100 %. In a frame the video always fills the box with `object-fit` contain /
    cover + `object-position` (bars are the box's ink). Player and loop alike; the poster (and the reduced-motion `<img>`)
    is placed the same way. Text boxes are in % of the column slice and don't follow a full-viewport video (validation
    warning). The follow-up bar stays fixed at the viewport bottom (its spacer keeps the end reachable).
- **Creator slice tab "Hintergrund" / "Background"** (owner 2026-10-01, was "Bild"): image or video (the radio stays);
  a video has "Dateien je Qualität" = **one resolution slot** (`media-variants-slot.svelte`, since 2026-10-03 – see
  "Creator: season page": the preview picks the whole asset, chips 480p · 720p (default) · 1080p replace / clear one
  level, the default = `video.src` follows the levels, uploads go to the chapter's folder; the earlier path fields and the
  "Standard" radio are gone), the flags player / loop / autoplay / controls, then **"Platzierung"**
  (`placement-picker.svelte`; owner 2026-10-01: "less descriptive, more simple UI, basic" – after a 4 × 3 icon grid and
  then picture tiles with sentences): "In der Lesespalte" / "Bildschirmfüllend"; in the column the select
  **"Seitenverhältnis"** with short labels "Original · 5:8 · Keyvisual · 9:16 · 1:1 · Eigenes" (Eigenes = two number
  fields w : h, clamped to 4:1 … 1:4); then one segmented row **"Ganz · Breite · Höhe · Füllen"** (the chosen one
  inverted, house buttons: grey outline, hover ink; the long explanation only as a tooltip) and **"Oben · Mitte · Unten"**
  as a second row only when the position changes something (bars or a crop above/below on one of the screens), nothing
  shown otherwise; one small preview of the poster placed: a phone, plus a desktop in full-viewport mode, or the frame.
  No help sentences. The size line is short: "390 × 693" (frame), "Bildschirm · 1,89:1" (full viewport), else the image
  size as before. Sizes in `em` of the creator's fixed 13 px base (its root `rem` is the reader's fluid one). "Original"
  with a poster whose ratio differs from the video's (> 2 %) shows a warning (the slice would shift while loading).
- **Linked groups** (owner 2026-10-06, DATA.md "Linked groups"): groups with `continues` share one chapter page – a
  chain read top to bottom (vertical → strip → strip → vertical reads smoothly: each strip pins, moves sideways,
  releases). One URL: `?label=` of a group inside a chain renders the whole chain and jumps (instant) to that group,
  kept aligned while the page above settles. One `{#key}` per chain; the follow-up bar only after its last group, "↺
  Nochmal" from the chain's top. Creator: "Durchgehend" / "Continuous" (not on the chapter's last group), "Ende"
  (set / removed with a button: the jumpTo target fields + "sofort") on a group that doesn't continue; a Feather link
  mark in the chapter tree on a group that continues. VN in any group of a chain (DATA.md "Linked groups", the case
  table): one box per page, scripts one after another; while the story waits the page doesn't scroll and a Feather
  `pause` mark sits directly below the logo (an indicator, no button, white ground, `title` = why). `?label=` into a
  chain: the groups above count as read. Creator: a jumping group that continues into an unlabelled one gets a note
  above its script with "Label vergeben"; unreachable groups / slices / events and never-true tests carry a warning
  chip ("unerreichbar", "nie wahr", "Endlosschleife").
- **Flow panel** (owner 2026-10-06; `components/flow/`, @scutoons/content flow.ts): a read-only, zoomable graph of the
  story flow ("Ablauf" / "Flow") on every level, one look (boxes, edges, buttons, minimap). Opened by a round Feather
  `git-merge` button in the top bar (a page's `usePageNav({ actions })`; its `title` says what it
  shows and how to use it – no intro line in the panel); docked on the right like Problems / History on EVERY level
  (in the AppShell's right dock, the page makes room – never an overlay; drags wider up to half the window,
  one stored width, 28rem by default; `?flow=1` opens it (a deep link); the Flow sits next to Problems / History, the Reader
  and Info – several panels side by side, one per kind, each in its place (10.57); every page hands its panel to the
  shell with `useDock` – agents/DOCKING.md "Several panels"; it folds to a rail like every dock panel; a click on a box
  moves the main and, with the Reader open, the Reader too). ONE panel across pages (UI-CONCEPTS E.3 b): `FlowDock` (`kind: 'flow'`) stays mounted toon →
  season → chapter (a box, the sidebar, a crumb – the next page's Flow takes it over), a new level is rebuilt at the
  kept zoom; the Flow only reads the current place and navigates only on a click on a box. Svelte Flow + dagre are loaded only when it
  opens; zoom / fit are one square segmented group (a toolbar: "Round vs square"), a minimap bottom right. A box navigates (read-only otherwise).
  The panel is edge to edge (owner 2026-10-06): its header is every dock panel's (one control row, title, round close
  button, the divider across), the canvas has no frame of its own; the toon's dead-things line sits under a divider.
  - Readable first: the first view is 1:1 – centred on what's open (the group in the editor), else the entry (first
    group / chapter / toon) near the top; "Show everything" fits down to 0.5 at most (smaller is unreadable – the
    minimap shows the rest). Edges follow dagre's route and their labels sit where dagre made room for them (a label is a
    node in dagre's layout), so the labels of edges leaving one box never overlap (`flow-edge.svelte`).
  - **Pictures** (owner 2026-10-06): every box shows small images – the graph model names them as data (`FlowNode.image`
    / `badge` / `faces`, flow.ts), the creator resolves them (`flow/flow-pictures.svelte.ts`): a group – its first slice
    (left, the low variant) and a face stack of the characters acting in its script (joins + speakers + askText, in
    order of appearance, persona / look / mood as named; 3 + "+n"); a chapter – its thumbnail; a toon – its avatar
    (round); a slice – its image on top; a run / step – the speaker's face, a question – who asks it; an item tap – the
    catalog item; a jump / exit – where it lands (the label's group's first slice, else the chapter's thumbnail; another
    toon's avatar as a small badge). Faces = the registry's avatar view (`characterImage`, the reader's rule). Other
    chapters / toons come from the flow index (each toon's `avatar`, per chapter `thumbnail` + label → first slice –
    `chapterPictures`, TS and PHP agree). Icons (Feather, in the title): `message-square` messages, `git-branch` a
    question, `corner-down-right` a jump, `help-circle` askText, `type` a text box, `check` once for good, `repeat` again
    (per page view / period, an endless loop), `log-in` an entry point (a group something outside jumps to); a group's
    layout at the title's end (`align-justify` vertical, `columns` horizontal, `film` full-width strip). Rules: only small
    files; loaded lazily (one IntersectionObserver on the canvas, a box within one canvas size of the view); every box
    kind has a fixed rem size with its picture place (`boxSize`), so the layout never moves while pictures load; no
    picture / not found / broken = the dashed empty slot; an image's `title` is its name; zoomed out below 0.6 faces hide
    and pictures fade (their places stay). No toggle.
  - Expressions step 3: an `if: { expr }` is a condition like a simple test (dashed, its text on the edge – the
    expression as written; never judged "never true"); `askText` is a step of its own on the group / slice level
    ("fragt → name / reader.name" + the question) and sets its variable (scope toon) for reach and dead things; a
    trigger / item tap that runs once for good shows "✓ einmal für immer" (tooltip: at most once per reader), once per
    page view its words; triggers group by their meaning (no `repeat` = every time).
  - **Chapter** (chapter editor, nothing selected): `chapterFlow` over the editor's own `chapterReach` (so it says what the
    chips say). Boxes: group (2px ink; slices · Script · unreachable slices), script points (hairline: point, question,
    divert, jump), slice triggers / item taps that branch, exits (pill: another chapter / season / toon, the next chapter).
    Only through a condition = dashed; unreachable / never true / endless loop = the warning colour + its chip; the group
    open in the editor = inverted. Edges: linked (thick), follow-up bar, end, choices (their text), conditions ("wenn …"),
    else. A box opens its group / slice / event.
  - **Group** (a group selected) – `groupFlow`: every script event; plain ones (messages, joins / leaves, variables, text
    boxes) collapsed into runs between the key points ("„first message“ · 3 Nachr. (who) · kommt: … · setzt …"); a plain
    event under a test is a dashed step in line, the test on the edge into it; every slice trigger / item tap with its own
    events; the groups the reader comes from and goes on to as dashed hairline stubs.
  - **Slice** (a slice selected) – `sliceFlow`: the slice, its triggers and item taps with every event, the points of the
    group's script they divert into (stubs), the script events that reveal its text boxes ("zeigt").
  - The level follows the editor's selection (decision: no separate picker); a small segmented "Kapitel · Gruppe · Slice"
    above the graph goes back up (only the levels the selection allows; a new selection follows again).
  - **Season** (season page) / **toon** (toon page) – `toonFlow` over `toonReach`: one box per chapter ("Kapitel 2 ·
    Ärger", status · groups · unreachable groups; unreachable in the toon = warning chip). Toon reach: entries are each
    season's first chapter, the published chapters (readable from the overview) and chapters a visible map place opens;
    from a reached chapter its follow-up bar leads to the next chapter of the season (only when the chapter can get
    there – a chapter that always jumps away breaks the order), its jumps / ends to chapters / seasons / the toon page.
    Edges: the season order (thick), jumps / ends between chapters (parallel ways merged, their choice texts), exits
    (other seasons on the season level, other toons, products, XR). The toon adds the map places ("Karte: …", one box per
    chapter, dashed when they are all hidden = unlocked by reading). A chapter box opens the chapter with its Flow.
  - **SCUTOONS** (Universum → "Ablauf", `/flow`, next to the world map) – `universeFlow` over the server's flow index
    (`GET /api/creator/flow-index`, so the browser doesn't load every toon): one box per toon (status · seasons ·
    chapters), the jumps between toons (merged, counted, their texts), products / XR as exits. A toon box opens the
    toon page with its Flow (the seasons and chapters).
  - **Dead things** (season / toon / SCUTOONS, `deadThings`; the server's index uses the same rules – TS and PHP agree on
    the dev DB): one muted line under the graph in words, only the parts there are – "Ungenutzt: 1 Variable · 1 nie
    gesetzt · 9 Labels" / "Unused: 1 variable · 1 never set · 9 labels" (set never read · read never set · points nothing
    diverts to · labels nothing jumps to) – opening into the lists, each with where it is (a link to its chapter).
    Nothing dead: nothing shown. Sets = setVariable, a choice's set / add, an askText's variable; reads = simple tests,
    `if.expr` and every text's `{…}` through the expression analyser (names of one part – `reader.name` is no story
    variable). A group label counts as used when something jumps there or the follow-up bar leads there (the group before
    doesn't continue and has no end).
- **Creator group panel** (owner 2026-10-06, `chapter/group-panel.svelte`): compact – one tight field grid (`data-cols="fill"`):
  Name · Layout ("vertikal / horizontal", "vertical / horizontal") · Volle Breite / Full width (horizontal only) ·
  Durchgehend / Continuous · "Ende festlegen"; the explanations are the fields' `title` tooltips, no hint lines; the
  swiper settings only for a horizontal group that isn't full width. Preset (owner 2026-10-06): switching a group to
  horizontal turns Continuous on (not on the chapter's last group, not when continues / an end is already set); full
  width stays off – on demand. No "Interaction" field (`Group.interaction` is gone
  from the model; old stored values are ignored). The **strip preview** of a full-width group below it: the mouse wheel
  down moves the strip sideways while the pointer is over it (like the reader's page scroll); at either end the page
  scrolls on; a horizontal wheel / trackpad scrolls natively. A **vertical** group has the same preview (BACKLOG 10.49,
  UI-CONCEPTS E.6): the same screen (phone 390:844 / desktop 16:9), the slices stacked at the reading column's width
  (phone: the screen's; desktop: the reader's 500 px column of a 1440 px window, centred), scrolled up / down inside
  the screen's fixed height with the wheel's own scroll (no remap; at either end the page scrolls on). A horizontal
  swiper group has none.
- **Reader panel** (BACKLOG 10.49, agents/DOCKING.md "The Reader panel"): the chapter editor's top bar has a round
  `smartphone` button left of the Flow's; it docks the viewer's draft preview – a phone (or, the square
  phone / desktop group in its header, a 1440 × 810 window) scaled into the panel, following the group / slice open in
  the editor, reloaded after every saved draft.
- **Follow-up bar** = the chapter's **end actions** (`chapter/next-bar.svelte` on the chapter page, owner 2026-10-01;
  since 2026-10-07 choices of the VN message box, BACKLOG 10.48 / UI-CONCEPTS E.5): what comes after the shown group
  (of linked groups: after the last one).
  - **When:** the group is done = its last slice was seen (≥ 50 % on screen or its bottom edge, `sliceSeen` in
    track-reading.ts), or – a group ending with a player video – the video **ended** (playing it again hides the bar);
    a page with VN additionally needs every script it must run finished – of linked groups all from the shown group on
    – none in line and nothing waiting (no message waiting for its click, no choice open, no trigger running;
    `vn.state.pageDone` and not `frozen`, owner 2026-10-06). A script that ends with `jumpTo` navigates itself, the bar
    never shows there.
  - **Look** (owner 2026-10-07: "the box at the bottom is where I act"): no own bar – the VN message box
    (`vn/message.svelte`, every chapter page has it, also without a VN – `vn-page.svelte`) shows them: the text (if any)
    as the box's text, every action as a box choice in the story choices' look (`.vn-end-choice` = `.vn-choice`, the
    style lives in message.svelte), the primary first, at most 2. Handed over as a snippet (`vn.setEnd`); the box
    reports its height, so the VN characters stand on it as on a message. A spacer (`.next-host`, the box's height +
    2rem) keeps the last slice clear of it (a full-width strip: none, the box sits over its last screen).
  - **Rules** (`nextStep` in `@scutoons/content` progress.ts + tests, over the season's whole `sequence` – a draft is
    never skipped –, `isItemPublic` for drafts in dev, the membership context):
    | case | text | primary | secondary | countdown |
    |---|---|---|---|---|
    | the group has an authored end (`Group.end`, 2026-10-06) | – | "Weiter", to a chapter like the next-chapter case ("Kapitel 2: Ärger"); with `auto` no bar: it goes there at once | "↺ Nochmal" | yes |
    | more groups follow | – | "Weiter" (`?label=` of the next group) | "↺ Nochmal" | yes |
    | the next chapter readable (preview or member) | – | "Kapitel 2: Ärger" (`chapterName` + title); from the prologue "S1 K1 lesen" | "↺ Nochmal" | yes |
    | the next chapter members-only, not signed in | "Kapitel 1 ist für Mitglieder: melde dich an oder löse deinen Code ein." | "Anmelden" (`/account?return=<chapter>`, back there after the login) | "Code einlösen" (the QR camera = the claim flow) | no |
    | … signed in, no membership | "Kapitel 1 ist für Mitglieder: löse deinen Code ein." | "Code einlösen" (the QR camera) | "Wo bekomme ich einen Code?" (the shop, like the blocker's "buy access") | no |
    | the next chapter coming soon | "Kapitel 3 erscheint am 1. Dezember 2025" (no date, or a draft not public here: "erscheint bald") | "Zur Übersicht" | – | no |
    | the season's end | "Staffel 1 geschafft" | "Zur Übersicht" | "Musik herunterladen" (only with `season.download.music`) | no |
    After a group ending with a player video there is no secondary: its replay is the ↺ on the video (player overlay);
    moving the video away from its end (↺, the seek line) hides the bar until it ends again. "↺ Nochmal"
    scrolls to the top and starts the group again (a VN group runs its script again: the page's `{#key}` gets a replay
    counter). A redeemed code / a login turns the members case into the countdown case by itself.
  - **Countdown** (only next group / next chapter): a thin `ink` line along the bottom of the primary choice fills in
    **10 s** (owner 2026-10-07; was 5), a small "Abbrechen" link under it ("Weiter in 9 s" for screen readers only),
    then it navigates. Stops (the choice stays): "Abbrechen", the end scrolled out of view, a
    click anywhere outside the end actions (the main menu, a toggle), Escape, an overlay (chapter list, map, inventory), the
    camera or a claim sheet. A hidden tab pauses it (the time away doesn't count). Reduced motion: no countdown.
  - Strings: paraglide `next_bar.*`. The per-group override is `Group.end` (built 2026-10-06, table above).
- **Credits colophon** (`@scutoons/reader` `CreditsColophon`, owner 2026-09-30, typography after the back of the vinyl
  cover; content = `credits:<toon>` as it is): `0.8125rem` (≈ 11.4 px on a phone), line-height 1.5, centred, max
  36em; section headings in capitals, bold, letter-spacing 0.08em, inside a 2px `ink` framed bar (square); names as
  running text "Name (role, role), Name (role)"; toon crew groups as "**Label:** names"; songs / performances as a
  bold title line (+ "venue | date") then the names; chapter sections are left out. Ends with "UNTERSTÜTZT VON" /
  "SUPPORTED BY" + the season's `supporters` logos: a centred wrapping row, one height (`clamp(28px, 2.125rem, 36px)`),
  forced to ink (`filter: brightness(0)`), linked when `url` is set, `alt` = name; no logo → the name in bold fine
  print. Replaces the old "Credits" button; `/<toon>/credits` (printed URL) keeps the full credits page.
- **DialogueBox** (VN) — fixed bottom panel, `paper` bg, 2px `ink` border, square, `space-1` padding; hidden by
  `translateY(14rem)`, shown over 0.1s; speaker in `label-uppercase`; narrator italic; pulsing `...` when awaiting tap.
- **ChoiceButton** (VN) — full width, left aligned, 2px `border-muted` → `border-hover` → `ink`, stacked with `space-075`.
- **VN layout** (2026-09-30, owner) — the author picks each character's side: the view (`variant`) of its
  `joinScene` ref *is* the side, no extra field (creator: "Slot / Ansicht"). **Everyone in the scene is always visible**,
  standing on the DialogueBox (`viewer/src/lib/visual-novel/stage.ts`, `vn/characters.svelte`).
  - **Order** on each side, front → back: the speaker, then by recency of speaking (who just stopped speaking stands right
    behind the speaker), then the ones who haven't spoken since they joined, in join order.
  - **Speaker vs. the rest:** the speaker stands in front of the box (z above it), feet on the box top (4px into it),
    with the glow `drop-shadow(0 0 0.5rem <persona color>)` + wiggle. Everyone else is behind the box (z below it),
    dimmed (`brightness(0.85)`) and **1rem lower**. Changes animate over 0.3s (ease, no bounce).
  - `avatar` → the **left side**: 4rem × 4rem, `radius-sm`, 2px `ink` border, persona colour behind the head (the avatar
    image, cover). The front one 0.25rem inside the box's left border; each one behind **+1rem to the right**. Nothing
    left of the box border. In the stories: Cat Pope and his personas.
  - `portrait` → the **right side**: 9rem, each step back 9 % smaller (min 50 %), further right, the backmost at the
    box's right border. The one behind overlaps the one in front by **1/3 of its (scaled) width**; crowded → the overlap
    grows evenly for the whole side up to **2/3**; still too wide → the whole side scales down evenly. Nobody is hidden.
    Room at 1/3 (right side alone / next to one avatar): 4 / 3 portraits at 375px, 5 / 4 at 500px; up to 2/3 without
    scaling: 11 / 8 at 375px, 12+ / 11 at 500px.
  - The two sides share the width: each gets what it needs at its default spacing; when both don't fit, the smaller
    need is kept (up to half), the other side gets the rest.
  - `full` → the **centre group** (owner 2026-09-30), breaking the 4th wall: feet on the dialogue box's **bottom** edge
    (the box covers the lower body; it always stays on top and readable), 80 % of the viewport height (≤ the width minus
    1 rem per side, images 1 : 2). The front one (the speaker, else the most recent – the same recency order) stands
    exactly centred at full size; the ones behind step back **alternating left and right by 1.5 rem** per step (2nd
    1.5 rem left, 3rd 1.5 rem right, 4th 3 rem left …), each step 9 % smaller. The group stands behind the avatars and
    portraits (z 70 − depth), dimmed; a full character that speaks is un-dimmed with the glow and in front of the stage
    (z 95), still under the box (z 100). `stage.ts` `fullLayout`, `vn/characters/full.svelte`; vn-check asserts it.
  - No mood text anywhere (the art shows the mood).
  - One click per message, the last one included: the click on the last message also jumps (`jumpTo` with `wait`), and
    a group's box closes after its last message is clicked away.
- **Onomatopoeia** — a text box on the slice that a `showTextBox` event reveals with an effect (`shiver`, `pulse`,
  `wobble`, none); see Part 2 "Onomatopoeia = a text box". (The old full-screen onomatopoeia with its own text, size and
  variants `default` / `outline` / `thorns` / `bubble` was removed on 2026-10-03 – never used in content.)

- **Landing page** (`routes/+page.svelte` + `+page.ts`, owner 2026-10-01, design "D · Brutal": artboards "(desktop)" /
  "(phone)" of https://claude.ai/artifact/XSq3qwDDuf8qDmKSAhK64a; everything from the API, no hard-coded releases).
  Top to bottom: the fixed animated logo (top left, the layout's) + the menu button as on every page; the big static
  wordmark (`/SCUTOONS-logo.svg`, with the katakana, full width) + the slogan in bold, a 2px ink rule; **TOONS** (1.5rem
  bold): square tiles, 2px ink frame, the image square on the tile colour (`toonTileColor`: settings `tile.color`, else
  the star's registry colour – catpope blue, yellowmouse yellow, toptoy black), 2px ink rule under it, the name bold, a
  caption line in `text-muted`. Published (`/library/toons` status published): the whole tile links to the toon page,
  caption "Staffel 1 · Das Erwachen" (`home.season`: its newest season with a published chapter), the primary "Lesen →"
  (16 px arrow, 2px stroke). Announced (`announced`): caption "bald" / "soon", no link, no date. Order: published first,
  then announced, list order. ≥ 768 px three tiles in a row (gap 1.3333rem); phone: a published tile across the row (name
  + caption left, "Lesen" right), the announced ones two by two. **LIMITED EDITIONS**: the intro line, "Digitaler
  Download inklusive · weltweiter Versand" (caption, muted), then each featured edition (`/products/featured/`, chosen in
  the admin's "Startseite") under a 2px ink rule: name (1.25rem bold), sizes and colours (sold out struck through, "(ausverkauft)"
  for screen readers), price bold, "Limitiert auf N Stück" + the product's credits (caption, muted), "Kaufen" (primary,
  the Bandcamp link; "Ausverkauft" disabled when every colour is sold out) and the existing Swiper slider
  (`product-slides.svelte`: 1-up < 768 px, 3-up from 768 px). Desktop ≥ 1024 px: text left (17.5rem), slider right;
  below: centred text, slider underneath. "Alle Editionen" (secondary, 2px `border-muted`) → the shop. Footer: 2px ink rule,
  "SCUTOONS · Keine Werbung, kein Tracking." bold + "Termine · Quellenangaben" (`/events`, `/attributions` – there are no
  Impressum / Datenschutz routes yet). Only finished things: no release dates, no chapter announcements. Strings:
  paraglide `home.*`. Check: `node scripts/check-landing.mjs`.
- **Menu, announced toons**: after the published toons, greyed (`text-muted`, image at 50 %), "<name> bald", a `<span>`
  (`aria-disabled`), never a link; the image is the tile image, not the avatar (Mausboss' avatar shows the mouse).

### Iconography & logos

12 single-path solid-black SVG icons in `viewer/src/lib/icons/` (account, chapter, headphones, inventory, list, map,
qr, video, watch, horizontal-swipe-gesture, vertical-scroll-gesture, scroll), each at its own native viewBox, ink baked
in (not `currentColor`). Logos: `viewer/static/SCUTOONS-logo.svg` (wordmark), `SCUTOONS-logo-animated-square.svg`
(square mark). Never recolor, always on `paper`.

---

## Part 2 — Text box on slice (chapter 2 onwards)

**Texts and presets (owner 2026-10-03):** the creator says "text", not "text box" – a text's look is one of five presets
(`creator/src/lib/editor/text-presets.ts`, only existing Dialog fields): **Narration** (box, italic mono, left, middle),
**Speech** (box, mono, left, middle), **Onomatopoeia** (no box, Bangers 15 %, white fill, 0.1 em ink outline, −7°),
**Story** (no box, mono, left, top; 10 % from both sides = x 10 / width 80, height 20 %). **Scene** (box, italic upper-case mono, left, middle: "19 EXT. POOL"). "+ Add text" shows them previewed
on the current slice; the inspector switches the look (words, position and size stay) and fine-tunes it. The old variant
choice (speech bubble / caption) and the drawn-bubble setting are gone from the creator; their data still renders.
Position is x + width (right = 100 − x − width, never stored).

Goal: speech bubbles, captions and narration are **live text over text-free art**, so every locale shares the same
images and only the words change.

### Principles

- The bubble **shape** belongs to the art (exported from Figma `art` frame). The viewer renders only the words.
- Geometry in **% of the slice box** (same convention as triggers/samples) → resolution-independent.
- **Geometry is the same in every language** (owner 2026-10-03): position, size, rotation and scale of text boxes and
  placed items; only the text is per language. Since 2026-10-06 a chapter is one language-neutral document
  (agents/NEUTRAL.md): geometry exists once, only the texts are locale maps (`unifyGeometry` is gone).
- **One layer stack** (owner 2026-10-07, BACKLOG 10.43): text boxes and placed items share one paint order – `layer`
  on each entry, higher = on top; none = items under the text boxes (agents/ITEMS.md "Placed items"). The creator's
  slice inspector lists them in one "Layers" tab, the top row in front, reordered with the shared Sortable.
  **Layers 2** (owner 2026-10-07, BACKLOG 10.54): the click / hover areas join the same stack – an event area (the
  events sharing one click / hover trigger with a rect = one row: Feather `zap`, "first type +n · Tap") and a sample's
  area (Feather `music`, "sample id · Tap"). They stay listed and edited in their own tabs (Events / Audio); in Layers a
  row only selects the area on the canvas (region `slice`, `sc-selected`), a square "Events" / "Audio" button under the
  list opens its tab. On the Layers tab the canvas lets these areas take the pointer too (in the stack's order), not the
  whole-slice / in-view events. Whole-slice events, other starts and samples without an area are in their tabs only.
- Font size in **% of slice width** rendered as `cqw` (`.slice { container-type: inline-size }`) → lettering scales
  exactly like the art at any column width (≤ 500px).
- It is comic lettering, not UI chrome: the UI tokens above do not apply, except `ink`/`paper` as default colors.

### Schema (`viewer/src/lib/models/content.ts`)

```ts
export type DialogVariant = 'bubble' | 'caption' | 'narration';

export interface Dialog {
  id?: string;                 // stable key = Figma node id → i18n/s1c2.json
  rect: { x: number; y: number; width: number; height?: number }; // % of slice (top-left origin)
  text: string | string[];     // array entries = lines / paragraphs
  variant?: DialogVariant;     // default 'bubble'
  frame?: boolean;             // caption/narration: paper box + 2px ink border, default true; false = bare text
                               // on the art (pair with `mask` to cover baked text)
  size?: number;               // font size, % of slice width (Figma px / frame width * 100); default 4
  align?: 'left' | 'center' | 'right';        // default: bubble 'center', caption/narration 'left'
  valign?: 'top' | 'middle' | 'bottom';        // default 'middle' (needs height)
  font?: 'lettering' | 'mono';                 // default: bubble 'lettering' (Bangers, OFL), caption/narration 'mono' (IBM Plex Mono)
  weight?: 400 | 700;
  uppercase?: boolean;
  letterSpacing?: number;      // em of the font size (owner 2026-10-03); default the font's own
  color?: string;              // default ink
  rotation?: number;           // degrees
  mask?: string | true;        // DEPRECATED (owner 2026-10-03): re-import the art text-free, keep the dialog data.
                               // Fills the rect (+ `pad`) to cover text baked into legacy art (chapter 1). true = '#ffffff'.
  maskShape?: 'rect' | 'ellipse';  // default 'rect'; ellipse for round bubbles
  pad?: number;                // extra mask padding in % of slice width, default 0.5
}
```

Backwards compatible with the existing chapter-2 entry (`rect {x,y,width}` + `text[]`).

### Rendering

- `dialogs.svelte`: absolute layer `inset: 0`, `pointer-events: none`, above the `<img>`, below trigger hotzones.
- Each box: `position:absolute; left:x%; top:y%; width:w%; height:h%`, flex for `valign`,
  `font-size: calc(var(--size) * 1cqw)`, `line-height: 1.15`, `overflow-wrap: anywhere`, `hyphens: auto` with
  `lang` set from the chapter language (German compounds).
- Variants: `bubble` — plain text inside the art's bubble; `caption` — `paper` box, 2px `ink` border, square
  (on-brand panel caption); `narration` — like caption, italic.
- **Drawn bubble (owner 2026-10-03, "bubble is currently caption").** Why it looked like a caption: `bubble` only ever
  drew the words (the shape is in chapter 1/2 art), so on text-free art there was no bubble at all, and the creator's
  new default box is a caption. Now `bubble` + `frame: true` draws a speech bubble: paper ellipse, 2px ink outline, a
  tail (`tail`: bottom-left (default) · bottom · bottom-right · top-left · top · top-right · none). `frame` stays off by
  default for bubbles, so published chapters render exactly as before; the creator sets it when a box becomes a bubble.
  `shape` (round 3, owner: "loud and intense stuff such as screaming and singing"): `box` (default, speech – a paper
  rectangle with an ink outline and a tail; owner 2026-10-10: "alle sprechblasen in rechteckige mit einem pfeil … ich
  brauche keine runden", "oval is redundant" – an oval cut its text; an older `oval` is drawn as a box) or `wobbly`
  ("Schrei / Shout") – a roughly rectangular box whose ink outline trembles on all four sides, drawn as an SVG for the
  box's laid-out size (`dialog/wobbly.ts`, deterministic per box id: the same bumps on every render); its tail defaults
  to none.
- **Outline** (`stroke` colour, `strokeWidth` em, default 0.14): letters with an outline on any variant – the onomatopoeia
  look (white letters, ink outline) without a variant of its own.
- **No `sfx` variant** (removed 2026-10-03, never used in content): an onomatopoeia is a bubble without frame,
  uppercase, white letters with an ink outline, revealed by an onomatopoeia event.

### Onomatopoeia = a text box (owner 2026-10-03, option B; legacy removed the same day)

A `showTextBox` event reveals a text box: `params` = `{ dialog, effect?, duration?, delay?, mode?, wait? }`,
`dialog` (required – an event without it is a validation error and shows nothing) = the id of a text box on its slice
(slice events) or on one of the group's slices (a group script). The box is hidden until the event fires, then appears
with the event's `effect` / `delay` / `duration` (`@scutoons/reader` `src/effects`, filling the box) and **stays**;
`wait` (a tap continues) and `mode` (mono = one at a time, queued) as before; a missing box still completes after its
time. The box gives the look (variant, font, colour, outline, rotation, scale, text per language), the event the timing.
`stay` (default true): after its effect the box stays, or – `false`, "the box disappears again" – is hidden again after
`delay` + `duration`. A placed item's tap events may reveal boxes on its slice too (hidden in advance like the others).
Creator: the event's "Text box" field is one select "choose or create new" (owner 2026-10-06): the slice's boxes (a
group script: the group's slices' boxes, "Slice n · …") labelled by the start of their text in the header's text language
(never the id; the same text twice gets "(2)"), last "+ New text box" (slice and placed-item events) – a lettering box
in the middle of the slice (the onomatopoeia preset: white, ink outline, uppercase; text "Text" in the source language),
revealed by the event, selected on the canvas with its text field focused. The event line names its box by the same
label. The canvas marks a revealed box "hidden until ⚡ n" / "script n".

Naming (owner 2026-10-06): the event is `showTextBox` (was the misspelt `showOnomatopeia`, no alias; stored versions
rewritten by migration `2026-10-06-show-text-box`), its params `ShowTextBoxParams`, its effect `TextBoxEffect`; in the
viewer `showTextBox`, `activeTextBoxes`, `ActiveTextBox` (@scutoons/reader), `text-box-wait`; in the creator
`event-show-text-box`, the reveal presets (`REVEAL_PRESETS`, messages `creator.reveal_*`). "Onomatopoeia" remains only
for the look – the text preset `onomatopoeia` (lettering, outline) and the style guide; there is no slice-level
onomatopoeia field (the old one was dropped 2026-10-03).

- Overflow guard: if translated text exceeds the box, shrink font down to 80% via a `fit` pass
  (ResizeObserver), then allow overflow; the exporter report lists boxes where `en` is > 130% of `de` length.

### Figma → config mapping

| Figma | Config |
|---|---|
| slice frame `s1c2-slice-NN` (W × H) | `slice.image = images/webtoon/s1c2/s1c2-slice-NN.webp` (1x, 390 × 625; `width`/`height` set) |
| TEXT node bbox (x, y, w, h relative to frame) | `rect = {x/W·100, y/H·100, w/W·100, h/H·100}` (2 decimals) |
| `style.fontSize` | `size = fontSize / W · 100` |
| `textAlignHorizontal` / `textAlignVertical` | `align` / `valign` |
| `textCase: UPPER` | `uppercase: true` |
| `fontWeight` | `weight` |
| solid fill | `color` (omit if `#000000`) |
| layer name `#caption` / `#narration` | `variant` |
| node id | `id` → `i18n/s1c2.json` key |
| `characters` (split on `\n`) | `text` (de source; `en` from i18n file) |

## Creator navigation (owner 2026-09-30)

The creator is a hierarchy: **Creator → toon → season → chapter → group (scene) → slice**, plus the platform-wide
**Universe** (characters, moods, wanted images). Editing deeper must never change the frame around you.

- **One top bar, the same on every page.** It holds only the breadcrumb of the full path
  (`SCUTOONS › Cat Pope › Season 1 › Chapter 2 › scene-14 › Slice 3` – the logo is the root crumb; the "Creator" word
  is gone, the app name stays in the tab title, owner 2026-10-07) and, from 1600px, the signed-in account. Every crumb
  is a link with its full text – never a per-crumb ellipsis. The structure never changes between pages, only the depth.
  **Breadcrumbs** (`@scutoons/ui` `Breadcrumbs`, UI-CONCEPTS E.2): when the trail doesn't fit, the crumbs after the
  root collapse from the left into ONE "…" button (square – a text control) that opens a small menu of them (Enter /
  ↓ open, ↑ ↓ Home End move, Esc back); the last two never collapse; when even they don't fit the trail wraps. The
  season crumb is the chapter's real season (the season whose sequence lists it). Burger for the sidebar below 768px.
- **Left sidebar = the children of the current level:** home → toons; toon (and its cast / credits / settings) →
  seasons + Cast + Credits + Script + Toon settings; season → its chapters with the status per language; chapter → Meta,
  Audio, groups → slices (the chapter tree). A leaf page shows its siblings with the
  current one marked. The **Universe** section (Characters, Moods, Wanted images) is pinned to the bottom of the
  sidebar on every page.
- **Content** edits the selected item. **Bottom ActionBar** is the same component everywhere: Info (left-most),
  "up" (the parent level), page status / filters, secondary actions, primary action right-most.
- **URLs follow the hierarchy.** The existing routes map cleanly and stay: `/<toon>`, `/<toon>/season/<n>`,
  `/<toon>/chapter/<n>` (a chapter belongs to its season; the breadcrumb shows it), `/<toon>/cast|credits|toon`,
  `/characters?tab=moods|wanted&id=`. Inside the chapter the selection is in the query:
  `/<toon>/chapter/2?group=scene-14&slice=3` (`group` = the group's label or scene name, else its number;
  `slice` 1-based; `?section=audio`). Deep links, crumbs and back/forward all work through the URL.
- **Keyboard:** Alt+ArrowUp = one level up (the parent crumb); Alt+ArrowDown = the first child in the sidebar.
- **Names, not files:** the UI never shows storage keys (`chapter2.json` …): one mapping `entityLabel()` in
  `@scutoons/ui` → "Chapter 2", "Season 1", "Scene 14", "Toon settings", "Characters", "Moods", "Wanted images".
  **No version numbers anywhere** (owner 2026-09-30; the ids are store-wide row ids, kept in the API only): state chips
  read "de · Published", "Draft · not published" (newer unpublished changes), "Not published" (never published), with
  the last change's date in the title; the History panel lists versions by date, author and note (restore / publish
  stay); the admin content overview shows the same state words + "N versions · changed <date>".
- **Toon page** (owner 2026-09-30): the toon's display name (UI language, `CreatorNav.toonName`) is the title and links to
  Toon settings; "Season 1: <title>" links to the season editor; a chapter row (its title link covers the row) opens
  the chapter – no separate "Edit season" / "Toon settings" / "Open" buttons. **Statuses in lists are read-only badges**
  (published / draft / coming soon per language, `status-badges.svelte`: toon page, season editor, season sidebar);
  the status is changed only on the chapter page (Meta).
- **Help lives in Info** (`@scutoons/ui` InfoPanel via the ActionBar's "Info" toggle, closed by default, remembered),
  not above the fields. Info is a panel of the shell's right dock (agents/DOCKING.md): it sits next to Problems /
  History / Flow / Reader / a list's detail (outermost – 10.57), drags wider and folds to a rail like them. Buttons and links carry no decorative arrows; icon-only buttons (reorder, remove, prev/next
  slice) have an accessible label.

Implementation: `creator/src/lib/nav.svelte.ts` (`CreatorNav` context: route crumbs from the root layout, `extra`
crumbs + `sidebar` + `fill` registered by a page with `usePageNav`), root layout = `AppShell` with the top bar and
sidebar (`nav-sidebar.svelte` for the default levels), `up-link.svelte` in every ActionBar.

### Creator wording (owner 2026-10-06)

Short, plain, no marketing, no exclamation marks; de and en say the same. Messages: `creator/messages/{de,en}.json`
(paraglide, keys `creator.*`); no user-facing text hard-coded in `creator/src` (field labels included – a label is a
word, never a data path; technical detail at most in the label's `title`). One term per concept:

| de | en | means |
|---|---|---|
| Toon | toon | a series; its settings page = **Toon-Einstellungen / toon settings** (the toon entry: languages, texts per language, tile, map) |
| Staffel | season | |
| Kapitel | chapter | |
| Gruppe | group | a part of a chapter (its slices, layout, VN script); one stored on its own is still a group in the UI ("Gruppe scene-14") |
| Szene | scene | only the screenplay's scene (script, map unlock) |
| der Slice | slice | |
| Textbox | text box | the inspector tab says "Textboxen / Text boxes" (owner 2026-10-06; the count a small trailing number) |
| Vorlage | preset | a text box's preset (Erzählung, Rede …) – never "Look" (that is a character's) |
| Gegenstand | item | catalog items, also "Verwandte Gegenstände" |
| Event | event | slice and VN events; **VN-Skript / VN script** = a group's events |
| Drehbuch | script | the screenplay page |
| Figur, Besetzung, Stimmung | character, cast, mood | |
| Entwurf / Entwurf speichern | draft / save draft | |
| veröffentlichen / zurückziehen | publish / unpublish | status words: veröffentlicht, Entwurf, demnächst / published, draft, coming soon |
| Version | version | (never "Fassung") |
| Sprache, Quellsprache, Übersetzung, pro Sprache | language, source language, translation, per language | "Für alle Sprachen gleich" / "The same in every language" |
| der Reader / Leser:innen | the reader / readers | the app vs the people |
| Gewünschte Bilder | wanted images | |

Never in the UI: file names or paths of documents (`chapter2.json`, `<toon>/<l>/…`), "store", "per-locale document",
overrides per language – a chapter, group, season or the toon settings is one thing with its texts per language
(agents/NEUTRAL.md). Real files stay files: images, audio, video on the CDN. A bool select reads "ja / nein" with the
default in brackets ("(nein)"); a select's default likewise ("(Links)").

### Empty states (owner rule 2026-10-06) – creator, admin, embassy; the reader in spirit

> "No-content warnings should be reviewed app-wide and standardized, also a rule when to bring them. Unless really
> necessary it makes no sense to waste space."

- **An empty list shows nothing** – its add button (or add select, search field, filter) is enough. No "No items yet.",
  "Nobody found.", "No problems." (the counts already say 0), no "Nothing matches." under a search.
- **An empty-state text only when ALL hold:** (1) the page / main area would otherwise be completely blank with nothing
  to act on, (2) the reason isn't obvious, (3) it says what to do (or what happens next).
- **Never** in tabs, inspectors, side panels, pickers / dropdowns, or lists that have an add button. A heading whose
  section is empty and read-only goes with it (hide the section, not explain it).
- **Never framed** (no dashed box, no warning colour): one muted line in the flow – `<p class="sc-empty">` (tools.css,
  the one shared style); buttons that act on it follow below. `DataTable` and the command palette have no empty text.
- Kept (2026-10-06 sweep): creator – "Select a slice on the left." (chapter main area), "Pick a character on the left, or
  create a new one." (characters main area), the script's "No {locale} script yet …" (+ its start buttons), the cover's
  "No square image yet – choose it under Art." and "This season doesn't exist yet – create it on the toon page", the
  convert page's "nothing to convert"; admin – the asset library's "No assets – run Rescan …". (The Ask panel's
  intro went with Ask, 2026-10-09.)
- The reader is another audience: there a text is justified more often (an empty inventory, "No places discovered
  yet – keep reading …", "No shop carries … yet" + buy online, nothing to wear for a slot) – the same test, read with a
  reader in mind; the reader keeps its own look (no tools.css).

### Reorderable lists (owner 2026-10-07, BACKLOG 10.53) – creator, admin

- **One mechanism**: `use:sortable={{ onmove }}` on the list + `data-sort-row` on each row + a `SortGrip` (the drawn
  ⋮⋮, frameless, in the row's colour) at the row's start – @scutoons/ui `sortable.ts` / `sort-grip.svelte`;
  `DataTable` takes `onmove` (a grip column first). No ↑ / ↓ / chevron buttons, no own HTML5 drag code.
- **Pointer**: drag by the grip – a mouse right away, touch after a ~300 ms press-and-hold on the grip (a swipe over
  the list still scrolls). The row is **lifted** (a hard ink offset of 0.25rem – the one shadow, only while dragging),
  a **drop line** in `--primary-color` (`--border-width-focus` thick) shows where it lands; near the scroll
  container's edges the list scrolls; Esc cancels. Chip rows (featuring, related items) drag sideways (`axis`).
- **Keyboard**: Alt+↑ / Alt+↓ on a focused row or its grip (the grip takes the plain arrows too); the new place is
  announced (aria-live "3 / 7"), the focus follows the row. Elsewhere Alt+arrows keep their navigation meaning.
- **One reorder = one undo step**: the list only reports `onmove(from, to)` (`to` = the row's index after the move,
  like `moveAt`); the page makes ONE edit of its document (creator `doc.edit`, admin: one POST reorder).
- Rows keep their own classes and selection: with a selection (chapter tree groups, slices, VN events: Shift / Ctrl
  click) a drag by a picked row's grip moves all picked rows (`moveMany`). Lists nest (a group's slices inside the
  group rows): each row belongs to the nearest sortable list.

### Notifications (owner 2026-10-07, BACKLOG 10.44, UI-CONCEPTS E.1) – creator, admin, embassy

Three kinds, three places; none moves the layout:
- **Transient info → a toast** (@scutoons/ui `createToaster` + `ToastStack`, one per app: admin / embassy
  `lib/toasts.ts` context, the creator a module toaster `lib/toasts.ts` – `toast(message, { type, timeout, action })`).
  Fixed bottom-left (owner 2026-10-07: bottom-right covered fields in use), **above the ActionBar** when the page has one, never in the page flow; fades after 4 s.
  Creator: Saved, Published, Nothing to save / publish, review submitted, event ids assigned (8 s), "added to the cast"
  (+ **Undo**). Errors are toasts that stay until dismissed (`type: 'error'`, announced as an alert). A toast may carry
  ONE `action` (`{ label, run }`, a secondary button before the ×; it runs, then the toast goes; such a toast stays 8 s).
  An editor page's new save / publish replaces its previous toast (`DocSession.notify`).
- **State that needs doing → in the ActionBar**, where the doing happens: unsaved changes = the Save button's dot
  ("Save draft •"; phones "Save •"); the languages that aren't live = ONE chip next to Publish (`status/live-chip.svelte`:
  the codes + the worst state, its colour = that state – "de en · draft ▾", phones "de en ▾"; a neutral chapter /
  scene / season stands for all its text languages) that opens the file-status list upwards (per file: label,
  languages, state in words; Esc / outside click closes). Nothing when everything is live.
- **Blocking problems → a bar in the flow, only these**: a save conflict (Reload / Overwrite), an autosave to recover,
  a converted document (`page/doc-notices.svelte`). Contextual hints inside an editor's own area (legacy texts, "label
  the next group") stay where they belong; they are not notifications.

### Copy / paste (owner 2026-10-07, BACKLOG 10.39, UI-CONCEPTS E.7) – the creator's chapter editor

- **One system:** Cmd/Ctrl+C / X / V act on the LIVE region's selection only (the selection model, E.8); Ctrl/Cmd+
  Shift+V pastes the settings only onto the selection. In a text field, or with page text selected, the keys stay the
  browser's. Ctrl+X cuts (copy + remove); Delete / Backspace only remove.
- **What pastes where:** the slice canvas + inspector (one region): a text box, a placed item, a sound area, an event
  area – as a new one on the open slice (a step offset when it would sit on another, on top of the layer stack);
  Shift+V: a box's look / a placement's action, qty, repeat, respawn, events / a sound area's sound + trigger / an event
  area's event, onto the selected one of the same kind. VN events (a group script): after the last picked one; Shift+V
  = params + `if` of the copied event of the same type. Slices: after the last picked slice, with boxes, items, events
  and their sounds; Shift+V = audio + events. Groups (the tree): after the last picked / open group (a separate scene
  is pasted as a group of the chapter), labels kept unique; Shift+V = renderer, end, `continues`.
- **Ids are fresh, media are references:** new ids on paste (boxes the chapter's next free ones, a showTextBox follows
  its box; events via assignEventIds; placements `<item>-<7 random>`); a paste into another toon references the source
  toon's files (`../<toon>/…`, ASSETS.md "Paths in content") and adds the sounds it uses to the chapter's library.
- **Feedback:** a toast – "Copied: text box", "Pasted: slices (2) · files from catpope: 4", "Settings pasted onto: …";
  a paste over an item's placement limit is refused (an error toast that stays; ITEMS.md "Item limits").
- **The menu:** a right-click (on a phone the long-press) inside a copy region opens Copy · Cut · Paste · Paste
  settings at the pointer (`clip-menu.svelte`; the browser's own menu stays in fields, on grips, on links). Phones (no
  keys): the same four as one square group in the ActionBar's "More". A menu paste reads the system clipboard where the
  browser allows it, else the clip copied last in this tab.

### Slice editor: tool strip + zoom (owner 2026-10-07, BACKLOG 10.47, UI-CONCEPTS E.4) – the creator's chapter editor

- **The strip, left of the canvas** (never over the art; no toolbar row above it): two square vertical groups
  (`sc-seg is-column`, `Segmented vertical`). **View**: the grid (one button, pressed = snap on, the size in its
  corner; hold, right-click, the corner number or → open the sizes 5 / 10 / 15 / 30 – picking one switches snap on),
  the zoom (a zoom icon), and – when the slice has texts with `{…}` – the sample-reader preview (an eye, pressed when
  not "as written"). **Content**: text · item · sound · event (each opens its inspector tab).
- **Flyouts**: one at a time, beside its button, over the canvas; Esc / ← / a click outside closes it, the focus goes
  back to its button. The zoom flyout is the Flow's look – one square group: zoom out · percentage · zoom in · fit ·
  **1:1** (original size).
- **Zoom**: 50–300 % of the slice's 1x width, remembered on this device (`scutoons_creator_zoom`; fit = null). Fit =
  the area's height, at most 2x the slice (a phone: the full width, the area scrolls down). Zoomed, the canvas is laid
  out at that width – no transform – and the area scrolls; boxes, handles and snapping stay in slice coordinates.
  Ctrl / Cmd + wheel (a trackpad pinch) over the area zooms around the pointer, flyout open or not.
- **Phones** (< 768): the same strip at the canvas's left, the canvas the rest of the width; buttons ≥ 2.75rem.

### Phones and tablets (owner-approved phone draft 2026-10-07, BACKLOG 10.50, UI-CONCEPTS E.9) – creator, admin, embassy

- **The drawer** (< 768, the sidebar): its fold button closes it (no rail on a phone – a fold remembered on a desktop
  never shows there); a pick in it (a link, Meta, Audio, a group that opens, a slice – `data-drawer-pick`) closes it.
- **Content first:** the toon page lists its chapters, then its languages (every width); the chapter page on a phone
  leads with its groups (meta) and a group with its slice thumbnails – a tap opens the slice editor.
- **The slice bar** (chapter editor, < 768): a row above the ActionBar – ‹ Slice n/m ›, undo, redo, ⚠ (the problems'
  count in its corner, opens Problems); round icon buttons. "More" keeps the rest (copy / paste, preview, History, the
  note). Toasts sit above it (`--sc-above-action-bar`).
- **More on a phone** is a sheet flush on the ActionBar, the page above it dimmed (a tap there closes it; the toasts
  wait under the dim).
- **The inspector on a phone** is a sheet below the canvas (45 % of the editor) with a grab handle: drag it, tap it
  (45 ↔ 80 %), ↑ / ↓ on it. Its tabs are one row: icon + count, the label the accessible name and tooltip.
- **Touch targets** (coarse pointer): every control ≥ `--control-height` high (44px), frameless icon-only buttons and
  sort grips as wide as high, crumbs and logo links full height, canvas handles 1rem with a 2.75rem target, map
  markers a 2.75rem target, a Toggle keeps its switch track with a full-height target. Standalone text links are still
  open (E.9).
- **No page wider than the screen** at 375 or 768: lists of fields stack below 1024 (moods, the admin homepage), grid
  columns are `min(Nrem, 100%)` wide, an sr-only word sits inside a positioned scroll box, the embassy's top nav shows
  icons only from 768 to 1023; a page's own bar actions go into "More" when narrow (the bar's end keeps the primary
  ones).

### Creator: content language (i18n) + translate mode (owner 2026-10-02)

Translators switch back and forth to copy, adapt and review originals, and there will be more languages than de/en.
- **One language at a time** (owner 2026-10-06: no "all side by side", no "each field in its source" mode any more):
  every translatable field shows exactly the chosen language; the default is the source language – the toon's, or the
  platform's `de` on pages without a toon (characters, moods, roles, map names).
- **The switch** sits in the top bar, left of the search: a compact square frame (a language switcher: square) – Feather
  `globe` + the current code ("DE", "KO") + the source mark (★, below) when it is the source; inverted while a
  translation is shown. A native select invisibly over it (its own list: the toon's languages, the source marked "★" –
  a native option holds only text). Tooltip / accessible name: "i18n: Content language of the translatable fields … –
  not the interface language". The UI language stays in the Ctrl K palette (with sign out). Offered: a toon page → the
  toon's languages (source first); the universe pages → every language of every toon (`de` first); hidden with one
  language. Remembered per browser (`localStorage` `scutoons_creator_text_language` = a language code, try/catch;
  nothing stored = the source; an old stored "all" / "source" reads as the source); a language the current toon
  doesn't have shows the source.
- **The source mark** (owner 2026-10-06): the Feather `star` (the toon page's "make source" star) right after the
  language code it marks – `source-mark.svelte`, 0.9em, `title` "Quellsprache / Source language" – everywhere the
  source language is marked: the switch, field labels (LocaleText, LocaleMapField), status per language, the
  translate hint, cover previews and own-text fields, chapter badges, the character's language list; "★" where only
  text fits (native options, the publish dialog's language list). Never "· source" / "(source)" words.
- **A translation shown → translate mode**: under the field one muted caption line "DE ★: <source text>" + a small
  text link "Copy original" (writes the source into the field; on touch the link gets the 44 px target on its own line).
- Where: `LocaleText` (the texts of a chapter, group, season or the toon settings: titles, descriptions, text boxes, VN messages / choices) and
  `LocaleMapField` (`{ <locale>: text }` values: characters, moods, cast roles, credits, map names) – the two shared
  field components (`creator/src/lib/editor/text-language.svelte.ts` + `components/text-original.svelte`). The chapter
  canvas, strip preview and inspector show the chosen language (a problem in another language switches the header to
  it), the script page opens / switches to it, the cover editor (canvas, own text, "as readers see it") shows only it.
  Per-language status (chapter meta panel, toon settings): the chosen language's only. Tooltips may still list the
  other languages' texts (`otherTexts`).
- **Chapter lists** (toon page, season editor + its sidebar; owner 2026-10-02 "no spreadsheet vibe", "only show badges
  when something is up"): a plain list, no header row – number (muted), the title in the chosen language (else the
  source; a missing translation shows the source's title muted), the whole row a link; the other languages' titles in
  the row's tooltip. Right-aligned filled badges with a word only when something is up (`chapter-badges.svelte`):
  visibility draft / coming soon (season sequence), "Not published" (info), "not saved" (warning), "Missing" (error),
  "· ko" when only some languages are concerned. Everything published: nothing.
- **Header on phones** (< 768px, where the sidebar is the ☰ drawer; owner-approved draft 2026-10-07): ☰ · logo ·
  search (round); the crumbs in their OWN row under the header (`… › Chapter 1 › blotter-deal › Slice 1`, every part
  tappable, no root – the logo is above); the text-language switch and the page's Flow button at the top of the drawer.
- Language-neutral fields (`PropField`, media slots: the same in every language) don't follow the switch.

### Expressions in text fields (owner 2026-10-06, agents/EXPRESSIONS.md step 4)

One field for every text that can hold `{…}` (`fields/expr-text.svelte` via `LocaleText expressions`): text box text,
VN message, choice label, askText question, and the "Only if" expression (expression mode: the whole value is code).
askText's placeholder / default stay plain fields (the reader shows them as written). The stored value stays plain text.
- **Chips**: a top-level `{…}` (closed) is drawn as a chip – the code on the cyan tint with a thin cyan ring, rounded.
  Click = the builder; double-click / long-press = edit in place (plain until the caret leaves it, Escape, blur or its
  `}`); Backspace / Delete next to it removes it whole; ←/→ step over it; a selection grows to whole chips; copy /
  paste is plain text. The chip's tooltip (`title`) says what it does ("if coins >= 3: rich | else: poor – click:
  builder · double-click: edit").
- **Problems**: a wavy underline – `--state-error` for errors (syntax: saving is refused), `--state-warning` for doubts
  (unknown names / functions / arguments; saving works) – on the whole chip, or on the exact range outside chips / while
  editing in place; the tooltip = the problem in the creator's language with its place and the suggestion ("unknown
  name `map.totl` – did you mean `map.total`? – at 2–9"). They settle 300 ms after typing. The browser's spell check is
  off while a text has chips. The Problems panel words them the same way; a row selects the range in its field.
- **Builder** (in-page popover at the caret / chip, `fields/expr-builder.svelte`): What = Value · If … else · Per visit
  · Code (a Segmented, the long words as titles); the value is one search over the toon's story variables, the reader
  state (grouped by namespace, with descriptions) and the functions (description + example; a function picked after a
  value wraps it: `clock(time)`); then an operator with a second value, the then / else texts (the field's language) or
  the list's items; a live preview "Preview as" new guest / halfway / everything collected, and the code (muted, mono).
  Escape closes, Enter applies. A chip the builder can't show (shuffle once, nested constructs) opens as Code.
- **Opening it** – no extra button (less UI): typing `{` shows a small "Enter = builder" at the caret (Enter opens it
  there, typing on writes the code); Ctrl+Enter opens it anywhere in a field (on the chip next to the caret, else a new
  one); in a condition plain Enter.
- **Canvas**: a slice whose text boxes hold `{…}` gets one small select in the canvas toolbar – as written / new guest
  (default) / halfway / everything collected (the same choice as the builder's preview).
- **Technique** (robust + accessible over pretty): a real `<textarea>` with transparent text (caret, selection, IME,
  undo, paste, labels, screen readers stay native) over a mirror that draws the text with chips and underlines, plus a
  transparent hit layer on top for the chips' / ranges' clicks and tooltips. So a chip shows its code (the same glyphs),
  not a summary – the summary is its tooltip. The field's own edits go through `insertText` (Ctrl+Z works).

### Creator: script page (owner 2026-10-01)

`/<toon>/script` – "Drehbuch" / "Script" in the toon's sidebar between Credits and Toon settings (DATA.md "Script").
- **One big text per language**: a plain `<textarea>` in IBM Plex Mono at the body size (1rem, line 1.4 – the tool
  type scale; owner 2026-10-01, the earlier 13/20 px is superseded), `white-space: pre`, `wrap="off"` (line n
  = row n, horizontal scroll instead of wrapping), spellcheck off, as wide as the content column, `tab-size: 4`. Tab
  inserts 4 spaces (the format counts spaces), Shift+Tab removes up to 4 leading spaces, **Esc then Tab** leaves the
  text (keyboard users); edits go through the browser's own command so Ctrl+Z inside the text works.
- **Language**: the top bar's i18n switch (the only language switch; one script at a time, `?lang=` opens one). A missing translation shows "Start empty" / "Copy the de script" (no "create locale" bars: the page's
  MetaPage has `translationsOptional`). A toon without a script opens with an empty, clean (not dirty) source text.
- **Title page**: a collapsed block with the 6 meta fields (per language, text).
- **Bottom bar**: the normal MetaPage ActionBar (Save draft, Publish, History with restore, undo/redo, problems, note).
- **Outline** (the page's left region, agents/DOCKING.md: its Divider drags it 16rem … half the page, its fold button
  or a drag folds it to a rail "Outline", both remembered; above the text under 900px; own scroll): chapters (uppercase, "KAPITEL 1 · DENIAL"), their scenes (number + heading, one
  line, ellipsis) and songs (italic); a click puts the cursor at the line and scrolls it to the top; the item the cursor
  is in is marked. Below it a collapsible **Format rules** panel (also in the Info panel): a summary of the screenplay
  editor's Writing Guide + a link to its README.
- **Performance** (1–2k lines): the textarea is the working copy; it is written into the doc 300 ms after the last
  keystroke (and before save / publish / language switch / outline jump / blur), so a keystroke never copies,
  stringifies, validates or parses the whole text. Measured on the 1834-line Cat Pope script: one frame per keystroke
  (~17 ms round trip headless), no long tasks.
- Check: `npm run check:creator-script` (SCUTOONS_API / SCUTOONS_CREATOR for other ports).

### Creator: cover editor (owner 2026-10-02)

`/<toon>/season/<n>/cover` – covers are per season (owner 2026-10-02; the toon is shown with its avatar). Crumbs
"Creator / <toon> / Season n / Cover", Ctrl K "Season n · Cover"; the season editor's "Cover" section shows a thumbnail,
the state ("Lettering: Off / Default / Own layout"); the cover preview is the link into the editor (see "Creator: season
page"). The toon settings have no cover.
- **Toolbar:** "Lettering" segmented Off · Default · Own layout (the preview language is the top bar's i18n choice – no
  own switch). The hint line under it says what the mode does.
- **Canvas** (left, the art as big as the screen height allows, ≤ 36rem): the reader's own `CoverArt` with editing boxes on
  top in the same geometry (dashed white / ink frame; selected = cyan frame, the live-selection colour). Drag a box to
  move it (snaps to the vertical centre ±1.5 %, a cyan guide; Alt = free); the selected box has three handles: side =
  width (symmetric), corner = size (scales with the distance from the centre), the round one above = angle (Shift: 5°
  steps). Arrow keys nudge 0.5 % (Shift 5 %), Delete removes, Esc deselects. Touching a line of the Default layout makes it
  an own layout (the default's lines copied first). Undo / redo / autosave / Save draft / Publish = the MetaPage bar.
- **Inspector** (right): first the **Art (text-free)** slot – `cover.square` as a media slot (`media-slot.svelte`: the
  preview is the button that opens the asset picker, uploads go to `<toon>/s<n>/`, the path only as the tooltip, × clears),
  once for every language, stored once in the language-neutral season document like the lettering; then (lettering on) the "Lines" list (source label + the text in the preview language, the selected row inverted),
  "Add line" (an own text at 60 %), then the selected line: Text (Toon title / Season title / Own text; own text = one
  field, in the i18n switch's language), "In quotes", X / Y / Size / Width / Angle / Skew (number fields, two per
  row), Align (Feather align-left / center / right, segmented), Fill / Outline (palette selects with a swatch), up / down /
  Remove.
- **As readers see it:** the chosen text language only (owner 2026-10-06; 14rem) – switch the header's language to check
  long translations one by one.
- Square only (the reader shows no portrait cover). Tools rules: regular weight, rem sizes, `--control-height` controls,
  Feather icons, tooltips via `title`; the lettering itself is comic lettering (cqw, exempt).
- Check: `npm run check:creator-cover` (SCUTOONS_API / SCUTOONS_CREATOR / SCUTOONS_VIEWER / SCUTOONS_CDN for other ports;
  `-- --toon=catpope`, `-- --shots=<dir>`).

### Creator: season page (owner 2026-10-03)

Owner: "clean this season view up, like we did on the chapter page" (chapter meta, 876abb0). `/<toon>/season/<n>`
(`season-editor.svelte`), top to bottom:
- **Meta:** title, description (per language, `LocaleText`); a warning + "Fix" only when a document's `language` field is
  not its own language (the old always-visible "language: de: de · en: en" line is gone); release (date) · authors (list)
  in one row; one caption says once that everything but title and description is the same in every language (the
  language-neutral fields have no "per language" switch any more; an existing override still shows with "use shared").
  No number field: the season's number is its identity (URL, entity key), like the chapter's.
- No chapters list (owner 2026-10-03): the sidebar lists the season's chapters with their visibility badges.
- **Cover and trailer:** media as slots, paths only as tooltips: the **cover** preview (`CoverArt` with its lettering) is
  the link into the cover editor (an empty dashed square when there is none) with "Lettering: …" under it; the **trailer
  loop** as one `media-variants-slot.svelte`; the **poster** as a `media-slot.svelte`; "Opens chapter" = a select of the
  season's chapters (default 0) once a trailer exists. No `cover.portrait` (no reader shows it; data keeps it).
- **Related items** (item selector drawer), **Supporters** (logo as a media slot → `<toon>/s<n>/supporters/`, name + url).

**Media with quality levels = one slot** (owner 2026-10-03: "simplify the multiple resolutions situation"):
`media-variants-slot.svelte` + the pure helpers `media-variants.ts` (both in `@scutoons/media`, shared/media/src, since 2026-10-04) work on the `{ src, variants }` shape
(PLAN.md "one file per quality level"; the trailer maps `loop` → `src`). The preview is the button: picking an asset in
the picker fills every level from its renditions (480p / 720p / 1080p, `mediaFromAsset`), the default = medium, else
the LOWER one (low), else high (owner 2026-10-03: less data first). Under the preview one interactive chip per level (`.sc-chip-control`, `--control-height`): "480p ·
720p (default) · 1080p", a missing level dashed / muted (the file in `data-file`, the default `data-default` for checks); a chip opens the picker on that file's asset (its files listed)
and replaces only that level, its × clears only that level; the default follows the levels (`withVariant`); an older
single file without levels stays the default ("One file for every quality"). The round × beside the preview (`.sc-round`,
icon only → round) clears the medium. The
data model is unchanged (`trailer.loop` / `variants` / `poster`; re-choosing drops the deprecated `sources`). `kind="image"`
labels the levels 1x / 2x / 3x for the later slice images. Used by the season trailer and the slice video ("Hintergrund";
`check-media-quality.mjs` step `creator`).

## Icons (owner rule, 2026-10-01)

- **UI icons come from Feather** (`feather-icons`, MIT) through the shared component `Icon` in `@scutoons/ui`
  (`<Icon name="zoom-in" />`, names: feathericons.com). No other icon sets, no inline copies; if a Feather icon is
  missing, add the SVG to the shared library (`shared/ui`), never into an app.
- **Icons where a word makes no sense** (map tools, zoom, close, drag handle, play/pause); **text where a word is clearer**.
  The close / remove cross is the exception to Feather: the one drawn `CloseIcon` (see "Close cross").
  An icon-only control always has an accessible name (`label` / `aria-label`) and a tooltip.
- Brutalist defaults: stroke 2.5, square caps, `currentColor`, 1.25em (scales with the fluid root type).
- The brand's own hand-drawn icons (viewer: chapter, map, inventory, account …) stay as they are.
- **No "hand" tool for panning:** canvases pan with the middle mouse button or Space + drag (and two fingers on touch).


## Brutalist tools (owner rule, 2026-10-01)

> "The UI should use the font sizes I used in the reader even in all the admin tools (creator, admin, embassy, shop).
> I don't like the tiny stuff; the font doesn't look good too small." · "I want clear brutalism, not something in
> between. It makes it more playful, even if we lose some space."

Applies to the creator, the admin and the embassy (opt-in class `sc-tool` on the app root → `@scutoons/tokens/tools.css`,
inert without it). The shop already uses the reader's rem scale and keeps the reader's look.

**Type = the reader's.**
- The root is the reader's fluid root from base.css: **14px < 480px, 16px, 24px ≥ 768px**. No app sets `html { font-size }`
  or a px base on its root (the admin's 16px/14px and the creator's 13px are gone).
- Only the type styles: `body` / `button` / `label` 1rem (running text, table cells, inputs, sidebar entries, crumbs,
  buttons); `caption` 0.75rem = **the smallest text** (field labels, table headers, chips, hints, meta);
  `--type-title-size` 1.5rem = the one page title (h1, bold) and big numbers (regular). `nav-micro` is not used in tools.
- **No px font sizes anywhere** (also not 0.6em-style shrinking); sizes come from `--type-*` tokens.
- **Bold only very rarely** (owner rule 2026-10-02: "only use bold very rarely"; it supersedes the 2026-10-01
  "bold sparingly" list, which was not followed):
  - **Default weight is regular** (`var(--font-weight-regular)`) everywhere: body, list / row titles, table cells and
    headers, buttons, tabs, chips / badges, breadcrumbs (incl. the current crumb), field labels, panel / section
    headings, dialog / sheet titles, counts, big numbers, codes, hints.
  - **Bold is allowed only for** the page's single main title (h1) where the page has one, and a rare, deliberate
    emphasis of a whole short phrase where meaning needs it (e.g. a destructive confirmation, a stale-data warning):
    `<strong class="sc-emphasis">`. If unsure, regular.
  - **Hierarchy comes from** size (the type scale tokens), uppercase + letter-spacing (`--tracking-label` 0.08em;
    bold caps, if ever, get `--tracking-label-bold` 0.12em), spacing and ink / muted colour – not weight. Selected /
    current items are shown by inversion, not by bold.
  - Markup: no `<b>`; `<strong>` only as the emphasis above (plain text or a `<span>` otherwise). tools.css sets the
    browser-bold defaults (h2–h6, th, legend, summary, optgroup) to regular; only `h1` is bold.
  - Enforced by `npm run check:tools-css` (bold `font-weight`, `<b>`, `<strong>` without `sc-emphasis`; a line with
    `bold-ok` + the reason is exempt).
- Headings: h1 = title (bold); h2/h3 = 1rem regular uppercase, 0.08em (`.sc-heading`). Field labels, table headers,
  sidebar group heads, legends, summaries = the **tool label**: caption, regular, uppercase, 0.08em, ink
  (`.sc-label`, `.sc-nav-head`).
  Hints / meta = caption in `text-muted` (`.sc-caption`). Badges with data (codes, ids) stay in their case.

**Frames, colour, shape.**
- Structure = **2px ink** (`--border-width-regular` + `--ink`): fields, panels, sections, tables, top bar,
  ActionBar, sidebar edge, tab groups, segmented controls (buttons: the viewer's, see Controls). Table rows are split by 1px ink (`hairline`), never grey.
- Hard ink / paper. `border-muted` only for **inactive** things (the secondary button at rest, the quiet panel, an
  inactive tab).
  No `surface-*` greys and no tinted fills for structure or state: a state is the frame's colour + a thick left rule
  (0.5rem / 0.5em) in `state-error` / `state-warning` / cyan, the text stays ink. Unsaved = an inverted bar.
- Square everything (`radius-none`; `radius-sm` only for avatars). **No shadows** (also no inset box-shadow tricks) –
  two exceptions: the live selection's 2px inset frame (rows would overlap as outlines) and a dragged row's lift.
- **Round vs square** (owner 2026-10-07, every app – creator, admin, embassy, viewer, shop): a **standalone icon-only
  button is round** – a `--control-height` circle, 2px ink ring (`class="sc-round"`, tools.css; `IconButton`,
  `FoldButton`, `MaximizeButton`, `Rail`; the viewer's top-right toggles; the palette trigger in its icon-only state).
  **Grouped controls are square**: segmented groups (`sc-seg` / `Segmented`), toolbars (the Flow's zoom / fit is one
  `sc-seg`, the ActionBar, canvas tools), tab rows, and every **language switcher** (the creator's text-language
  switch, the viewer menu's languages – one joined frame, the chosen one inverted – the shop's). A control that shows
  text (label, code, "Ctrl K") is square too. Exception still open: the frameless close cross (`CloseButton`).
- Cyan (`primary-cyan`) only for **live** states: the "published" badge, the live selection (see "Selection" below),
  the canvas selection. Never for the current nav entry or a selected row outside the live region.
- **Badges are FILLED** (owner 2026-10-01): status chips, tags, counts = a solid fill, **white text where it reaches
  4.5:1, else ink**; caption size, regular weight (status words in uppercase with `--tracking-label`; ids / codes keep
  their case). The colours (the owner's refined palette) live in one place, `shared/tokens/tokens.css`
  `--badge-<name>` + `--badge-<name>-text`:

  | Badge | Fill | Text | Contrast (white / ink) | Used for |
  |---|---|---|---|---|
  | `published` | `#00d1ff` | ink | 1.82 / **11.56**:1 | published, live, signed |
  | `draft` | `#6b6b6b` | white | **5.33** / 3.94:1 | saved draft, unclaimed |
  | `soon` | `#b300ff` | white | **4.82** / 4.36:1 | coming soon, announced |
  | `error` | `#ff1900` | ink | 3.90 / **5.39**:1 | error, revoked, missing |
  | `warning` | `#ff6600` | ink | 2.94 / **7.15**:1 | warning, unsaved, expired, too long |
  | `info` | `#0066cc` | white | **5.57** / 3.77:1 | info |
  | `neutral` | `#000000` | white | **21** / 1:1 | claimed, source language, tags, plain counts |

  `@scutoons/ui` `Badge tone` = these names (aliases: `accent` = published, `ink` = neutral); tools.css `.sc-chip`
  (neutral) + `.is-<name>`.

**Controls.**
- **Sizes in rem** (owner 2026-10-01, no px): fields, nav entries and clickable rows are `--control-height` =
  2rem + the 2px frame (32 · 36 · 52px), as tall as a compact button.
- **Touch minimum only on touch screens** (owner 2026-10-01): one rule in tokens.css, `@media (pointer: coarse)`
  raises `--control-height` to max(44px, 2rem + frame); fields, list rows, nav entries and buttons use it as
  `min-height`, so phones and tablets get ≥ 44px targets while a mouse desktop stays rem-only.
- Inputs / selects / textareas: paper, 1rem; **text inputs and textareas a thin grey frame** (1px `border-muted`, ink
  on focus – owner 2026-10-03: tells fields apart from selects and buttons), selects look like secondary buttons (2px `border-muted`, ink on hover / focus, a Feather chevron); padding `0 space-05` (one control height, see below;
  textarea `space-05`), placeholder `text-muted`; disabled = `border-muted` + `text-muted`; invalid = `state-error` frame.
- **Buttons = the viewer's** (owner 2026-10-01, no new invention): primary = ink bg / paper text; secondary (the
  normal one) = paper + 2px `border-muted`, **on hover the outline turns ink**, no inversion on hover; ghost = no frame,
  `border-muted` frame on hover; disabled = opacity 0.5. Height = `--control-height` (see "One control height"), side
  padding as in the viewer `space-3`; compact `space-15` where the bars need it (`Button size="sm"`, the creator's
  plain buttons) – `sm` is narrower, never lower. Square, 1rem.
  On phones (< 480px) the ActionBar narrows its buttons to `space-1` at the sides and shows Info as its icon only.
- **Focus = a 3px ink outline** (`--border-width-focus`) 2px outside the frame (`--focus-offset`), everywhere.
- **Selection** (owner 2026-10-07, BACKLOG 10.40, UI-CONCEPTS E.8) – one look per meaning, app-wide (creator, admin,
  embassy); the only selection CSS is tools.css:

  | Meaning | Look |
  |---|---|
  | **Selected** – the edited object (what the inspector / editor / detail shows) and every picked one (Shift / Ctrl) | inverted: ink bg, paper text – `class="sc-selected"` |
  | **Live selection** – selected AND in the live region (what Delete / arrows / Esc / copy act on) | `--select-live-bg` (= `--surface-tint-cyan`), a 2px **inset** frame in `--select-live-border` (= the primary colour; `box-shadow: inset`, neighbouring rows' outlines would overlap), text `--select-live-text` #005a70 (≈ 7.5:1), not bold – automatic: `[data-live] .sc-selected` |
  | Keyboard highlight in a listbox (palette, pickers, comboboxes, the expression builder) | `sc-selected` in a region that is live while it is open / focused → the live look |
  | Picker tile being chosen (`.sc-card.is-selected`) | the live look (the open sheet is the live region) |
  | Current value of a picker field (`.sc-card.is-current`) | cyan left bar |
  | Nav "you are here" (`.sc-nav-item.is-current` / `aria-current`, the Flow's node, the script outline) | inverted; the live look only when it is also the live selection (the chapter tree's group: both classes) |
  | Toggle on / tab / segment / pressed (`aria-pressed`, `.active` buttons), the pager's current page | inverted (small controls); pager links and menu triggers otherwise look like secondary buttons |
  | Hover | **never inverted**: a 2px ink frame inside the row / a thicker ring on `.sc-round` |
  | Focus (keyboard) | the 3px ink ring everywhere – it wins over a component's `all: unset` (tools.css); a control that draws its own focus is a `role="separator"` or carries `data-focus-own` |
  | Drag / drop | the primary drop line, the moving row lifted (DESIGN "Reorderable lists") |
  | In progress (admin batches' running row) | not a selection: `--surface-tint-cyan`, not inverted |

  **Regions** (`@scutoons/ui` `selection.svelte.ts`): `use:region={'slices'}` (or `{ name, initial, modal }`) on each
  selectable area; `pointerdown` / `focusin` inside it makes it live (`data-live`; the innermost region wins, so the slice
  list inside the chapter tree is its own region); `liveRegion.current` / `liveRegion.is(name)` tell the page which one.
  `initial` = live on page load when nothing else is (the main editor area), `modal` = live while mounted (sheets, the
  palette), closing returns to the region before. Keys (Delete / Backspace / Ctrl+X, arrows, Esc, later copy / paste)
  act on the live region only. One region shows at a time on a phone: it is live, nothing extra shows.
  **Canvas objects** (slice canvas, cover, map) can't be inverted – the art is under them: selected = the kind's frame;
  live = the frame + handles + its tag. **Multi-pick**: every picked row has the selected look (all cyan when live).
- Checkboxes / radios 1.25rem, `accent-color: ink`. fieldset = 2px ink frame, legend = the tool label.
  details/summary = a regular uppercase label row (`--control-height`).
- **Icons:** Feather via `Icon` (see "Icons"; the close cross is NOT Feather, see "Close cross"): menu, chevrons for prev/next, rotate-ccw/cw for undo/redo,
  arrow-up/down for reorder, plus, info, search, sort arrows; icon-only controls carry `aria-label` + `title`.

**Room.**
- Tables are roomy: cells `space-075 space-1`. `DataTable` `secondary` columns show from **1600px** (they used to from
  768px): at 1440 a list keeps its key columns, the rest is in the detail.
- Generous padding: panels `space-1`, page content `space-15`, gaps between fields `space-1` (`space-15` after a
  `Field`). Inspector forms use **two fields per row**.
- The tool frame (tokens): top bar = ActionBar height `--tool-bar-height` max(48px, 3rem) (48 · 48 · 72px), sidebar
  `--tool-sidebar-width` 12rem (admin 14rem with descriptions), Info / side panels `--tool-panel-width` 16rem. Every
  region (sidebar, the main, the dock panels, the outline, the inspector) follows ONE model – tiles, one Divider,
  one fold to a rail (every rail's button at the height of the region headers' buttons), remembered sizes; several page
  panels side by side; maximize = fold everything else: agents/DOCKING.md.
- The ActionBar is the page's chrome, not a region: it stays while a panel / the inspector is maximized or the main is
  folded to its rail (10.57 – Save / Publish / undo still act on the main's document; DOCKING.md "Maximize").
- ActionBars that don't fit move `start` + `secondary` into "More" (`breakpoint`); the primary actions stay. Filters
  that need more than one line leave the bar and sit above the list (`.toolbar`), the pager stays in the bar.

**Check:** `npm run check:tools-css` (part of `npm run check`): no px font size and nothing under caption in the
tools' CSS (creator, admin, embassy, `@scutoons/ui`, tools.css; a line with `px-ok` + the reason is exempt, e.g. the
lettering preview). `npm run check:tools-ui` (dev servers + `npm run dev:test-user`; `SCUTOONS_CREATOR|ADMIN|EMBASSY`
for other origins): every tool page at 1440 px (fine pointer) and 390 px (touch) – no visible text under the caption
size (comic lettering exempt), and on touch no button / field / nav entry under 44 px; since 2026-10-02 also every
single-line control exactly `--control-height` ("One control height" below).

**Vocabulary** (`tools.css`): `.sc-label`, `.sc-heading`, `.sc-caption`, `.sc-emphasis`, `.sc-chip` (`is-ink`, `is-live`, `is-off`),
`.sc-frame`, `.sc-nav-head`, `.sc-nav-item` (`aria-current` / `.is-current` inverted), `.sc-selected` (+ `[data-sc-region][data-live]`, "Selection" above), `.sc-chip-control`, `.sc-control`
("One control height"). Creator-wide button classes stay
`.secondary`, `.ghost`, `.active` (creator layout).

## One control height (owner rule, 2026-10-02) – tools AND the viewer

> Owner (2026-10-02, screenshot of the creator toon settings "Languages": the select "de · Deutsch", the chip
> "en · English ★ ×", the select "Add a translation …" and "+ Add" – all different heights): "There is a general problem
> across all apps and views: buttons, selects and input fields have varying heights. A general rule should apply for
> those."

- **Every single-line control is exactly `--control-height`** – in the creator, admin, embassy, shop, the auth forms
  (`@scutoons/auth`) and the viewer: buttons (also link-buttons `.btn` / `Button href`, icon buttons, the round icon
  toggles of the viewer's top bar, the palette trigger, the creator's i18n switch, tabs, menu triggers and entries,
  segmented buttons, close crosses), selects, text / email / number / date / search / url / password inputs, and the
  **interactive chips** that sit in a row with controls (a removable language, a tag / role / filter chip with ×).
- `--control-height` (tokens.css) = **2rem + the two 2px frames** (32 · 36 · 52 px at the fluid root 14 · 16 · 24 px);
  on a coarse pointer `max(44px, …)`. There is **no small variant** and no second token: `Button size="sm"` and the
  creator's compact buttons only have less side padding.
- How: `box-sizing: border-box`, the height from the token (`height` on fields, `min-height` on buttons so a label
  that would wrap is a visible bug, not clipped text), **no vertical padding** (`padding: 0 <side>`), line-height
  normalised (`--type-button-line` 1 on buttons), the content centred by the box (`<button>` does it natively; links,
  labels and chips use `inline-flex` + `align-items: center`). The frame is the same 2px everywhere
  (`--border-width-regular`; a primary button may draw it in its fill colour or leave it out – the height is the same).
  Everything else keeps its look (colours, frames, weight regular, the viewer's 3rem side padding).
- Vocabulary (tools.css, global, so the viewer can use them too): `.sc-chip-control` = an interactive chip (2px ink
  frame, body type, its buttons span the chip's full height over the frame, no frame of their own until hover);
  `.sc-control` = a custom single-line control (a framed label over an invisible native select, a link that is a
  button) – zero specificity, it only adds the box. `.sc-chip` stays the **status badge** (caption, filled): badges are
  not controls and keep the caption size.
- Labels that don't fit a narrow column stay on one line with an ellipsis and the full text as the tooltip (the
  chapter editor's "Add group" / "Add separate scene").
- **Exempt** (not single-line controls): textareas and other multi-line fields, switches (`role="switch"`),
  checkboxes / radios, cards and tiles that are buttons (slice thumbnails, picture cells, inventory items, avatar slots,
  item choices, two-line character rows, the chapter navigation cards with kicker + title + picture, the menu's toon
  rows with their 3rem avatar), map / canvas markers and handles, visual-novel dialogue choices (they may wrap), and
  comic lettering. In markup an exempt control carries `data-control-exempt="<reason>"`; in CSS a line that sets a
  height / vertical padding on a control selector carries `control-ok` + the reason.
- **Check:** `npm run check:tools-css` (static, part of `npm run check`) fails on `height` / `min-height` / vertical
  `padding` on a control selector (button, `.btn*`, select, input, `.sc-button`, `.sc-icon-button`,
  `.sc-palette-trigger`, `.sc-control`, `.sc-chip-control`, `[role=button]`) in the CSS of every app that doesn't come
  from `--control-height`. `npm run check:tools-ui` (headless Chrome, dev servers + test user) measures every visible
  single-line control on every tool page **and the viewer's main pages** (home, toon page, chapter, account, inventory,
  events, download) at 1440 px (fine pointer) and 390 px (touch) and fails when one differs from the computed
  `--control-height` by more than 1px.

## Close cross (owner rule, 2026-10-02) – every app

> Owner (2026-10-02, screenshot of the reader map's place card "Kloster" with a × at the top right): "Same height as the
> headline and same width. Crosses across the app for closing inside dialogs should be 2 px wide – that is the width of
> the boxes' borders – or 1 px, to match the font width of IBM Plex Mono."

- **One drawn icon** for every close / dismiss / remove cross – dialogs, sheets, drawers, cards, info panels, split-view
  details, overlays (map, menu, camera HUD), toasts, notices, pickers, the × of interactive chips and the remove × of list
  rows – in the viewer, creator, admin, embassy, shop and `@scutoons/auth`: **`CloseIcon`** from `@scutoons/ui` (the
  viewer imports it from there too). Two straight lines corner to corner, **square** (width = height).
- **Height = the headline's**: the cross is as high as the capitals of the headline (or text) it sits next to – the cap
  height of IBM Plex Mono, 0.698 × that text's font-size. `size` = that font-size (default `--type-label-size` 1rem, the
  sheet / panel / card titles), e.g. 1rem headline → a 0.698rem cross (11.2 · 16.75 px at the 16 · 24 px root).
- **Stroke 2px** (`stroke={2}`, the default) = the boxes' 2px border (`--border-width-regular`); **1px** (`stroke={1}`)
  only where the cross sits on thin-line UI – the camera HUD's 1px lines. The stroke stays 2 / 1 CSS px at every size
  (`vector-effect: non-scaling-stroke`); ends are cut square by the icon box.
- **No font glyphs** (× ✕ ✖ `&times;`) as a control's content and **no Feather `x`** (its stroke 2.5 and 24-grid inset
  make a different, smaller cross).
- **The clickable area is the control, not the icon**: `CloseButton` (`@scutoons/ui`: a frameless `--control-height`
  square, ≥ 44px on touch, border-muted frame on hover, `label` → aria-label + title) or the app's own
  `--control-height` button (creator `ghost` row buttons, the viewer's round 2px icon toggles, the camera HUD's framed
  square, a `.sc-chip-control`). The icon is centred in it, and the control's centre sits on the headline line's centre
  (header rows `align-items: center`; the map card positions its absolute close from the title line: card padding + half
  the line − half the control).
- Exempt: none so far. A real need for a glyph (e.g. "3 × item" counts, sizes "390 × 625") is text, not a cross.
- **Check:** `npm run check:tools-css` (part of `npm run check`) fails on a × ✕ ✖ `&times;` glyph as the content of a
  button / link / `Button`, and on `<Icon name="x">`, in the markup of every app (`close-ok` + the reason exempts a line).
