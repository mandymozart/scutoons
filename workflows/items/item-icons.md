# Item icons

Icons for **virtual items** (camera, cucumber, cucumber camera, …) in the same look as the SCUTOONS character
renders, so items and characters feel like one world in the inventory and on the avatar. Physical merch keeps its
cut-out product photos.

## Spec

| | |
|---|---|
| Canvas | 512 × 512 px, transparent background |
| Framing | the object centred, ~6 % padding, slight three-quarter view from above, no cast shadow outside the object |
| Style | identical rendering to the character portraits: soft realistic 3D render, same lighting direction and softness, same material quality and colour palette (pale sphynx pinks, cobalt blue ornaments, warm mouse yellows) |
| Content | one object, no text, no logo, no background, no hands |
| File | `catpope/items/<item_id>.png` (+ the WebP twin from the asset job on upload), variants `<item_id>_<variant>.png` |

## Style references (attach both to every prompt)

- `media/characters/catpope/catpope_portrait-neutral.webp`
- `media/characters/yellowmouse/yellowmouse_portrait-neutral.webp`

## Brief (image generator or designer; attach the style references)

```
Attached are two character renders from the SCUTOONS universe as STYLE references only (rendering, lighting,
materials, colour palette). Do not include the characters.

Create a single object icon in exactly that rendering style: <OBJECT DESCRIPTION>.
Soft realistic 3D render, same light direction and softness as the references, slight three-quarter view from
above, the object centred with ~6 % padding, 512 × 512 px, transparent background, no text, no logo, no hands,
no shadow outside the object.
```

Object descriptions for the current catalog:

| item_id | Object description |
|---|---|
| `camera` | a small, slightly old-fashioned compact camera with a round lens, cobalt-blue ornament engravings like the Cat Pope's cape, pale cream body |
| `cucumber` | one fresh green cucumber with little bumps, a soft highlight, a tiny blossom at the tip |
| `cucumber_camera` | the same compact camera as `camera`, but its lens is a cross-section slice of a cucumber and a cucumber serves as the carrying strap — it can only see cucumbers |

Keep `camera` and `cucumber_camera` visibly the same camera (generate `camera` first, then attach it as a third
reference for `cucumber_camera`).

## Steps

1. Generate with the prompt above (style references attached), one item at a time.
2. Owner approves; artist-made icons always win and can replace generated ones later (same file name).
3. Upload as `catpope/items/<item_id>.png` in the creator / admin → Assets (the asset job makes the WebP twin); check
   it on a checkerboard / white / black background.
4. The catalog (admin → Catalog → Items, the item's picture) already points at `catpope/items/<item_id>.*` — switch the
   extension to `.webp` if needed.
