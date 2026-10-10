# Luma: character reference sets (paste-ready)

Owner 2026-10-10, from the Runpod / Qwen-Edit trial: character identity is the weak point when a shot gets one small
3D portrait as its reference, and a reference that contradicts the prompt (PLF "human" given a cat-headed image) mixes
the two. One clean set per character + persona + look fixes both, and the same set trains a LoRA later.

## Instruction to paste into Luma (one character at a time)

```
Build a character reference set for <CHARACTER / PERSONA>, from the attached images.
Identity (never changes): <face, build, costume, markings – e.g. "human priest, no cat features at all, long white papal
robe with blue embroidery, red mark on the forehead">.
Keep identity, costume and proportions exactly the same in every image. Plain light grey background, even soft light,
no text, no logos, no frames, one figure per image, the whole figure or head inside the frame (nothing cut off).

1. Turnaround, full body, neutral standing pose, 2:3 – front, three-quarter front left, profile left, three-quarter back,
   back (5 images).
2. Head and shoulders, 1:1 – front, three-quarter, profile, each neutral; then front views angry, scared, happy, sad,
   surprised (8 images).
3. Action, mixed framing (close / half / full figure about a third each), 2:3 and 9:16 – walking, running, reaching
   out, arms raised in fear, sitting, kneeling, seen from above, seen from below (8–12 images).
4. Details, close-up, 1:1 – the things that must not drift: <e.g. robe embroidery, forehead mark, hands; for the tiger
   the blue runes> (3–6 images).
5. In our style: the same character drawn as a finished black-and-white ink webtoon frame with collage / photo elements
   like the attached style images, in 5–10 different scenes.
```

## What to keep

- **Size:** at least 1024 px on the short side (Luma's 2K output is fine), lossless PNG; never under 1024 (today's
  600 × 600 portraits are too small).
- **Count:** 20–50 varied images per character / persona / look – variety beats quantity; near-duplicates teach the
  model to copy one pose.
- **One identity per set:** PLF human and Cat Pope are separate sets (and later separate LoRA trigger words, e.g.
  `plf_human`, `catpope`); the tiger its own.
- **Name:** `<character>-<persona>_<set>-<view>-<n>.png`, e.g. `catpope-human_turn-front-1.png`,
  `catpope-human_face-scared-1.png`, `catpope-mountaintiger_detail-runes-1.png`.
- **For a shot in Luma:** attach the layout render first, then 1–2 images of each character from these sets (the face
  view for close shots, the turnaround view matching the camera for full figures) – Luma takes up to 9 references.
- **For LoRA training later:** one short caption per image describing pose, framing and setting, not the identity
  (that is the trigger word's job).
</content>
</invoke>
