<!-- status: concept, first draft for the owner (2026-11-24, lap 3, epic #903) – from a section of the story to the finished webtoon, video, show or experience, thought without the border between creator and deck. Decided 2026-11-24 (lap3/DECISIONS.md): the section, one workspace for Chapter and Storyboard, the blocking view as the engine's through wasm, one timeline, one thing that plays with three ways to say when; drawn as S1–S4. The other open points stand. No tickets. "Today" is read from shared/content/src/storyboard/types.ts and lap2/STORYBOARD.md; everything under "The concept" is a proposal. -->
# From the storyboard to the finished thing – concept, round 1

## The owner's words (2026-11-24)
- *"Storyboard needs a proper clean-up, since there are like workspaces inside this workspace. We need to figure out a concept."*
- *"Eventually I am thinking of one integrated view, where storyboard elements are just another developing state of a section of the story … and it can skip some or not."* (thought aloud)
- *"Start a concept for storyboard to finished experience, show, webtoon or video. Don't think about deck or editor boundaries, think it all the way."*
- Before it, lap 2: *"We use the storyboard as transition point for many workflows after the canonicals."*

## Today
- A chapter has a storyboard document; in it scenes (from the script) → groups → **shots**. A shot holds its text (action, dialog), its framing (format, aspect, camera, figures, place, light), the source it wants, a slot (`asset`: a picture, a clip or a generator), handheld takes, a review and a status.
- The finished things are other documents: a chapter's **slice** points back at its shot (`Slice.shot`, shared/content content.ts:399); a shot points at the deck **scene** and the XR **experience** it was made into (`Shot.made`). Since today a shot has an address of its own (`shot:<toon>:<chapter>:<id>`, on main with #874) that the Library lists and both apps open.
- A show's **row** fires a scene and can pick one of that scene's camera shots (deck/show document.rs `shot`) – the same word for another thing: a camera position of a scene, not a moment of the story. A video cut as a document was not found (not searched beyond the Storyboard's Timeline and Export).
- The Storyboard page has six steps – Script · Shots · Layout · Handheld · Generate · Export – each with its own panels: the workspaces inside the workspace.
- So today the storyboard is a place and a document beside the finished work. A shot is handed over and then lives twice.

## The concept
1. **One thing, not two: the section.** A section of the story (today's shot; a scene is a run of them) is one thing with one address for its whole life. The storyboard is not where it lives – it is how the section looks while it is young.
2. **A section develops through states, and each state is something it holds.**

   | State | The section holds | Today's name |
   |---|---|---|
   | Written | its text: what happens, who speaks | script line, action, dialog |
   | Blocked | the frame: place, who stands where, camera, light | the shot's framing, the schematic 3D view |
   | Sketched | a rough picture or clip, with a length | a sketch, a handheld take, a generated still, the animatic |
   | Made | the real material | drawn art, a render, the final clip, the built scene, sound |
   | Placed | its place in a medium, with what that medium adds | a slice with lettering, a clip in the cut, a scene on a show's row or an XR target |
   | Out | readers or decks have it | Publish, Flash |

3. **States can be skipped, and nobody sets them.** A drawn slice was never blocked; a show's scene may never be sketched; a look-around may stay a blocking for months. The state is read from what the section holds – no status field to keep, and nothing is flagged as missing (RULES H17).
4. **Earlier states stay, under the later ones.** The blocking lies under the finished picture like a layer: look back at it, compare, go back to it. A section's states are its layers.
5. **One view: the story in order, every section in the furthest state it has.** A chapter in work is a strip where some sections are finished art, some sketches, some only words. It can be read or played at any time as far as it is – the rough webtoon, the animatic, the run-through. A lens shows every section in one chosen state (all as text = the script; all as blocking = the storyboard; all as made = the finished thing).
6. **The six steps become states and tools.** Script is the Written lens; Shots the Blocked lens; Layout is Placed for the webtoon; Handheld and Generate are two ways to fill a section's slot (beside the Library and drawing); Export is sending out. Tools show by what is picked, not by a step – no workspaces inside the workspace.
7. **A medium is an order, not a copy.** The same section can be a slice in the chapter, a clip in the trailer and a scene in the show. What differs is how sections are ordered and what the medium adds:

   | Medium | Order | Adds |
   |---|---|---|
   | Webtoon | reading order, scrolled | lettering, the page, reading time |
   | Video | time, in lanes | cuts, sound, a length |
   | Show | fired by events, by hand or by a clock | who plays it, where (outlets), votes |
   | Experience (XR) | where the reader is and what they find | targets, surfaces, the room |

8. **Blocking is a scene in a simple look.** Thought without the app border: a blocked section is the engine's scene with a camera – place, rigs in a pose, light – drawn schematic. Building the real 3D scene is the same scene worked further, not a second one. One 3D view then serves blocking, a character, an XR scene and a deck scene (today: one in the Storyboard and one in XR, both in the browser, and the deck's).
9. **Three ways to order, one thing ordered.** A section with a length is a clip. A timeline lays clips on time, in lanes (picture, sound, camera, events). Spores wake clips by events. A strip lays them in reading order. Timeline, spores and strip are views of an order – the sections in them are the same. (The full comparison of spores, clips and timelines is its own concept – NOTES.md note 26.)

## What it would replace
- The storyboard document beside the chapter, the hand-over of a shot, and the two pointers (`Slice.shot`, `Shot.made`).
- The Storyboard's six steps as sub-workspaces; possibly Chapter and Storyboard as two workspaces (the same strip through two lenses).
- The browser's two 3D views.

## Open
1. **Decided 2026-11-24: the unit is a section** (S1). Still open in it: is a scene a section too. ~~The word.~~ Section, beat, shot – what is the unit called, and is a scene a section too (sections inside sections) or only a run of them? "Shot" already means two things (a storyboard's shot, a scene's camera shot in a show).
2. **One section in several media**, as drawn above – or one per medium that remembers where it came from? (A slice and a clip of the same moment are cut differently.)
3. **Does a show's row hold a section, or point at one?** A row today fires a scene; a scene is not a moment of a story.
4. **Review and approval:** the shot has a review (approved · change) today. Per section, per state, or gone?
5. **Decided 2026-11-24: Chapter and Storyboard become one workspace** (S1); its name is open. ~~Where the one view lives:~~ in the Chapter workspace, in the Storyboard, or one workspace that replaces both – and what the toon's seasons and chapters are in it (sections of sections?).
6. **Sound from the start:** is sound a state of a section (it "has" its music) or a lane beside the sections?
7. **The lens and the Library:** a section's slot takes a file, a generator, a take – as decided for the slot in lap 2. Does the Library list sections?
8. **How far in lap 3:** this is a model change (documents, addresses), and lap 2 was a design and UX round. Draw it in lap 3 and build it later, or build a first step (the strip with states, on today's documents)?
