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

## 7. Design drafts (in the chat, files in `468-input-concept/`)
1. **The map:** hardware → adapter (in / beside / far) → profile → source → mapping → parameter, with every family on it.
2. **Inputs in the editor:** the devices this deck sees, their sources with live values, learn and the profile; one mapping with range, curve and smoothing; the controller map as a list.
3. **Builds:** the profiles as columns, the adapters as rows, and what a show needs against what a deck has.
4. **The phone surface:** XY, knobs, pads, tilt, as a far source of a deck.

## 8. For the owner to decide
1. **Three places** (in / beside / far) as the rule, with the phone staying a browser. Recommended: yes.
2. **The smallest machine a deck runs on.** Recommended: a small Linux board (Pi Zero 2 class) with the `tiny` profile. A microcontroller is a device that speaks MIDI, OSC or serial to a deck, not a deck.
3. **Device names in addresses** when there are two of a kind (`midi:a4/cc74` or a place in front). Recommended: a place in front, like decks.
4. **Camera tracking:** a helper beside the deck (recommended), and whether face or body comes first.
5. **Order.** Recommended: MIDI notes in (#439) → gamepad and keys as sources (#228) → the phone surface over the relay → OSC in with learn → audio in → DMX in, Art-Net → camera → IMU, lidar.
6. **Privacy** as proposed in #312: on the device, off by default, shown before it is enabled.
