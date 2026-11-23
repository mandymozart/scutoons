<!-- status: concept draft for the owner, 2026-11-24 – how modules sit in SCUTOONS' DNA, spores included. A draft to correct, not a decision; the owner: "a major design task we need to finish now before this redesign lap can start". Picture: modules-dna.html. Facts about today's data: MODULES-RIGS.md. -->
# Modules in the DNA – draft

## The owner's words
- 2026-11-24: *"About the modules. Spores are possible modules as well. I think we haven't finished how modules integrate into our DNA yet. Draft this as well. This is also a major design task we need to finish now before this redesign lap can start."*
- 2026-11-24: *"A module is the most general version. Rigs combine modules. Character rig is a specific type of rig."* · *"Modules should primarily be edited in the deck. Also the wiring for nodes happens there. So it is universal. Modules will also be edited a lot with MCP and with an external text editor."*
- Standing: 2026-11-02 [rigs-in-core] *"rigs are module groups"* – module → instrument → rig are layers of one mechanism in the one core · 2026-11-09 [rigs] an instrument *"is entered, like a lower level of a tree … from outside only the outlets"* · 2026-11-22 [rigs] *"a rig is any combination of modules … a character is a specific rig … certain slots take certain rigs or other assets by their definition"* · 2026-11-21 (#763, network) a node is a spore, connecting is weaving, what runs it is a driver; ports carry data, pores carry organic matter · 2026-11-21/22 (AI-HOOKS §0) a spore chain is a document, a generator; a slot takes an asset or a generator as its stand-in.

## The draft in one paragraph
Everything that **does** something is a **module**. Everything **made of** modules is a **rig**. Everything that **takes** one is a **slot**. Everything that **connects** them across scenes, shows, chapters and devices is the **network**. A spore is a module that is not run by the engine every instant but by an agent, an LLM or a tool, once per run – the same shape, another driver.

## The six words
| Word | What it is | Today | Examples (production) |
|---|---|---|---|
| **Module** | the leaf: ports in and out, parameters, and a **driver** that runs it | 20 modules (Rust / wasm) + 35 node kinds built into the engine; spores drafted, not built | `osc`, `reverb`, `cloud`, `bamboo` · built in: `screen`, `space`, `video`, `mixer` · spores: image → 3D, auto-rig, crop, the output spore |
| **Driver** | what runs a module, and how often | the engine, every block (128 frames) – the only one built | engine (sound, picture, control) · an agent or LLM (a spore's run) · an external tool · a job (night run, publish) |
| **Rig** | modules wired together; a rig can hold rigs | 5 rigs, all a character's, made of instruments; 8 instruments | closed, one outlet, few knobs, ownable = an **instrument** (`drone`) · belongs to a character, a set of values per mood = a **character rig** (Sandy's) · made of spores, ends in an output = a **generator** |
| **Preset** | saved values for the knobs a module or rig shows | presets on instruments (7); a rig's moods are the same thing by another name | `sand_song.bright` · Sandy's "angry" |
| **Slot** | a place that takes a module, a rig or an asset by a definition (what fits) | scenes' holders (who, where, spots); a look's 3D model; a rig's tracks; the generator stand-in is drafted | "who ← a character" in `Solo` · a look's model takes a GLB or a generator whose output is a GLB |
| **Network** | how what one thing fires reaches another – across a scene, a show, a chapter, a device | drafted (NETWORK.md, round 3–4); the deck's wiring and the show's "fires on" are built | a reader's choice → a story event → a show's row → a scene on a deck |

## What this would settle
1. **One ladder, one tree:** module → rig (→ rig …) → slot. Entering a rig goes one level down the same tree (2026-11-09), down to the module; the module's own inside is its code – opened in the deck or an external editor, never drawn twice.
2. **Instrument and character rig are not kinds beside the rig but rigs with a property** (closed and ownable; bound to a character with moods). Whether the word "instrument" stays is the owner's call.
3. **Presets on every level**, moods being presets with a mood's name.
4. **Spores join the same catalog as modules** – one list of leaves with ports, told apart by the driver. A generator is then a rig, found, versioned and owned like one.
5. **One place to wire:** the deck (decided today for modules and node wiring). The creator assigns – it fills slots – and previews.

## What is not finished (each needs the owner's word)
1. **Where the spore canvas lives.** It is drawn in the creator (Generate). If spores are modules and wiring is the deck's, the canvas is the deck's wiring view with another driver – or the creator keeps it because generating is a creator's craft. This decides screen C11.
2. **Two catalogs of leaves today:** the engine's 35 built-in node kinds and the 20 `module:` documents. One list, or do the built-ins stay apart as "the engine's own"?
3. **Rigs of modules, or of instruments?** The stored rigs name instruments per track; 2026-11-22 says a rig is any combination of modules; the cloud and the bamboo are stored as modules though 2026-11-22 calls the bamboo forest a rig.
4. **Where a module is written.** 2026-11-02: "any module we write through our creator editor – the sound designer is the proof of concept." 2026-11-24: modules are edited in the deck, by MCP and an external editor. If today's word stands, the creator's Sound design workspace goes or becomes a player.
5. **A spore's ports and pores against a module's ports.** A module's ports are audio, control and events at the engine's rate; a spore's ports carry files (a picture, a text, a mesh) and its pores "organic matter" (a whole being). One port system with more types, or two that meet at the slot?
6. **Rights.** Instruments, presets and rigs carry play / tweak / derive / copy and can be owned or lent. Do modules, generators and spores carry the same?
7. **Scenes.** A deck scene is itself nodes wired on a stage. Is a scene a rig with a stage (then one more rung on the ladder), or does it stay a container apart, as decided 2026-11-20 for what travels between containers?
