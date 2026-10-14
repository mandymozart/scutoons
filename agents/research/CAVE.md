Status 2026-10-14: concept – Q1, Q2, Q4, home per avatar, full screen decided; D2–D7 decided 2026-10-14 (free placement, base rooms: cave + tatami room; furniture stays in the room); a home per character, a WG of up to 5; draft `/account/lab/cave`; nothing built (FEATURES 10.98).

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

**D1 – home, start, where the avatar may walk.** **Each avatar has its own home** (owner 2026-10-14: "home for each
avatar is where they get first introduced in any story. or set in character definitions. places that do not exist need
to be added as new ones"): the character definition gets `home?: <map place id>` (next to `origin` / `residence`,
characters.ts, edited on the creator's character page); none set = the place where the character is first introduced in
any story (in reading order). A home place that doesn't exist yet is added as a new map place (the map builder, its own
image). The avatar's home = the home of its body's character (ITEMS.md "Reader avatar bodies"); **switching the
character moves you to that character's home** (owner 2026-10-14: "if they switch character. yes, they move, each
character has their own, but they can move around"). **A WG / shared flat:** a reader's avatars (the characters they
have) can move in with each other – at most **5 of your avatars in one home** (owner 2026-10-14: "the most that is
allowed is to house 5 of your avatars in one home. its claled WG oder shared flat"); moving in = that home becomes the
character's home for this reader (reader state, the character's default `home` stays). Every avatar starts there. It walks only along **revealed routes to unlocked
places** (no spoilers – the fog of war stays the story's); tap a place → "Hingehen" / "Walk here" → the marker walks the
shortest revealed path (the existing marker animation, ≈ 1.2 s per segment); no path → no button. The position is
world-wide reader state next to the map state (MAP.md §2.4: `MapState.avatar: { location } | { route, waypoint }`),
synced like progress (newer write wins), a guest's on the device, merged at sign-in. A home place is always shown for
its avatar, even before the story unlocks it.

**D2 – entering** (owner 2026-10-14: yes). The home place's card: "Eintreten" / "Enter" when the avatar is there (free); otherwise "Nach Hause
gehen" (walk, when a path exists) and, with a VR headset in the inventory, "Mit VR-Brille betreten" / "Enter with the
VR headset". The VR headset = a virtual item with a new skill `enter_cave` (ITEMS.md "Skills"); the avatar stays where
it is on the map. Rides for cucumbers: step 2 (the networks).

**D3 – the look: a full-screen view, a base room, free placement, phone first.** Full screen (owner 2026-10-14:
"the cave design is a full screen i will need this later in the live scenario as well"): built as its own component in
`@scutoons/reader` (no app state, like the ScoreView) so live mode (research/LIVE.md) reuses it; a bar with where you
are + "Leave". **Free placement** (owner 2026-10-14: "free placement of course"): furniture stands where the reader
drags it (a position in the room, in % of the room – resolution-independent). **Base rooms** (owner 2026-10-14:
"background can be diffrent base rooms, we start with a cave and a japanese monks room with tatami and with nothing in
there"): the room's background is one of the base rooms – to start: **the cave** and **a Japanese monk's room** (tatami,
empty); a 2.5D diorama in the map places' style (three-quarter view, MAP.md §5.1), the owner's art later (the draft
draws placeholders). The avatar's A-pose stands in the room. Phone: the room on top, the furniture's contents and the
reader's items scroll below. Desktop: the room fills the height, the panel at the side.

**D4 – furniture = items that live in the room.** Racks and the workbench are catalog items, category `furniture`
(new). A rack is a container (`holds`, ITEMS.md "Containers"): `cd_rack` (CDs), `cassette_rack` (tapes),
`record_shelf` (vinyl). **Every home has a workbench** (owner 2026-10-14: "workbench. yes, we can add that to each
apartment, place it somehwere"): it comes with the room, the reader moves it like any furniture; recipes marked
`at: "workshop"` run at a workbench (the broken song book's repair, SCORES.md §2a – at a friend's). **Furniture can't
leave a room for now** (owner 2026-10-14: "furniture items can not leave a room for now"): once placed it stays in the
room – moved freely inside it, never back into the inventory, never given away. Placing: select a furniture item in the
inventory, tap a point in the room → it stands there (`inventory.room_pos` { x, y }, next to `container_id`).

**D5 – items between inventory and cave** (owner 2026-10-14: "just answered" – by D4: furniture stays in the room;
the rest as proposed). Only while you are in your cave (there or by VR headset). Into a rack: the container move that
exists (drag, or select-then-tap – `POST /api/inventory/move`); out: back into the inventory. Gifts and campaigns land
in the inventory, as now. Giving away works from the inventory only – take it out of the rack first.

**D6 – a friend's cave** (owner 2026-10-14: yes). No friend list now: a member shares a **visit link** ("Freund:in einladen" / "Invite a
friend", a token, valid 7 days, revocable) – the link opens the friend's cave directly (it isn't on your map, so no
travel). A visitor sees the room and the racks' contents read-only and may use the **workshop** with their own items
(a repair takes the visitor's broken book and gives it back repaired). No visitor can take or move the host's things.
Server: a `cave_visits` token table; the host's cave answer (furniture + contents, no account data but the
display name).

**D7 – guests and what it does NOT do now** (owner 2026-10-14: "no more rooms, no chat and presence, selling").
Guests have a cave on the device (their furniture in the local inventory), can walk and visit a friend's link; inviting
needs an account (the server must answer for your cave). Not now: more rooms (one home, one base room), decorating
beyond the base room, a friend list, live presence / chat, a per-reader map spot, rides (step 2), selling.

## Open for the owner
- Moving in (WG): does the moving avatar's furniture come along (it can't leave its room – D4)?; who picks the base room (the reader, or set per character like the
  home?) and can it be changed; how a new furniture item arrives (in the inventory until placed – proposed); rack
  capacities (D4).
- "cave can go step two": read as "design the open points now". If it means the cave itself moves to step two (walking
  first, then the cave with the rides), the order changes – the owner says which.
