<!-- status: concept review for the owner, round 1 (2026-11-24) – the Storyboard as the point where the work splits after the canon: for the webtoon, for video, for 3D scenes and for XR. Nothing decided except what is marked. Facts read from storyboard:catpope:3, shared/content/src/storyboard, creator/src/lib/components/storyboard and today's page. Pictures: draft.html `?s=C3`, `C3b`, `C3c`, `C3d`. -->
# The Storyboard – review, round 1

## The owner's words (2026-11-24)
- *"The tree before was more like a filmstrip, which we have in the XR. I remember the UI is like: the timeline shows as many shots next to each other as selected."*
- *"Let's review that concept to work with both our webtoon and with video material, or scenes for 3D or XR. We use the storyboard as transition point for many workflows after the canonicals."*
- *"The generator in the storyboard – I need to see how the canvas actually looks like."*
- Standing: 2026-10-19 – *"the left side is our timeline vertically, the toon order so to speak"* · *"the other timeline underneath … only shows those shots that are selected"*.

## Today
| Thing | What it is |
|---|---|
| The document | one per chapter (`storyboard:<toon>:<chapter>`): scenes from the script (heading, cast, places, light) → groups → shots. Cat Pope · chapter 3: 10 scenes, 62 shots |
| A shot | an id · a **format** with its aspect (vertical 1:3 to 1:5 · viewport 9:16 · loop360 3:1) · its framing (size, angle, move) · who is in it · its dialog and action · a view (real · belief · narration · memory) · the **source it wants** (photo · direct · unreal · generated · art) · its reading time · a review |
| The page | six steps – Script · Shots · Layout · Handheld · Generate · Export |
| The shot list | the chapter's shots in order, by scene, each with its frame; a press is the current shot, Ctrl and Shift add to the picked set. In production today the left panel shows the toon's pages instead |
| The main view | the Viewport (the current shot in 3D) or Variants (a band of generations per picked shot) |
| The Timeline | one lane: the picked shots' clips back to back; shown as Shot · Take · Rendered · Generated |
| A Filmstrip view | named in lap 1's plan as one view for "an ordered strip of the open item's parts" – a storyboard's shots, a chapter's slices, an experience's scenes |

## The concept
1. **The shot is what travels.** Canon → script → scene → shot. After the shot the work splits by medium.
2. **Two facts on a shot say where it goes.** Its format is the frame: vertical is a slice that scrolls (webtoon), viewport is one screen (webtoon and video), loop360 is a look-around (XR). The source it wants is who makes the picture: a photo, directed in 3D, Unreal, a generator, drawn art.
3. **One view for the order: the Shots filmstrip.** Decided for the Storyboard (your words above). Picking in it drives the rest: the stage shows the current shot, the Timeline the picked ones back to back, Variants a band for each.
4. **One slot per shot: its picture or clip.** It takes a file from the Library or a generator, a take from the hand camera, or a render.
5. **The storyboard keeps the order, the framing, the wanted source and the review. Making the picture happens elsewhere** and comes back into the slot.

## Where a shot goes
| Medium | The shot becomes | Worked on in | Comes back as |
|---|---|---|---|
| Webtoon | a slice of the chapter (vertical or viewport) | the Chapter workspace | the slice's picture |
| Video | a clip: a take, a render or a generated clip | the Storyboard's Handheld step and Timeline; Export | the shot's clip |
| 3D scene | a scene with the shot's place, cast and camera | the deck ("Edit in deck") | a still or a render of that scene |
| XR | a look-around or a target's scene | the XR workspace | – |
| Any | a generated picture or clip | the generator (C3d) | the slot's file |

Which of these hand-overs exist today was not checked; the table is the proposal.

## Open
1. **One Filmstrip for shots, a chapter's slices and XR scenes** – the same panel and the same picking in all three?
2. **How a shot hands over:** a link that opens the slice, the deck scene or the XR scene – or does the storyboard create it there?
3. **The stage with several shots picked:** today it shows the current one; should it show the picked ones side by side, as the Timeline does?
4. **"Unreal" as a wanted source:** still its own way, or does the deck take its place?
5. **A storyboard's scene and a deck scene:** the same thing under one address, or does the first point at the second?
6. **The generator's canvas for a shot (C3d)** in lap 2 is the creator's stub; the canvas as drawn is the deck's and comes with #764. Is the stub enough for this lap?
