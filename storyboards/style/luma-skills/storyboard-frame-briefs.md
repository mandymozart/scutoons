# Luma agent: how it briefs a storyboard frame (exported 2026-10-10)

Read from the owner's Luma board "Characters" (project 4172a1e5…, board 062afb3f…): the creative-job briefs the Luma
agent wrote for the webtoon frames made with GPT Image 2.5 on 2026-10-09 (sb-28-2, sb-30-2, sb-33-1, the restyles).
The board has no saved directives/memory and its two skills are about character view sets (see the other files here) –
the quality comes from **these briefs**: the agent turns the owner's short request into a long, fixed-structure brief.
Our `comfy.mjs` instruction is a short paragraph; this is what to copy.

## The rules the briefs follow (distilled)

1. **Fixed sections, every time:** DELIVERABLE · INPUT ASSETS · EXPECTED OUTPUT · TECHNICAL REQUIREMENTS · WORKFLOW STEPS ·
   CREATIVE DIRECTION · CONTEXT · ORIGINAL REQUEST. The key facts are repeated in several sections (deliverable, expected
   output, workflow steps) – redundancy on purpose.
2. **Each reference gets a named role:** "image:X is the STRONG LAYOUT reference (our 3D render) — keep its EXACT
   composition, camera, poses, and light placement; it IS the layout." · "image:Y = the mountaintiger (identity)" ·
   "image:Z = catpope-human full 'scared' (pose/identity)" · a style reference "for the style only".
3. **Identity in words, next to the image:**
   - **catpope-human (PLF):** human body, white papal cassock + short mozzetta cape, **hairless sphynx-cat SILICONE MASK
     head with very large upright ears** — NOT a cartoon cat.
   - **catpope (regular, narrator):** CARTOON Cat Pope — cartoon sphynx-cat head, very large upright ears, white papal
     robe.
   - **catpope-mountaintiger:** huge white mountain tiger, glowing rune / glyph sigil on forehead and body (six limbs in
     the tiger panels); "the tiger IS Cat Pope himself".
   - Costume (turnaround jobs): head not hood; the blue graphics are a two-layer silk-screen print (dark blue +
     pearlescent blue), never embroidery; motifs only from the approved screens (no guitars / skulls).
4. **Placement in numbers:** where (RIGHT-MIDDLE, LEFT-BOTTOM, CENTRE-MIDDLE), how big (~80 % / ~30 % / ~55 % of the
   height), facing (back to camera, profile facing LEFT), and an empty band ("keep TOP 12 % calm/empty").
5. **Camera in words:** close-up / medium, low angle looking up, normal perspective.
6. **Light is the most important part:** ONE extremely hard key, its source and direction (snow from BELOW, candle from
   below, ring light from the front at eye height), "NO fill" or "a little fill (softness ~0.5) — shadow side dark but
   READABLE, solid black only in the deepest shadows", what turns white and what turns black ("teeth white, throat solid
   black").
7. **Look, always the same words:** "clean crisp contour lines + solid black shadow shapes, intense noir B&W — NO
   hatching, grey, sketch, or gradients." Restyles name a finished frame as the style reference (the tiger panel
   `oZtqespq` = `storyboards/style/Characters_sb-28-2_Storyboard_Frame_oZtqespq.png`).
8. **Negatives, always:** no text, lettering, numbers, speech bubbles, panel borders, watermark, or colour. UI elements
   as "icon SHAPES only".
9. **Creative direction:** two or three sentences of mood ("A nightmare looming up out of the snow — …").
10. **Context:** shot id, aspect, scene heading, the story meaning ("subjective POV: PLF sees a monstrous tiger; it is
    really himself").

## Example: sb-28-2 (the tiger frame, GPT Image 2.5)

```
THIS JOB'S DELIVERABLE: One 9:16 storyboard frame, clean black-and-white LINE ART, matching the layout of image:Q-08v81X. Composition: the frame is mostly filled by the HEAD of a HUGE white mountain tiger — jaws WIDE OPEN, enormous teeth, warm breath steaming into cold air, angry, half-upright with big claws, a rune glyph on the forehead and body, facing camera, close-up, in the RIGHT-MIDDLE at ~80% of the height. Small in the LEFT-BOTTOM at ~30% height: PLF (catpope-human, white papal robe + mozzetta, sphynx-mask head, big ears) terrified, arms raised, BACK TO CAMERA, backing away, in the shadow of the tiger's head. CAMERA: close-up, low angle looking up, normal perspective (per the layout). LIGHT (most important): ONE extremely hard SNOW key light from BELOW the faces (menacing), long hard cast shadows, NO fill — surfaces facing the light pure white, everything else solid black; snow reflection from below turns the maw into a grotesque mask (teeth white, throat/gullet solid black), PLF small in the head's shadow. LOOK: clean crisp even contour lines + solid black shadow shapes, intense noir B&W — NO hatching, grey, sketch, or gradients. No text, lettering, speech bubbles, panel borders, watermark, or colour.
INPUT ASSETS: artifact:Q-08v81X, artifact:OnugoXgF, artifact:Y8-NShcH
EXPECTED OUTPUT: 1 image — 9:16 vertical. Clean B&W line art (crisp contours + solid black shadow shapes; no hatching/grey/gradient/sketch). Keep composition/camera/light of image:Q-08v81X. Tiger head ~80% right-middle, jaws wide open; catpope-human small left-bottom ~30%, back to camera, terrified. Hard snow key from below. No text/borders/watermark/colour.
TECHNICAL REQUIREMENTS: Exploration tier. 9:16.
WORKFLOW STEPS: 1. Follow image:Q-08v81X exactly for composition/camera/pose/light. 2. Tiger = image:OnugoXgF identity (white, rune glyph), angry, jaws wide open, teeth, steaming breath, ~80% right-middle, facing camera, low angle. 3. PLF = catpope-human (image:Y8-NShcH), terrified, arms raised, back to camera, ~30% left-bottom, in the tiger's shadow. 4. Clean B&W line art, solid black shadows from ONE hard snow key BELOW (teeth white, throat black). 5. No hatching/grey/gradient/sketch, no text/borders/watermark/colour.
CREATIVE DIRECTION: A nightmare looming up out of the snow — the white tiger's maw thrown open, teeth blazing white, gullet pure black, breath smoking into the freezing air, while the tiny robed figure cowers in its shadow with arms flung up. Hard uplight carving everything into stark black-and-white. Clean, graphic, terrifying.
CONTEXT: One vertical-scroll webtoon storyboard frame, shot sb-28-2, 9:16. image:Q-08v81X is the STRONG LAYOUT reference (our 3D render) — keep its EXACT composition, camera, poses, and light placement; it IS the layout. image:OnugoXgF = the mountaintiger (catpope-mountaintiger portrait, white fur, glowing rune/glyph sigil) — the tiger IS Cat Pope himself. image:Y8-NShcH = catpope-human full 'scared' (human body, white papal cassock + mozzetta, sphynx-cat silicone-mask head, big upright ears). Scene 28 EXT. forest clearing, snow — subjective/belief POV (PLF sees a monstrous tiger; it is really himself).
```

## Example: sb-30-2 (entranced at the shelf)

```
THIS JOB'S DELIVERABLE: One 9:16 storyboard frame, clean B&W line art, matching the layout of image:IflLqD6t: PLF (catpope-human) ENTRANCED, reaching out with one hand flat on the shelf, eyes closed, shown in PROFILE facing LEFT, medium shot, CENTRE-MIDDLE of the frame at ~55% of the height. White WAVE-LINES ripple through the wooden shelves and book spines; candle flames quiver. CAMERA: medium, low angle looking up. LIGHT: ONE extremely hard candle key from BELOW the face (menacing) PLUS a little fill (softness ~0.5) — surfaces facing the key white, shadow side dark but READABLE, solid black only in the deepest shadows; one clear direction, never flat; the polished floor throws light up onto PLF's closed eyes; white wave-lines white over black wood. Keep TOP 12% calm/empty. LOOK: clean crisp contour lines + solid black shadow shapes, noir B&W — NO hatching, grey, sketch, or gradients. No text, lettering, speech bubbles, panel borders, watermark, or colour.
CONTEXT: … PLF = CATPOPE-HUMAN (human body, white papal cassock + short mozzetta cape, hairless sphynx-cat SILICONE MASK head with very large upright ears) — NOT a cartoon cat. image:YQtEaU3r = catpope-human full 'surprised' for pose/identity; image:tK17U3zb = the silicone-mask identity. Scene 30 INT. library …
```

## Example: sb-33-1 (the streamer, regular Cat Pope) – the UI rule

```
… Frame it like a livestream screen but with ICON SHAPES ONLY (NO letters/numbers): a small LIVE-badge shape, a few rising heart icons, a side chat column of blank bars. … Character is the REGULAR CARTOON Cat Pope (NOT the human variant): cartoon sphynx-cat head with very large upright ears, white papal robe — the narrator speaking to the reader.
```
</content>
</invoke>
