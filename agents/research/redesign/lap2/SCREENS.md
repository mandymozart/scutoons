<!-- status: for the owner's review, 2026-11-24 (second version) – the screens of lap 2 after the planning session: what each screen shows once today's decisions are applied, and how far its picture is. The first version (the list before drawing, 46 screens) is in git (123d4a282). Decisions: DECISIONS.md. Pictures: draft.html `?s=<id>`, kit-d.html, show.html, spores-view.html, spores-demo.html. -->
# Lap 2 – the screens

**Picture** says how far the drawing is:
- **current** – drawn in the approved kit with today's decisions;
- **shape** – the layout follows today's decisions, the colours and tabs are still the old ones;
- **old** – the first draft: old palette, outlined tabs, changes of today not in;
- **words** – needs your words before it can be drawn.

Every screen is 1440×900, night first, drawn from today's screen (`now/`) with each change marked.

## A · The shell (creator and deck)
| # | Screen | After today's decisions | Picture |
|---|---|---|---|
| A1 | The bar | one bar for both apps: item tabs · workspace row with its ⌄ always · undo, redo, Publish, account, ☰ as icons; no Show pill | old |
| A2 | The ☰ menu | your list: Views with Save workspace · Search · Settings · Docs · the build · the edit language; Views only here | old |
| A3 | The switcher | over the blurred bar, stage and panels; no bar, no search field, no Views; one list for both apps; cards are full captures; the content has a maximum width | shape |
| A3b | Enter on a deck card | the browser asks once to open the deck; running, the deck comes up on that workspace | shape |
| A4 | Search | its own overlay (Alt Space); the field says "Find" | shape |
| A5 | The user menu | drops under the account icon | shape |
| A6 | The status bar | as today | old |
| A7 | First open | the crafts question, asked once; the deck's Home | old |
| A8 | Resume on another device | built (#838) | old |
| A9 | A craft shapes the bar | the translator's bar with the edit language | old |

## B · Designing a workspace (creator and deck)
| # | Screen | After today's decisions | Picture |
|---|---|---|---|
| B1 | Views: every view, four states | in the ☰ menu only | old |
| B2 | The workspace tab's own menu | open: does the tab keep a menu at all | words |
| B3 | The four stage layouts | built in lap 1; a reference sheet, not a screen – keep or drop | old |
| B4 | A panel taken full | a box close to the window's edges over the blurred workspace; Esc back | shape |
| B5 | The Info view | hover text shows here; no info in panels | old |
| B6 | The Log | the type filter as icon chips | old |
| B7 | The switcher's cards | full captures, the text over the picture (#842) | shape |

## C · The creator's workspaces
| # | Screen | After today's decisions | Picture |
|---|---|---|---|
| C1 | Toon | kit look | old |
| C2 | Chapter | kit look | old |
| C3 | Storyboard | kit look | old |
| C4 | Characters | the Rig panel without Bones and Clips: the model's line and "Open in the deck"; no Import images; 2D-first or 3D-first is open | old |
| C5 | Scenes: the scene page | open: what it edits; "Edit in deck" | words |
| C6 | Rigs | "Open in the deck" | old |
| C7 | Flow | kit look | old |
| C8 | Sound design | open: goes, or becomes a preview (modules are edited in the deck) | words |
| C9 | Catalog | no Import images; see F1 | old |
| C10 | Map | the 0 px bug fixed; no Import images; kit look on its tool strip | old |
| C11 | Generate | the stub stays in this lap; the deck's canvas comes with #764 | old |
| C12 | Library | stills for scenes and places; "Items to open" is open | old |
| C13 | Settings | kit look; the colour-blind mode as a setting | old |
| C14 | Kanon · Notifications · Credits · Edit · Guide | kit look | old |
| C15 | XR | a creator workspace, with the Place tab | old |

## D · The deck's workspaces
| # | Screen | After today's decisions | Picture |
|---|---|---|---|
| D1 | The switcher over a workspace | as A3; Home stays when nothing is open | shape |
| D2 | Build | the shared bar, Views in the ☰, kit look | old |
| D3 | Show | its sheets are views; the row's ⚡ slot with Find; the tab "Spores"; no thick left bar | current – show.html, spores-view.html |
| D4 | Module | the shared bar, kit look | old |
| D5 | An asset open | the one place an asset's inside is shown; "Edit in deck" lands here | old |
| D6 | Rigging | #830 | old |
| D7 | ~~XR~~ | dropped – XR is the creator's | – |
| D8 | A scene opened from the creator | the deck comes up with that scene | old |

## E · Features drawn before, brought into the set
| # | Screen | After today's decisions | Picture |
|---|---|---|---|
| E1 | Map as a view | unchanged | old |
| E2 | XR at a place (geolocation) | unchanged | old |
| E3 | Spores: generators and slots | the canvas is the deck's (#764); the trigger as in the demo | old – spores-demo.html `?d=gen` is current |
| E4 | The network | now the Spores view, level by level | current – spores-view.html |
| E5 | Scenes follow-ups | `event:<id>` decided | old |
| E5b | "Fires on" | no sheet: the picker under the row's ⚡ slot | current – show.html `?v=1`, spores-view.html `?v=1` |

## F · Concepts
| # | Screen | State |
|---|---|---|
| F1 | Catalog and Library as one concept | words |
| F2 | The one-look sheet | done as kit-d.html (approved) |
| F3 | The scene page's contents | words (C5) |

## Pages outside the set
| Page | What it is | State |
|---|---|---|
| kit-d.html | the kit in palette D, with your picks | approved |
| show.html | the Show view, five ways to pick an event | settled by the Spores demo: the slot and the cable |
| spores-view.html | the Spores view, level by level | drawn with the signal colours |
| spores-demo.html | two chains played step by step | approved |
| universe.html | the twelve kinds as one tree, production counts | for reading |
| modules-dna.html | module · rig · slot · network | draft, six points open |

**Count:** 10 + 7 + 15 + 8 + 6 + 3 = 49 screens. **Current: 3 · shape: 7 · old: 32 · words: 5 · done or dropped: 2.**
