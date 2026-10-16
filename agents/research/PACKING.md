Status: draft, owner to review (2026-10-16) – nothing built. Mockup: research/packing-drafts.html = https://claude.ai/artifact/Kh9WKKgRNNYn91MXCULmLG

# PACKING — putting items into containers (one small pack action, format rules)

Owner 2026-10-16: rethink how items go into other items. Today the item detail's sticky bar shows one big text button
per fitting container ("In CD-Turm legen", "In Kassettenregal legen", "In Plattenregal legen" –
`container-actions.svelte`), and the CD tower and cassette rack take the LP because they hold `categories: ["media"]`.
Wanted: one small pack action, a pick-the-target mode in the inventory (only valid targets lit, the rest dimmed with
the reason), the way back out, and format rules (CDs → CD case / CD tower, LPs → DJ trolley / record bag / record
shelf, cassettes → cassette rack).

## 1. Research (what fits a touch-first reader)

**Stardew Valley – chests.** On PC the chest opens as two grids (the chest above, the backpack below) and one click on
a stack moves it to the other side; right-click takes one, Shift + right-click half a stack ([Controls][sdv-controls];
players describe items that "just pop between" inventory and chest – [Steam thread][sdv-steam]). Since 1.4 an
**"Add to existing stacks"** button moves every backpack item the chest already holds in one go, and chests can be
stacked into chests ([Chest][sdv-chest]); a chest holds 36 kinds, 999 per slot. On the Switch, Y moves one, holding Y
several ([Controls][sdv-controls]). On mobile there is no stack button: you **drag**, and a long press opens a counter
for part of a stack ([Mobile Controls][sdv-mobile], [forum][sdv-ios]). Lessons: (a) the one-tap move works because the
**target is already open** – the chest view is the context; (b) a "put all that belongs here" button is the big time
saver; (c) the mobile port's drag + long-press counter is the weakest part (fiddly, hard to hit exact numbers).

**Minecraft – bundle.** Two directions, both click-click: hold the item and click the bundle, or pick up the bundle and
click the items to add; right-click takes the last one out, a fullness bar on the icon turns red when full
([Minecraft Wiki – Bundle][mc-bundle]). A shulker box never goes into a bundle. Lessons: **container-first and
item-first** are both natural; the fill on the icon is the feedback; a container-in-container rule is normal.

**Animal Crossing: New Horizons – storage.** Pick an item in the pockets → a menu → "Put in Storage", one item at a
time; players have asked for multi-select since 2020 ([Twinfinite][acnh-qol], [GameRevolution][acnh-store]). Lesson:
item → menu → one target is calm and readable but slow for twenty records – we need a bulk path too.

**Accessibility – WCAG 2.2 SC 2.5.7 Dragging Movements (AA).** Every drag needs a single-pointer alternative (tap,
tap) ([W3C Understanding 2.5.7][wcag-257]). Our desktop drag onto a container already exists; the pack mode is that
alternative – drag stays an extra.

**What fits SCUTOONS.** Phones first, one hand, no right-click, no hover. So: tap-then-tap (Stardew PC / Minecraft) as
the main flow, never a drag on touch; a container view that works like Stardew's chest for many items; a "put all
fitting" button for bulk; drag on desktop as an extra.

## 2. The flow

### 2a. Item first – the pack mode (the main flow)
1. **Item detail:** the per-container buttons go. The bottom row gets ONE round icon button among the existing icons
   (download, combine, wear, gift): **Einpacken** (Feather `package`, label on hover / for screen readers). It shows when
   `containersFor(entry)` would find anything ignoring the set-up and capacity rules (so a reader sees why it can't go
   anywhere), i.e. when any of the reader's containers takes this format / category / id. An item that lies in a
   container gets a second round icon **Herausnehmen** (`corner-up-left`) instead – one tap, no mode, it lands loose in
   the inventory.
2. **Tap Einpacken →** the detail closes, the inventory grid shows in **pack mode** – the same pattern as today's
   combine mode (`combineWith` + `.combine-bar`): a bar on top "Wohin mit *Awakening LP*?" + ✕. The item's own tile is
   marked (`.source`).
3. **Targets:** containers that take it now are **lit** (the cyan glow of `.drop-target`, their fill "12 / 90" stays);
   everything else is **dimmed**. A dimmed container carries its reason as a one-line caption under the tile instead
   of its name: *Nur CDs* · *Nur Kassetten* · *Erst zu Hause aufstellen* · *Voll (25 / 25)*. Non-containers are just
   dimmed, no caption. Order of reasons: format/kind first, then set-up, then full.
4. **Tap a lit target →** moved (`moveWithWarning` – the overfill dialog stays as it is), the mode ends, a short toast
   at the bottom: "In DJ-Trolley gepackt · Rückgängig" (5 s; undo = move back to where it was). One item, one move –
   like a Stardew click. Tap a dimmed tile → nothing moves, the toast says the reason. ✕ / Escape / the back gesture →
   leave the mode, back to the item's detail.
5. **Desktop extra:** the existing drag onto a container stays; while dragging, the grid shows the same lit / dimmed
   state (one visual language for both).

### 2b. Container first – the chest view (the bulk flow)
Opening a container already shows its own grid (`openBox`, "← Zurück", fill). It becomes Stardew's chest: **its
contents above, a rule, then "Passt hinein" – the reader's loose items that fit below** (only fitting ones; the
section is left out when none fit). Tap an item below → in; tap one above → out (no detail; a small ⓘ or long-press
opens the detail if the owner wants one – open question 3). A round icon in the container bar, **Alles Passende
einpacken** (Feather `download`), moves every fitting loose item in, up to the capacity, oldest first – the "Add to
existing stacks" of SCUTOONS. Furniture not set up opens read-only with the line "Erst zu Hause aufstellen – dann
kannst du es füllen." (a carried shelf shows its contents if it has any, e.g. a moved-in shelf).

### 2c. Rules that stay
- Furniture is filled only where it stands (`canFill`, 409 `not_set_up`); a carried container anywhere. Placed
  furniture is listed in the inventory grid as today, so a shelf at home is a target from anywhere (owner to confirm –
  open question 2).
- A container never goes into a container; a worn item can't be packed (409 `worn`) – in the pack mode it isn't
  offered (Einpacken hidden while worn, the detail says "Erst ablegen").
- **Stacks:** an entry moves as a whole (the server moves the entry with its `qty`). Records, CDs and tapes are not
  stackable, so for the music carriers nothing changes. For a future stackable that a container takes, the move would
  need a split (`qty` in `POST /api/inventory/move`) – not now (open question 5).
- **Guests vs members:** the same UI; members go through `POST /api/inventory/move`, guests change the device
  inventory (`inventory.move` already does both). Nothing new for guests; the import keeps `in`.

### 2d. Accessibility
- Every move works by tap / Enter (SC 2.5.7); drag is only an extra on desktop.
- Entering the pack mode: focus moves to the first lit target; a polite live region says "Einpacken: 2 passende
  Behälter. Wähle einen aus oder Escape zum Abbrechen." Tab moves through the tiles in grid order (the inventory has no arrow-key grid today; `@scutoons/ui` grid-keys.ts could add it).
- Lit targets: `<button aria-label="In DJ-Trolley packen, 12 von 90">`. Dimmed ones stay focusable with
  `aria-disabled="true"` and the reason in their label ("CD-Tasche – nimmt nur CDs"), so a screen-reader user hears why
  – not removed from the tab order.
- After a move: the live region "Awakening LP liegt jetzt im DJ-Trolley", focus goes to the target's tile; the toast's
  Rückgängig is a real button (reachable by Tab, stays while focused). Escape leaves the mode and returns focus to the
  item's tile.
- The lit / dimmed state is never colour only: lit = cyan glow + a 2 px ink frame, dimmed = 35 % opacity + the
  caption. Reduced motion: no pulse on the targets.

## 3. Format rules – the data model

### 3a. `holds.formats`
`ItemHolds` gets **`formats?: MediaFormat[]`** (`vinyl | tape | cd`, `albums.ts`). An item fits when its category OR
its id is listed OR **its `format` is listed** – one more OR, the rule stays "a simple rule we can edit"
(SCORES.md §2). `holds` needs at least one of `categories`, `items`, `formats`.

- `shared/content/src/containers.ts`: `ItemHolds.formats`, `fitsIn` reads `item.format` (its `Pick` gains `format`),
  `validateContainers` (`formats` ⊆ `MEDIA_FORMATS`, the "needs categories or items" error names formats too); a new
  `whyNot(container, containerEntry, item, count)` → `null | 'kind' | 'not_set_up' | 'full'` for the captions
  (`canFill` + `fitsIn` + `moveInCheck` in one place, shared by the reader's pack mode and the chest view).
- `server/src/Domain/Inventory/Containers.php` `putIn` + the craft check at line ~126: the same OR with
  `$item['format']` (one private `fits()` for both).
- `server/src/Domain/Catalog/CatalogValidator.php`: `holds.formats` ⊆ `MEDIA_FORMATS`.
- Schemas: `catalog.schema.json` (server/schema/content + shared/content/schema) `holds.formats`.
- `shared/catalog/src/components/container-field.svelte` (the creator's item form): a "Formats" chip row (Vinyl · CD ·
  Kassette) next to categories and items; messages de/en.
- `viewer` inventory context: `containersFor` unchanged (it calls `fitsIn`); the pack mode uses `whyNot`.
- Tests: containers.test.ts (format fit, validation), PHP `ContainersTest` (an LP into the CD tower → 409
  `does_not_fit`), catalog-validator test.
- No migration: no stored format changes – `holds` lives in the catalog documents; the existing three keep working
  until edited.

### 3b. Catalog changes (data, edited in the creator; drafts over the MCP, the owner publishes)
| item | today | proposed |
|---|---|---|
| `cd_rack` CD-Turm (furniture) | `categories: [media]`, max 50 | `formats: [cd]`, max 50 |
| `cassette_rack` Kassettenregal (furniture) | `categories: [media]`, max 50 | `formats: [tape]`, max 50 |
| `record_shelf` Plattenregal (furniture) | `items: [awakening_vinyl, awakening_test_press]`, max 50 | `formats: [vinyl]`, max 50 (every future LP fits without editing the shelf) |

New carried containers (no `furniture`, so filled anywhere; category `tool` like the players – or a new category
`bag`, open question 6; not stackable, giftable, virtual, icons placeholder):
| id | DE / EN | holds | why this size |
|---|---|---|---|
| `cd_case` | CD-Tasche / CD case | `formats: [cd]`, max 25 | owner: 25 CDs, "für Gigs zum Mitnehmen" |
| `dj_trolley` | DJ-Trolley / DJ trolley | `formats: [vinyl]`, max 90 | a real LP trolley takes about 90 LPs ([Magma LP-Bag 100 Trolley: 90 × 12″][magma-100]) |
| `record_bag` | Plattentasche / record bag | `formats: [vinyl]`, max 25 | owner: ~25; real ones take 35–50 ([Riot LP-Bag 50][magma-riot]) |

How a reader gets them (shop product, recipe, story grant) is open (question 7). The DJ trolley is the owner's
exception to "furniture only at home" – it is no furniture, so it is filled anywhere by the existing rule.

## 4. Implementation plan (after the owner's yes; one session, viewer + shared + server)
1. **Model** – `holds.formats` in containers.ts + `whyNot`, Containers.php `fits()`, CatalogValidator, schemas, tests.
2. **Creator** – container-field formats chips; then drafts for `cd_rack`, `cassette_rack`, `record_shelf` and the
   three new items over the MCP (`save_draft item:<id>`), link to the owner to publish.
3. **Reader: pack mode** – `inventory.svelte` gets `packing: Entry | null` next to `combineWith` (the same bar, the same
   `pick()` branch), lit / dimmed tiles + captions, the toast with undo (a small `pack-toast.svelte` or the existing
   status line), live region; `container-actions.svelte` shrinks to the two icon buttons (Einpacken / Herausnehmen) +
   the open-container button; drag shares the lit / dimmed state.
4. **Reader: chest view** – the open container's grid gets the "Passt hinein" section and the "Alles Passende
   einpacken" icon (sequential `inventory.move` calls, stop at `full`, one summary toast).
5. **Checks** – `npm run check`, server PHPUnit + PHPStan, viewer build, the inventory browser check on phone + desktop
   (agent@scutoons.test with an LP, a CD case, a set-up shelf). ITEMS.md "Containers" updated (formats, the two flows).
Messages (de/en): `items.pack`, `items.unpack`, `items.pack_where`, `items.pack_done`, `items.pack_undo`,
`items.pack_why_kind` ("Nur {formats}"), `items.pack_why_not_set_up`, `items.pack_why_full`, `items.pack_all`,
`items.pack_fits_here`.

## 5. Open questions for the owner
1. Both flows (item-first pack mode **and** the chest view with "Alles Passende einpacken"), or only the pack mode?
2. A shelf standing at home: a target from anywhere (you're on the map, the LP goes on the home shelf), or only while
   you're at home (inside `HomeView`)? Today's rule allows anywhere once it stands.
3. In the chest view, does a tap on an item move it (Stardew) or open its detail (as everywhere else)? Proposal: move;
   the detail stays one tap away in the normal grid.
4. Exactly one lit target: still ask (predictable, proposed) or pack straight away with the undo toast?
5. Stacks: whole entry only (proposed – no stackable goes into a container today), or a counter later?
6. Category for the carried containers: `tool` (like the players) or a new `bag`?
7. Capacities 25 / 90 / 25 and how readers get the CD case, DJ trolley and record bag (shop, recipe, story).
8. 7″ singles later: a `vinyl.format` (`lp12` today) could narrow a shelf to 12″ – not modelled now (keep the
   configuration small).

[sdv-controls]: https://wiki.stardewvalley.net/Controls
[sdv-mobile]: https://wiki.stardewvalley.net/Mobile_Controls
[sdv-chest]: https://wiki.stardewvalley.net/Chest
[sdv-steam]: https://steamcommunity.com/app/413150/discussions/0/2445965959027011970
[sdv-ios]: https://forums.stardewvalley.net/threads/no-add-to-existing-stacks-button-for-chests-on-ios.2931/latest
[mc-bundle]: https://minecraft.wiki/w/Bundle
[acnh-qol]: https://twinfinite.net/2020/05/13-qol-changes-for-animal-crossing-new-horizson/
[acnh-store]: https://www.gamerevolution.com/?p=640298
[wcag-257]: https://www.w3.org/WAI/WCAG22/Understanding/dragging-movements
[magma-100]: https://djcity.com.au/product/magma-lp-bag-100-trolley-fits-90-12-records-40500
[magma-riot]: https://frontend-rel.gak.co.uk/en/magma-lp-100-trolley-black/124128
