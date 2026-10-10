<!-- Exported 2026-10-10 from the owner's Luma skill "Visual Novel Expression Set" (skills/visual-novel-expression-set/SKILL.md, v8, updated 2026-10-05). -->

# Visual Novel Expression Set

Build a swap-ready set of dialogue portraits from ONE locked character reference: every frame shares framing so they overlay in a dialogue box, and only the face and head move.

Per-use variables:
- **Character reference** — the locked design image; the single identity/framing anchor.
- **Character token** — short name for filenames (e.g. `cat`, `mouse`, `frog`).
- **Emotion list** — defaults to the standard 17 below; extend with extras (e.g. `dying`, `dizzy`, `unhinged`) or override entirely on request.
- **Occlusion note** — if a helmet, goggles, or mask hides the eyes, convey emotion through mouth/muzzle, ears, and head posture instead.

Locked constants:
- Head-and-shoulders bust; identical crop, camera distance, scale, shoulder position, and eye-height line in every frame so they overlay cleanly.
- Only facial expression and small natural head posture change; outfit, identity, colours, lighting stay fixed.
- True transparent background; generate square (1:1), then downscale to exactly **600×600** px.
- Filenames `<character>_portrait-<state>.png`, mapped strictly by input position.

## View zoom lock (never varies within a set)
A jumping crop frame-to-frame is the most common failure — forbid it. Choose the view's zoom ONCE and hold the identical crop, subject scale, and eye-line across every mood in the set. Anchor the whole set to ONE reference frame's framing (e.g. a strong mid-set frame like `furious`) and chain it into every generation. Canonical view zooms, subject fully in frame, transparent background:
- **avatar** — head and neck only (cropped at the neck; NO shoulders or chest).
- **portrait** — head to waist.
- **full** — entire body, feet included, even margins.
Never blend two views' zooms; if any frame drifts in zoom, regenerate it against the anchor.

## Generate the aligned set
Batch the whole set in ONE generation job for model consistency, on a transparency-capable image model. Establish the neutral bust FIRST from the character reference, then hold that exact identity and framing across every state (chain the neutral as an added reference). Pass the character reference to every generation for continuity. Change only expression + head posture; keep shoulders, crop, scale, and the eye-height line fixed.

## States
Standard 17, in order: neutral, happy, laughing, sad, crying, angry, furious, surprised, scared, disgusted, confused, thinking, smug, embarrassed, denial, bargaining, acceptance. Append any caller extras after these (e.g. `dying` = two X-shaped pupils with bloodshot, KO'd; `dizzy`; `unhinged`). Generate extras with the same anchor and framing; name them `<character>_portrait-<extra>.png`.

## Normalize, de-halo, and name
In a deterministic code job, for each frame in order, remove the transparent-background HALO with the GENTLEST step that works and PRESERVE the subject's natural anti-aliased edge — never erode into the subject. Start soft: zero only the faint outer wash (very low alpha, e.g. < ~24) and keep the largest connected opaque region. Inspect: only if a prominent semi-transparent colour ring still remains (it lives in the alpha ~128–249 band) escalate to a firmer cut (raise the alpha floor, at most a 1px edge clean), and stop the moment the halo is gone. Over-aggressive thresholding or multi-pixel erosion hardens and eats the cutout edge, which the user will notice and dislike — a soft, clean edge beats a hard one. Then Lanczos-resize to the target size preserving alpha (no crop, no fill). Save strictly by position as `<character>_portrait-<state>.png`. Verify zero residual halo-band pixels before delivering.

## Consistency & artifact check
Before delivering, judge the full set as an overlay against neutral: identity, outfit, colours, framing/scale, and eye-height line must match; only face/head differ. Scan every frame for artifacts — leftover halos or fringe, alpha speckle, clipped edges, and anatomy drift (e.g. stray teeth on a beak/muzzle, a misread emotion, or a head turned so far it breaks alignment). Report findings to the user and offer targeted re-renders; never silently swap a frame.

## Delivery manifest (always)
Always ship a manifest alongside the set so the user can bulk-import/rename. Luma's download wraps every file as `<index>_<assetname>_<assetID>.png` — the user cannot get the bare filename from the export alone. Emit a CSV (`write` with `format="csv"`, columns: `intended_filename,asset_id,state`) listing every delivered frame with its exact intended name (`<character>_portrait-<state>.png`) and its board asset ID, in order. This manifest is the bulk-import map and also drives a post-download rename (strip the leading `\d+_` and trailing `_<ID>`). Name intended files WITHOUT any embedded dots other than the final extension, so the download's core name stays clean. Produce the manifest on every run, not just on request.
</content>
</invoke>
