Status: decided 2026-10-16 (owner's answers in §5), built on branch `packing` – how it is now: ITEMS.md "Containers". Drafts: research/packing-drafts.html = https://claude.ai/artifact/Kh9WKKgRNNYn91MXCULmLG

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
4. **Tap a lit target →** moved (`moveWithWarning` – the overfill dialog stays as it is), the mode ends, a status line
   says "Awakening LP liegt jetzt in: DJ-Trolley" (no undo – taking out is one tap in the container view). One item,
   one move – like a Stardew click. Tap a dimmed tile → nothing moves, the line says the reason. ✕ / Escape / the back gesture →
   leave the mode, back to the item's detail.
5. **Desktop extra:** the existing drag onto a container stays; while dragging, the grid shows the same lit / dimmed
   state (one visual language for both).

### 2b. The container view (decided: no bulk button)
Opening a container shows its own grid (`openBox`, "← Zurück", fill). A tap on a thing in it takes it out (Stardew's
chest – owner §5.3), holding takes only some (§5.5); the detail stays one tap away in the normal grid. No "Passt
hinein" section and no "Alles Passende einpacken" (owner §5.1: "one way only"). Furniture's grid stays read-only here
with the line "Möbel räumst du zu Hause ein und aus." – it is filled and emptied in the home view (§5.2).

### 2c. Rules that stay
- Furniture is filled only where it stands (`canFill`, 409 `not_set_up`) and only while the reader is there – in the home
  view (select a thing, tap the rack; owner §5.2). In the inventory's pack mode a shelf is dimmed "Zu Hause einräumen";
  a carried container (a bag) anywhere.
- A container never goes into a container; a worn item can't be packed (409 `worn`) – in the pack mode it isn't
  offered (Einpacken hidden while worn, the detail says "Erst ablegen").
- **Stacks** (owner §5.5): a tap moves the whole entry, but never more than fits – the rest stays; press and hold counts
  1, 2, 3 … faster and faster (Stardew's picker), letting go moves that many (`qty` in `POST /api/inventory/move`). A
  stackable joins the same item's stack where it lands; a part becomes its own entry; a grant joins the loose stack.
- **Guests vs members:** the same UI; members go through `POST /api/inventory/move`, guests change the device
  inventory (`inventory.move` already does both). Nothing new for guests; the import keeps `in`.

### 2d. Accessibility
- Every move works by tap / Enter (SC 2.5.7); drag is only an extra on desktop.
- Entering the pack mode: focus moves to the first lit target; a polite live region says "Einpacken: 2 passende
  Behälter. Wähle einen aus oder Escape zum Abbrechen." Tab moves through the tiles in grid order (the inventory has no arrow-key grid today; `@scutoons/ui` grid-keys.ts could add it).
- Lit targets: `<button aria-label="In DJ-Trolley packen, 12 von 90">`. Dimmed ones stay focusable with
  `aria-disabled="true"` and the reason in their label ("CD-Tasche – nimmt nur CDs"), so a screen-reader user hears why
  – not removed from the tab order.
- After a move: the live region "Awakening LP liegt jetzt im DJ-Trolley", focus goes to the target's tile. Escape leaves the mode and returns focus to the
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

New carried containers (no `furniture`, so filled anywhere; the new category `bag` – §5.6; not stackable, giftable,
virtual, icons placeholder; for now only through the story, `giveItem` – §5.7):
| id | DE / EN | holds | why this size |
|---|---|---|---|
| `cd_case` | CD-Tasche / CD case | `formats: [cd]`, max 25 | owner: 25 CDs, "für Gigs zum Mitnehmen" |
| `dj_trolley` | DJ-Trolley / DJ trolley | `formats: [vinyl]`, max 50 | owner §5.7: 50 (a real one takes up to ~90 – [Magma LP-Bag 100 Trolley][magma-100]) |
| `record_bag` | Plattentasche / record bag | `formats: [vinyl]`, max 25 | owner: ~25; real ones take 35–50 ([Riot LP-Bag 50][magma-riot]) |

The DJ trolley is the owner's exception to "furniture only at home" – it is no furniture, so it is filled anywhere.

## 4. What was built (2026-10-16, branch `packing`)
- Model: `holds.formats`, `fitsIn` by format, `roomIn`, `packBlock` (kind → at home → full) in containers.ts; category
  `bag`; Containers.php `fits()` + `qty` (cap by room, merge, split), CatalogValidator `holds.formats`; schemas; tests.
- Creator: the container field's format checkboxes; `bag` in the item form's categories.
- Reader: one round "Einpacken" icon in the item detail (and a round take-out icon); the pack mode (`pack-mode.svelte.ts`,
  `inventory-grid.svelte`): lit / dimmed tiles with the reason, a tap packs all that fits, a hold counts
  (`hold-count.ts`, pointer + Enter / Space), Escape back to the detail, a polite live region; the container view takes
  out on a tap; drag on desktop moves the same way. Guests the same on the device.
- Catalog: the drafts for `cd_rack`, `cassette_rack`, `record_shelf` and the three bags are saved after the deploy
  (production refuses `holds.formats` until then); the owner publishes.

## 5. The owner's answers (2026-10-16)
1. One way only: the item-first pack mode. No "Alles Passende einpacken".
2. A shelf at home is a target only while you are at home (the home view) – you have to go there.
3. In the container view a tap moves the thing (out); the detail stays one tap away in the normal grid.
4. Exactly one valid target: still the pack mode (predictable) – the reader taps it.
5. Stacks: the whole entry by default, plus Stardew's press-and-hold counter; never more than fits, the rest stays.
6. Category for carried containers: `bag` (agent's recommendation – it reads clearer for filters and shop grouping and
   costs one enum value: the type, the schema, the item form's list).
7. Capacities: DJ trolley 50 LPs, record bag 25 LPs, CD case 25 CDs; for now only through the story (`giveItem`) – shop,
   QR codes, recipes later.
8. No 7″ singles for now – nothing to model.

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
