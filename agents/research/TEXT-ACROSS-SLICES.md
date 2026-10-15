Status 2026-10-15: investigation only, nothing built – the owner: "not important for right now"; FEATURES 10.100.

# Text boxes across slice edges

Owner 2026-10-15: "sometimes boxes … are cutting over the end of slices. we dont have a proper rendering for something
like this yet" · "having text boxes pass is good as a tool" · "eventually i want to not have to worry about slices,
they will become a background thing". Seen in chapter:catpope:2 (slices 39, 84, 92 – moved up by hand, draft 641).

## How it is now
- A text box's `rect` is in % of ITS slice. `.slice` (shared/reader/src/slice.svelte) has `container-type: inline-size`
  (font sizes in cqw) – that makes every slice a stacking context, so its text boxes can't paint above a neighbour.
- Slices stack in DOM order (viewer `renderer/vertical.svelte`): a later slice paints over an earlier one.
  - A box running past its slice's **top** (negative `rect.y`) shows over the slice above. Chapter 1 relies on this:
    its seam-crossing boxes (4/5, 5/6, 11/12, 12/13, 14/15, 74/75, 86/87) live in the LOWER slice
    (archive/MEMORY.md "Boxes crossing a slice seam").
  - A box running past its slice's **bottom** is painted UNDER the next slice's image: cut off. That's the bug.
- Sideways the slice clips (`overflow-x: clip`, BACKLOG M.2); the validator warns about the side only
  (validate.ts checkDialog). The creator canvas edits one slice at a time, so a crossing box just runs out of it.

## Options
1. **Lift a crossing slice** (small, the reader only): a slice with a box past its bottom gets `z-index: 1` on `.slice`
   (a helper in content, used by `Slice`, so viewer and creator agree). Its box then paints over the next slice; top
   crossers keep working (same z → DOM order). One seam crossed from both sides at once still conflicts (the lower
   slice wins, as today) – rare; a validator warning for that case only. No data change.
2. **Text boxes per group, not per slice** (the owner's direction: slices become the background): a group is one tall
   strip; text boxes / items get a rect in the group's coordinates (y in % of the group width, so the strip's height
   doesn't matter) and paint in one layer above all slice images. Slices = background images only. Needs: a stored
   format change (migration: slice rect → group rect, chapter 1's negative-y boxes become plain), the creator canvas
   editing a group strip instead of one slice, events / items / areas following, the VN and horizontal / strip
   renderers deciding what a group rect means there. Big – a plan of its own.
3. Do nothing but warn (validator: box past the bottom) – off the table: the owner wants crossing as a tool.

## Recommendation
Option 1 now if crossing boxes show up in the next chapters before option 2 is planned (an hour, no migration); option 2
when the storyboard → chapter pipeline (research/STORYBOARD.md) is redone, since shots already think in strips, not slices.
Open for the owner: Q1 option 1 as a stopgap, or wait for 2? Q2 option 2 – every renderer, or the vertical one first?
