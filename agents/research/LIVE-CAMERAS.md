Status 2026-11-19: steps 1–4 built, step 5 open (#605; round 1 feature/605-live-cameras, round 2 feature/605-cameras-round2): relay claims per camera (deck/relay cameras.rs), the phone's hold and the deck camera following it (deck-space live_cameras.rs), the one handheld (@scutoons/content/storyboard `Handheld`, @scutoons/ui `HandheldStage`; the creator's /h/<code> on it), the reader's /live/cameras and /live/cameras/<id> with the phone's proof, the DOP panel's PHONES row (stored as the camera's and each shot's `phones`), the Session sheet's cameras with a QR per camera, the row word `scene:<id> > camera[:<shot>]` (the relay picks the phone by the row's filter), the native and browser decks following a held camera of their scene, §6 and §9 as delegated. Open: Record (step 5).

# LIVE-CAMERAS – a phone takes a deck's camera by QR

## 1. The idea in one line
Every camera of a live session is a resource the phone can take. Scan its QR code (or pick it from a list) on the reader's camera page. If the editor's rules allow it, the phone holds that camera and steers it with its own motion, until it lets go or someone takes it over. Recording turns a take into a camera move.

## 2. Where it lives
- **Reader routes:**
  - `/live/cameras`: the cameras this person may take in the session (name, scene, who holds it now, a small view of it).
  - `/live/cameras/<id>`: the handheld page for one camera.
  - Both carry the session as `?live=<code>`, like the linked reader.
- **Camera id:** the camera's address from #582, e.g. `scene:duet > shot:close` or a scene's primary camera.
- **One handheld component** in shared/, the browser side (device orientation and motion, recenter, the pose stream). The creator's storyboard uses it, and the reader's camera page uses it. Today's `/h/<code>` handheld in the creator becomes one client of it: the storyboard's camera is just a camera with an id. No second copy of the pose code. This is the refactor the owner asked for.
- **The core** (deck-space `linked.rs`): a `camera_link` next to `reader_link`, which builds the QR's URL. The relay gets a claim per camera id, next to `reader.drive`.

## 3. QR wiring (as the linked reader does it)
- In the editor, each camera row in the DOP panel (#582) and Perform's Session panel (#531) has a **QR** with copy and "open here".
- Scanning opens `/live/cameras/<id>?live=<code>` on the phone. There is one QR for the list as well (`/live/cameras?live=<code>`).
- Same transport as now: the live road (QUIC first, WebSocket last), one relay session.

## 4. Who may take a camera
- The editor sets it **per camera**, in the DOP panel's access row, with the **one condition control** (#553), so there is no second rule language:
  - **Who:** user types (guest, member, crew) and roles (left, right, conductor …).
  - **When:** conditions such as "only while row 3 plays", "only after the vote", "only this deck's outlet", using the row addresses and cells the control already speaks.
- **The core evaluates and the relay enforces.** A claim that fails the rule is refused with the core's sentence ("this camera is for crew only"). The list on `/live/cameras` only shows cameras the person may take, plus the ones they could take later with the reason ("from row 3").

## 5. Taken: the singleton rule from the linked reader (#527/#528)
A camera has **one holder**, under the same rule we settled for one reader driving a session:
- The phone **asks first**. If someone else holds it, the question names the take-over ("Maya holds this camera – take it over?"). Only the yes claims.
- **The newest claim wins.** The holder before hears it at once ("taken over by …") and gets **Take it back**.
- **Leave** lets go. So does leaving the page, or **no pose for a set time**: the camera returns to its owner.
- **Per camera option, for shows:** "locked while held". Nobody can take it over until the holder lets go or the conductor frees it. The conductor can always free any camera.

## 5a. Persistence, flash, force (owner 2026-11-17)
- **A held camera persists.** The claim is kept for a resume, the way the reader's drive is kept (#527: the relay keeps `driver` for a resume).
  - If the phone goes offline (Wi-Fi hiccup, the browser in the background, a dead battery) and comes back, it **takes that camera again by itself**, with no question asked, as long as nobody else claimed it meanwhile.
  - The camera holds its last view while the phone is away, until the no-pose timeout returns it to its owner. A returning phone reclaims it.
  - Identity is the phone's client id plus its handheld token, kept in the browser, so a reload is the same holder. In a performance this matters most.
- **A remote camera can be flashed.** The conductor pushes a camera's settings (lens, height, which camera it is, its access rule) to the phone holding it, the way a deck is flashed: Flash sends the tree down. The phone shows what it now is.
- **Forced assignment.** The conductor can force an active phone camera to become a certain camera of the show or scene, from the Session panel or by a show row's address (`… > camera:<id>`). The phone switches at once. Its page names the new camera, and the old one returns to its owner. **So a show's cells switch the cameras on the phones:** a cell whose piece is `camera:<id>` (with the phone's role or holder as its target) fires the switch like any other cell, on its row, its moment and its quantize.

## 6. On the deck (INPUT.md's takeover rule)
- While a phone holds a camera, that camera follows the phone. The deck's own camera (a shot, a move, the editor's view) is put aside, not changed, and returns when the phone lets go. **Recenter** starts from the camera the deck had, so nothing jumps.
- Moves (#582) and the phone can both be in play: a move triggered while a phone holds the camera plays over the phone's pose for its duration, starting from the phone's view, then hands back – the phone goes on from where the move ended (orchestrator 2026-11-19, [delegated]).
- A deck follows a held camera of the scene it plays (its primary camera, or a shot: the first pose starts from that shot's view), one at a time; another phone taking it goes on from the view held now.

## 7. Recording
- On the camera page: **Record** saves the take as a camera move on the scene (#582's move engine: a path plus timing on beats or seconds) under a name.
- A recorded take is replayed by its address (`scene:<id> > move:<id>`) like any move, so a phone performance becomes part of the show.
- The storyboard's existing recording becomes the same thing: a take is a move.

## 8. Steps
1. Core and relay: camera claims per id, the singleton rule, the access check through #553. Tests as for #527.
2. The shared handheld component; the creator's `/h/<code>` moved onto it (the refactor).
3. Reader: `/live/cameras` (the list) and `/live/cameras/<id>` (handheld, Leave, Take it back).
4. Editor: the DOP panel's access row and QR, and Perform's Session panel listing the cameras and who holds them.
5. Record a take as a move.

## 9. Open for the owner
- ~~The default when a camera is taken~~ – answered: the same rules as the reader (newest wins, with the warning and Take it back).
- ~~Do guests see cameras they can't take yet?~~ – only the ones they can (orchestrator 2026-11-19, [delegated]).
- ~~How long without a pose before a camera returns?~~ – no default: `release_ms` per camera (the scene stores it with `phones`; the panel has no control for it yet); unset, only Leave, a take-over or the conductor frees it ([delegated]).
