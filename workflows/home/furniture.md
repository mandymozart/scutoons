# Furniture images (the avatar's home)

Furniture items (CD rack, record shelf, workbench, sofa, …) need **two pictures**: the **icon** (inventory, item detail –
the item-icon spec, `workflows/items/item-icons.md`) and the **home image** – the piece as it stands in a home, in the
home scene's perspective (agents/research/CAVE.md D3, D4, "Data model"). Both come from **one render**: generate the
home image, cut the icon from it – the reader recognises the same object in both places.

## How big a piece is: not the image – the item's `size`

The image only has to be cropped right (its foot at the bottom). How wide it shows in a home is the item's
`furniture.size` = its width in % of the room's width at the front edge. **The room is 4 m wide at the front**, so
`size = real width in metres ÷ 4 × 100`. The height follows the image's own proportions; further back it is drawn
smaller (depth), further front covers what stands behind.

| item_id | real width | `size` | notes |
|---|---|---|---|
| `cd_rack` | 0.5 m | 12 | a 90s CD tower / wall rack, 50 CDs |
| `cassette_rack` | 0.4 m | 10 | a cassette tower, 50 tapes |
| `record_shelf` | 0.8 m | 20 | a low cube shelf, LPs upright, 50 records |
| `workbench` | 1.4 m | 35 | every home has one |
| `sofa` | 1.8 m | 45 | two-seater |
| (a plant, a lamp) | 0.3–0.4 m | 8–10 | for later |

New piece: measure (or guess) the real width, divide by 4, × 100 – done. Keep it between 5 and 60.

## Spec (the home image)

| | |
|---|---|
| Canvas | generate at 1024 × 1024 |
| View | three-quarter view from above, about 30°, the camera in the room's front corner (like the map places, agents/MAP.md §5.1) |
| Facing | the piece **turned so its back is to the LEFT wall** – its front faces the lower right. The home mirrors it for the right wall ("Turn"). |
| Ground | none – no floor patch, no rug; only a soft contact shadow directly under the feet |
| Background | transparent |
| Content | one piece, empty (racks without CDs / records – the contents come from the inventory), no text, no logos, no people, no hands |
| Style | the item-icon style: soft realistic 3D render, the character portraits' light and palette |
| File | `scutoons/furniture/<item_id>.webp` (home image) + `scutoons/items/<item_id>.webp` (icon) |

## Brief for Luma (image generation; attach the two style references of item-icons.md)

```
Attached are two character renders from the SCUTOONS universe as STYLE references only (rendering, lighting,
materials, colour palette). Do not include the characters.

Create one piece of furniture in exactly that rendering style: <FURNITURE DESCRIPTION>.
Soft realistic 3D render, same light direction and softness as the references. Three-quarter view from above,
about 30 degrees, the piece turned so its back faces the left and its front faces the lower right (it will stand
against the left wall of a room). Empty, nothing on or in it. No floor, no rug, only a soft contact shadow directly
under its feet. Centred, ~6 % padding, 1024 x 1024 px, transparent background, no text, no logo, no people.
```

| item_id | Furniture description |
|---|---|
| `cd_rack` | a slim 90s CD tower rack, black lacquered wood with narrow slots for 50 CD cases, a small cobalt-blue ornament on top |
| `cassette_rack` | a small 90s cassette tower, pale cream plastic with slots for 50 cassettes |
| `record_shelf` | a low wooden cube shelf with two open compartments for vinyl records standing upright |
| `workbench` | a sturdy wooden workbench with a vice on one end, a pegboard-free top, a few tool marks |
| `sofa` | a small two-seater sofa, pale sphynx-pink fabric, cobalt-blue piping |

Keep a series consistent: generate the first, approve it, then attach it as a third reference ("same scale, same
light, same view").

## Steps

1. Generate with Luma (the brief above, the references attached); one piece at a time; the owner approves.
2. **Home image:** trim to the alpha bounding box (the foot = the lowest opaque row stays at the bottom edge), WebP q85
   with alpha, longest side 512 px → `scutoons/furniture/<item_id>.webp`.
3. **Icon:** the same render, padded to a square with ~6 % padding, 512 × 512 → `scutoons/items/<item_id>.webp`
   (item-icons.md); check it on a checkerboard / white / black.
4. Upload both in the admin → Assets; in Catalog → Items set the icon, `furniture.image` and `furniture.size` (the
   table above).
5. Artist-made images always win later (same file names).
