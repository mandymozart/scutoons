Status 2026-10-14: concept decided (Q1–Q4, D1–D7, the data model; the home scene per avatar, racks max 50, the mover 50 cucumbers – may come with the first build); open: the build order ("cave can go step two"), the mover's form; draft `/account/lab/cave`; nothing built (FEATURES 10.98).

# CAVE — the avatar's home (concept, 2026-10-14)

**Terms** (owner 2026-10-14: "cave or tatami room means this examples for a home scene … i said it to be clear in the
data model"): a **home** = an avatar's place on the map + its room (the feature; it started as "the cave"); a **home
scene** = what a home looks like – `cave`, `tatami_room` are the first two; **furniture** = items standing in a home;
a **WG** (shared flat) = a home several of your avatars live in. "Cave" in this file's name and older lines = the home.

Owner 2026-10-14: "cds will need a cd rack or a cd case remember like in the 90s. cassettes will be put into a little
toy suitcase, or a cassette rack. these racks will be designed in a later session. not inventory but cave... the home
of the avatar". The inventory = what the reader carries (incl. carried containers: song book, DJ trolley, CD case, toy
suitcase – research/SCORES.md §2b); the home = what the reader keeps there (racks, furniture).

## Decided
- **Q1 – a place on the world map** (owner 2026-10-14): the home is reached on the map, not as a panel tab or page.
- **Q2 – getting there costs something** (owner 2026-10-14): you enter your home when you are there on the map, and
  travelling home costs something: walk for free or ride for cucumbers. **The VR headset** (an item) enters your home from anywhere, even if you
  didn't get there on the map (owner 2026-10-14: "with a VR headset, you can enter your home from remote. also if you
  dont got here on the map"). A new skill (ITEMS.md "Skills"), with the code.
  **Being there = entering for free** (owner 2026-10-14: "if you are there you can just enter for free of course") –
  only the trip home costs. **Payment = cucumbers** (owner 2026-10-14: "currently we accept cucumbers as payment"):
  a ride home is paid in cucumber items (consumed).
- **A walking mode on the map** (owner 2026-10-14: "i do want a walking mode but no base is implemented, perhapse we
  have to do that now as well"): the reader walks on the map for free; today only the story moves a toon's hero
  (`travel` events, MAP.md §3) – there is no reader-driven walking yet. A prerequisite of the home (FEATURES 10.98).
- **Q4 – the reader's avatar walks** (owner 2026-10-14): the avatar gets its own marker and its own world-wide
  position ("where my avatar is"), separate from the toon heroes the story moves (MAP.md §2.4 `positions`); the reader
  walks it along revealed routes for free. The home is where the avatar lives.
- **Rides run on networks – a second step** (owner 2026-10-14: "the rides go along a subway, tram, and special
  creature airlift taxi network. and are animated in their respecitve map layer. this needs to be build in a second
  step"): three ride networks – subway, tram, creature airlift taxi – each animated in its own map layer (MAP.md
  `MapLayer`); paid in cucumbers. Step 1 = the home + walking (+ the VR headset); step 2 = the ride networks.

## Inputs from other sessions
- **Workshop + visiting a friend's cave** (scores session, owner 2026-10-14: "or only at a friends cave with a
  workshop. thats better"): a broken DIY song book (`songbook_diy_broken`) is repaired only at a friend's cave that has
  a workshop – research/SCORES.md §2a. The home needs visits to a friend's home and a workbench (D4) where repairs
  (recipes) run.

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
character's home for this reader (reader state, the character's default `home` stays). **Furniture stays behind**
(owner 2026-10-14: "yes, furniture stays behind") – it belongs to the home, not to the avatar – **unless you can afford
a mover** (owner 2026-10-14: "unless they can afford a mover"): a paid move – **50 cucumbers** for now (owner 2026-10-14: "the price is 50 cucumbers for now") – takes the
moving avatar's furniture along into the new home (the one exception to D4's "furniture can't leave its home") –
**not required for the first build, may come
along** (owner 2026-10-14: "which we dont have to build yet", "but hey why not"). Every avatar starts there. It walks only along **revealed routes to unlocked
places** (no spoilers – the fog of war stays the story's); tap a place → "Hingehen" / "Walk here" → the marker walks the
shortest revealed path (the existing marker animation, ≈ 1.2 s per segment); no path → no button. The position is
world-wide reader state next to the map state (MAP.md §2.4: `MapState.avatar: { location } | { route, waypoint }`),
synced like progress (newer write wins), a guest's on the device, merged at sign-in. A home place is always shown for
its avatar, even before the story unlocks it.

**D2 – entering** (owner 2026-10-14: yes). The home place's card: "Eintreten" / "Enter" when the avatar is there (free); otherwise "Nach Hause
gehen" (walk, when a path exists) and, with a VR headset in the inventory, "Mit VR-Brille betreten" / "Enter with the
VR headset". The VR headset = a virtual item with a new skill `enter_cave` (ITEMS.md "Skills"); the avatar stays where
it is on the map. Rides for cucumbers: step 2 (the networks).

**D3 – the look: a full-screen view, a home scene, free placement, phone first.** Full screen (owner 2026-10-14:
"the cave design is a full screen i will need this later in the live scenario as well"): built as its own component in
`@scutoons/reader` (no app state, like the ScoreView) so live mode (research/LIVE.md) reuses it; a bar with where you
are + "Leave". **Free placement** (owner 2026-10-14: "free placement of course"): furniture stands where the reader
drags it (a position in the room, in % of the room – resolution-independent). **Home scenes** (owner 2026-10-14:
"background can be diffrent base rooms, we start with a cave and a japanese monks room with tatami and with nothing in
there"): a home's look is one home scene – to start: **`cave`** and **`tatami_room`** (a Japanese monk's
room, tatami, empty). **The home scene is set per avatar** (owner 2026-10-14: "home scene is set per avatar"): the
character definition says it (`home_scene`), the reader doesn't pick it; a WG shows the scene of the avatar whose home
it is; a 2.5D diorama in the map places' style (three-quarter view, MAP.md §5.1), the owner's art later (the draft
draws placeholders). The avatar's A-pose stands in the room. Phone: the room on top, the furniture's contents and the
reader's items scroll below. Desktop: the room fills the height, the panel at the side.

**D4 – furniture = items that live in the room.** Racks and the workbench are catalog items, category `furniture`
(new). A rack is a container (`holds`, ITEMS.md "Containers"): `cd_rack` (CDs), `cassette_rack` (tapes),
`record_shelf` (vinyl) – **at most 50 each for now** (`max: 50`; owner 2026-10-14: "the cd and vinyl? most 50 for the
moment"). **Every home has a workbench** (owner 2026-10-14: "workbench. yes, we can add that to each
apartment, place it somehwere"): it comes with the room, the reader moves it like any furniture; recipes marked
`at: "workshop"` run at a workbench (the broken song book's repair, SCORES.md §2a – at a friend's). **Furniture can't
leave a room for now** (owner 2026-10-14: "furniture items can not leave a room for now"): once placed it stays in the
room – moved freely inside it, never back into the inventory, never given away (only a mover takes it along, D1).
**New furniture waits in the inventory until it is placed** (owner 2026-10-14: "yes, furniture waits"). Placing:
select a furniture item in the inventory, tap a point in the room → it stands there (`inventory.home_id` + `home_x` /
`home_y` – data model below).

**D5 – items between inventory and cave** (owner 2026-10-14: "just answered" – by D4: furniture stays in the room;
the rest as proposed). Only while you are in your cave (there or by VR headset). Into a rack: the container move that
exists (drag, or select-then-tap – `POST /api/inventory/move`); out: back into the inventory. Gifts and campaigns land
in the inventory, as now. Giving away works from the inventory only – take it out of the rack first.

**D6 – a friend's cave** (owner 2026-10-14: yes). No friend list now: a member shares a **visit link** ("Freund:in einladen" / "Invite a
friend", a token, valid 7 days, revocable) – the link opens the friend's cave directly (it isn't on your map, so no
travel). A visitor sees the room and the racks' contents read-only and may use the **workshop** with their own items
(a repair takes the visitor's broken book and gives it back repaired). No visitor can take or move the host's things.
Server: `home_visits` (data model below); the host's home answer (furniture + contents, no account data but the
display name).

**D7 – guests and what it does NOT do now** (owner 2026-10-14: "no more rooms, no chat and presence, selling").
Guests have a cave on the device (their furniture in the local inventory), can walk and visit a friend's link; inviting
needs an account (the server must answer for your home). Not now: more rooms (one home = one room, one home
scene), decorating beyond the home scene, a friend list, live presence / chat, a per-reader map spot, rides (step 2), selling.

## Data model (proposal – the names to build with)
Catalog / content (edited in the admin / creator):
- **Home scene** – a catalog table `catalog_home_scenes` (CATALOG.md, like albums): `{ id, name: Localized, image }` –
  `image` = the 2.5D background (CDN path, MAP.md §5.1 style), language-neutral; first rows `cave`, `tatami_room`.
- **Character** (characters.ts, next to `origin` / `residence`): `home?: <map place id>` (none = the place of its first
  introduction, D1) and `home_scene: <home scene id>` (set per avatar, owner 2026-10-14; until a character has one: `cave`).
- **Furniture item** (ItemDef): `category: "furniture"`; racks carry `holds` (ITEMS.md "Containers"); the workbench
  `workshop: true`; recipes `at?: "workshop"` (run only at a workbench).
- **VR headset** (ItemDef): skill `{ type: "enter_home" }`.

Reader state (server tables for members; the same shape in localStorage for guests, imported at sign-in):
- **`homes`** `{ id, user_id, place, scene, created_at }` – one per home the reader has (created when a character's
  home is first needed: `place` / `scene` from the character).
- **`home_residents`** `{ home_id, character }` (unique per user + character) – who lives where; at most 5 per home
  (the WG); a character without a row lives in its own home (created on demand).
- **Furniture in a home**: `inventory.home_id` + `home_x`, `home_y` (% of the room); set when placed, never null again (furniture can't
  leave a home – D4) – only a mover changes `home_id` (paid, D1 – later); its contents keep `inventory.container_id`.
- **Avatar position**: `MapState.avatar: { location } | { route, waypoint }` (MAP.md §2.4, world-wide game state).
- **`home_visits`** `{ token, home_id, expires_at, revoked_at }` – the friend's visit link (D6).

## Open for the owner
- Later: the mover's form (a service in the home, or an item).
- "cave can go step two": read as "design the open points now". If it means the home itself moves to step two (walking
  first, then the home with the rides), the order changes – the owner says which.
