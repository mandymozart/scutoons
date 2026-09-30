# Character base views

Turns **one** provided image of a character into the character's **four base views**, normalized, named and
registered, so the character works everywhere: VN dialogue (avatar, portrait), full-body scenes (full) and the
reader avatar (A-pose).

```
any 1 of 4 views  ──►  check against spec  ──►  generate the missing views (Luma)  ──►  normalize
                                                                                         │
             register in the character registry  ◄──  WebP + avatar  ◄──  name + place ◄─┘
```

Moods are a separate step (`moods.md`, to come): every mood is a variation of the **portrait** view only.

## The four base views (the spec)

| View | Purpose | Canvas | Framing | Background | Mood |
|---|---|---|---|---|---|
| **portrait** | VN portrait stage (right), base for all moods | 600 × 600 px | head + shoulders, facing the viewer, eyes on the horizontal line at ~40 % from the top, head centred, top of the head/ears ~6 % below the top edge | transparent | neutral |
| **avatar** | VN avatar slot (left), lists, pickers | 192 × 192 px | derived: the portrait scaled down (never generated separately) | transparent | = portrait |
| **full** | full-body scenes | 600 × 1200 px (1 : 2) | whole figure in the character's characteristic standing pose, feet ~4 % above the bottom edge, head ~4 % below the top | transparent | neutral |
| **apose** | reader avatar body (items are drawn on it) | 768 × 1152 px (2 : 3) | strict A-pose, front view, symmetrical, arms ~30–40° away from the body, palms towards the thighs, legs slightly apart, feet flat and fully visible, figure centred, same scale for every character (head top ~5 %, soles ~92 % of the height) | white with a soft floor shadow (as the existing A-poses) | neutral |

A-pose looks: `swim` (plain swim shorts/pants, the default) and `naked` (cartoon-neutral, no anatomical detail, as the
existing Cat Pope and Yellow Mouse A-poses). Story looks (e.g. helmet) are **not** A-pose looks.

Style for all views: identical to the reference — same rendering, line quality, colours, lighting, proportions,
clothing of the look. No text, no watermark, no props unless they belong to the look.

## Prompt definition for Luma (per view to generate)

Attach the provided reference image every time. Replace `<name>`, `<persona>`, `<look>`.

```
Recreate the character from the attached reference image EXACTLY: same character (<name>, persona <persona>,
look <look>), same art style and rendering, same colours, lighting, proportions, face, markings and clothing.
Do not redesign, restyle or add anything.

Produce the <VIEW> view:
- portrait: head and shoulders, facing the viewer, neutral expression, eyes on a horizontal line at 40 % of the
  image height, head centred, 600 × 600 px, transparent background.
- full: the whole figure standing in its natural, characteristic pose, neutral expression, feet fully visible,
  600 × 1200 px, transparent background, small even margin above the head and below the feet.
- apose: strict front-view A-pose, symmetrical, arms 30–40° away from the body with palms facing the thighs, legs
  slightly apart, feet flat and fully visible, neutral expression, 768 × 1152 px, white background with a soft
  shadow under the feet, figure centred; wearing <plain swim shorts | nothing (cartoon-neutral, no anatomical
  detail)>.

Consistency check before delivering: overlaid on the reference, the character must be recognisably identical;
only framing and pose may differ.
```

File names for the delivery (the importer also accepts any name inside a folder named after the character):
`<character>[-<persona>][_<look>]_<view>-neutral.png` · A-pose: `<character>_apose-<swim|naked>.png`

## Workflow steps

1. **Take whatever is provided.** Any one of portrait / avatar / full / apose (an artist's drawing or a generated
   image). Identify character, persona, look and view (folder name / file name / ask).
2. **Check against the spec** (canvas, framing, background, pose):
   - **matches** → keep it as that view.
   - **close** (wrong size, margins or background) → normalize: crop/pad/scale to the canvas, remove the
     background, align the eye line (portrait) or feet/head margins (full, apose). Show before/after.
   - **does not match** (e.g. a scene still, a 3/4 pose for an A-pose, cropped feet) → do **not** force it into the
     view; use it **only as the Luma reference** and generate that view too.
   - An **avatar** given as the only source is too small to be a good reference: generate the portrait from it
     first, then derive the avatar from the portrait (replacing the given one only if the owner agrees).
3. **Generate the missing views** with the prompt above (Luma), one view at a time, reference attached. Portrait
   first when available — it is the most faithful reference for the face; full and apose use portrait + the provided
   image together.
4. **Review** (owner): approve each view or regenerate; artist-made images always win over generated ones and can
   replace them later (same file name → re-import).
5. **Name and place** into `cdn/root/characters/<character>[-<persona>]/` with the convention names.
6. **WebP + avatar**: `python cdn/tools/make_character_webp.py` (WebP next to every PNG, 192 px avatar from the
   portrait).
7. **Register**: `node --experimental-transform-types shared/content/scripts/migrate-characters.ts` (+ import into the
   store) — the paths are picked up by convention; missing views stay open in "Wanted images".
8. **Upload** with `cdn/sync.py` (option 3 → `characters`).

## Later (see agents/AUTOMATION.md)

This workflow becomes a creator action: "Complete base views" on a character → the creator lists which views are
missing, calls the generator with the available reference, shows the results for approval, and writes them through
the API (no manual file handling).
