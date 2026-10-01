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
  chapter navigation and the coming-soon page name it the same way (`chapterName`, viewer `lib/utils/chapter-name.ts`). No "featuring" in the reader (owner 2026-09-30: the reader must stay super logical
  and simple; the data – `Chapter.featuring`, the creator's featuring field, `/cast?chapter=` – stays for later).
- **Toon page, top to bottom** (owner 2026-09-30): cover, toon title + description, "Staffel 1: <title>", the season
  text, **the start button**, the chapter list, **the season's music** (a secondary button "Musik herunterladen" /
  "Download the music" → `/<toon>/download`, only when `season.download.music` is set; with `season.download.image`
  the record's cut-out sits left of it, decorative (`alt=""`), lazy, ≈ 77 px on a phone / ≤ 96 px on a tablet, the pair
  centred; owner 2026-10-01), **the credits colophon**.
  Chapters are named only in the chapter list (never in the music or the colophon).
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
- **Top-right icon group (all widths, all pages):** the menu button (round 2rem = 48 px at the desktop root, 3-line icon
  `icons/menu.svg`, `aria-label` "Menü" / "Menu") is the right-most icon and the **only** way to open the slide-out; on
  toon pages inventory, map and chapter list follow to its left (3rem / 5.5rem / 8rem from the right). The slide-out's
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
  video "Roar Little Kitty"): instead of the muted in-view loop a video to watch – sound, native controls (fullscreen
  through them), no loop, `playsinline`, `preload="metadata"`, the slice image as poster. **Autoplay** (owner 2026-10-01,
  the reader only: `autoplayPlayer`, the creator's preview stays still): once half of it is in view it starts by itself,
  with sound when the browser allows it (a click earlier in the same document: the in-app way from the hero's "Trailer
  ansehen ▶", the start button or the chapter list), otherwise muted with a centred "Ton an" / "Sound on" button (primary style, `player.sound_on`)
  that unmutes and keeps playing (unmuting through the native controls hides it too); the `play()` rejection is handled
  (`NotAllowedError` → muted, anything else → nothing; not even muted → the poster + the native play). Reduced motion: no
  autoplay. The slice's
  `width` / `height` are the video's ratio at 390 px (390 × 206 for 1280 × 676 / 2048 × 1080), `object-fit: contain` on
  ink. While it plays the chapter's music pauses and comes back after a pause or the end (bubbling `sliceplayer` event →
  the viewer's audio action → `holdForPlayer`). `SliceVideo.sources [{ src, media? }]` = other encodings as `<source
  media>` before `src` (chapter 0: 480p for `(max-width: 600px)`, 720p for `(max-width: 1400px)`, 1080p as `src`); the
  browser picks once at load by viewport. No adaptive streaming: HLS (e.g. Bunny Stream) may come later.
- **Follow-up bar** (`chapter/next-bar.svelte` on the chapter page, owner 2026-10-01): what comes after the shown group.
  - **When:** the group is done = its last slice was seen (≥ 50 % on screen or its bottom edge, `sliceSeen` in
    track-reading.ts), or – a group ending with a player video – the video **ended** (playing it again hides the bar);
    a VN group additionally needs its script run to the end without a jump (`vn.state.eventsDone`, set by `endOfEvents`)
    and nothing waiting (no message waiting for its click, no choice open). A script that ends with `jumpTo` navigates
    itself, the bar never shows there.
  - **Look:** fixed at the bottom of the viewport, full width, `paper`, a 2px `ink` top rule, IBM Plex Mono, square,
    content max(500px, 30rem) centred; z 110 (above the VN box's 100 – it only shows once the scene finished – under the
    toggles and overlays ≥ 1000); a spacer keeps the last slice clear of it. An optional one-line text, then the
    secondary as a text link (left) and the primary button (ink, "→", right, never wraps). At most 2 actions.
  - **Rules** (`nextStep` in `@scutoons/content` progress.ts + tests, over the season's whole `sequence` – a draft is
    never skipped –, `isItemPublic` for drafts in dev, the membership context):
    | case | text | primary | secondary | countdown |
    |---|---|---|---|---|
    | more groups follow | – | "Weiter →" (`?label=` of the next group) | "↺ Nochmal" | yes |
    | the next chapter readable (preview or member) | – | "Kapitel 2: Ärger →" (`chapterName` + title); from the prologue "S1 K1 lesen →" | "↺ Nochmal" | yes |
    | the next chapter members-only, not signed in | "Kapitel 1 ist für Mitglieder: melde dich an oder löse deinen Code ein." | "Anmelden →" (`/account?return=<chapter>`, back there after the login) | "Code einlösen" (the QR camera = the claim flow) | no |
    | … signed in, no membership | "Kapitel 1 ist für Mitglieder: löse deinen Code ein." | "Code einlösen →" (the QR camera) | "Wo bekomme ich einen Code?" (the shop, like the blocker's "buy access") | no |
    | the next chapter coming soon | "Kapitel 3 erscheint am 1. Dezember 2025" (no date, or a draft not public here: "erscheint bald") | "Zur Übersicht →" | – | no |
    | the season's end | "Staffel 1 geschafft" | "Zur Übersicht →" | "Musik herunterladen" (only with `season.download.music`) | no |
    After a group ending with a player video the replay is "↺ Nochmal ansehen" (the video again in place); "↺ Nochmal"
    scrolls to the top and starts the group again (a VN group runs its script again: the page's `{#key}` gets a replay
    counter). A redeemed code / a login turns the members case into the countdown case by itself.
  - **Countdown** (only next group / next chapter): a 2px rule under the primary fills in **5 s** + "Weiter in 4 s ·
    Abbrechen" in caption type, then it navigates. Stops (the button stays): "Abbrechen", the end scrolled out of view, a
    click anywhere outside the bar (the main menu, a toggle), Escape, an overlay (chapter list, map, inventory), the
    camera or a claim sheet. A hidden tab pauses it (the time away doesn't count). Reduced motion: no countdown.
  - Strings: paraglide `next_bar.*`. **Later idea (not built):** a per-group override `group.next` (e.g. a different
    target or no bar after a cliffhanger) – only when the owner needs it (keep the configuration small).
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
- **Onomatopoeia** — bold uppercase mono, size (rem) / align / rotation per placement, color from `primary-cyan` or the
  acid trio. Variants: `default`, `outline` (8-way white text-shadow), `thorns` (jittered echo + glow), `bubble`
  (white ellipse, 3px currentColor border). Effects: `shiver`, `pulse`, `wobble`.

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

Goal: speech bubbles, captions and narration are **live text over text-free art**, so every locale shares the same
images and only the words change.

### Principles

- The bubble **shape** belongs to the art (exported from Figma `art` frame). The viewer renders only the words.
- Geometry in **% of the slice box** (same convention as triggers/samples) → resolution-independent.
- Font size in **% of slice width** rendered as `cqw` (`.slice { container-type: inline-size }`) → lettering scales
  exactly like the art at any column width (≤ 500px).
- It is comic lettering, not UI chrome: the UI tokens above do not apply, except `ink`/`paper` as default colors.

### Schema (`viewer/src/lib/models/content.ts`)

```ts
export type DialogVariant = 'bubble' | 'caption' | 'narration' | 'sfx';

export interface Dialog {
  id?: string;                 // stable key = Figma node id → i18n/s1c2.json
  rect: { x: number; y: number; width: number; height?: number }; // % of slice (top-left origin)
  text: string | string[];     // array entries = lines / paragraphs
  variant?: DialogVariant;     // default 'bubble'
  frame?: boolean;             // caption/narration: paper box + 2px ink border, default true; false = bare text
                               // on the art (pair with `mask` to cover baked text)
  size?: number;               // font size, % of slice width (Figma px / frame width * 100); default 4
  align?: 'left' | 'center' | 'right';        // default: bubble/sfx 'center', caption/narration 'left'
  valign?: 'top' | 'middle' | 'bottom';        // default 'middle' (needs height)
  font?: 'lettering' | 'mono';                 // default: bubble/sfx 'lettering' (Bangers, OFL), caption/narration 'mono' (IBM Plex Mono)
  weight?: 400 | 700;
  uppercase?: boolean;
  color?: string;              // default ink
  rotation?: number;           // degrees
  mask?: string | true;        // fill the rect (+ `pad`) with a color before drawing text, to cover text that is
                               // baked into legacy art (chapter 1). true = '#ffffff'. Omit for text-free art.
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
  (on-brand panel caption); `narration` — like caption, italic; `sfx` — hands off to onomatopoeia styling.
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
| layer name `#caption` / `#narration` / `#sfx` | `variant` |
| node id | `id` → `i18n/s1c2.json` key |
| `characters` (split on `\n`) | `text` (de source; `en` from i18n file) |

## Creator navigation (owner 2026-09-30)

The creator is a hierarchy: **Creator → toon → season → chapter → group (scene) → slice**, plus the platform-wide
**Universe** (characters, moods, wanted images). Editing deeper must never change the frame around you.

- **One top bar, the same on every page.** It holds only the breadcrumb of the full path
  (`Creator / Cat Pope / Season 1 / Chapter 2 / scene-14 / Slice 3`) and, on wide screens, the signed-in account.
  Every crumb is a link. The structure never changes between pages, only the depth (fixed 64 px; the middle crumbs
  shrink first, "Creator" and the current level stay whole; burger for the sidebar below 768 px).
- **Left sidebar = the children of the current level:** home → toons; toon (and its cast / credits / settings) →
  seasons + Cast + Credits + Toon settings; season → its chapters with the status per language; chapter → Meta,
  Audio, groups → slices (the chapter tree); draft preview → its groups. A leaf page shows its siblings with the
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
  not above the fields. Buttons and links carry no decorative arrows; icon-only buttons (reorder, remove, prev/next
  slice) have an accessible label.

Implementation: `creator/src/lib/nav.svelte.ts` (`CreatorNav` context: route crumbs from the root layout, `extra`
crumbs + `sidebar` + `fill` registered by a page with `usePageNav`), root layout = `AppShell` with the top bar and
sidebar (`nav-sidebar.svelte` for the default levels), `up-link.svelte` in every ActionBar.
