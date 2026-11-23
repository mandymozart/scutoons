<!-- status: for the owner's review, 2026-11-24 (third version: every screen redrawn on the kit) – the screens of lap 2 after the planning session: what each screen shows once today's decisions are applied, and how far its picture is. The first version (the list before drawing, 46 screens) is in git (123d4a282). Decisions: DECISIONS.md. Pictures: draft.html `?s=<id>`, kit-d.html, show.html, spores-view.html, spores-demo.html. -->
# Lap 2 – the screens

**Picture** says how far the drawing is:
- **current** – drawn in the approved kit (kit-d.html, palette D) with today's decisions;
- **moved** – an earlier feature draft brought in unchanged: its palette moved to D (no accent, no lime, see-through grounds, no outlined tabs, no blur on panels), its layout and its own notes as drawn then (decided "decide" cards removed);
- **words** – needs your words before it can be drawn: today's screen on the kit where a capture exists, the open point as one caption.

Every screen is 1440×900, night first, drawn from today's screen (`now/`, the workspace captures `now/ws-*`) with each change marked. Redrawn on the kit 2026-11-24 (branch docs/626-lap2-redraw); shots in `shots/<id>-night.png`, A and B also `-day`.

## A · The shell (creator and deck)
| # | Screen | After today's decisions | Picture |
|---|---|---|---|
| A1 | The bar | one bar for both apps: item tabs · workspace row with its ⌄ always · undo, redo, Publish, account, ☰ as icons; no Show pill | current |
| A2 | The ☰ menu | your list: Views with Save workspace · Search · Settings · Docs · the build · the edit language; Views only here | current |
| A3 | The switcher | over the blurred bar, stage and panels; no bar, no search field, no Views; one list for both apps; cards are full captures; the content has a maximum width | current |
| A3b | Enter on a deck card | the browser asks once to open the deck; running, the deck comes up on that workspace | current (the browser's own dialog as Chrome draws it) |
| A4 | Search | its own overlay (Alt Space); the field says "Find" | current (drawn as today: the window dimmed; the level is an open point) |
| A5 | The user menu | drops under the account icon | current |
| A6 | The status bar | as today | current |
| A7 | First open | the crafts question, asked once; the deck's Home | current (the deck's Home redrawn on the kit) |
| A8 | Resume on another device | built (#838) | current |
| A9 | A craft shapes the bar | the translator's bar with the edit language | current |

## B · Designing a workspace (creator and deck)
| # | Screen | After today's decisions | Picture |
|---|---|---|---|
| B1 | Views: every view, four states | in the ☰ menu only | current |
| B2 | The workspace tab's own menu | open: does the tab keep a menu at all | words – today on the kit (the tab opens no menu) and the open point |
| B3 | The four stage layouts | built in lap 1; a reference sheet, not a screen – keep or drop | current |
| B4 | A panel taken full | a box close to the window's edges over the blurred workspace; Esc back | current |
| B5 | The Info view | hover text shows here; no info in panels | current |
| B6 | The Log | the type filter as icon chips | current |
| B7 | The switcher's cards | full captures, the text over the picture (#842) | current |

## C · The creator's workspaces
| # | Screen | After today's decisions | Picture |
|---|---|---|---|
| C1 | Toon | kit look | current (from now/ws-toon) |
| C2 | Chapter | kit look | current (panels from presets.ts; the page not captured) |
| C3 | Storyboard | kit look | current (panels from presets.ts; the page not captured) |
| C4 | Characters | the Rig panel without Bones and Clips: the model's line and "Open in the deck"; no Import images; 2D-first or 3D-first is open | current |
| C5 | Scenes: the scene page | the scene's preview covers the whole back; "Edit in deck" on it; the facts and versions in a panel | current |
| C6 | Rigs | "Open in the deck" | current |
| C7 | Flow | kit look | current (from now/ws-flow) |
| C8 | Sound design: the module editor (Parameters · Nodes · Rust · Assignment, Problems, Console, the knobs) | never designed – today's editor is captured (now/ws-sound). Thought aloud: it might have to be removed, because modules are edited in the deck; then a module in the creator is a preview with "Edit in deck" | words – today on the kit (from now/ws-sound) and the open point |
| C9 | Catalog | no Import images; see F1 | current (from now/ws-catalog) |
| C10 | Map | the 0 px bug fixed; no Import images; kit look on its tool strip | current |
| C11 | Generate | the stub stays in this lap; the deck's canvas comes with #764 | current (panels from presets.ts; the stub not captured) |
| C12 | Library | stills for scenes and places; "Items to open" is open | current – the panel; C12b the browser, C12c the pick |
| C12b | Library: the browser | the same parts as the panel, from the kit; over the stage as today | current |
| C12c | Library: the pick | "Pick for" and the slot as a chip; only what fits | current |
| C13 | Settings | kit look; the colour-blind mode as a setting | current |
| C14 | Kanon · Notifications · Credits · Edit · Guide | to be redesigned for a good writing experience – not only the kit look; today's pages are not captured yet | words – no capture of today: empty frames and the open point |
| C15 | XR | a creator workspace, with the Place tab | moved – epic-drafts `?s=714` |

## D · The deck's workspaces
| # | Screen | After today's decisions | Picture |
|---|---|---|---|
| D1 | The switcher over a workspace | as A3; Home stays when nothing is open | current |
| D2 | Build | the shared bar, Views in the ☰, kit look | current (the deck's panels redrawn on the kit over the shot) |
| D3 | Show | no pill of its own in the bar: Cues, Problems, Who, Session, Vote and Demos are views; the row's ⚡ slot with Find; the tab "Spores"; no thick left bar | current – show.html `?v=0`, spores-view.html (the draft shows show.html) |
| D4 | Module, nothing open | the shared bar, kit look | current |
| D4b | **Module, a module open** (new) | never drawn: its Parameters, Problems, the module heard alone on the stage, Derive for a factory module; the code opens in the external editor | words – no capture: a frame and the open point |
| D5 | An asset open | the one place an asset's inside is shown; "Edit in deck" lands here | current |
| D6 | Rigging | #830 | moved – crafts-v2 `rig` |
| D7 | ~~XR~~ | dropped – XR is the creator's | – |
| D8 | A scene opened from the creator | the deck comes up with that scene | current |

## E · Features drawn before, brought into the set
| # | Screen | After today's decisions | Picture |
|---|---|---|---|
| E1 | Map as a view | unchanged | moved – epic-drafts `?s=714` |
| E1b | Map as a view – on the stage | unchanged | moved – epic-drafts `?s=714m` |
| E2 | XR at a place (geolocation) | unchanged | moved – epic-drafts `?s=714r` |
| E3 | Spores: generators and slots | the canvas is the deck's (#764); the trigger as in the demo | current – spores-demo.html `?d=gen` (the draft shows it) |
| E4 | The network | now the Spores view, level by level | current – spores-view.html (the draft shows it) |
| E5 | Scenes follow-ups | `event:<id>` decided | moved – epic-drafts `?s=706` · E5c `?s=707` (its decide cards gone: decided) · E5d `?s=708` |
| E5b | "Fires on" | no sheet: the picker under the row's ⚡ slot | current – show.html `?v=1` (the draft shows it) |
| E5c | Scenes follow-ups: events + choices in the address book | `event:<id>` decided | moved – epic-drafts `?s=707` |
| E5d | Scenes follow-ups: a show with several toons | unchanged | moved – epic-drafts `?s=708` |

## F · Concepts
| # | Screen | State |
|---|---|---|
| F1 | Catalog and Library as one concept | words |
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

**Count:** 10 + 7 + 17 + 9 + 9 + 3 = 55 screens (C12b, C12c, E1b, E5c, E5d drawn as their own screens). **Current: 39 · moved: 8 · words: 5 · done or dropped: 3 · old: 0.**
