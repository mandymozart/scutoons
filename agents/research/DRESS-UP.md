Status 2026-10-13: RESEARCH – free part done (inventory, Test A from existing assets, cost math, 3D on paper); the paid runs (Luma Test A/B ≈ $2.50, 3D Test C) wait for the owner's go. Nothing built into the app.

# DRESS-UP — looks with their A-pose + an avatar that dresses per region

Owner 2026-10-12: "nackt does not exist anymore. a poses come per look … how to do two things. looks with their base
apose and additionally how we could create an avatar that can dress up with items for each region. and visually
matching. 3d could be an option would require a different asset generation pipeline. or can we task a runner and luma
generator to create variants for each avatar or for each combination, but i believe that would really run into a huge
variation." Then: "yes. we need to do tests. eventually 3d is awesome also for live performance storyboard and
references. but we need pipeline to generate assets."
Today: ITEMS.md "Wardrobe" (full-body `wear` overlays per character on the swim A-pose). The fallback (one A-pose per
look, items only in the avatar page's slots, no overlay) is built by another session – not part of this.

## 1. Inventory – the combination space (production catalog + registry, 2026-10-13)

- **Characters / looks (season-1 cast):** Katzenpapst 6 personas, 8 looks (regular/default, human/silicon,
  cat/default + bathrobe, mountaintiger/default, biker/default + helmet, human-2/default); boy 1, crows 1, Top Toy 1
  → **11 looks**. Registry media also has yellowmouse, yellowbubu, billthefrog, passerby. A-pose images exist only for
  Katzenpapst (swim, tourist, the dressed example), yellowmouse and Top Toy (swim; the naked ones are obsolete).
- **Wearables: 9 items in 5 regions**, options incl. colour variants:

| Region | Items (variants) | Options |
|---|---|---|
| torso | SCUTOONS T-shirt, Dago hoodie (black/beige/grey), Dago heavy T (black/white), Kangaroo cat hoodie | 7 |
| legs | swim pants, shorts | 2 |
| feet | sandals | 1 |
| neck | scorpion amulet (purple) | 1 |
| face | sunglasses (plain + 5 colours) | 6 |
| head, back, hands, accessory | – (no item yet) | 0 |

- **Combinations per look** (each region empty or one option): 8 × 3 × 2 × 2 × 7 = **672**; × 11 looks =
  **7,392 images per view and mood**. It multiplies: one more torso item = +12.5 %; ten options in each of the five
  regions = 11⁵ ≈ 161,000 per look.
- **Layers per look** (Test A approach): 17 options, but colour variants can be recoloured from one layer → **9
  generations per look** (+1 for the look's A-pose) → ~110 for all 11 looks. Linear: a new item = 11 layers.
- Found on the way: the catalog's `sunglasses` `wear` paths (`characters/catpope/wear/sunglasses*.webp`) answer 404 on
  the CDN – the viewer falls back to the icon. Fix or drop the paths (one line in ISSUES if the owner wants it tracked).

## 2. Test A – 2D layers per look (free part: the existing overlays)

![Layers per look](dress-up/test-a-layers-per-look.jpg)
![Swim look vs tourist look, same layers](dress-up/zoom-swim-vs-tourist.jpg)

- **On the look they were made for, layers work** (tile 2): T-shirt, shorts, sandals extracted from the dressed
  reference sit on the swim A-pose within 1–3 px; light and material match because they came out of the same render.
- **On another look of the same character they don't** (tiles 4, 5, zoom): the tourist A-pose is a different render –
  bigger head, wider shoulders, thicker arms, other leg stance, no visible tail. The same layers leave the tourist's
  own sleeves peeking under the shirt, its blue shorts between the legs, its sandals beside the layer's sandals. The
  outline drift (tile 5) is 10–40 px at 768 × 1152. **So layers are per look (per A-pose image), never per character** –
  which confirms the owner's "a poses come per look".
- **An item without a layer** (tile 6, today's fallback): the icon at the slot anchor – readable, not "dressed".
- **The paid part** (not run, ≈ $1.50): per look the A-pose (Luma `image_edit`, source = an existing A-pose, image 1 =
  the look's full image, "same pose, same light") and per item `image_edit` "put on image 1, change nothing else" →
  the layer = the difference to the base (the swim − naked method of ITEMS.md, which gave clean layers). Risk: Luma
  re-renders the whole figure slightly (pixel drift of the face / hands) – the difference mask has to be limited to the
  item's region (slot mask per region) and feathered.
- **Limits of layers:** a long garment over a long garment (hoodie over the papal robe) needs the layer to hide the
  look's own clothes – the layer must be drawn on the *look* and cover what's under it; tuck-in / overlap order
  (hoodie over amulet or amulet over hoodie) is fixed by `WEAR_ORDER`, not per combination.

## 3. Test B – one full render per combination (not run; evidence + cost)

- **Consistency:** the swim and tourist A-poses above ARE two full renders of the same character with different
  clothes – and the body changed (proportions, head, tail, skin tone). Every combination would be such a render: the
  avatar's face and body would shift whenever a reader changes a sock. Reference images reduce it, don't remove it
  (STORYBOARD.md: "neither holds a small figure's exact size").
- **Cost** (uni-1-max ≈ $0.10 per image, measured on the map: 59 regions ≈ $6.50): the whole space = 7,392 × $0.10 ≈
  **$740 per view and mood** before retries (≈ $1,100 with 1.5 tries), +$90 for every new torso item – against ≈ $20 for
  all layers (§1).
- **Cutting it:** (a) only common combinations (the basic dress ± one item: ~1 + 16 per look ≈ 190 images, $19) –
  everything else falls back to layers or icons; (b) on demand + cache: pay only for worn combinations, but a reader
  waits 30–90 s for a new one, every image needs moderation, and it is an **in-app AI feature** – RULES.md "No in-app AI
  features" forbids it unless the owner changes that rule.
- Verdict: good for **hero images** (a toon's cover, a shop image of a look) – not for the avatar.

## 4. Test C – a 3D pipeline (on paper; no spend)

The SCUTOONS character look already IS a stylised 3D render (the A-poses, the prompts' "high-detail stylised 3D render
like a premium collectible") – a real 3D model renders straight into it; no toon shader needed.

| Step | Options | Cost / effort |
|---|---|---|
| image → mesh | **Tripo** API (30 credits = $0.30 textured, auto-rig +$0.25, 2,000 free credits on signup ≈ 35 rigged models); **Meshy** ($20/mo, 1,000 credits, ≈ 30 per model, API rigging 5 credits; API assets deleted after 3 days); **Rodin** (best geometry, API only on the business plan, $0.50+); open: **TRELLIS.2** (Microsoft, MIT, PBR, ≥ 24 GB) and **Hunyuan3D 2.1** (Tencent, best textures, low-VRAM mode) on RunPod (L40S 48 GB $1.09/h) | $0.30–0.55 per model, or ≈ $4.50 for a RunPod test day |
| rig (A-/T-pose) | Tripo / Meshy auto-rig (Mixamo bone names); open **UniRig** (MIT, skeleton + skin weights); Blender Rigify by hand | minutes – hours per character |
| items per region | the same image → mesh per item, then fitted to each body in Blender (shrink-wrap + weight transfer from the body) – per **body shape**, not per look or combination | ~1 h per item × body by hand at first |
| back to 2D | **render-to-sprite at build time** (Blender, the local RTX 2070S is enough): the A-pose per look + one transparent layer per item, all from one camera / light – pixel-exact layers, no drift; or **real-time three.js** in the reader (any combination, turnable, but a GLB of 5–20 MB per character on phones, and the painted look must survive real-time light) | build-time: free once the scene exists |

- **Main risk:** image-to-3D textures are softer than the painted A-pose; face detail (eyes, the forehead mark) usually
  needs a hand fix or a projection of the painted front. Thin parts (claws, ears, tail) come out lumpy.
- **What the same assets give:** **storyboards** – a posable character for the Unreal scenes that exist
  (STORYBOARD.md §"Existing material"), pose / depth images for ControlNet, any camera; **live performance** – LIVE.md
  already plans `model3d` cues (glTF in a three.js layer); **references** – turnarounds, expressions sheets and the
  A-pose of every new look rendered instead of generated.

## 5. Recommendation

1. **Now: 2D layers per look** (Test A) – linear cost (~$2 per look, ~$2 per new item per look), fits today's `wear`
   code (a key per look instead of per character), a run of a script like `scripts/region-signs.mjs`.
2. **In parallel, a small 3D pilot** for Katzenpapst + 1 item (Tripo's free credits or one RunPod day) – judged on the
   storyboard / reference use first. If its render-to-sprite layers look as good as the painted ones, the 3D render
   replaces Luma as the layer source (layers without drift, every look from one model).
3. **No full renders per combination** for the avatar; only hero images.

**First pipeline step** (after the owner's go, ≈ $2.50): `scripts/wear-paint.mjs` – input a look's A-pose + an item
icon → Luma `image_edit` → dressed A-pose → difference layer limited to the item's region → `logs/wear-paint/`;
run on Katzenpapst swim + tourist with hoodie (torso), sunglasses (face), sandals (feet) and show the composites.

## 6. Owner decisions

- [ ] Spend ≈ $2.50 (cap $4) on Luma for the paid Test A (+ the 8 Test B combinations for comparison)?
- [ ] 3D test: create a Tripo account (free credits – only the owner can sign up) or a RunPod day (≈ $4.50, cap $8)?
- [ ] Layers keyed per look (`catpope_tourist`) – i.e. every look carries its own A-pose and its own item layers?
- [ ] Colour variants: one layer recoloured locally (cheap, exact) or one generation per colour?
- [ ] Stay with "no in-app AI" (no on-demand generation when a reader dresses)?
- [ ] Head region: no head item exists yet – which item for the tests (sunglasses = face)?
