<!-- status: for the owner's review, 2026-11-24 (fourth version: the extended Library of LIBRARY.md drawn, 2026-11-24) – the screens of lap 2 after the planning session: what each screen shows once today's decisions are applied, and how far its picture is. The first version (the list before drawing, 46 screens) is in git (123d4a282). Decisions: DECISIONS.md. Pictures: draft.html `?s=<id>`, kit-d.html, show.html, spores-view.html, spores-demo.html. -->
# Lap 2 – the screens

**Picture** says how far the drawing is:
- **current** – drawn in the approved kit (kit-d.html, palette D) with today's decisions;
- **moved** – an earlier feature draft brought in unchanged: its palette moved to D (no accent, no lime, see-through grounds, no outlined tabs, no blur on panels), its layout and its own notes as drawn then (decided "decide" cards removed);
- **words** – needs your words before it can be drawn: today's screen on the kit where a capture exists, the open point as one caption.

Every screen is 1440×900, night first, drawn from today's screen (`now/`, the workspace captures `now/ws-*`) with each change marked. Redrawn on the kit 2026-11-24 (branch docs/626-lap2-redraw); the extended Library (LIBRARY.md) drawn on branch docs/626-lap2-library; shots in `shots/<id>-night.png`, A and B also `-day`. A yellow "confirm" card marks each of LIBRARY.md's six "follows – confirm" lines where a screen shows it.

## A · The shell (creator and deck)
| # | Screen | After today's decisions | Picture |
|---|---|---|---|
| A1 | The bar | one bar for both apps: item tabs · workspace row with its ⌄ always · undo, redo, Publish, account, ☰ as icons; no Show pill | current |
| A2 | The ☰ menu | your list: Views with Save workspace · Search · Settings · Docs · Help · Go to · the build · the edit language; Views only here | current |
| A3 | The switcher | over the blurred bar, stage and panels; no bar, no search field, no Views; one list for both apps; cards are full captures; the content has a maximum width; no foot | current |
| A3b | Enter on a deck card | the browser asks once to open the deck; running, the deck comes up on that workspace | current (the browser's own dialog as Chrome draws it) |
| A4 | Search | its own overlay (Alt Space); the field says "Find"; it finds things and files by name, each with its type or kind | current – on the overlay level, blurred behind (H15) |
| A5 | The user menu | drops under the account icon | current |
| A6 | The status bar | as today | current |
| A7 | First open | the crafts question, asked once; the deck's Home | current (the deck's Home redrawn on the kit) |
| A8 | Resume on another device | built (#838) | current |
| A9 | A craft shapes the bar | the translator's bar with the edit language | current |
| A10 | Publish: the dialog | on the overlay level, opened by the bar's icon: every draft readers do not see yet, ticked; All · None; "Publish 8"; the same box as Flash (D3b) | current |

## B · Designing a workspace (creator and deck)
| # | Screen | After today's decisions | Picture |
|---|---|---|---|
| B1 | Views: every view, four states | in the ☰ menu only | current |
| B2 | The workspace tab's own menu | answered: no menu of its own in this lap; Views and Save workspace are in the ☰ | done or dropped – today on the kit |
| B3 | The four stage layouts | built in lap 1; a reference sheet, not a screen – keep or drop | current |
| B4 | A panel taken full | a box close to the window's edges over the blurred workspace; Esc back | current |
| B5 | The Info view | hover text shows here; no info in panels | current |
| B6 | The Log | the type filter as icon chips | current |
| B7 | The switcher's cards | full captures, the text over the picture (#842) | current |

## C · The creator's workspaces
| # | Screen | After today's decisions | Picture |
|---|---|---|---|
| C1 | Toon | the tree of one toon: the season, its chapters and a Cast branch; the season's cast listed and edited here (order, role per language, personas, Add from the Library, ✕); a character opened from it goes to Characters | current (from now/ws-toon, drawn with Cat Pope; card: the Toon tree's top level) |
| C2 | Chapter | kit look | current (panels from presets.ts; the page not captured) |
| C3 | Storyboard | drawn from today's page (Cat Pope · chapter 3): the six steps and every choice as segments, the tools in their pills, the shot's page on the stage, the Inspector, the Timeline with the picked shots back to back; the left panel is the Shots filmstrip | current |
| C3b | Storyboard · 4 Handheld: connecting a phone | the Handheld panel on the kit: on or off for the shot, the code to scan, waiting, Disconnect · Recenter · Record, the takes, trim, smooth, render; the conditions and the trim's sentence as hover text | current |
| C3c | Storyboard · 5 Generate: the shot's asset slot | the slot takes a picture or a video – picked from the Library, uploaded, or a generator added from the Library; with a generator in it, "Open" goes to its canvas; "by hand" below as today | current |
| C3d | Storyboard → the generator: the canvas for a shot | as designed 2026-11-22: every spore shows its result as its face, a free item on the canvas, the hooked spore's driver, takes at the output and one assigned to the slot, by hand or automated. In lap 2 the creator has the stub | current |
| C4 | Characters | the Rig tab without the Bones and Clips lists – the rest as today (the look's 3D model, the model's line, Idle, Body, A-pose) – and "Open in the deck"; no Import images; 2D-first or 3D-first is open | current |
| C4b | Characters: the Cast tab | "cast in": the seasons the character is cast in, each a link to the toon's season – not an editor | current (card: the Cast tab) |
| C5 | Scenes: the scene page | no Tree of all scenes (the Library opens scenes); the scene's preview covers the whole back; "Edit in deck" on it; the facts and versions in a panel | current |
| C6 | Rigs | "Open in the deck" under the model's line; the rest of the Rig tab as C4 | current; built: a rig opened from the Library or Search (`/edit/rig:character.<id>`) lands in Sound design, not on the Rig tab (#896 brings it there) |
| C7 | Flow | kit look | current (from now/ws-flow) |
| C8 | Sound design: the module editor (Parameters · Nodes · Rust · Assignment, Problems, Console, the knobs) | never designed – today's editor is captured (now/ws-sound). Thought aloud: it might have to be removed, because modules are edited in the deck; then a module in the creator is a preview with "Edit in deck" | built in the kit look (#875): Parameters · Nodes · Rust · Assignment, Problems, Console, the knobs – still a fixed column and bands, not panels (#896); not drawn |
| C9 | Catalog | an entry open, no list of every entry (the Library finds them); no Import images | current (from now/ws-catalog; the kind tabs kept as today; card: the Catalog's top level) |
| C10 | Map | the 0 px bug fixed; no Import images; kit look on its tool strip | current |
| C11 | Generate | the stub stays in this lap; the deck's canvas comes with #764 | current (panels from presets.ts; the stub not captured) |
| C12 | Library | things and files in one list; the type the first filter (things · files), "Items to open" gone; places: this item · this season's cast · this toon · everything; a thing's tile its type, a file's its kind; videos a preview picture (still pending where none) | current – the panel (card: one list); C12b the browser, C12c the pick |
| C12b | Library: the browser | the cast's place (Cat Pope · Season 1) with Cat Pope stepped into: personas → looks → moods, its files beside them, "used in"; no "To do" / Missing credits (H17) | current (card: "By cast") |
| C12c | Library: the pick | opened from a chapter event's character slot: Characters only, "Cat Pope · Season 1 · cast" first, everyone else after; picking beyond the cast adds to it | current (card: one picker) |
| C13 | Settings | kit look; the colour-blind mode as a setting | current |
| C14 | The writing page (Kanon · Notifications · Credits · Edit · Guide) | the text on the stage in one reading column, no sheet or box; the tools come to the selection; Outline and Facts as panels; words, reading time and the save state in the status line – drawn on your word "go with the flow" (#876), yours to correct | current – drawn new, no capture of today |
| C14b | The writing page: translating | Compare: the source beside the edit language, two columns of the same width; an untranslated paragraph is an empty line, never a warning | current – drawn new |
| C15 | XR | a creator workspace, with the Place tab | moved – epic-drafts `?s=714` |

## D · The deck's workspaces
| # | Screen | After today's decisions | Picture |
|---|---|---|---|
| D1 | The switcher over a workspace | as A3; Home stays when nothing is open; no Needs review box | current |
| D2 | Scene (today "Build") | the shared bar, Views in the ☰, kit look; the Library the same panel as C12 | current (the deck's panels redrawn on the kit over the shot; its colours – mixer, thumbnails, gizmo – as shot) |
| D3 | Show | no pill of its own in the bar: Cues, Problems, Who, Session, Vote and Demos are views; the row's ⚡ slot with Find; the tab "Spores"; no thick left bar | current – show.html `?v=0`, spores-view.html (the draft shows show.html, the Library drawn over it as C12) |
| D3b | Show · Flash: the same dialog as Publish | opened from the bar beside Publish; on the overlay level: the devices ticked, All · None, "Flash 2", then the confirm, then the report; the words as today | current |
| D3c | Show · Scene: a dialog | on the overlay level: a new deck scene or a recast – id, stage, the cast from the toon's rigs; saves itself as a draft; the words as today | current |
| D4 | Module, nothing open | the shared bar, kit look | current; built: the Module workspace is in neither the row nor the switcher – reached only by `--module` (#909) |
| D4b | **Module, a module open** (new) | never drawn: its Parameters, Problems, the module heard alone on the stage, Derive for a factory module; the code opens in the external editor | words – no capture: a frame and the open point |
| D5 | An asset open | the one place an asset's inside is shown; "Edit in deck" lands here; the Library as C12 | current |
| D6 | Rigging | #830 | moved – crafts-v2 `rig` |
| D7 | ~~XR~~ | dropped – XR is the creator's | – |
| D8 | A scene opened from the creator | the deck comes up with that scene; the Library as C12 | current |

## E · Features drawn before, brought into the set
| # | Screen | After today's decisions | Picture |
|---|---|---|---|
| E1 | Map as a view | unchanged | moved – epic-drafts `?s=714` |
| E1b | Map as a view – on the stage | unchanged | moved – epic-drafts `?s=714m` |
| E2 | XR at a place (geolocation) | unchanged | moved – epic-drafts `?s=714r` |
| E3 | Spores: generators and slots | the canvas is the deck's (#764); the trigger as in the demo | current – spores-demo.html `?d=gen` (the draft shows it) |
| E4 | The network | now the Spores view, level by level | current – spores-view.html (the draft opens `?v=3`, the tab "Spores") |
| E5 | Scenes follow-ups | `event:<id>` decided | moved – epic-drafts `?s=706` · E5c `?s=707` (its decide cards gone: decided) · E5d `?s=708` |
| E5b | "Fires on" | no sheet: the picker under the row's ⚡ slot | current – show.html `?v=1` (the draft shows it) |
| E5c | Scenes follow-ups: events + choices in the address book | `event:<id>` decided | moved – epic-drafts `?s=707` |
| E5d | Scenes follow-ups: a show with several toons | unchanged | moved – epic-drafts `?s=708` |

## F · Concepts
| # | Screen | State |
|---|---|---|
| F1 | Catalog and Library as one concept | current – answered by LIBRARY.md: the entries found in the Library (type Items), opened in the Catalog (card: the Catalog's top level, one list) |
| F2 | The one-look sheet | done as kit-d.html (approved) |
| F3 | The scene page's contents | answered: a preview and the link to the deck (C5) |

## Pages outside the set
| Page | What it is | State |
|---|---|---|
| kit-d.html | the kit in palette D, with your picks | approved |
| show.html | the Show view, five ways to pick an event | settled by the Spores demo: the slot and the cable |
| spores-view.html | the Spores view, level by level | drawn with the signal colours |
| spores-demo.html | two chains played step by step | approved |
| universe.html | the twelve kinds as one tree, production counts | for reading – palette D ("Creator", "Deck" quiet chips) |
| modules-dna.html | module · rig · slot · network | draft, six points open – palette D (see-through grounds) |

**Count:** 11 + 7 + 21 + 11 + 9 + 3 = 62 screens (A10, C3b, C3c, C3d, C4b, C12b, C12c, D3b, D3c, E1b, E5c, E5d drawn as their own screens). **Current: 47 · moved: 8 · words: 4 · done or dropped: 3 · old: 0.**
