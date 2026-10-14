Status 2026-10-14: concept – Q1, Q2, Q4 decided; the rest is a design proposal (D1–D7) waiting for the owner; draft `/account/lab/cave`; nothing built (FEATURES 10.98).

# CAVE — the avatar's home (concept, 2026-10-14)

Owner 2026-10-14: "cds will need a cd rack or a cd case remember like in the 90s. cassettes will be put into a little
toy suitcase, or a cassette rack. these racks will be designed in a later session. not inventory but cave... the home
of the avatar". The inventory = what the reader carries (incl. carried containers: song book, DJ trolley, CD case, toy
suitcase – research/SCORES.md §2b); the cave = what the reader keeps at home (racks, furniture).

## Decided
- **Q1 – a place on the world map** (owner 2026-10-14): the cave is reached on the map, not as a panel tab or page.
- **Q2 – getting there costs something** (owner 2026-10-14): you enter the cave when you are there on the map, and
  travelling home costs something: walk for free or ride for cucumbers. **The VR headset** (an item) enters your home from anywhere, even if you
  didn't get there on the map (owner 2026-10-14: "with a VR headset, you can enter your home from remote. also if you
  dont got here on the map"). A new skill (ITEMS.md "Skills"), with the code.
  **Being there = entering for free** (owner 2026-10-14: "if you are there you can just enter for free of course") –
  only the trip home costs. **Payment = cucumbers** (owner 2026-10-14: "currently we accept cucumbers as payment"):
  a ride home is paid in cucumber items (consumed).
- **A walking mode on the map** (owner 2026-10-14: "i do want a walking mode but no base is implemented, perhapse we
  have to do that now as well"): the reader walks on the map for free; today only the story moves a toon's hero
  (`travel` events, MAP.md §3) – there is no reader-driven walking yet. A prerequisite of the cave (FEATURES 10.98).
- **Q4 – the reader's avatar walks** (owner 2026-10-14): the avatar gets its own marker and its own world-wide
  position ("where my avatar is"), separate from the toon heroes the story moves (MAP.md §2.4 `positions`); the reader
  walks it along revealed routes for free. The cave is where the avatar lives.
- **Rides run on networks – a second step** (owner 2026-10-14: "the rides go along a subway, tram, and special
  creature airlift taxi network. and are animated in their respecitve map layer. this needs to be build in a second
  step"): three ride networks – subway, tram, creature airlift taxi – each animated in its own map layer (MAP.md
  `MapLayer`); paid in cucumbers. Step 1 = the cave + walking (+ the VR headset); step 2 = the ride networks.

## Inputs from other sessions
- **Workshop + visiting a friend's cave** (scores session, owner 2026-10-14: "or only at a friends cave with a
  workshop. thats better"): a broken DIY song book (`songbook_diy_broken`) is repaired only at a friend's cave that has
  a workshop – research/SCORES.md §2a. The cave needs visits to a friend's cave and a workshop (furniture or room
  feature) where repairs (recipes) run.

## Design proposal (2026-10-14, for the owner – not decided)
Owner 2026-10-14 (via the session manager): "cave can go step two and make designs for what is missing". Each point
is a proposal with the reason; the owner says yes / changes it. Draft on the real page: `/account/lab/cave` (dev only).

**D1 – start and where the avatar may walk.** Every avatar starts at home: one map place `home` ("Zuhause" / "Home"),
the same spot for everyone, never hidden. The avatar walks only along **revealed routes to unlocked places** (no
spoilers – the fog of war stays the story's); tap a place → "Hingehen" / "Walk here" → the marker walks the shortest
revealed path (the existing marker animation, ≈ 1.2 s per segment); no path → no button. The position is world-wide
reader state next to the map state (MAP.md §2.4: `MapState.avatar: { location } | { route, waypoint }`), synced like
progress (newer write wins), a guest's on the device, merged at sign-in. Where `home` stands on the map: [owner].

**D2 – entering.** The home place's card: "Eintreten" / "Enter" when the avatar is there (free); otherwise "Nach Hause
gehen" (walk, when a path exists) and, with a VR headset in the inventory, "Mit VR-Brille betreten" / "Enter with the
VR headset". The VR headset = a virtual item with a new skill `enter_cave` (ITEMS.md "Skills"); the avatar stays where
it is on the map. Rides for cucumbers: step 2 (the networks).

**D3 – the look: one drawn room with fixed spots, phone first.** A 2.5D diorama in the map places' style (three-quarter
view, MAP.md §5.1), one room, **fixed furniture spots** (proposal: 6 – two wall spots, two floor spots, a corner, the
workbench spot); the avatar's A-pose stands in the middle. No free placement, no walls / colours to decorate now.
Phone: the room fills the width, the reader's items in the pull panel below; tapping a piece of furniture opens its
contents as a sheet. Desktop: the room in the page, the furniture's contents in the right dock (DOCKING.md).

**D4 – furniture = items placed in the cave.** Racks and the workshop are catalog items, category `furniture` (new),
found / gifted / bought / crafted like any item. A rack is a container (`holds`, ITEMS.md "Containers"):
`cd_rack` { categories? / items: CDs, max }, `cassette_rack` (tapes), `record_shelf` (vinyl). The workshop
(`workbench`) has no `holds`; recipes marked `at: "workshop"` run only in a cave with a workbench (the broken song book's
repair, SCORES.md §2a). Placing: in the cave, tap an empty spot → pick a furniture item from the inventory → it stands
there (`inventory.cave_spot`, one column next to `container_id`); "Wegräumen" / "Put away" takes it back into the
inventory (its contents stay in it). Furniture can't be worn or carried into a container.

**D5 – items between inventory and cave.** Only while you are in your cave (there or by VR headset). Into a rack:
the container move that exists (drag, or select-then-tap – `POST /api/inventory/move`); out: back into the inventory.
Gifts and campaigns land in the inventory, as now. Giving away works from the inventory only – take it out of the rack
first (a rack is never given away while it stands). Inventory = what you carry, the cave = what you keep.

**D6 – a friend's cave.** No friend list now: a member shares a **visit link** ("Freund:in einladen" / "Invite a
friend", a token, valid 7 days, revocable) – the link opens the friend's cave directly (it isn't on your map, so no
travel). A visitor sees the room and the racks' contents read-only and may use the **workshop** with their own items
(a repair takes the visitor's broken book and gives it back repaired). No visitor can take or move the host's things.
Server: a `cave_visits` token table; the host's cave answer (furniture + contents, no account data but the
display name).

**D7 – guests and what it does NOT do now.** Guests have a cave on the device (their furniture is in the local
inventory), can walk and visit a friend's link; inviting needs an account (the server must answer for your cave).
Not now: free placement, more rooms, decorating, a friend list, live presence / chat in a cave, a cave on a per-reader
map spot, rides (step 2), selling from the cave.

## Open for the owner
- Yes / changes to D1–D7; where `home` stands on the map; how many spots (D3); rack capacities (D4).
- "cave can go step two": read as "design the open points now". If it means the cave itself moves to step two (walking
  first, then the cave with the rides), the order changes – the owner says which.
