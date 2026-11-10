Status 2026-11-10: concept for the owner's review (#468), nothing built from it. Owner's brief: decisions/deck.md 2026-11-10 [platform]. Drafts: the shared notes folder `468-input-concept/` (sent in the chat). §8 is what the owner decides.

# Inputs: everything that comes into a deck

## 1. The question
The owner wants one concept for audio in, MIDI, DMX, OSC, gamepads, keyboard, phone XY and knobs, camera, motion sensors, lidar and whatever comes later. The worry: the browser gave us many of these for free (Web MIDI, Gamepad, DeviceMotion, getUserMedia). A native deck has to bring each one itself, and a deck must still build small for very small machines.

The answer in one line: **every input is a source with an address; where its adapter runs is a separate choice with three places (in the deck, beside the deck, far away), and the rig never sees the difference.**

## 2. What exists today
| Thing | State | Where |
|---|---|---|
| Sources by address | `sound:level|bass|mid|high`, `clock:beat|bar|phase|…`, `being:bounce` as target | `deck/space/src/rig/address.rs` `SOURCES`, RULES A14 |
| Mapping source → target with a range | in presets (`face:jawOpen` is already a test value) | `rig/preset.rs` `Mapping` |
| World nodes for events | `source = "midi" | "generator" | "far"` | `deck/space/src/world.rs`, #430, #442 |
| Far sources between decks | `deck.listen` / `deck.events` over the relay | `deck-space far.rs`, #442 |
| Adapters with one trait | MIDI (midir), OSC (rosc, UDP), DMX USB Pro (serial) | `deck/adapters/*`, `adapters/core` `Adapter` |
| Build features on the native deck | `midi`, `osc`, `outputs`, `gamepad` (gilrs, not default), `asio`, `modules`, `live`, `space` … | `deck/native/Cargo.toml` |
| MIDI | clock in and out works; notes in do not reach a real deck | #439 |
| Gamepad | `pad.rs` maps sticks straight onto controls, not as sources | #228 |
| Audio in | none as a source (cpal is there for output) | – |
| Camera, motion, position | design issue only | #312, #212 |
| Rule | devices are adapters feeding the core; a module never calls a platform API | RULES A5 |

So the model is half there. What is missing is the general form: a scope per device family, one place that turns raw device data into named sources, and a rule for where adapters live.

## 3. The model: three steps from hardware to a parameter
1. **Adapter** (platform side, no decisions, A5). It talks to the OS or the wire and hands over raw, timestamped data: "controller 74 on channel 1 = 93", "axis 2 = −0.41", "OSC /fader/3 0.7".
2. **Profile** (core side, data, not code). A small document per device type that names the raw controls and their ranges: "axis 2 is `stick_right_x`, −1 … 1". One standard profile per family (the universal gamepad, a generic MIDI controller, the phone surface); specific ones for a known device (Analog Four, a DMX desk). Learn (#278) writes a profile entry.
3. **Source** (core side). What rigs, shows and mappings see: an address `<scope>:<name>`, either a **value** (control rate, smoothed) or an **event** (a note, a trigger, with its time). Audio and video are **streams** and go to the audio graph or a tracker, never through a mapping.

A mapping stays what it is: source, target, range (curve and smoothing to add). The controller map page (#279) is the list of mappings; the monitor (#304) is the list of sources with live values.

## 4. Scopes (proposal, naming by RULES A14)
| Scope | Values | Events | Comes from |
|---|---|---|---|
| `audio:` | `level`, `bass`, `mid`, `high`, `pitch` per input | `onset` | audio in (the stream itself is a node in the audio graph) |
| `midi:` | `cc<n>`, `bend`, `pressure` | `note`, `clock`, `start`, `stop`, `program` | MIDI ports |
| `osc:` | one per learned path | one per learned path | UDP |
| `dmx:` | `ch<n>` of a universe in | – | USB widget in, Art-Net, sACN |
| `pad:` | `stick_left_x/y`, `stick_right_x/y`, `trigger_left/right` | `a b x y`, shoulders, d-pad, `start` | gamepads (standard layout) |
| `key:` | – | a key by name, down and up | keyboard |
| `touch:` | `x`, `y`, `pressure`, `knob<n>`, `fader<n>` | `tap`, `pad<n>` | a phone or tablet surface, a touchscreen |
| `motion:` | `tilt_x`, `tilt_y`, `turn`, `accel` | `shake` | phone sensors, an IMU on a small machine |
| `face:` | the blendshapes (`jawOpen`, `smile` …), head `yaw pitch roll` | `blink` | camera + tracker |
| `body:` | joints, `hands_up`, `lean` | `clap`, `jump` | camera + tracker |
| `depth:` | `nearest`, `zones<n>`, `presence` | `enter`, `leave` | lidar, depth camera |
| `place:` | `lat`, `lon`, `accuracy` (coarse by default) | – | GPS, the browser |
| `knob:` | `<n>` | `button<n>` | bare hardware on a small machine: ADC, encoders, GPIO |

With more than one device of a kind, the device's name sits in front as a place, the same way decks already do: `/deck:left/pad:stick_left_x`, `midi:a4/cc74` (A14 needs a decision on the device part, §8).

## 5. Staying native: three places an adapter can run
| Place | How it reaches the core | Latency | For | Cost to the build |
|---|---|---|---|---|
| **In the deck** (a cargo feature) | a function call | lowest | what must be tight and is small: audio in, MIDI, OSC, DMX, gamepad, keys, bare knobs | each one a feature; the binary grows only by what is switched on |
| **Beside the deck** (a helper process on the same machine) | a local socket, in our protocol's source messages | a few ms | what is heavy or vendor-bound: camera tracking, lidar and depth SDKs, exotic HID | none: the deck needs only the `osc` or local-link feature; the helper is its own program, in any language |
| **Far** (over the relay) | `deck.listen` / `deck.events`, already built (#442) | network | phones and other decks | none on the receiving deck |

This is the answer to "the browser did that for us":
- **The phone stays a browser** (PWA, decided 2026-10-16). Its browser reads touch, tilt, shake, camera and position and sends them as far sources. Nothing of that is rebuilt natively. Phone XY, knobs and the accelerometer need no native code at all.
- **What a native deck needs itself** is short and mostly there: MIDI (midir), OSC (rosc), DMX (serial; Art-Net and sACN are plain UDP), gamepads (gilrs; on Linux it needs libudev at build time, a bare evdev reader is the small alternative), keys (winit with a window, evdev without), audio in (cpal, already linked).
- **What is heavy stays out of the deck binary.** A face tracker needs a model runtime of tens of MB; a lidar or depth camera usually comes with a vendor library. Those run beside the deck and talk to it. A small machine without one simply has no `face:` sources.

### Build profiles
A profile is a named set of features, written down once (one file, read by the build scripts and shown by `deck-check`):
| Profile | In it | For |
|---|---|---|
| `tiny` | audio, clock, MIDI, OSC, `knob:`; no GPU, no space, no editor, no modules compiler | a musician's box, a sensor node, a Pi Zero class machine |
| `stage` | today's default + gamepad | an AV deck |
| `studio` | stage + editor + modules | the owner's PC |
| per hardware | a profile plus one feature (`asio`, a DMX widget, an IMU) | a specific box |

Sizes today, for scale (release, Windows): `deck.exe` 21.5 MB, `deck-stage.exe` 12.7 MB, `bridge.exe` 1.8 MB. What `tiny` weighs is not measured.

### A deck says what it has
A rig, a module (#312 `needs`) and a show row name the sources they use. A deck knows which scopes its build and its plugged devices give. The flash report (#91) is the place where the two meet: "this deck has no `pad:`" is a line in the report and on the HUD, never a crash and never a block on GO. An absent source reads as zero plus a flag (#312).

## 6. Time
- Events carry the adapter's timestamp; the core places them against the clock (MIDI notes at key down, off at key up, #439).
- Values are sampled at control rate and smoothed in the mapping, not in the adapter.
- Far sources play on arrival today (#442's known gap); relay-time alignment is its own piece.
- Nothing here runs on the audio thread except audio in.

## 6a. The handheld camera session (owner 2026-11-10)
The storyboard editor's handheld camera (#126, research/HANDHELD-CAMERA.md, built and tested on the S22) is an input of this concept, and its code is shared, not rebuilt (#213 asked the same for the deck's camera).

- **A pose is a source.** Scope `pose:`: `pose:camera` (position, rotation, lens as one), each part also as a value (`pose:yaw`, `pose:roll`, `pose:x` …), `pose:record` and `pose:recenter` as events. Mapped whole onto a camera or a being's head, or in parts onto any parameter.
- **A session** hands one target to a device for a time: a QR code with a one-time code (the phone's only right, no sign-in), Recenter, steer, Record, and one button on the deck takes it back. The phone surface (§4 `touch:`, `motion:`) pairs the same way: one small phone page with a camera mode and a surface mode.
- **Time:** a pose is latest-wins (a late one is dropped); a take is on the clock.

| Part | Storyboard today | For the deck |
|---|---|---|
| Phone page `creator/src/routes/h/[code]`, `lib/handheld/phone-session.svelte.ts` (WebXR `immersive-ar`, else DeviceOrientation + drag) | built | the same page, as is (it runs on the phone, so it is no deck logic) |
| Pairing `lib/editor/storyboard-pairing.ts`, `server/.../Storyboard/Pairings.php` (one-time code, offer / answer, `pose` unordered + `ctl` reliable) | built | the QR code and the one-time code stay; offer / answer and STUN go with the WebRTC road |
| Pose maths `lib/editor/storyboard-handheld.ts` (origin / recenter, mapPose, smoother, sampleAt, bake; 204 lines, tested) | TypeScript | the deck needs it in Rust (no deck logic in TS): one crate, built to wasm for the creator, the TS file's tests as its vectors. The phone page keeps only `orientationQuat` and `dragStep` |
| Take: `TakeSample {t, position, quaternion, fov}` (`@scutoons/content/storyboard`) | built | the same format: a storyboard take plays on a deck as a camera move (#213), a deck's camera performance is recorded as a take and opens in the storyboard editor |
| Road for poses | WebRTC data channel, phone ↔ PC browser (same Wi-Fi, no TURN) | **the live road, for the deck and the storyboard alike (owner 2026-11-10):** QUIC first through the relay's WebTransport door, WebSocket last, with the built ladder (research/RELAY-TRANSPORTS.md). See "The road" below |
| What the pose drives | the shot camera (`Stage3d.hold`) | the deck's camera in deck-space: new |

**The use case (owner 2026-11-10).** *"Hand held camera takes over a decks current camera as long as it is active in the 3d scene and its Vectors can also be assigned to spatial Vectors so movement i space controls parameters mapped to anything through our rules"*

- **Takeover while active.** A handheld that is active in a deck's 3D scene is that deck's camera: the deck's current camera (a preset, a move of #213, the editor's view) is put aside, not changed, and comes back the moment the handheld is no longer active (the session ends, the phone leaves, the deck takes it back, or no pose arrives for a set time). Recenter starts from the camera the deck had, so the picture does not jump at the handover.
- **Its vectors are sources.** Besides the whole pose, the handheld gives vectors: `pose:position` (x, y, z), `pose:rotation`, `pose:forward` (where it points), `pose:velocity`. A vector source is assigned to a spatial vector of the scene with one mapping (a being's position, a light's direction, a sound's place in the room, an emitter's wind), and each part is a value like any other (`pose:x`, `pose:yaw`), so walking through the room drives any parameter through the same mapping rules: range, curve, smoothing, and the layers of §6b.
- **Both at once.** The takeover and the mappings do not exclude each other: the phone can be the camera and drive a filter with its height in the same session; or drive only parameters, with the deck keeping its camera (the session's "camera" switch off).
- **New for this:** a vector as a kind of source and of mapping target beside value and event (today a mapping is one value onto one parameter); "active" as a state the deck knows and shows; the put-aside camera.

**The road (owner 2026-11-10: real time is key, QUIC and the fallback rules apply).** The handheld leaves its own WebRTC connection and rides the live road like every other client (#438, research/RELAY-TRANSPORTS.md):

- **Poses are datagrams:** unreliable, unordered, latest wins, as values already travel on QUIC. Each carries its time, so a late one is dropped.
- **Record, recenter, state are on the reliable stream,** as cues are.
- **The ladder as built:** QUIC gets 250 ms, then a WebSocket is dialled beside it and the winner says `hello`; QUIC dying mid-session resumes on the WebSocket as the same client (poses gone for about 3 s, no event lost); from the WebSocket it tries QUIC again after 60 s.
- **On the WebSocket the session still works but is not real time** (measured for values at 1 % loss: p99 14 ms on QUIC against 33 ms on the WebSocket, with stalls of a quarter second for what must arrive). The phone and the deck show which rung they are on; the smoothing is raised there.
- **What this settles:** a native deck needs no WebRTC, so the question "relay first or a WebRTC feature" is gone. The phone no longer has to be in the same Wi-Fi as the PC. The pairing's offer / answer and the STUN server go; the QR code and the one-time code stay and now name the session and the target on the relay.
- **What it needs:** a relay with a certificate near the phone (a LAN relay under `*.lan.scutoons.com`, or the hosted one once its UDP endpoint exists, #441; until then the hosted relay is WebSocket only). The phone's browser must have WebTransport; one without goes to the WebSocket at once.
- **Not measured:** the pose's delay phone → relay → deck against today's direct WebRTC, and WebTransport on the S22 inside an AR session. Both are one test before the WebRTC road is removed from the storyboard.

Later: any source recorded and replayed as a take (a gamepad run, a fader ride).

## 6b. Mapping layers (owner 2026-11-10)
Peripheral mappings are written ahead, one device can overwrite them for itself, and a flash can force the defaults.

1. **Precoded:** what a device type does out of the box (the standard gamepad drives the selected being, #212; a known controller's knobs have names and targets). In the profile, ships with the build. The floor.
2. **Flashed:** the mappings a show, its rigs and presets carry (`Mapping {source, target, range}` exists). Wins over 1.
3. **Local:** what was changed on one deck (a learned knob, another range, a source switched off). Stored on that deck only, wins over 2, each change marked and undoable by itself ("Reset this mapping", "Reset this device", "Send to the editor" to make it part of the show).

- **Flash:** replaces layer 2, local changes stay; the flash report lists them per deck.
- **Flash, force defaults:** replaces layer 2 and clears the local layer, on all decks or the chosen ones; asks once.
- Proposed on top: a **lock** per show (no local changes until the next flash), and the cleared local layer kept as one backup on the deck.

## 7. Design drafts (in the chat, files in `468-input-concept/`)
1. **The map:** hardware → adapter (in / beside / far) → profile → source → mapping → parameter, with every family on it.
2. **Inputs in the editor:** the devices this deck sees, their sources with live values, learn and the profile; one mapping with range, curve and smoothing; the controller map as a list.
3. **Builds:** the profiles as columns, the adapters as rows, and what a show needs against what a deck has.
4. **The phone surface:** XY, knobs, pads, tilt, as a far source of a deck.
5. **The handheld camera session:** the session, what the deck sees, what is shared with the storyboard editor, one take both ways.
6. **Mapping layers:** precoded, flashed, local; flash and forced flash.

## 8. For the owner to decide
1. **Three places** (in / beside / far) as the rule, with the phone staying a browser. Recommended: yes.
2. **The smallest machine a deck runs on.** Recommended: a small Linux board (Pi Zero 2 class) with the `tiny` profile. A microcontroller is a device that speaks MIDI, OSC or serial to a deck, not a deck.
3. **Device names in addresses** when there are two of a kind (`midi:a4/cc74` or a place in front). Recommended: a place in front, like decks.
4. **Camera tracking:** a helper beside the deck (recommended), and whether face or body comes first.
5. **Order.** Recommended: MIDI notes in (#439) → gamepad and keys as sources (#228) → the phone surface over the relay → OSC in with learn → audio in → DMX in, Art-Net → camera → IMU, lidar.
6. **Privacy** as proposed in #312: on the device, off by default, shown before it is enabled.
7. **The road for poses: decided 2026-11-10,** the live road, QUIC first and WebSocket last (§6a). Open under it: whether the storyboard's WebRTC road is removed at once or after the S22 test (recommended: after the test).
8. **The pose maths as one Rust crate** (wasm for the creator) instead of a second copy. Recommended: yes; it touches the storyboard editor, so not while that is being worked on.
9. **Forced flash: decided 2026-11-10.** Warned and accepted, it forces. The deck keeps the replaced local layer as its own copy; the deck's operator gets a notice and, if their rights allow, goes back to it or switches between the forced version and their own (SHOW.md §4, #471). Open under it: which right allows going back, and the per-show lock (recommended: a locked flash leaves the switch to the deck's owner only).
10. **When a silent handheld gives the camera back.** Recommended: after 2 s without a pose the deck's own camera returns with a short blend, and the handheld takes over again when poses come back.
