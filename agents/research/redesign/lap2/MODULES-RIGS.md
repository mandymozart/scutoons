<!-- status: answer to the owner's question in the lap 2 planning session, 2026-11-24 – what a module, an instrument, a preset and a rig are in today's data, and what follows; nothing decided. -->
# Module · instrument · preset · rig – what they are today

Owner 2026-11-24, on the universe tree (character rigs, modules, instruments / presets): *"Those three are all very much the same. So presets first of all would apply to all of them … or how do we differentiate. A module is the most general version. Rigs combine modules. Character rig is a specific type of rig. Instrument is a module limited to …? What exactly?"*

Standing decisions: 2026-11-02 [rigs-in-core] "module → instrument → rig are layers of one mechanism"; 2026-11-09 [rigs] "an instrument is entered, like a lower level of a tree … from outside only the outlets"; 2026-11-22 [rigs] "a rig is any combination of modules … a character is a specific rig".

## In the stored documents (production, read 2026-11-24)
| | What the document holds | Example |
|---|---|---|
| **Module** (20, + 35 built-in node kinds) | code: one Rust/wasm leaf with ports and parameters | `module:osc`, `module:cloud` |
| **Instrument** (8) | a wired group of nodes: `node` (which modules), `link` (the wiring), one `out`, and `expose` – a short list of named knobs with ranges; a `kind` ("pad"); rights (play / tweak / derive / copy) – it is also an item one can own or be lent | `instrument:drone` = osc + osc + mixer, exposes "drone pitch" and "drone" |
| **Preset** (7) | values for one instrument's knobs, with a `parent` it inherits from, and tags | `preset:sand_song.bright` on `sand_song`, parent `sand_song.base` |
| **Rig** (5, all character rigs) | a `subject` (character.sandy), `track`s – each an instrument + a preset + gain / pan / mute –, `sticks` (which knobs the two sticks move), and `moods` – values per mood | `rig:character.sandy` = worm_rumble + sand_wind, five moods |

## So the answer to "an instrument is a module limited to … what?"
It is not a module but a **closed group of modules**: limited to one outlet, to a handful of exposed knobs (the only things a preset, a mood or the sticks may touch), with its inner wiring hidden until you enter it – and it is a thing a member can own.

## What is the same, what differs
- **The same:** all three are nodes with wiring, parameters and outputs – one mechanism at three depths (module inside instrument inside rig).
- **What each level adds:** the module adds code. The instrument adds the closing: one outlet, few knobs, ownership. The rig adds the mix (tracks with gain / pan / mute), a subject it belongs to, the sticks and the moods.
- **Presets are already on two levels under two names:** a preset is saved values for an instrument's knobs; a rig's `moods` are saved values for the rig's knobs, one set per mood. The same thing. A module's parameter defaults are the third.

## Where today's data and the decisions disagree
- 2026-11-22 says a rig is any combination of **modules**; the stored rigs combine **instruments** (a track names an instrument). The cloud and the bamboo are modules, not rigs.
- Every stored rig is a character's; the decision says a rig can be a light piece, a tree, a 16-deck setup.
- Presets exist only for instruments; moods only for rigs.

## What follows if they become one thing (for the owner to decide)
One nestable thing – by the 2026-11-22 words, a **rig**: any combination of modules and other rigs. Then:
- an **instrument** is a rig that is closed (one outlet, few knobs) and can be owned;
- a **character rig** is a rig whose subject is a character, with a preset per mood;
- a **preset** is saved values for any of them – a module's, an instrument's, a rig's; a mood is a preset with a mood's name;
- in the universe tree the three branches become one, edited in the deck (decided today for modules).
Open: whether "instrument" stays as a word for the closed, ownable kind, or goes.
