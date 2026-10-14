Status 2026-10-14: concept with the owner – Q1, Q2, Q4 decided (travel: walk free or ride for cucumbers); nothing built (FEATURES 10.98).

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

## Open (one at a time)
- Walking: where the avatar starts, how fast, which routes / places it may use (revealed only?); guests. (Rides: step 2.)
- Visiting a friend's cave: how you get there (link from the friend, a friend list, the map?), what a visitor may do.
- Racks (CD rack, cassette rack, record shelf) and the workshop as cave furniture: reuse the `holds` rule
  (SCORES.md §2b) for racks; how furniture is obtained and placed.
- Items between inventory and cave (only while there?); gifts / campaigns land in the inventory?; guests
  (device-only inventory) vs members – does a guest have a cave?
- The look: a drawn room with slots, a 2.5D diorama like the map places (MAP.md §5.1), free placement; phone first.
- What it does NOT do now (keep the configuration small).
