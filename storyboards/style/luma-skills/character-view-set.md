<!-- Exported 2026-10-10 from the owner's Luma skill "Character View Set" (skills/character-view-set/SKILL.md, v12, updated 2026-09-30). Its project-note reference: character-view-set.toptoy-note.md. -->

# Character View Set

Recreate an established character exactly and output its complete set of base views as separate images. Never redesign, restyle, recolour, add props, or change anything — only framing and pose differ between views.

## Inputs and Constants

Per-use variables:
- Character source — a reference image when available; otherwise a text identity: name, persona, look.
- Character name (plus optional persona/look tags) — used only to name the output files.

Locked constants:
- Exact recreation: same character, art style, rendering, colours, lighting, proportions, face, markings, and clothing. Do not redesign, restyle, add props, or change anything. Reproduce localized colour markings and gradient shading exactly — including gradients on the hands, feet, and torso — and never flatten or omit them.
- Neutral expression in every view.
- Produce all four base views (portrait, full, apose-swim, apose-naked) except any the source already satisfies (see decision below).
- The per-view specs (position, dimensions, background) are fixed — never relax them.
- Deliver every view as its own separate image at its exact pixel dimensions.
- Do NOT create the small avatar; it is made from the portrait afterwards.
- Always run the consistency check before delivering.

## Decide What the Source Is

First classify the provided image:
- If it already IS one of the views below (same framing and pose), keep it as that view and create ONLY the missing views.
- If it is NOT one of the views (a snapshot, action pose, sketch, or scene), use it ONLY as the identity reference and create ALL views.
- With no image at all, establish the look from name/persona/look, generate the portrait first as the identity anchor, then create the remaining views from it.
- If a project note exists for this character (e.g. the toptoy project note), read it first for locked design facts — digit counts, markings, colour rules; otherwise extract those facts from the reference image.

## View Specifications

Produce each view with a neutral expression, as its own image:
1. PORTRAIT — head and shoulders, facing the viewer, eyes on a horizontal line at 40% of image height, head centred. 600 × 600 px. Transparent background.
2. FULL — the whole figure standing in its natural, characteristic pose, feet fully visible, small even margin above the head and below the feet. 600 × 1200 px. Transparent background.
3. A-POSE (SWIM) — strict front-view A-pose, symmetrical, arms 30–40° from the body with palms facing the thighs, legs slightly apart, feet flat and fully visible, figure centred, head top at ~5% and soles at ~92% of height. 768 × 1152 px. Transparent background (an optional soft contact shadow under the feet may be kept on the alpha). Wearing plain swim shorts.
4. A-POSE (NAKED) — identical to view 3 in every pixel of pose, scale, and framing, but without the swim shorts (cartoon-neutral, no anatomical detail). Create it by editing the swim A-pose to remove only the shorts, so the two overlay exactly.

## Generation, Dimensions, and Delivery

1. Pass the reference (or the portrait identity anchor) into every generation call so identity, colours, and rendering stay continuous; keep each prompt about framing and pose only, and state that nothing about the character may change.
2. Use an image model with true alpha transparency for every view — `portrait`, `full`, and both A-poses; a soft contact shadow under the A-pose feet may be kept on the alpha.
3. Hit exact pixel dimensions: generate at the nearest supported ratio, then resize to the target width and crop or pad the other axis so the required margins survive — never centre-crop so aggressively that feet, the head margin, or the A-pose 5%/92% positions are lost.
4. Consistency check: placed side by side with the reference, the character must be recognisably identical in every view — only framing and pose may differ, and the two A-poses must overlay exactly. Regenerate any drifted view with the reference re-passed and the drift named explicitly.
5. Deliver each view as its own file, named: `<character>[-<persona>][_<look>]_portrait-neutral.png`, `<character>[-<persona>][_<look>]_full-neutral.png`, `<character>_apose-swim.png`, `<character>_apose-naked.png`.
</content>
</invoke>
