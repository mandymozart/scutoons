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
- **ChapterItem** — grid `6rem auto 3rem` (`5rem …` < 768px), hover `surface-hover`, meta in `caption` + `text-muted`,
  coming-soon row at 0.5 opacity. No "featuring" in the reader (owner 2026-09-30: the reader must stay super logical
  and simple; the data – `Chapter.featuring`, the creator's featuring field, `/cast?chapter=` – stays for later).
- **Toon page, top to bottom** (owner 2026-09-30): cover, toon title + description, "Staffel 1: <title>", the season
  text, **the start button**, the chapter list, **the season's music** (a secondary button "Musik herunterladen" /
  "Download the music" → `/<toon>/download`, only when `season.download.music` is set), **the credits colophon**.
  Chapters are named only in the chapter list (never in the music or the colophon).
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
- **Reading progress** (`reading/track-reading.ts` on the chapter page): a slice counts as read when ≥ 50 % of it is on
  screen (or, taller than the screen, its bottom edge); position = (slices of the earlier groups + furthest slice + 1)
  / the chapter's slices, < 1 until the **last slice of the last group** is reached (= finished). Stored as a
  `Checkpoint { uri: "<toon>/chapter/<n>", isComplete, position }` in the existing game state (localStorage
  `scutoons_game_<toon>`, a member's `/api/progress/<toon>` via the existing last-write-wins sync).
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
