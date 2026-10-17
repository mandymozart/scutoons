Status 2026-10-17: built – compact property controls in @scutoons/ui; pilot = the creator's text box inspector; the other inspectors follow one by one (owner decides).

# Editor inputs – compact property controls for the creator's inspectors

Owner 2026-10-17: the inspectors (text box: X %, Y %, WIDTH %, … BOX) waste space and the numbers are hard to play with.
Wished for: drag a value (scrub the label / icon, Shift coarse, Alt fine, arrows step), click or double-click to type,
icons that say what a field is for, segmented icon buttons instead of selects, an inline colour swatch – in SCUTOONS'
look, not Figma's.

## 1. What the others do

| Tool | Number field | Choice | Colour | Notes |
|---|---|---|---|---|
| Figma | label / icon is the scrub handle (also Alt + hover over the field); Shift = ×10; typed math (`*2`, `+10`); "Mixed" for a multi-selection | icon segmented rows (align, constraints) | swatch + hex + opacity in one row | W/H "constrain proportions" lock between the pair; the handle's narrow hit area on icon-less fields is a known complaint |
| Penpot | numeric inputs evaluate simple maths from the old value; a precision per field (2 decimals); scrubbing asked for by the community (a wide left scrub zone, not Figma's edge) | icon toggles | swatch + hex | open source; the request for a *dedicated* scrub zone is the useful lesson |
| Framer | scrub on the label, arrow steppers on hover | segmented | swatch | units in the field (px, %, fr) |
| Rive | label scrubbing, value + unit | icon toggles | swatch | dense two-column rows "X  Y" with a row label left |
| Blender | the whole slider is the handle: click = type, drag = scrub, Ctrl = snap, Shift = fine; pointer wraps (no edge) | enum dropdowns / expanded icon rows | swatch opens a wheel | Python expressions in any number field |
| Theatre.js | the label scrubs, pointer lock while dragging | – | swatch | pointer lock = no screen edge |
| Tweakpane / Leva | a scrub area left of the number (Leva: drag the label), steps from `step`, Shift / Alt | radio grid | swatch + text | small, dependency-free – the closest to what we need |

Common ground: the **label or icon is the scrub handle** (the number itself stays a normal text field you click into),
**modifier keys change the step**, **arrow keys step**, **typed maths**, **units shown in the field**, a **lock** links
W/H, a **swatch + text** for colours, **icon segments** for small enums. Pointer lock (Theatre, Blender's wrap) removes
the screen edge while scrubbing.

## 2. SCUTOONS' version (what is built)

Not Figma's grey micro-UI: our tokens, IBM Plex Mono, square frames, ink, **every control exactly
`--control-height`** (DESIGN "One control height" – no small variant; ≥ 44 px on touch). The room comes from putting
the label *inside* the control (the handle) instead of a caption line above it, from icon segments instead of selects,
and from grouping fields into labelled rows – not from smaller type.

| Control (`@scutoons/ui`) | What it does |
|---|---|
| `ScrubNumber` | one framed field: a handle (an icon or a short mono tag like `X`, `W`) + the number + its unit. Drag the handle left / right (Shift ×10, Alt ×0.1; the drag distance per step is in rem); a mouse drag takes the pointer lock (no screen edge); Esc during the drag puts the old value back. Click the number to type; the handle's double click focuses it too. Typed maths: `12+3`, `*2`, `/2`, `+10`, `-5` (relative to the old value), parentheses; nothing else is evaluated. Arrows ↑ ↓ step (Shift / Alt), PageUp / PageDown ×10, Enter commits, Esc reverts. `role="spinbutton"` + aria-valuenow / min / max / valuetext; empty = the default (shown as placeholder) – `mixed` shows "Mixed". |
| `Segmented` + `square` / `fallback` | the existing segmented control: icon-only options as squares; `fallback` marks the default option (ink frame, not inverted) while nothing is set – a click on the chosen option again clears it (`clearable`). |
| `PropSelect` | a select inside the same frame as the number fields, with an icon or tag on the left instead of a caption above; the default option says "(default)". |
| `ColorField` | a swatch (the native colour picker over it) + the hex text in one frame; empty = the placeholder colour, "none" draws a diagonal line. |
| `LinkedPair` | two `ScrubNumber`s + a square lock toggle (aria-pressed) between them: locked, a change of one scales the other by the ratio; disabled while one is empty (auto). |
| `PropRow` | one row of the inspector: a tool label on the left (6rem column) and the controls sharing the rest; in a narrow panel (container query < 18rem) the label goes above. |

Icons: Feather where it has one (align-left / center / right, rotate-cw, maximize-2, type, link / link-2); the missing
ones (vertical align top / middle / bottom, letter spacing, outline width) are drawn in the Feather grid and live in
`shared/ui/src/icons-extra.ts` – `Icon` looks there after Feather (DESIGN "Icons": a missing one goes into shared/ui).

Accessibility: every control is keyboard-reachable, the 3px ink focus ring (DESIGN "Focus"), handles are
`aria-hidden` (the input carries the name), segments keep aria-pressed, touch: the handle has `touch-action: none`
and the controls follow `--control-height` (44 px on touch); `prefers-reduced-motion` turns off the only transition
(the frame colour).

## 3. Pilot and next

Pilot: the text box inspector (`creator/src/lib/components/text/dialog-inspector.svelte`) – Position (X, Y), Size
(W ⟷ H), Text (font, weight, size, letter spacing), Align (horizontal + vertical segments), Case, Transform
(rotation, scale), Colour, Outline (+ width), Box, ID. The position rows are their own block so the 9-point anchor
(a parallel change) slots in as one more row.

Next (owner to decide the order): the item inspector, the group panel, the cover lettering lines, the map place
inspector – each a plain swap of `PropField` for the compact controls; `PropField` stays for page forms.
