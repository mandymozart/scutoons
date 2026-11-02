Status 2026-11-02: review material for the owner (not decided) – does the design dogma get a dense tier for work surfaces? Decision issue: see the `decide` issue linked from #269.

# DENSE-UI – small sizes for work surfaces (the rig designer, the map editor, graphs, tables)

Owner 2026-11-02: "i do want to review our design dogma for the design, the map editor already started using smaller sizes for specific sections, and it will be important to do so too in this [the designer] because we might end up with extreme granularity."

## 1. What the dogma says today (agents/DESIGN.md "Brutalist tools", "One control height")
- Type = the reader's scale; **`caption` 0.75rem is "the smallest text"** (`--type-caption-size`); `--type-nav-micro-size` 0.625rem exists in tokens.css but "is not used in tools". No px font sizes.
- **One control height** (`--control-height` = 2rem + the 2px frame; raised to ≥ 44px on `pointer: coarse`).
- Spacing has three densities (`.sc-fields[data-density="page|panel|dense"]`, tools.css) – spacing only, not type size or control height.
- Escape: `data-control-exempt="<reason>"` on a control that may not be control-height.
- `check:tools-css` forbids px font sizes and bold; it does **not** forbid a rem size below caption.

## 2. What the map editor does (found, not designed)
- `map-canvas.svelte:1015` sets `font-size: 0.55rem` (marker labels on the canvas) – below the dogma's smallest text, allowed only because the check looks at px / em, not rem.
- Markers and resize handles are `data-control-exempt` (canvas objects, not controls).
- Everything else follows the dogma. So small sizes appear in **canvases**, as one-off exceptions without a rule.

## 3. What the designer needs
A node graph (a labelled box per node, port names, many links), a parameter table (dozens of rows), a mapping table, a code editor (mono), later a matrix (rows = outputs, columns = inputs). "Extreme granularity": hundreds of cells on one screen on a desktop, readable and hit-able with a mouse – and a phone, where the answer is different (the matrix, not the graph; 44px targets).

## 4. Options
- **A – keep one tier.** The designer follows the dogma (0.75rem minimum, 2rem controls); graphs rely on zoom and panning. Simple; tables get tall, a screen shows about 20 rows.
- **B – a dense tier for named work surfaces.** `data-scale="dense"` on a region (a canvas, a graph, a table, an editor – never on headers, dialogs or forms): minimum text 0.625rem (the token that already exists), row / control height 1.5rem, spacing `dense`. **Only on fine pointers**; on `pointer: coarse` the region falls back to the standard tier (44px targets stay the law on touch). It belongs to **edit mode** (research/RIG-EDITING.md §9a) – performance mode never has it. `check:tools-css` learns the rule: a font size below caption is an error unless it sits inside a `data-scale="dense"` region (like `bold-ok`, with a reason) – which also puts the map editor's 0.55rem under a rule (raise it to 0.625rem, or mark the canvas dense).
- **C – zoom instead of tiers.** Work surfaces get a user zoom (the slice editor has one) and keep caption-size text; granularity is answered by zooming. Good for graphs and canvases, no help for tables and code.

## 5. Recommendation
**B for tables, matrices and code (dense, fine pointers only, edit mode), C for graphs and canvases (zoom), A for everything else.** One named region attribute instead of ad hoc `0.55rem`s; the touch minimum stays; the performance UI stays big and readable.

## 6. For the owner
1. Does the dogma get a dense tier (B), zoom only (C), or stay as it is (A)? Recommended: B + C.
2. The tier's floor: 0.625rem text and 1.5rem rows (recommended) – or other numbers?
3. The map editor's 0.55rem: move it under the rule (0.625rem or a dense canvas)?
