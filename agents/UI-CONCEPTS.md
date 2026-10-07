# UI concepts — the refactors behind BACKLOG E.1–E.6 (drafted 2026-10-07, for the owner to pick)

Written by the night run from the code as it is (file:line refs as of commit 964d6676). Nothing here is built yet –
HANDOVER's UI rule: design drafts before bigger UI. Each concept: **today**, **proposal**, **steps** (in dependency
order), **owner decides**. Order = the order to build them in (each one frees room or context the next one needs).

---

## E.1 One notification model (M.5 + M.13)

**Today.** The creator's messages are `.sc-bar` rows **in the page flow** above the editor
(`creator/src/lib/components/page/doc-notices.svelte`): unsaved changes (inverted), autosave recovery, converted
document, event ids assigned, conflict (Reload / Overwrite), and `DocSession.notice` – one string with a × for
"Published", "Saved", "Nothing to save", errors (`editor/doc-session.svelte.ts:50`, set at :100–:162). Plus the
chapter editor's cast notice (Undo) and the file status row (`status/file-status.svelte`: a chip per language that
isn't live). All of them push the canvas down; the slice canvas then fits what is left, so every message shrinks the
slice. **Toasts already exist** in `@scutoons/ui` (`toasts.svelte.ts`, `toast-stack.svelte`: fixed bottom-right,
aria-live, 4 s, errors stay) – admin and embassy use them, **the creator doesn't**.

**Proposal – three kinds, three places, none moves the layout:**
1. **Transient info → toast** (fades by itself): Published, Saved, Nothing to save / publish, review submitted, event
   ids assigned. The creator mounts the shared ToastStack (like admin's `+layout.svelte:221`) **above the ActionBar**
   (bottom-right, over the content, never in flow).
2. **State that needs doing → in the ActionBar**, where the doing happens: unsaved changes = the Save button's dot
   (already "Save draft •") – the inverted bar goes; not-live languages = a compact status chip next to Publish
   ("de · draft") that opens the file-status list as a popover; the cast notice's Undo = a toast with an action.
3. **Blocking problems → one bar, only then:** conflict (Reload / Overwrite), autosave recovery, converted document –
   rare, they must be answered; they stay in flow but only these three.
Errors: a toast that stays until dismissed (the shared stack already does that).

**Steps.** (a) `DocSession.notice` → `toast()` (one context in the creator, like admin's `lib/toasts.ts`), the bar
goes; (b) the dirty bar → the ActionBar's Save state; (c) `file-status` → a chip + popover in the ActionBar (meta pages
too); (d) the cast notice → a toast with Undo (the shared toast needs an `action` – a small addition); (e) checks:
creator-chapter / creator-dock look for these bars by testid – update.
**Size:** medium, creator only + one toast option in shared/ui. **Frees:** the rows above the slice (M.13).
**Owner decides:** toasts bottom-right (admin's place) or top-center? The language status as a chip in the bar – yes?

---

## E.2 Header + breadcrumbs (M.14)

**Today.** The creator builds its top bar itself (`creator/src/routes/+layout.svelte:257–290`): logo + "CREATOR"
(`.brand-name`) + `<nav class="crumbs">` from `lib/nav.svelte.ts` (`base` from the route: toon, **season** (hard-coded
season 1 for chapters, `:106–107`), chapter …; `extra` from the page: group, "Slice n"). Overflow: `nowrap`, middle
crumbs shrink with an ellipsis each (min 3em), under 479 px only the current crumb. No "…" menu.

**Proposal.** One crumb trail SCUTOONS › Toon › Season › Chapter › Group › Slice:
- the logo is the root crumb (SCUTOONS) – the "CREATOR" word goes (owner: "the creator title can go"); the app name
  stays in the tab title and the sidebar's top;
- when the trail doesn't fit, the left crumbs (after the root) collapse into one **"…" button** that opens a small menu
  of them (keyboard: Enter / arrows); the last two crumbs never collapse; each crumb keeps its full text (no
  per-crumb ellipsis – that made them unreadable);
- the season comes from the chapter's real season (the season list knows where the chapter is), not "1".
A shared `Breadcrumbs` component in `@scutoons/ui` (admin / embassy use the same pattern later).

**Steps.** (a) `Breadcrumbs.svelte` (measures with a ResizeObserver, collapses from the left); (b) the creator's
layout uses it, `.brand-name` goes; (c) the season crumb from the chapter's season; (d) Alt+↑ (`parent`) unchanged.
**Size:** small–medium. **Needed by** E.3 (the trail is the hierarchy).
**Owner decides:** the "…" menu vs a horizontal scroll of the crumbs.

---

## E.3 Navigation hierarchy (M.6)

**Today.** Two navigations: the left sidebar (chapter tree → selection, the URL follows by `replaceState`,
`chapter-editor.svelte:385–424`) and the Flow in the right dock. The chapter Flow's level is the deepest selection
unless picked; **every selection change remounts the whole graph** – `flow-panel.svelte:118`
`{#key `${level}:${current}:${slice}`}` → SvelteFlow + dagre again → `flow-controls.svelte:18–30` centres the
current box at zoom 1 again. That is the "jump". The toon / season Flow navigates with `goto(…?flow=1)`, which
remounts the dock panel on the next page.

**Proposal.** The sidebar + the main area ARE the hierarchy (with E.2's trail); the Flow and Ask only **read** the
current place and never navigate by themselves except when the author clicks a box (an explicit "go there").
- **The Flow keeps its viewport**: no `{#key}` remount on a selection change – the graph stays, the current box is
  highlighted, and the view only pans (animated, no zoom change) when the current box is outside it. A level change
  (chapter ↔ group ↔ slice) is the only rebuild, and it keeps the zoom.
- The Flow's level follows the main area by default (as now), the Segmented stays as "look wider"; a click on a box =
  select it in the main area (as now), never a level switch by itself.
- Toon / season Flow: the same panel instance across `goto` (the dock remembers the panel kind; the page hands it the
  new input) – no remount when moving toon → season → chapter.
**Steps.** (a) **the bug fix first** (small, can go ahead without the rest): replace the `{#key}` by updating the
graph's nodes / edges in place + "pan only when off-screen"; (b) the dock keeps a panel across navigation when the new
page registers the same kind; (c) Ask reads the trail (it already uses the crumbs).
**Size:** (a) small, (b) medium (dock.svelte.ts). **Owner decides:** (a) can be done right away – yes?

---

## E.4 Slice editor room (M.4)

**Today.** `slice/slice-toolbar.svelte`: Snap toggle + a grid select (5/10/15/30, localStorage) + a Segmented of the
tools (text / item / sound / event). `slice-canvas.svelte` fits the column's height and is **capped at the slice's 1x
width** (`max-width: min(<width>px, 100cqh / ratio)`) – on a big screen it never grows; no zoom. The inspector is a
22rem region beside it (below it under 1280 px).

**Proposal.**
- **A vertical tool strip** on the canvas's left edge (round icon buttons, the same tools), the toolbar row goes;
- **Snap = one small button** (the grid icon, pressed = on); a click-and-hold / a small chevron opens the sizes (5 /
  10 / 15 / 30 px);
- **the canvas fills** the area (cap at 2x the slice's width instead of 1x) with **zoom**: −/+ / fit buttons at the
  bottom-right like the Flow's (the same look, `flow-controls` vocabulary), Ctrl + wheel, the zoom remembered per
  device; handles and snapping work in slice coordinates (they already do – the canvas scales them);
- with E.1 the rows above the canvas are gone – the slice takes that room too.
**Steps.** (a) the tool strip + the Snap button (layout only); (b) the cap 1x → 2x + a `zoom` factor on
`.canvas-wrap` (CSS transform-free: width × zoom, the scroll area scrolls); (c) the zoom controls (a small shared
`ZoomControls` the Flow could use too).
**Size:** medium. **Owner decides:** zoom range (50–300 %?), the strip left of the canvas or inside it.

---

## E.5 The VN message box + end actions (M.3)

**Today.** The message box is fixed at the bottom (`toon/vn/message.svelte:83–99`, slides out by `translateY`); its
height is reported (`vn.setBoxHeight`) and the avatar / portrait stand on the box's top
(`characters.svelte:47–50`: ground = box height + 1rem). `offsetHeight` ignores the slide-out transform, so **with
no box showing they still stand where the box's top would be** – the owner's "should sit lower". The end actions
(`chapter/next-bar.svelte`) are a separate fixed bar (z 110 > the characters' 101), appear without a transition and
cover the characters' feet.

**Proposal.**
- **ground = the visible box's top, else the page bottom**: the VN context reports 0 while no box is shown (the box's
  `shown` state, not its offsetHeight) → the characters slide down to `1rem`; when a box (or the end actions) appears,
  they slide up with it (they already animate `bottom` 0.3 s).
- **The end actions inside the message box** (owner's rethink): the next bar's choices (next chapter, back to the toon,
  the season's music, buy …) become the box's choice buttons – the same style as `showChoices`, so the reader learns
  one gesture: "the box at the bottom is where I act". The chapter's last text box and the end actions can share the
  box (text above, the actions as choices).
- The separate fixed bar goes; `next-host`'s reserved 9rem stays (the page can scroll its end clear of the box).
**Steps.** (a) the ground fix (small, viewer only – could go first); (b) the next bar's content as a "box mode" of
`message.svelte` (choices + the cover / music row); (c) check-next-bar / chain-vn checks.
**Owner decides:** (a) right away? (b) drafts on the real page (phone + desktop) first – which actions in the box.

---

## E.6 Strip preview for vertical groups (M.9) and the viewer as a dock panel (M.10)

**Strip preview today** (`chapter/strip-preview.svelte`): one reader screen (phone 390:844 / desktop 16:9), the
slices side by side at screen height with the reader's own `Slice`, wheel → sideways; mounted only for a horizontal
full-width group (`chapter-editor.svelte:800–802`). **For vertical groups:** the same component with a vertical track
(slices stacked at the screen's width, `overflow-y`), the frame's fixed height, no wheel remap; mount it for every
group. Small. Better after E.4 (the same editor's room).

**The viewer as a panel (exploration):** a third dock panel next to Flow / Ask: an iframe of the viewer's draft
preview (`?draft=1`, the creator's session) at phone width, reloaded on every saved draft (DocSession's save event),
and a `postMessage` "show group / slice" when the selection changes (the viewer scrolls there). Needs: the viewer to
accept the message (only from the creator's origin), the dock's E.3(b) (a panel that survives navigation). Explore
after E.3. **Owner decides:** worth a spike?

---

## E.7 Copy / cut / paste (BACKLOG 10.39)

**Today:** one canvas selection (`ChapterView.selection`, chapter-state.svelte.ts), each list keeps its own
multi-selection (`picked`, multi-select.ts – groups, slices, events); Ctrl+X / Delete / Backspace delete the canvas
selection (chapter-editor.svelte onKeydown); no copy, no duplicate, no context menu.

**Proposal:**
- **The active area:** a small `clipboard.svelte.ts` records the area clicked last (canvas, event list, slice list,
  group tree). Its selection is drawn in the accent colour, the others' in grey (the OS convention for an inactive
  selection) – no new controls. Cmd/Ctrl+C / X / V act on the accent selection; with none, nothing happens. In a text
  field the keys stay the browser's.
- **What pastes where:**

  | Active selection | V (as new) | Shift+V (settings only, onto the selection) |
  |---|---|---|
  | text box | a new box (same slice: one grid step offset) | the look: variant, frame, shape, tail, font, stroke, rotation, scale |
  | placed item | a new placement (random id) | action, qty, repeat, respawn, events |
  | sound / event area | a new area | trigger + params |
  | VN events (several) | after the selected event | params of the same type + `if` |
  | slices | after the selected slice, with boxes, items, events | audio + events |
  | groups | after the selected group | renderer, `end`, `continues` |

- **Feedback:** a toast ("3 events copied", "pasted – uses 4 files from catpope", "only 1 placement allowed – also in
  catpope ch. 3"). With E.1 (one notification model).
- **Later, optional:** the same actions in a right-click menu on the canvas and the lists.

**Steps:** the active area (draft on the real chapter page, desktop + mobile) → text boxes → the rest (BACKLOG 10.39).
**Owner decides:** the draft of the active / inactive selection colours.
