Status 2026-10-20: RESTARTED for the Top Toy workshop (owner: the full Must list, the stage experience on Wed 2027-02-17 instead of the first show Feb 1–14); building from Mon 2026-11-16 in lane 2 of research/KANON.md §8, issues #89–#91 (milestone "Top Toy workshop"). The B.10 dates below are void – the new ones: KANON §8. (Parked 2026-10-12 → 2026-10-20.)
2026-10-30: a deck renders its outlet's cells natively; storyboard, slice editor and Live = three lenses on one flow (#186, DECISIONS) – research/LIVE-LENSES.md; the wire `Cell` / `Layer` / `media.preload` / `media.ready`: deck/protocol/schema.json.
2026-10-30: the full picture after the Rust-first / cyberdeck decisions, the Live stage on a deck, the modular core: research/LIVE-NIGHT-2026-10-29.md (decide #183–#187).
2026-10-14: a score view (MusicXML rendered by Verovio, pages, zoom, pedal keys) is built for score items – research/SCORES.md §4; musicians' sheets can use it, and score annotations and turning parts of a score on / off (performance groups) are planned here (SCORES §6a, §7) – the "no score rendering" below is superseded.

# LIVE — performance mode (plan, final; not started)

Owner 2026-10-07: plan only. **No development until the rest of the backlog is done**; building starts **Mon 2026-11-02**,
target: **show-ready February 1st, 2027**, the first show in the first two weeks of February (B.10). Builds on the reader's feedback cues (agents/research/FX.md, BACKLOG 10.56 –
its own plan, not redesigned here) and the reader's **event bus** (B.1). Decisions of the 2026-10-07 plan session: B.11.

Drafts (basic, iterated in the plan session): artifact "Scutoons Live drafts"
https://claude.ai/artifact/GNwrpUc1VxpYT7bohnTEFD – stage builder (row + matrix), venue patch, conductor, rehearsal,
audience / performer phones, musician tablet, the bridge app, the feedback-cue storyboard. The real UI is built from
`@scutoons/ui` (B.5.5), not from the drafts' markup.

A chapter played live in a room: vertical projections and monitors, performers with cues on their phones, the audience
playing along on theirs, a DAW / synths / lights following the story over MIDI, DMX and OSC.

---

## B.1 The foundation: the reader event bus
Every trigger already runs through the viewer's `EventProcessor` (`viewer/src/lib/visual-novel/event-processor.ts`).
Add one internal **bus**: every slice enter / leave, fired VN event (by its event id – the creator assigns them since
10.44), FX cue, pickup, choice and variable change is published as `{ seq, t, kind, id, payload }`. Normal reading:
nobody listens. Performance: the conductor forwards it to the relay (B.6). The FX cues (agents/research/FX.md) are bus messages.
The bus is also the conductor's **input**: a vote result is applied as the choice event the reader would have fired, so
the story logic (variables, jumps, the save game) stays the toon's own – the show never re-implements it.

## B.2 Roles (one viewer build, the role from the URL / join code)

| Role | Device | Does |
|---|---|---|
| **Conductor** | any laptop with the **Scutoons Live** app (B.7) | the authoritative position (scroll, clicker, MIDI pedal / pad), runs the reader + bus; the cue list with go / next / hold, vote tally + override, panic (stop all, black out); picks the venue patch |
| **Output** | projectors, monitors (a browser window each, fullscreen, `/live/<code>/out/<outlet>`) | renders its screen outlet's cell (B.5.2): the slice scroll (eased follow), one slice large, surtitles, characters, live media, black |
| **Bridge** | the Scutoons Live app on the conductor laptop | the local relay + every device: MIDI in / out, DMX (Art-Net / sACN / USB), OSC in / out (B.7) |
| **Performer** | actors' and singers' phones | their lines and cues ahead of time, a countdown to the entrance, a "ready" tap back |
| **Musician** | a tablet on the stand (portrait) | a **free-form sheet** (owner 2026-10-07: keep it simple) – PDF pages, images or text per row / cue, whatever the musician brings; the page jumps with the cue, swipe / pedal turns pages; what comes next; "ready" |
| **Audience** | visitors' phones (QR) | join a **group** (B.5.3); instructions, sounds, haptics, colours (phones as light pixels), votes, pickups, reactions (a spatial phone choir) |

## B.3 MIDI mapping
- **Default map, no authoring**: slice enter = note on ch 16 (note = slice index mod 128), each event type its own
  channel (showTextBox ch 1, sample ch 2, fx ch 3, choice ch 4 – choice index as note, pickup ch 5 …), variables as CC.
  A DAW (Ableton MIDI map) learns it straight away. A MIDI-out device has "default map" on / off.
- **Per-cell / per-cue messages** in the show (B.5): note / CC / program change / channel / velocity, note off after n ms.
- **Clock**: MIDI clock / start / stop out of the bridge, or the DAW as master (clock in → timed cues).
- **In**: notes / CCs mapped to conductor actions (next slice, go, fire event x, choose option n) – the mapping lives in
  the show's MIDI-in device, learnable ("press the pedal now").

## B.4 Relay and hosting
vibesss (`mandymozart/vibesss`) is used **as a concept only**: a socket relay between a host that speaks MIDI and grouped
clients that receive instructions. Its stack (Socket.io in Next.js, Keystone, Prisma, broadcast to everyone with
client-side filtering, a raw cache of 100 messages) is not taken over. New, in this repo:
- **Superseded 2026-10-29 (owner: Rust first, DECISIONS, #171):** the relay and the devices are Rust crates in the
  `deck/` Cargo workspace – `deck/relay`, `deck/bridge`, `deck/adapters/*` (research/CYBERDECKS.md §4); the protocol
  types are generated from one schema for TypeScript and Rust. The original plan was: a `live/` workspace (Node 22 +
  `ws`, TypeScript): `live/relay` (sessions, roles, groups routed on the server,
  snapshot + replay, vote tally, clock), `live/devices` (MIDI / DMX / OSC drivers), `live/app` (the Electron shell, B.7),
  `shared/live` (`@scutoons/live`: the protocol types + the client used by viewer and creator, the show schema helpers).
- **Hosted: its own bunny Magic Containers app `scutoons-live`** at **`wss://live.scutoons.com/live/<code>`** (owner
  2026-10-07; the Rust `deck/relay` since 2026-10-29; #199). **Exactly one instance in one region** (state in memory);
  it could be stopped between shows (not any more once the build service runs beside it, below). Built (#199):
  - **The image:** `deck/relay/Dockerfile` – build context `deck/` (`deck/.dockerignore`), a static musl binary on
    `scratch` (~3 MB), user nobody, `RELAY_ADDR=0.0.0.0:7700`, `RELAY_LEAD_MS=150`, plain log lines, SIGTERM stops it.
    TLS ends at bunny; the container speaks ws on 7700.
  - **The workflow** `.github/workflows/live-image.yml` (its own, like Ask's `ask-image.yml` in git history): build →
    `ghcr.io/mandymozart/scutoons-live:latest` + `:<sha>` → `BunnyWay/actions/container-update-image`. Runs on a push
    to main touching `deck/relay/**` / `deck/protocol/**`, and by hand ("Run workflow", input `force`). Settings in
    the environment `production`: variables `BUNNY_LIVE_APP_ID`, `BUNNY_LIVE_CONTAINER`, secret `BUNNY_API_KEY`;
    until all three are set a push skips with a notice (green) and "Run workflow" only builds and pushes the image.
    **Deploy freeze:** it asks `https://live.scutoons.com/status` (`{ sessions, live }`) first and refuses while
    `live > 0` (a session with a conductor) unless `force`; no answer = not live.
  - **The defaults** (nobody types an address): /live/deck joins `<VITE_LIVE_URL>/live/<code>` (default
    `wss://live.scutoons.com`; the code from `/live/deck/<code>`, `?code=`, else `CATH`; `?relay=` overrides with a
    local relay). deck-native joins `wss://live.scutoons.com/live/CATH` when deck.toml's `relay` / `--relay` is empty and
    a scene joins (space `join` = deck / conductor) or an outlet is set; its clients speak wss (rustls + webpki roots).
    Unreachable = the status line's `NO RELAY … AGAIN IN n S`, retried with backoff (1 2 4 8 10 s).
  - **The owner's setup in bunny** (once): 1. "Run workflow" once, so the image is on ghcr.io; make the package
    `scutoons-live` public (GitHub → Packages → settings) or give bunny a ghcr pull token. 2. Magic Containers → new app
    `scutoons-live`, image `ghcr.io/mandymozart/scutoons-live:latest`, one region (Frankfurt), autoscaling min = max = 1.
    3. Endpoint: CDN / HTTP on container port 7700 (WebSockets on; the relay pings every 5 s, so idle timeouts don't cut
    it). 4. Env `LIVE_TOKEN_SECRET` (the API's, once #180 / #186 mint tokens – until then unset = dev mode, any
    conductor token passes) and `RUST_LOG=info`. 5. Custom hostname `live.scutoons.com` with bunny's free TLS; DNS at
    all-inkl: a CNAME `live` → the endpoint's hostname (today the wildcard sends it to the all-inkl host). 6. In GitHub
    (environment `production`): `BUNNY_LIVE_APP_ID`, `BUNNY_LIVE_CONTAINER`, `BUNNY_API_KEY`. Check: `curl
    https://live.scutoons.com/status` → `{"sessions":0,"live":0}`.
- **The second container: the module build service (#281, RIG-EDITING.md §4)** – `deck/build` (binary `scutoons-build`, port
  7710), the compiler for creators' rig modules (Rust → wasm), in the same bunny app (one pod: shared localhost, CPU, lifecycle).
  The rules that come with the pod: **one compile at a time**, `CARGO_BUILD_JOBS=2`, `nice`, a 60 s / 4 GB / output limit with
  the whole process tree killed, and **no compile while `127.0.0.1:7700/status` reports `live > 0`** (`503 live_show`; a show
  starting mid-compile kills it). The pod stays up between shows now (members compile). The image
  `deck/build/Dockerfile` (`rust:1.99.1-slim` + the wasm32 target + the SDK + the vendored closure of its dependencies, built
  once at image build: every compile is `--offline`; user nobody) is built by `.github/workflows/build-image.yml` (own
  workflow, same deploy freeze through the relay's `/status`, one concurrency group with live-image.yml: an update of either
  restarts the pod). Reached **only through the API** (`POST /api/modules/compile`, server docs/API.md "Modules"), behind
  the shared secret `X-Build-Token` = `MODULE_BUILD_TOKEN`; the container holds no other secret.
  - **The owner's setup** (once; the code runs without it - the API answers `503 no_build_service`): 1. GitHub: Actions → 🛠
    Module build image → Run workflow once (`ghcr.io/mandymozart/scutoons-build`), make the package public like the relay's.
    2. bunny: the app `scutoons-live` → add a **second container** `scutoons-build`, image
    `ghcr.io/mandymozart/scutoons-build:latest`, env `MODULE_BUILD_TOKEN` = a long random string (the same one in 4.), no
    other env needed (it asks the relay at `127.0.0.1:7700`). 3. An endpoint for container port 7710 (HTTP, a hostname of
    its own, e.g. `build.scutoons.com` or bunny's generated one - the API lives on all-inkl, outside the pod, so it is
    public; the token is what guards it). Check: `curl https://<host>/health` → `{"ok":true,"waiting":0,"busy":false}`.
    4. GitHub (environment `production`): secrets `MODULE_BUILD_URL` = `https://<host>` and `MODULE_BUILD_TOKEN` (they
    become constants in the API's `config.php` with the next API deploy), variable `BUNNY_BUILD_CONTAINER` = the container's
    name from 2. (`BUNNY_LIVE_APP_ID` and `BUNNY_API_KEY` are there already.)
- **Local: the same relay on a deck in the venue** (or the conductor laptop; owner 2026-10-31: definitely, next to
  bunny – DECISIONS). Decks, outputs and the bridge join `ws://<ip>:7700`. **Phones on https pages need wss** (a
  browser may not open `ws://192.168.…` from https://scutoons.com), so since #201 (Phase A):
  - **The relay speaks wss** with a certificate (`--tls-cert` / `--tls-key`, `RELAY_TLS_CERT` / `_KEY`; rustls) on the
    same port as ws (the first byte decides); a renewed pair is used from the next handshake. deck-relay.service uses
    `/var/lib/deck/tls/{fullchain,privkey}.pem` when present (deck/os/README.md "wss in the venue").
  - **The name – no registration** (owner 2026-10-31: no KAS access in the app): the zone `lan.scutoons.com` lives at
    bunny DNS (one NS delegation in KAS); its wildcard is a script (`deploy/bunny/lan-dns.js`) that answers
    `192-168-1-116.lan.scutoons.com` with `192.168.1.116`, private addresses only (RFC 1918). A deck's name is its
    LAN IP with dashes (`deck-tls --status`).
  - **The certificate:** the wildcard `*.lan.scutoons.com` (Let's Encrypt, DNS-01 at bunny with acme.sh `dns_bunny`,
    the bunny API key only in GitHub) from `.github/workflows/lan-cert.yml`, monthly; the API keeps it
    (`PUT /api/deploy/lan-cert`); an approved deck fetches it with its own token (#180: `deck-tls`,
    `GET /api/decks/lan-cert` – no admin token on a deck), at boot and every 6 h.
  - **The link:** `https://scutoons.com/live/deck?relay=wss://192-168-1-116.lan.scutoons.com:7700/live/<code>`.
  - **Routers:** DNS rebind protection drops answers with private addresses – a FritzBox needs `lan.scutoons.com` under
    Heimnetz → Netzwerk → Netzwerkeinstellungen → DNS-Rebind-Schutz (Hostnamen-Ausnahmen); a venue router the same.
    Chrome may ask the phone once to allow access to the local network.
  - **The owner's one-time setup:** 1. bunny dashboard → DNS → Add zone `lan.scutoons.com`; in the zone a record of
    type **Script** (SCR), hostname `*`, with a new DNS script whose code is `deploy/bunny/lan-dns.js` as it is (Save
    + Publish). 2. KAS → DNS of scutoons.com: an **NS** record `lan` → each of the nameservers bunny shows for the zone
    (two, e.g. `kiki.bunny.net` / `coco.bunny.net` – take bunny's); check with `nslookup 192-168-1-116.lan.scutoons.com`
    → 192.168.1.116. 3. GitHub → environment `production`: the secret `BUNNY_API_KEY` (bunny → Account settings → API
    key; the same one the live container's deploy uses, #199) and `MIGRATE_KEY` (there). 4. Push, so the API knows
    `GET /api/decks/lan-cert` (and has forgotten KAS). 5. Actions → 🔐 LAN certificate → Run workflow (`staging` first
    if you like: shown, not stored); the summary shows `*.lan.scutoons.com` and the expiry.
  - **The X1 + S22 at home:** 1. An ISO with this on the X1's stick; it boots, the logo says `pair this deck: code
    K7Q-42M`, the stage shows the code + a QR in its corner. 2. Scan the QR with the phone (signed in) → the account
    page's Decks → approve. The card goes away; `deck-link status` on tty2 says `paired`. 3. Within 90 s (or `deck-tls`
    on tty2) the certificate arrives, `journalctl -u deck-relay -n 3` says `ws + wss`, `deck-tls --status` names
    `wss://192-168-1-116.lan.scutoons.com:7700`. 4. The FritzBox exception above. 5. `deck-set --restart relay
    ws://127.0.0.1:7700/live/CATH` lets the X1's own stage join it. 6. On the S22 (home Wi-Fi):
    `https://192-168-1-116.lan.scutoons.com:7700/status` shows the JSON, then
    `https://scutoons.com/live/deck?relay=wss://192-168-1-116.lan.scutoons.com:7700/live/CATH`.
  - **Next:** mDNS discovery, the uplink to bunny (below); the conductor's view of waiting / paired decks (#180).
- **Linked mode** (the normal show): the local relay is the session's primary – conductor, outputs, devices, performers
  on the LAN – and keeps an **uplink** to the hosted relay for the audience (who use mobile data / venue Wi-Fi). Audience
  messages are mirrored up / down by the uplink. Internet gone: the stage keeps running, the audience freezes on its
  last state and resyncs on reconnect. A show without a laptop on site (a reading at a café) runs hosted only.
- To verify once bunny runs it (#199): the WebSocket through bunny's endpoint for several minutes (the relay pings
  every 5 s, drops a client silent for 15 s), the one-instance setting, a rolling update with a session open (refused).

## B.5 The Show document (data model)

### B.5.1 Entity
- Its own entity (owner 2026-10-07): **`show:<toon>:<n>`** in `entity_versions` – versioned, saved, published like
  chapters; one language-neutral document, texts as locale maps (NeutralDoc). EntityKey `show => [true, true, false,
  false]`. Not readers' content: no library endpoint reads it (like `script`, CREATOR_ONLY_KINDS) – the conductor loads
  the published version (or a draft in rehearsal) and the relay hands each client only what its role / group needs
  (no spoilers, no full download on a phone).
- **One flow to rule them all** (owner 2026-10-07): the show never diverts the story. Triggers, choices, jumps follow
  exactly the toon's flow – across chapters, seasons, even into another toon (jumpTo / chapter targets with `toon`,
  one save game). A show is not tied to a chapter or a season: it has a **start** (toon + chapter + group label) and
  optional **ends**, and its rows are the scenes that flow can reach from the start (the creator's FlowIndex graph).
  `<toon>` in the key is the show's home toon (where it starts); the flow may lead elsewhere.
- The show only **adds**: its stage layer over the toon's scenes (by scene / slice / event id – the toon is never
  touched), **in-between rows** (live-only: intro, intermission, encore – played when the flow passes a given point),
  and **performance overrides** in the conductor (hold, vote override, panic, a manual jump = the same as a toon jump).
- Several shows per toon ("setups": a tour cut, a school version; one to start with). References that no longer resolve
  (a deleted slice, event or label) are **Problems errors** in the creator; the conductor refuses a show with errors.
- Who may conduct / edit: a new **`live` role** (owner 2026-10-07) in `roles` / `user_roles`, granted in the admin's
  Users page like `creator` – **for all toons, no per-toon limits** (roles are global today; a toon scope on
  `user_roles` can come later if a guest stage manager needs it). A stage manager can build and run shows without
  editing the toon; `creator` / `admin` don't imply `live`.

### B.5.2 Shape (TypeScript, `@scutoons/live` `show.ts`; JSON Schema next to the content schemas)

```ts
type Show = {
  v: 1; title: Loc;
  start: Place; ends?: Place[];                       // Place = { toon, chapter, label? } – the scope of the performance
  outlets: Outlet[];                                  // the builder's columns, in column order
  rows: Row[];                                        // in-between rows + scene rows that carry settings (scene rows
                                                      // themselves come from the flow, not stored)
  cells: Record<RowId, Record<OutletId, Cell>>;       // missing = keep (see "Inheritance")
  cues: Cue[];                                        // timed changes inside a row (the cue sheet)
  votes: Vote[]; fixtures: Fixture[]; looks: Look[];
  media: LiveMedia[];                                 // live-only assets (B.5.6)
  venues: VenuePatch[];                               // B.5.4
  problems?: Accepted[];                              // accepted warnings (DATA.md §5)
};

type Outlet =
  | { id; kind: 'screen'; name: Loc; aspect: [w: number, h: number]; room?: RoomPos }       // projector / monitor
  | { id; kind: 'group'; name: Loc; role: 'audience' | 'performer' | 'musician';
      colour: string; cast?: CharacterId[]; part?: string; room?: RoomPos }                // phones / tablets
  | { id; kind: 'midi-out' | 'midi-in' | 'dmx' | 'osc-out' | 'osc-in'; name: Loc;
      defaultMap?: boolean; channel?: number; inMap?: InBinding[] };                       // devices
type RoomPos = { x: number; y: number; rot: number; w?: number };   // floor plan, metres

type Row =
  | { id; scene: SceneKey }                                    // a toon scene / group (any chapter, season, toon the flow reaches)
  | { id; live: { name: Loc; at: 'start' | 'end' | { after: SceneKey; before?: SceneKey } } };
      // in-between row: plays when the flow leaves `after` (into `before`, if given), then the flow goes on
      // where the toon says – it never changes where the story goes

type Cell =
  | { mode: 'own'; layers?: Layer[]; send?: DeviceMsg[] }      // screens / groups: layers; devices: messages on enter
  | { mode: 'mirror'; of: OutletId }                           // "same as outlet X" (same kind only)
  | { mode: 'black' };                                         // screens: black; groups: idle screen; devices: all off

type Layer = {
  id; src: ToonSrc | LiveSrc;
  box: { x: number; y: number; w: number; h: number };       // % of the outlet, snapped like the slice canvas
  fit?: 'cover' | 'contain'; opacity?: number; loop?: boolean;
  enter?: Transition; exit?: Transition;
  on?: CueId; off?: CueId;                                     // missing = from row enter / until row leave
};
type ToonSrc = { toon:                                         // picked, never copied – follows the toon's edits
  | { kind: 'scroll' } | { kind: 'slice'; slice: SliceId } | { kind: 'panel'; slice: SliceId; crop: Box }
  | { kind: 'surtitles'; locale: string; of?: 'all' | CharacterId }   // the text boxes / dialogue as they fire
  | { kind: 'character'; character: CharacterId; pose?: string } | { kind: 'item'; item: ItemId }
  | { kind: 'map' } | { kind: 'audio' } };                     // the scene's own audio, on this outlet
type LiveSrc = { live:
  | { kind: 'video' | 'image' | 'model3d' | 'pdf'; media: MediaId; page?: number; camera?: CameraDrive }  // pdf: a musician's sheet
  | { kind: 'text'; text: Loc } | { kind: 'colour'; colour: string }   // colour on a group = phones as light pixels
  | { kind: 'camera'; input: string } };                        // a camera feed (getUserMedia on the output machine)

type Cue = { id; row: RowId; label: Loc;
  at: { slice?: SliceId; event?: EventId } | { manual: true };  // fires with the reader, or on "go"
  wait?: number;                                                // ms after the trigger
  do: Action[] };
type Action =
  | { stage: { outlet: OutletId; layer: LayerId; show: boolean } }   // switch a layer on / off
  | { say: { outlet: OutletId; text: Loc; fx?: FxCueId; colour?: string; haptic?: boolean } } // instruction to a group
  | { line: { outlet: OutletId; character: CharacterId; text: Loc; countdown?: number } }    // performer line
  | { vote: VoteId } | { send: DeviceMsg & { outlet: OutletId } } | { fx: { outlet: OutletId; cue: FxCueId } };
type DeviceMsg =
  | { midi: { type: 'note' | 'cc' | 'pc' | 'start' | 'stop'; ch: number; n?: number; v?: number; off?: number } }
  | { look: LookId; fade: number }                              // DMX
  | { osc: { address: string; args: (number | string)[] } };

type Vote = { id; event: EventId;                               // the chapter's choice event – options come from it
  rule: 'majority' | 'groups' | 'first'; n?: number;           // groups: each group's majority = one vote; first to n
  seconds: number; fallback: OptionIndex; groups?: OutletId[] }; // which groups vote (default: all audience)

type Fixture = { id; name; dmx: OutletId; address: number; profile: 'dimmer' | 'rgb' | 'rgbw' | 'rgbw-strobe' | 'moving' | Custom };
type Look = { id; name; values: Record<FixtureId, Record<ChannelName, number>> };

type VenuePatch = { id; name: string;                           // "Home studio", "Theater X"
  screens: Record<OutletId, { label: string }>;                 // which machine / projector (for the checklist)
  midi: Record<OutletId, { port: string }>;                     // port name as the bridge lists it
  dmx: Record<OutletId, { via: 'artnet' | 'sacn'; host?: string; universe: number } | { via: 'usb'; port: string }>;
  osc: Record<OutletId, { host: string; port: number }> };
```

**Inheritance** (owner 2026-10-07: play follows the toon's path): a missing cell = **keep** – at runtime nothing changes on
that outlet when the row starts, so it holds whatever the path actually played left there. The builder shows a keep
cell's look along the default path (story order, first option of each choice) and marks it "↑". `mirror` resolves at
runtime too. Device cells send on row enter; `keep` sends nothing.

### B.5.3 Groups and joining
- Audience join: **guest first, account optional** (owner 2026-10-07): QR → nickname → in. A signed-in reader joins
  with the account; items picked up in the show go to the inventory (the catalog's limits apply: `max_world`,
  `placements`); a guest gets "keep what you picked up" at the end → sign in → the existing guest-import path.
- Group membership (owner 2026-10-07): **each group can have its own QR** (one per seat block / side → spatial effects);
  the generic QR balances to the smallest audience group. The conductor can move people or reshuffle. The phone shows
  its group colour.
- Performer groups: one QR per performer (bound to a cast slot: the character(s) they play), musician groups per part.

### B.5.4 Venue patch
The show names outlets and devices **logically**; a **venue patch** (owner 2026-10-07) maps them to the real things:
which projector / machine, MIDI port name, Art-Net / sACN host + universe or the USB-DMX port, OSC host:port. Patches
are stored in the show (a list), picked in the conductor at start. The bridge lists the ports it sees; unpatched
outlets / devices are red in the conductor's checklist, simulated in rehearsal.

### B.5.5 The stage builder (UI)
- The creator shell: **sidebar = rows** (the scenes the flow reaches from the start, in story order, branches indented
  under their choice / jump, other chapters / seasons / toons as headed sections; in-between rows inserted between
  two scenes), **main = the row** (every outlet of the selected row at its real
  proportions: screens as canvases, groups as phones, devices as message lists; a floor-plan mini view), **right dock =
  the inspector** of the selected layer / message / cue. A **matrix** mode (rows × outlets as thumbnails, own / keep ↑ /
  mirror = / live-only marked) for the overview. Tabs: Stage · Cues · Media · Venues.
- Pickers stay drawers (`Sheet`): "From the toon …" (slices, panels, characters, items, surtitles, map, audio) and
  "Live media …". Toon layers carry a "toon" tag, live-only ones "live".
- ▶ on a row plays its cues through the row (simulated outlets); "Rehearse" opens B.8.
- **Not a clip launcher** (owner asked 2026-10-07 "are we rebuilding Ableton?"): the matrix looks like Ableton's
  session view (rows × tracks), but rows are never launched – the toon's flow decides which row is current, the matrix
  only says what each outlet does when the flow gets there. No launch quantization, follow actions, clip slots or
  per-clip transport. Where music needs that, Ableton stays in the room and a row enter fires its scene (MIDI).
- Kit: AppShell, Tabs / Segmented, Divider, DockFold, Sheet, Field, Badge, ActionBar, Sortable, the selection model,
  copy / paste (10.39) for cells and layers – no new chrome; rem sizes; round icon buttons / square groups.

### B.5.6 Live-only media
Videos (the media slots with resolutions), **3D scenes** (glTF / GLB in a three.js layer: a camera path, or a camera
driven by cues / MIDI CC, a model turntable, a room as backdrop), images / backdrops, PDFs (musicians' sheets – free form; since 2026-10-14 the score view where a score item exists, SCORES.md; pdf.js), light looks (B.5.2). Stored as show assets (the asset manager, prefix `live/<toon>/` on the
CDN). Every output / phone **preloads** its outlet's media before doors (a "ready" light per outlet in the conductor – a deck
outlet: research/LIVE-LENSES.md §3;
a phone preloads only its group's media, capped – big media belongs on screens).

## B.6 The relay protocol (`@scutoons/live` `protocol.ts`, v1)

**Envelope**: `{ v: 1, seq, at?, from, to?, kind, p }` – `seq` per session, assigned by the relay (monotonic); `at` =
relay time (ms) when the effect should happen; `to` = `{ role? , group?, client? }` (missing = everyone the sender may
reach); JSON over one WebSocket per client; the session is the connection's, not repeated in each message.

**Connect**
- `hello` (client → relay) `{ code, role, group?, nick?, token?, resume?: { client, seq } }` – audience / performer /
  musician / output join with the session code (+ group from their QR); conductor and bridge with a **session token**.
- `welcome` `{ client, group, snapshot, now }` – or, on `resume` within the replay buffer (last 500 messages / 5 min),
  the missed messages instead of the snapshot.
- Session token: the conductor creates a session via the API (`POST /api/live/sessions` → `{ code, token }`,
  the `live` role); the token is an HMAC-signed `{ session, show, role: conductor | bridge | uplink, exp }` with a secret shared
  by API and relay. Only conductor-role messages can change state; audience messages are rate-limited (5 / s) and
  nicknames are trimmed to 24 characters, the conductor can rename / kick (no word filter exists yet – a small
  blocklist in the relay); max 1000 clients per session.

**Clock**: `ping { t0 }` → `pong { t0, t1 }` every 20 s (also the keepalive); 5 samples at join, offset = median of
`t1 − (t0 + rtt / 2)`. Cues carry `at = now + lead` (lead 150 ms hosted, 50 ms LAN); a client plays at `at − offset`;
up to 300 ms late it plays at once, later it applies the state but skips the flash / sound.

**Conductor → relay** (state is authoritative, the relay only routes, tallies and remembers):
`state { row, slice, vars, inventory? }` (on every change; folded into the snapshot) · `cue { id, actions }` (the relay
splits actions per outlet → `to`) · `vote.open { vote, options, rule, n?, seconds, fallback?, groups?, show?, question?, labels? }` (`question` / `labels` = locale maps: a phone shows its device's language; the conductor sees the vote open too) · `vote.hold { vote, hold }` (#231: the countdown stops, votes still count; a held vote does not close when all have voted) · `vote.close { vote,
result?, override? }` · `assign { client, group }` · `panic { black?, stop? }` · `media.preload { outlet, items: [{ id,
url | set + spot + mode }] }` (#186; an output answers `media.ready { outlet, ready, missing, why? }` → the conductor).

**Relay → clients**: `snapshot` (on join / reconnect: row, the current cell per outlet for this client, open vote, last
instruction per group) · `cell { outlet, cell }` (row change) · `cue`, `say`, `line`, `fx`, `colour` · `vote.open`,
`vote.tally { counts, voters, left, paused?, groups?, expected? }` (to the conductor every 250 ms with the counts per group and the connected voting devices; to a voter only if the vote shows it AND only after that voter has voted) · `vote.result { …, groups? }` (a vote closes at its time, when every connected voter has voted, when a `first` option reaches n, or by the conductor) ·
`presence { groups: { id: n }, ready: [...] }` (to the conductor).

**Decks → relay** (#188): `deck.graph { nodes, links, character?, size?, moods?, expose? }` (a deck's being; kept per
client for the conductor's snapshot `decks`) · `deck.values { values: { "node.port": number[] }, mood? }` (only what
changed, ≤ 30 / s – the relay drops more; folded, never replayed; to the conductor, and to `to` too) · `deck.midi { msgs:
[{ bytes }] }` (#211: a deck's parameters as MIDI outputs for the conductor to route on; ordered, never merged or
replayed – deck/outputs) · `deck.listen { addresses }` (#442: the absolute addresses a deck's far world nodes name, to the
group `decks` on each welcome and every 10 s; the deck the address names answers) · `deck.events { events: [{ address, name,
props, dur_s? }] }` (#442: what left the asked-for event outputs, to that one subscriber; ordered, never merged or replayed;
the relay parses no address) · presence adds
`decks`, `routes` (who sent values to whom) and `links` (#223: each client's round trip, value rate, values replaced –
not the audience's) and `quality` (#220: each deck's last `deck.quality { level, scale, auto, frame, ceiling? }` – its
render quality, sent at once on a change, else ≤ every 2 s; kept, not forwarded). The deck side: deck/native/README.md "space", "Quality". **Send path** (#223): per client the latest
state-like message wins (`deck.values` per deck, merged; `presence`, `vote.tally`, `state`), ordered ones stay complete
and in order, a client too far behind is dropped (it resyncs from its snapshot), a link that stays behind gets values at
15 / 10 / 5 per s – deck/relay/README.md.

**Scene sync** (#195): `scene.sync { scene, sets?, characters?, beings? (#191, reserved), decks? }` (conductor only →
the group `decks`, or one copy per listed client id; logged, the last one kept: a deck that joins later finds it in its
welcome snapshot `sync`) · a deck fetches it into its cache and answers `deck.synced { scene, state: syncing | ready |
missing | offline, done?, total?, bytes?, ready, missing, why? }` (items `set:<id>` / `character:<id>`; progress, then
the final state) → the conductor; kept per deck for the conductor's snapshot `synced`, a ready deck is ready in
presence, a new push forgets the old answers. The deck side: deck/native/README.md "Sync"; the browser deck:
`@scutoons/live-deck` sync.ts.

**Clients → relay**: `vote.cast { vote, option }` (one per client, changeable until close) · `ready` · `pickup { item,
placement }` (the relay asks the API, like the reader does) · `react { kind }` (applause, a sound pad – rate-limited).

**Bridge**: `device { outlet, msgs[], at }` (relay → bridge, already resolved against the venue patch) · `device.in {
outlet, msg }` (bridge → conductor: MIDI / OSC in, mapped to actions there) · `devices { ports }` (the bridge's port
list for the patch UI). MIDI clock in is used only for timed cues (a cue's `wait` may be given in beats), never shown to musicians.

**Uplink** (linked mode): the local relay connects to the hosted one as `uplink`, forwards everything addressed to
audience groups, and returns `vote.cast` / `react` / `presence`. Sequence numbers are the local relay's.

## B.7 The device bridge: the **Scutoons Live** app
Superseded 2026-10-29 (owner: Rust first, #171): the bridge is the Rust binary `deck/bridge` (research/CYBERDECKS.md
§4 adapters, §5 split) for Linux decks, macOS and Windows – no Electron; its small status window is a native / `wry`
window, the conductor stays the deployed web page as below. The Electron paragraphs below are the original plan.
Owner 2026-10-07: **one local bridge for all devices, shipped as an installable app for macOS and Windows**, so any
computer can become the conductor.
- **Electron** (`live/app`): runs the same Node relay as the hosted one (B.4 local / linked) + the drivers: MIDI via
  RtMidi (`@julusian/midi`, prebuilt binaries), Art-Net / sACN over UDP, USB-DMX (Enttec Open DMX / DMX USB Pro) via
  `serialport`, OSC over UDP. Native modules built per platform in CI.
- Its window (small): status (relay, uplink, devices), the ports it sees, the venue patch picker, the LAN address +
  QR for outputs / performers, **"Open conductor"** (opens the conductor in the system's browser or its own window at
  `https://scutoons.com/live/<code>/conduct` – the conductor UI stays a web page, so it is always the deployed version).
- Builds: GitHub Actions matrix (macOS arm64 + x64, Windows x64) → GitHub release; auto-update via `electron-updater`.
  **Unsigned for now** (owner 2026-10-07): macOS "right-click → Open", Windows SmartScreen "Run anyway" – documented in
  the app's README. Signing (Apple Developer + a Windows cert) only when other people install it.
- MIDI timing: messages go out at `at` from the bridge's own clock (not a browser tab, never throttled).
- Phase 1 runs the same code as `npm run live` (no Electron) – packaging comes in phase 4.

## B.8 Rehearsal mode
All four (owner 2026-10-07):
- **Simulated room** in one window (`/live/<code>/rehearse` in the creator): every outlet as a tile at its proportions,
  group phones as phone tiles, devices as message logs + a light preview (fixtures as coloured dots on the floor plan).
- **Real devices partially**: what the chosen venue patch has (and the bridge sees) goes out for real, the rest stays
  a tile – e.g. Ableton real, projectors as tiles.
- **Fake audience**: n bots per group (in the relay, flagged `bot`), vote randomly or weighted, react, drop out and
  reconnect – votes, tallies and load (target: 500 bots without lag on the hosted relay).
- **Jump + record**: start at any row / cue with a chosen state (variables, inventory – from a save game or by hand);
  record a run (every bus + relay message with times, a JSON file) and replay it into the simulated room.
- Rehearsal runs the **draft** of the show (and the chapters' drafts if chosen); a show only runs published.
- 2026-10-19: a replayed run, the rows and the cues on a time axis → the shared Timeline (research/TIMELINE.md §2.4: ms, read-only
  with `follow`, no transport inside the view).

## B.9 What happens on a phone (audience, constraints)
- "Tap to join" is the audio unlock + the Screen Wake Lock request (the screen stays on).
- Haptics: the Vibration API works on Android; **iPhone Safari has none** for cue-driven haptics – iPhones get the
  sound + a flash instead (the account's haptics toggle stays per FX.md). Sound effects volume: the account's (FX.md);
  guests: a volume slider on the join screen.
- Effects stay within the viewport (FX.md rules); no reduced-motion setting.
- Background: a phone that sleeps / switches app reconnects with `resume` and gets the snapshot.

## B.10 Phases (void since 2026-10-20 – dates: KANON §8 lane 2; Electron / `live/` / serialport below: superseded by `deck/`, CYBERDECKS.md)
Owner 2026-10-07: **target February 1st** – the first show may be in the first two weeks of February. That is ~11
working weeks (holiday break 2026-12-21 → 2027-01-01 assumed), four fewer than first sketched, so the scope is cut to
what the first show needs. Must = the first show needs it; Could = after the first show, or earlier if a phase ends
early. The FX cues (FX.md) are not a dependency: phase 0 takes only the bus + the cue registry; FX's own phases stay
in BACKLOG 10.56.

**A try-out, no crunch** (owner 2026-10-07): the whole performance mode is a nice-to-try – what isn't ready for the
first show comes with the next shows. Must / Could is the order of work, not a promise; the dates are a target.

| # | Dates | Step | Proves |
|---|---|---|---|
| 0 | Nov 2 – Nov 6 | The bus (B.1) + the FX cue registry; **spikes**: bunny WebSocket + one instance, iPhone audio / wake lock, RtMidi + `serialport` in Electron on mac + win, a test frame through the USB-DMX interface to the strobe + the colour panel | the bus, the risky platform facts |
| 1 | Nov 9 – Nov 20 | `live/` workspace, local relay + MIDI drivers (`npm run live`): MIDI out default map → loopMIDI / IAC → Ableton, MIDI in → next / go; **USB-DMX** out (Enttec DMX USB Pro / Open DMX via `serialport`, ~40 frames/s, fades in the bridge): fixtures + looks in the JSON, a look on row enter / cue; a bare conductor; the Show schema (a hand-written JSON show) | the fun and the lights, without the network |
| 2 | Nov 23 – Dec 11 | Protocol v1 complete (B.6: tokens, snapshot, replay, clock), outputs (scroll, slice, live video / image), hosted `scutoons-live` + `live-image.yml` + deploy freeze, linked mode; the `show:` entity + the `live` role (a migration: one `roles` row) | several screens in sync, LAN + hosted |
| 3 | Dec 14 – Dec 18, Jan 4 – Jan 8 | Audience: QR per group, guest + account join, votes (rules, tally, override), instructions, sound / colour / haptics, pickups; performers (lines, countdown, ready); bots (join, random votes, reconnect) | the room and the cast play along |
| 4 | Jan 11 – Jan 22 | Creator: stage builder **row view**, cue sheet, venue patches, **fixtures + looks** (profiles, a look editor with sliders that drives the real fixtures through the bridge while editing), Problems for broken references; rehearsal: simulated room + real devices partially, jump to a row with a state; musician sheets (PDF / image / text); the Electron app (mac + win, unsigned) | authoring without code |
| 5 | Jan 25 – Jan 29 | Full run-through on our own setup: the patch, preload, load test (500 bots), drills (internet gone, laptop restart, phone sleep) | it holds |
| — | Feb 1 → | **Freeze**: no deploys of viewer / creator / `scutoons-live` until the show is done; tech at the venue the day(s) before; **first show in Feb 1 – Feb 14, 2027** | the show |

**Could** (cut line – after the first show unless time is left): the matrix view (the row view is enough to author),
Art-Net / sACN (the first show uses USB-DMX), OSC, 3D layers, camera feed,
MIDI clock in, record + replay, weighted / scripted bots. Starting before Nov 2 (if the backlog allows) buys these back.

## B.11 Decisions (owner 2026-10-07) and what is still open
Decided: (1) the relay is its own bunny app `scutoons-live`; (2) audience = guest first, account optional; (3) votes:
a rule per vote (majority / per group / first to n) + time + fallback, applied automatically, the conductor can hold
or override; (4) a show is its own entity `show:<toon>:<n>` (home toon); (5) logical outlets + venue patches; (6) groups by QR, else
auto-balanced, the conductor can move; (7) DMX = fixtures + looks (no effects engine; a real desk via OSC if a lighting
designer comes); (8) one local bridge for all devices, as the Electron app **Scutoons Live** for mac + win, unsigned
for now; (9) one flow: the toon's flow decides everything (triggers, choices, jumps – across chapters, seasons, toons); a show
has a start + ends, adds in-between rows and its stage layer, overrides are performance-only (hold, vote override,
panic, manual jump); rows are never launched (not a clip launcher); keep = no change at runtime; (10) building
Nov 2 → show-ready Feb 1 2027, first show Feb 1–14 (the B.10 cut line); (11) rehearsal: simulated room, fake audience, jump + record, real devices partially; (12) musicians get a
free-form sheet (PDF / images / text), no score rendering or bar following; (13) a global `live` role (all toons, no per-toon
limits) to build and conduct shows; (14) several setups (shows / venue patches) over time, one to start with; (15) show pickups:
any catalog item for now; (16) no surtitles on the audience phones for now; (17) DMX fixtures + looks are a Must for
the first show (an LED strobe and a multi-colour panel are available) over a **USB-DMX interface**; fixture models /
DMX modes follow at build time – generic profiles (dimmer, RGB, RGBW, RGBW + strobe) + custom channel lists until then.

Open:
1. The venue and the exact show date within Feb 1–14 (fixes the venue patch and the tech day) – not known yet.
