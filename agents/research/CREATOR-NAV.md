Status 2026-10-20: draft D chosen (§2a, issue #92) and built (agents/DESIGN.md "Creator navigation"); open: the dashboard's Problems (needs a server summary, Q8), the chapter tree's own fold; questions 3–9 open (built as drafted: Q3 squares in the bars, Q4 the areas, Q7 "Toons").

# CREATOR-NAV – the creator's top bar, menu and dashboard

Owner 2026-10-20 (paraphrased): the top bar takes too much room and its icons carry too much ornament (the round framed
buttons, the logo); "starting from the top level I already get confused"; the universe overlaps the toons; on phones
the menu icon pushes the logo aside – "that is all weird"; use the reader's logo to save room; a proper creator
dashboard; toons don't need to unfold (they are basically stories), characters are characters, the map is the map –
so the menu lists areas.

Pictures: `creator-nav/index.html` (now + drafts A, B, C at 1440 × 900 and 390 × 844; each draft's HTML takes `#dash`,
`#toon`, `#slice`, `#toon-menu`). Screenshots: `node agents/research/strip-editor/shot.mjs <abs dir> "a.html#dash:1440x900:a-dash-1440.png"`.

## 1. Review – what confuses and why

Measured on the local stack (test user, catpope, 2026-10-20; `+layout.svelte`, `nav-sidebar.svelte`, AppShell):

| | 1440 (root 16.8px) | 390 (root 14px) |
|---|---|---|
| header | 50.4px (3rem) | 48px **+ crumb row 53px = 101px** |
| ActionBar | 50.4px | 48px → chrome 149px = 18 % of 844 |
| sidebar | 12rem; the fold button has its own row (~4rem before the first entry) | drawer 18rem |
| logo | the wordmark at 1.25rem: 54 × 21px, hard to read | 45 × 44px, pushed right by ☰ |
| top bar icons | 38px circles with a 2px frame (Reader, Flow, Search, Bell) + a framed language box + the toon pill (avatar + gear) – three shapes in one row | 44px circles |

1. **The sidebar is not a menu, it is "the children of where you are"** (DESIGN "Creator navigation", 2026-09-30). At
   the start it lists the toons + Universe; inside a toon it swaps to Seasons + Meta; inside a season to chapters; inside
   a chapter to the tree. The areas vanish as soon as you go one step in, so the left side never means the same thing
   twice – and the start page's toon list is the sidebar's toon list once more.
2. **Universe only on the start page** (owner 2026-10-03): from a toon or from Characters itself there is no way to the
   Map or the Catalog except home or Ctrl K. On Characters the sidebar shows **Toons** – the page you are on is not in it.
3. **"Universe" mixes kinds:** Characters, Moods and Wanted images are three entries but one page (`/characters?tab=`);
   Map, Catalog and Flow sit next to them. A toon has its own **Cast** (= characters again) under "Meta" – and "Meta" is
   also the chapter's first entry. Characters live in two places with two names.
4. **The toon unfolds into levels that are mostly one deep:** Cat Pope has one season, still every chapter path reads
   SCUTOONS › Cat Pope › Season 1 › Chapter 2 › scene-17-24 › Slice 20. On a phone that collapses to "… › scene-17-24 › S".
5. **Phones: two rows on top.** ☰ + logo + spacer + Search + Bell, then the path in a second row (and, since #44 2d, the
   editor's ⚠ · Save · Publish · ••• at its end). ☰ sits before the logo, so the logo moves with the menu button.
   The drawer then starts with a fold button row and the language / Flow / Reader buttons before any navigation.
6. **Ornament:** every icon has a 2px circle, the language a box, the toon a pill; the crumbs are underlined links. The
   picture is busy although each part follows a rule.
7. **No place for "what next":** the start page lists toons only. What is not live yet, what has problems, what waits
   for review lives on each page.

## 2. The drafts

All three share: **the reader's square mark** (`viewer/static/SCUTOONS-logo-animated.svg`) as a full-height cell at
the top left instead of the wordmark crumb; **the path** without underlines (ancestors muted, the current place ink, `/`
between); **borderless icon squares** in the bars (as phone-e3: equal squares, hover = the nav outline, on = inverted;
an idle button keeps its square, hidden); **one row on phones** (48px instead of 101px – the strip editor gains 53px:
logo · path · ⚠ · Save · Publish · •••, exactly #44 2d's row with the logo in the ☰'s place); **the dashboard** as home.
They differ in where the areas live.

- **A – area rail + context list.** A 3rem icon rail (Dashboard, Toons, Characters, Map, Catalog, Flow; Search,
  Notifications, account at its foot), the logo in its top cell; next to it today's sidebar (the children of the place)
  where the place has some. Phones: the logo opens the menu – the rail sideways on top, the place's list below.
  + smallest change (AppShell gets a rail; `nav-sidebar` stays). − keeps "the sidebar changes meaning" (point 1);
  icons without words for the areas.
- **B – one menu by areas; pages own their trees.** The left menu lists the areas with words; under Toons the toons
  flat (never deeper). A toon's pages are tabs (Stories · Cast · Script · Credits · Settings – the toon pill goes); the
  chapter tree is the chapter editor's own column (DOCKING allows a page to split itself). The menu folds to a rail by
  hand (Ctrl \\) – folded it is A's rail. Phones: the logo opens the same menu.
  + the menu always means the same; matches "the menu lists areas"; one list for desktop and phone. − the editor has
  menu + tree + canvas + inspector at 1440 (fold the menu); every page with children needs its tabs / column.
- **C – like the reader.** No left menu: the logo is home (the dashboard), the areas are words in the top bar (the
  current one inverted), the trees belong to the pages (as B). Phones: ☰ on the right like the reader's, the menu
  slides in from the right with the areas and "Recent".
  + the most room; creator and reader feel like one product. − on a phone the ☰ takes a square from the editor's row
  (the path shrinks to "scene-…"); area tabs + path + actions are crowded below ~1280.

**Recommendation: B** – it says what the owner said (areas, toons don't unfold, characters in one place), its folded
state is A's rail, and its phone row is #44 2d's row as it is.

## 2a. Chosen: D (owner 2026-10-20)

"von A mag ich die Bar an der linken Seite … von B mag ich das Menü auf dem Desktop links … immer auch da bleibt … man kann
das ja dann einfahren und dann wird es zum Area Rail. Und von C mag ich das Mobile. Das ist einfach am saubersten."

- **Desktop = B:** the menu by areas on the left, always there, the place you are in marked (the toon under Toons); the
  toon's pages are tabs, the chapter tree the editor's own column.
- **Folded = A's rail:** the menu folds by hand (Ctrl \\, remembered) to the 3rem icon rail; its foot unfolds it.
- **Phones = C:** the logo is home (the dashboard), ☰ on the right like the reader's, the menu slides in from the right
  with the areas and "Recent"; the strip editor's row is logo · path · ⚠ · Save · Publish · ••• · ☰ (the path gets less
  room than in #44 2d's row – to watch when it is built).
- Pictures: `creator-nav/d.html` (`#dash`, `#toon`, `#toon-folded`, `#slice`, `#slice-folded`, `#toon-menu`).

**The dashboard** (the same in all three; desktop three columns, phone one): **Continue** (the last place edited, with
its picture, one Open), **Not live yet** (drafts newer than live: chapters, characters, map places), **Problems** (per
document, errors / warnings counts – opens the page with Problems docked), **Waiting for you** (open reviews, Wanted
images), **Toons** (avatar · name · state). The ActionBar keeps Publish (everything) · Reader.

## 3. What changes where (D)

- `creator/src/routes/+layout.svelte`: the header (mark cell, path, squares; one row on phones: the logo = home, ☰ on the
  right opens the areas from the right), the sidebar = the areas menu (folded: the rail); `nav-sidebar.svelte` → an areas menu; the start route → the dashboard.
- `@scutoons/ui` AppShell: none for the menu (it is the sidebar); Breadcrumbs: a `root`-less variant, no underline.
- Pages: the toon page gets its tab row (Cast, Script, Credits, Settings move from the sidebar); the season page and the
  chapter editor render their tree in a page column instead of `usePageNav({ sidebar })`.
- Docs + decisions superseded: DESIGN "Creator navigation" (sidebar = children; logo = root crumb; burger), the
  2026-10-03 "Universe only on the dashboard", the 2026-10-12 start page and toon pill; RULES "Round vs square" for bar
  icons (a bar is a group → square, as the rule already says for toolbars – to confirm).
- Data for the dashboard: "Continue" = a per-member last place (local first); "Not live yet" from the content index
  states; "Problems" across documents needs a server summary (today they are computed per open page) – the biggest cost.

## 4. Questions to the owner

1. ~~Which draft~~ – answered: D (§2a).
2. ~~Phones~~ – answered: ☰ on the right like the reader (C).
3. **Bar icons:** borderless squares instead of the round framed buttons, in the top bar and the ActionBar?
4. **The areas:** Dashboard · Toons · Characters (Moods, Wanted images as its tabs) · Map · Catalog · Flow – right?
   Where do the style guide, notifications and the account go (B: the menu's foot)?
5. **Cast:** keep it as a toon tab, or is it the Characters page filtered to the toon (one place for characters)?
6. **Seasons in the path:** show "Season 1" only when a toon has more than one season?
7. **The word:** "Toons" or "Stories" for the area?
8. **The dashboard:** are Continue · Not live yet · Problems · Waiting for you · Toons the right five? "Problems across
   everything" needs a server summary – worth it now or later?
9. **When:** the chapter 3 release comes first. Proposal: step 1 before it (small, one file): the phone header in one
   row with the reader's logo + borderless squares; step 2 after it: the areas menu + tabs; step 3: the dashboard.
