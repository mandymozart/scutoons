Status 2026-11-08: research for W2 of MODULE-WIRING.md (#203, #425), undecided. Recommendation: every event inside a document is a visible cable, the world's events enter through world nodes, and an input with no cable hears nothing. The owner decides.

# EVENTS-VS-PARAMS – how Unreal and TouchDesigner handle events against parameters, and what W2 should take

Owner 2026-11-08, on the finding that event cables are not in our graph and are matched by name across the whole rig: "need to get there as well. please check out events are handled in comparision to parameters in unreal and in touch designer. those are our compeitors."

Sources read 2026-11-08. Epic's pages are the Unreal Engine 5.8 documentation; Derivative's carry their own last-edited date, given below. Not verified: Unreal's RPC page came back empty, so how an Unreal event crosses machines is left out; I found no reference page for MetaSounds' Execute Trigger Parameter.

## 1. Our state today (read from the code)
- Audio, control and parameter links are graph links. Event ports are not: `Descriptor::graph` leaves them out (`deck/catalog/src/lib.rs:335`).
- `deck/engine/src/evhost.rs` joins an event output to **every** module event input that takes one of its names, across the rig. Two sequencer + synth pairs in one rig therefore hear each other. That is the defect W2 removes.
- Timing is already good: an event carries its frame. Module to module it arrives one block late at the same frame (`at + QUANTUM`), so node order never changes the sound. The world's events are placed on their frame; a late one lands on frame 0 and is counted.

## 2. Unreal
| Thing | Cable or name | Timing against a parameter | A source from outside | Across documents |
|---|---|---|---|---|
| [Blueprint exec and data pins](https://dev.epicgames.com/documentation/en-us/unreal-engine/nodes-in-unreal-engine) | Both visible cables: exec white, data coloured by type | Exec decides what runs and when; [pure functions](https://dev.epicgames.com/documentation/en-us/unreal-engine/functions-in-unreal-engine) on data pins run when their value is needed | A node in the graph | – |
| [Event Dispatcher](https://dev.epicgames.com/documentation/en-us/unreal-engine/event-dispatchers-in-unreal-engine) | A named signal. The listener is bound by a Bind node with a Target pin holding an instance; **no line from source to listener** | Fires when called; one to many | Needs a reference to the instance | Via that reference |
| [MetaSounds triggers](https://dev.epicgames.com/documentation/en-us/unreal-engine/metasounds-reference-guide-in-unreal-engine) | **Visible cables with their own icon**, to any number of nodes; a trigger pulses as it fires | Sample-accurate from timing nodes or gameplay; floats run at block rate; a trigger into a non-trigger node yields block rate. A delayed read is one block late, to avoid cycles | Graph **Input** nodes (On Play); interfaces add named inputs (Distance, Azimuth); gameplay [sets inputs by name](https://dev.epicgames.com/documentation/en-us/unreal-engine/creating-procedural-music-with-metasounds) | Graph nodes expose another graph's pins; outputs can be watched from Blueprint |
| [Niagara events](https://dev.epicgames.com/documentation/unreal-engine/events-and-event-handlers-in-niagara-effects-for-unreal-engine) | A handler picks its source from a dropdown; no cable. CPU only | Limit on how often per frame | The dropdown | [Data Channels](https://dev.epicgames.com/documentation/unreal-engine/data-channels-in-niagara-for-unreal-engine?lang=en-US): a shared named asset (labelled experimental in the 5.4 Python docs) |
| [Sequencer Event Track](https://dev.epicgames.com/documentation/en-us/unreal-engine/cinematic-event-track-in-unreal-engine) | A keyframe on a timeline | Once on its frame, or a repeater each frame; a setting places it in the evaluation | Bound under an actor, or in the sequence's Director Blueprint | Blueprint Interfaces |
| Global things: [Get Game State](https://dev.epicgames.com/documentation/en-us/unreal-engine/BlueprintAPI/Game/GetGameState), [subsystems](https://dev.epicgames.com/documentation/en-us/unreal-engine/programming-subsystems-in-unreal-engine) | **A node you place in the graph**; subsystems come as context-aware nodes found by search | Read when used | That node is the source | Lifetime set by its scope (engine, game instance, player) |

## 3. TouchDesigner
| Thing | Cable or name | Timing against a parameter | A source from outside | Across documents or machines |
|---|---|---|---|---|
| [CHOP channels and pulses](https://docs.derivative.ca/Trigger_CHOP) (2025-12-11) | Solid wires inside a family. **A pulse is a channel going 0 → 1 → 0**, so it is a signal, not a separate kind of thing | Threshold crossing fires. [Time slicing](https://docs.derivative.ca/Time_Slicing) (2022-08-15): a cook carries all samples since the last, so pulses land accurately when frames drop | An input CHOP | See Touch |
| [Parameter modes](https://docs.derivative.ca/Parameter) (2026-08-07), [`Par`](https://docs.derivative.ca/Par_Class) (2026-08-19) | Constant, expression, export (from a CHOP, shown green), [bind](https://docs.derivative.ca/Binding) (two-way, 2023-09-08). A pulse parameter acts at once, from a channel, Python or a click | Cooked by frame | By name in the expression | `op('…')` path |
| [Export](https://docs.derivative.ca/Export) (2021-10-15) | **A name that is still drawn:** a grey dotted link with an arrowhead that animates as it cooks. Mass export by channel name `path:parameter` | Exports and expressions now perform about alike | – | – |
| [CHOP Execute DAT](https://docs.derivative.ca/CHOP_Execute_DAT) (2022-05-23) | The CHOP is named in a parameter; the DAT's input is unused | Off-to-on, on-to-off, value change, as the CHOP cooks; can fire several times in one rendered frame | A script reaction | – |
| [Wires against references](https://docs.derivative.ca/TouchDesigner_Glossary) | A wire joins one family. **Every reference by name is shown as a grey dashed link** and is a [cook dependency](https://derivative.ca/UserGuide/Dependency); a key hides them | – | – | – |
| [Global OP shortcut](https://docs.derivative.ca/Global_OP_Shortcut) (2021-06-21) | A unique short name reachable from anywhere, in code; the page says nothing of a drawn link | – | The named component | Inside a project |
| [Touch In / Out](https://docs.derivative.ca/Touch_In_CHOP) (2025-10-28) | **An explicit pair of nodes** with an address and a port (TCP or UDP, multicast) | Arrives as time slices through a queue with a target length, repeating or dropping samples for drift; a Sync option keeps streams in frame | A node at the edge of the network | Machines |

## 4. What the two agree on
1. **Local events are visible.** MetaSounds, the closest thing to our engine, draws triggers as their own cable type. TouchDesigner goes further and draws even references made by name. Our by-name join is what they both avoid inside a graph.
2. **Names belong at the edge.** Unreal sets a MetaSound's inputs by name from gameplay, and Touch In names an address; in both the name stands at one marked place. Blueprint dispatchers and Global OP shortcuts are the invisible form, and they are the parts that are hard to read.
3. **An event has a timing of its own.** MetaSounds separates sample-accurate triggers from block-rate floats and tells you when one degrades into the other. Ours already keeps the frame and is one block late between modules.
4. **Crossing to another machine is a visible object** with an address, a queue and a stated way of failing (Touch In / Out).
5. **A running event shows on its cable** (MetaSounds pulses, TouchDesigner's arrowhead). That is also our rule that every mapping is audiovisual (RULES A10).

Where we go beyond both: the owner's world nodes with images of their origin. Unreal's getter nodes are only text, and TouchDesigner's shortcut is not drawn at all.

## 5. Recommendation for W2
**Module-to-module events become explicit cables only, and the world's events become those world nodes, which are the only place a name stands. An event input with no cable hears nothing.**

In plain words:
- **Event cables are a port type like audio and control** (`PortType::Event`, many to many, `Descriptor::graph` stops leaving them out), written in `route` / `link`. `evhost` joins by cable. There is no by-name fallback; this answers MODULE-WIRING question 2, and `catpope_analog` gains its one line.
- **World nodes.** MIDI, generators, carrier bag, VN state, and other scenes, characters, rigs and decks each appear as a node, with an image of the origin where there is one. A world node holds its name filter (the `listen` the dispatcher already has) or its address. It has ordinary event outputs, cabled like any other. Nothing else in a document names a world event.
- **This reconciles wide reach with portable rigs.** The cables in a rig stay relative. The one absolute thing is the address on the world node, so MODULE-WIRING §3 needs one change: a rig holds relative addresses except on its world nodes. Reach stays the one `Reach` rule, and "nothing travels unless asked" (2026-11-02) now has a visible form: a cable is the asking.
- **Another deck is a world node with a stated way of arriving:** over the relay, a late event dropped or moved to frame 0 and counted, shown on the node, as Touch In shows its queue. The relay stays a carrier and the resolving stays on the deck.
- **Event and parameter stay two types and the core refuses a cable between them,** with its sentence at the port. A level becoming an event, or the reverse, is a visible node when someone needs it (TouchDesigner's Trigger CHOP), not an implicit conversion. That node is not W2.
- **The timing does not change:** one block late at the same frame between modules, the frame kept from the world. The cable flashes when an event passes.
- **Not taken:** dispatcher-style binding in code, mass routing by channel-name paths, script callbacks (RULES A3: modules are Rust).

**What this adds to W2:** the world node as a node kind, in the core and in `wire_view`. It keeps W2 at M only if the first version has MIDI and one generator. The rest, including other decks, are more of the same node once W4's `Reach` exists.

**Done when (added to W2's):** two sequencer + synth pairs stay apart; a synth with no event cable stays silent; a MIDI world node cabled to a synth plays it, on native and headless.
