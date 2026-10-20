Status 2026-10-20: draft E (B's cards + C's editor) after the owner's first look, to confirm (issue #104) – nothing built beyond the small fixes (commit "Characters page: layout fixes").

# CHARACTER-PAGE – the creator's Characters area inside navigation D

Owner 2026-10-20: "fix and layout the character page better … a redesign Character Page include the new navigation
concepts and design ideas for desktop and mobile of creator and editor view". Frame: navigation D (CREATOR-NAV.md §2a) –
the menu by areas on the left (Characters marked); phones: logo = home, ☰ right, one row of functions with ‹ at the
bottom (#44), long press = context.

Pictures: `character-page/index.html` (A, B, C at 1440 × 900 and 390 × 844; `a.html#list`, `#char`, phones also
`#char-sheet`, `#char-press`). Screenshots: `node agents/research/strip-editor/shot.mjs <abs dir> "b.html#char:1440x900:b-char-1440.png"`.

## 1. Today (local stack, test user, 1440 + 390)

- **One long page:** list (20rem) | the character = identity fields, Import images, persona tabs, the 3D model, look
  tabs, the moods grid (18 moods × 3 views), the A-pose, transitions, wanted images (32 file names), prompts – about
  3600px at 1440, 5200px on a phone. Moods (the vocabulary) and Wanted images are tabs above the list.
- **Fixed now** (Refs #104): phones one field per row (two columns cut the selects), the colour row 7px too wide, the
  3D preview's frame 4px over, the persona / look add fields cut their placeholder, the list row named its own toon
  twice, px → rem (the list height, two gaps).
- **Left for the redesign:** the page tabs wrap to two rows on a phone; the toon's Cast (`/<toon>/cast`) is a second
  place for characters (CREATOR-NAV question 5); no place yet for the **Kanon** texts per character / persona / look
  (KANON.md, built after the release); "Remove" is a red text link at the top; the wanted list is a wall of file names.

## 2. Shared by all three

- **The area's own tabs:** Characters · Moods · Wanted images (D: "a toon's pages are tabs" – the same for an area);
  the path is `Characters / Katzenpapst [/ persona / look]`.
- **Persona and look pickers with faces** (the persona's avatar beside its word); `+` a dashed square at the end.
- **The moods grid** stays; the three views as dots on each picture (avatar · portrait · full, ○ = missing); "All
  views" is the list square next to it; Import images an icon square in the grid's head.
- **Kanon:** a card per character, per persona and per look (KANON.md headings: Backstory · Who · Look · Rules; a
  look: Look · When) with its state (not written / draft / published). Until KANON is built the card is left out.
- **Delete / Duplicate** go into `•••` at the head; the ActionBar stays (Problems · Save · History · Publish · More).
- **Phones:** list → tap → the character with the bottom row `‹` + icon squares; a long press on a mood or a row =
  its context (Replace image · Copy file name · Add to wanted images · Remove).

## 3. The drafts

- **A – list column + section tabs.** Overview = a table (avatar · name · toons · persona faces · counts · state). An
  open character keeps a 17rem list on its left; the character is split into tabs: Profile · Looks & moods · Kanon ·
  Wanted images · Prompts. Phone: those tabs are the bottom row.
  + nearest to today (the same components, a shorter page); switching characters stays one click. − tabs hide how
  persona, look and Kanon belong together; a persona's / look's Kanon ends up inside "Looks & moods" anyway.
- **B – gallery + the character's own tree (recommended).** Overview = cards (the neutral avatar big, toons, persona
  faces, state). An open character has a 15rem tree in the page: the character (Profile) · Personas → their looks ·
  Wanted images · Prompts – "trees belong to their pages", as D does for chapters. The picked node fills the rest: a
  look = its moods grid + its Kanon, 3D model and A-pose on the right; a persona = its fields + Kanon + model; the
  character = the profile fields + its Kanon. Phone: 2-column cards; the character is its tree as a list (drill in, ‹).
  + one place per Kanon text (the node it belongs to); matches D (areas menu, the page owns its tree); the shortest
  pages. − switching characters goes back to the gallery (or Ctrl K); one level more on phones.
- **C – stage + inspector.** Overview = rows grouped by toon (answers "Cast" without a second page). An open
  character is an editor like the slice editor: persona / look pickers on top, the current mood's full figure on a
  stage, the moods as a strip under it, the right dock an inspector with icon tabs (Profile · Kanon · Model · Wanted ·
  Prompts). Phone: the stage full width, the strip, the bottom row opens sheets (the #44 pattern).
  + the picture first – good for checking art; the same model as the slice editor. − the grid of all moods becomes a
  second mode (the grid square); the most new code.

## 3a. E – B's cards + C's editor (owner 2026-10-20)

"ich mag c in mobile schon gern. in b find ich die Kartenansicht gut auf der characters page" · "mir ist noch nicht
klar wie ich die verschiedenen avatar, portrait, full anwähle" · "looks und moods ist irgendwie verwirrend. weil wir
haben personas, looks, moods".

- A picture has four axes: **persona** (who: Regular, Mensch, Katze …) › **look** (what the persona wears) › **mood**
  (the face) · **view** (avatar = head, portrait = bust, full). E gives each axis ONE control: the persona faces row,
  a look select (most personas have one look), the moods strip under the stage, a segmented switch Avatar · Portrait ·
  Ganz. The stage names what it shows ("Regular › päpstlich › neutral · Ganz"); no "Looks & moods" anywhere.
- Overview = B's cards; the character = C's stage + inspector (desktop) and C's phone (bottom row, sheets).
- Round 2 (owner 2026-10-20: "the text field could cover the entire panel" · "anything sliding sideways … that
  transparent gradient overlay" · "the padding inside the list and the container full between the page dividers" ·
  "avatar, portrait, ganz, maybe create some symbols … those buttons are huge" · "i hope our stage is finger
  zoomable"): the Kanon is ONE text field filling its panel (phone: the sheet's head is the drag row – the chapter
  editor's phone inspector sheet, dragged to any height; desktop: the inspector's Kanon tab); the persona row and the
  moods strip run edge to edge, their padding inside the scroller, a fade at the edge that has more; the views are
  three icon squares (face · bust · figure, the word in the tooltip), the bottom row's "All moods" is the grid icon;
  the stage zooms like the slice canvas (pinch / Ctrl + wheel, double tap = fit).
- Pictures: `e.html` (`#list`, `#char`, `-avatar` / `-portrait`, phones `-sheet`); `CHARACTER-PAGE-drafts.html` = all
  drafts in one file with the pictures inside (opens in the app's browser pane).

## 4. Questions to the owner

1. **Which draft** – A, B or C (or parts of them, e.g. B's tree with C's stage for a look)?
2. **Overview:** cards (B), a table (A) or rows grouped by toon (C)?
3. **Cast:** does the toon's Cast page go (the Characters list filtered to the toon, or C's grouping) or stay a toon tab?
4. **Kanon:** a card next to the moods (B) or its own tab / sheet (A, C)? (Built after 2026-11-15; the layout keeps
   its place now.)
5. **When:** proposal – the small fixes now (done), the redesign after the chapter 3 release (2026-11-15), together
   with D's step 2 (the areas menu + tabs).
