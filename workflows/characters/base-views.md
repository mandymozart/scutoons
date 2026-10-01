# Character base views

Turns **one** provided image of a character into the character's **four base views**, normalized, named and
registered, so the character works everywhere: VN dialogue (avatar, portrait), full-body scenes (full) and the
reader avatar (A-pose).

```
any 1 of 4 views  ──►  check against spec  ──►  generate the missing views (image generator or designer)  ──►  normalize
                                                                                         │
             register in the character registry  ◄──  WebP + avatar  ◄──  name + place ◄─┘
```

Moods are a separate step (`moods.md`, to come): every mood is a variation of the **portrait** view only.

## The four base views (the spec)

| View         | Purpose                                       | Canvas                | Framing                                                                                                                                                                                                                                        | Background                                               | Mood       |
| ------------ | --------------------------------------------- | --------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------- | ---------- |
| **portrait**   | VN portrait stage (right), base for all moods | 600 × 600 px          | head down to above the waist, with arms, facing the viewer, eyes on the horizontal line at ~40 % from the top, head centred | transparent | neutral |
| **avatar**     | VN avatar slot (left), lists, pickers         | 600 × 600 px (shown at 192 px) | head with neck, facing the viewer, eyes on a horizontal line, fully inside the frame — generated as its own view (owner decision 2026-09-30); the 192 px display copy is downscaled from it | transparent | neutral |
| **full**     | full-body scenes                              | 600 × 1200 px (1 : 2) | whole figure in the character's characteristic standing pose, feet ~4 % above the bottom edge, head ~4 % below the top                                                                                                                         | transparent                                              | neutral    |
| **apose**    | reader avatar body (items are drawn on it)    | 768 × 1152 px (2 : 3) | strict A-pose, front view, symmetrical, arms ~30–40° away from the body, palms towards the thighs, legs slightly apart, feet flat and fully visible, figure centred, same scale for every character (head top ~5 %, soles ~92 % of the height); **multi-figure bodies** (duos, twins, triplets) that would exceed the width at that scale are scaled to fit the width (≥10 px side margins) with the soles still at ~92 %, so the heads sit lower (Toptoy ~22 %) | transparent with a soft floor shadow (as the existing A-poses) | neutral    |

A-pose looks: `swim` (plain swim shorts/pants, the default) and `naked` (cartoon-neutral, no anatomical detail, as the
existing Cat Pope and Yellow Mouse A-poses). Story looks (e.g. helmet) are **not** A-pose looks.

Style for all views: identical to the reference — same rendering, line quality, colours, lighting, proportions,
clothing of the look. No text, no watermark, no props unless they belong to the look.

## Brief (one instruction, creates everything that is missing; for any image generator that takes style references, or a designer)

Attach **one** image: either one of the base views (portrait, full, A-pose) or **any other reference** of the
character — a snapshot, an action pose, a concept artist's sketch. Replace `<name>`, `<persona>`, `<look>`.

```
Attached is a reference image of the character <name> (persona <persona>, look <look>).
It is the starting point. Create this character's complete set of base views as SEPARATE images, recreating the
character EXACTLY: same character, same art style and rendering, same colours, lighting, proportions, face,
markings and clothing of this look. Do not redesign, restyle, add props or change anything else.

First decide what the attached image is:
- If it already IS one of the views below (same framing and pose), keep it as that view and create ONLY the
  views that are missing.
- If it is NOT one of the views (e.g. a snapshot, an action pose, a sketch, a scene), use it ONLY as the identity
  reference and create ALL views below.

The views (all with a neutral expression):
1. AVATAR - head with neck, facing the viewer, eyes on a horizontal line, with in the frame. resolution 600x600px, transparent background
2. PORTRAIT — head till above waist with arms, facing the viewer, eyes on a horizontal line at 40 % of the image height,
   head centred, 600 × 600 px, transparent background.
3. FULL — the whole figure standing in its natural, characteristic pose, feet fully visible, small even margin
   above the head and below the feet, 600 × 1200 px, transparent background. if a reference photo or concept art is provided use that to imagine what clothing and outfit the person has.
4. A-POSE (SWIM) — strict front-view A-pose, symmetrical, arms 30–40° away from the body with palms facing the
   thighs, legs slightly apart, feet flat and fully visible, figure centred, head top at ~5 % and soles at ~92 % of
   the height, 768 × 1152 px, transparent background with a soft shadow under the feet, wearing plain swim shorts.
5. A-POSE (NAKED) — identical to 4 in every pixel of pose, scale and framing, but without the swim shorts
   (cartoon-neutral, no anatomical detail).


Deliver each view as its own image, named:
  <character>[-<persona>][_<look>]_avatar-neutral.png
  <character>[-<persona>][_<look>]_portrait-neutral.png
  <character>[-<persona>][_<look>]_full-neutral.png
  <character>_apose-swim.png
  <character>_apose-naked.png
Consistency check before delivering: placed side by side with the reference, the character must be recognisably
identical in every view; only framing and pose may differ, and the two A-poses must overlay exactly.
```

File names for the delivery (the importer also accepts any name inside a folder named after the character):
`<character>[-<persona>][_<look>]_<view>-neutral.png` · A-pose: `<character>_apose-<swim|naked>.png`

## Workflow steps

1. **Take whatever is provided.** Either one of the base views (portrait / avatar / full / apose) or **any other
   reference** (snapshot, action pose, concept art). Identify character, persona and look (folder name / file name /
   ask). A reference that is not a view is fine — it just means all views get created.
2. **Check against the spec** (canvas, framing, background, pose):
   - **matches** → keep it as that view.
   - **close** (wrong size, margins or background) → normalize: crop/pad/scale to the canvas, remove the
     background, align the eye line (portrait) or feet/head margins (full, apose). Show before/after.
   - **does not match** (e.g. a scene still, a 3/4 pose for an A-pose, cropped feet) → do **not** force it into the
     view; use it **only as the style reference** and generate that view too.
   - An **avatar** (head with neck) is a view of its own now; as the only source it is a weak reference for the
     body, so the full and A-pose views rely more on any other reference given.
3. **Generate the missing views** with the single brief above (any image generator or a designer): it keeps a matching view and creates all the
   others — portrait, full, A-pose swim and A-pose naked. If a result drifts, regenerate just that view with the
   approved portrait attached as a second reference (the most faithful source for the face).
4. **Review** (owner): approve each view or regenerate; artist-made images always win over generated ones and can
   replace them later (same file name → re-import).
5. **Name and place** into `cdn/root/characters/<character>[-<persona>]/` with the convention names.
6. **WebP**: `python cdn/tools/make_character_webp.py` (WebP next to every PNG, incl. full and A-pose). The 192 px
   avatar display copy is downscaled from a delivered **avatar** view (600 × 600); without one it is a square
   head-with-neck **crop of that mood's portrait** (owner rule 2026-09-30): one box per `<name>[_<look>]`, derived from
   the neutral portrait and grown to fit every mood's head, stored in `cdn/tools/avatar-crops.json` (set an entry's
   `"source": "manual"` to override the box). `--sheet=<png>` writes a contact sheet to check them.
7. **Register**: `node --experimental-transform-types shared/content/scripts/migrate-characters.ts` (+ import into the
   store) — the paths are picked up by convention; missing views stay open in "Wanted images".
8. **Upload** with `cdn/sync.py` (option 3 → `characters`).

## Later (see agents/AUTOMATION.md)

This workflow becomes a creator action: "Complete base views" on a character → the creator lists which views are
missing, calls the generator with the available reference, shows the results for approval, and writes them through
the API (no manual file handling).

## Rules (owner, 2026-09-30)

- **A-pose background:** transparent with a soft floor shadow, never white (a white box would show on the coloured avatar backgrounds). All existing A-poses are transparent.
- **Wide multi-figure bodies:** keep the floor line (soles ~92 %) and scale to fit the width; don't crop, stretch or widen the canvas. Each figure gets its own wear slots (`<type>-left/-right`, see `agents/ITEMS.md` "Slots per avatar body").

