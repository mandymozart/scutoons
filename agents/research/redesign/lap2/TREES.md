<!-- status: concept for the owner, round 2 (2026-11-24) – the tree concept rethought after the owner's answer on "inside a document"; nothing decided, nothing drawn yet. Round 1 is in git (b524eb5b8). -->
<!-- 2026-11-24, after the extended Library (LIBRARY.md): decided – a tree holds what one thing is made of; lists of what exists are the Library's; trees do not travel between workspaces (point 4 below is dropped); the Library's "items to open" goes (open question 2 is answered). The rest of this file stands as the thinking behind it. -->
# Trees – what they are for (round 2)

## The owner's words (2026-11-24)
- *"Tree is a very ambiguous concept at this point. Rethink what its purpose is and how it relates to hierarchies in the other deck edit views and in the XR editor and map editor etc. Some trees are very opinionated, others a bit more free. In the end they all first of all have a name. It seems like the workspace kind of mirrors the tree, so the name should be the same. Which would allow us to open trees across workspaces in case we need some referencing."*
- On a model's bones in the creator's Rig panel: *"I see this as a model view rather than a workspace that is about assigning stuff to a character. So like in the deck editor it's sort of an isolated asset view, in this case 3D. But we should [not] do this in two places. If the deck editor is our place for 3D manipulation or video etc., we will open the deck rather than showing information twice."* ("not" is the agent's reading of the dictation – the next sentence says it.)

## The one distinction that sorts everything: composing vs. the asset itself
- **A workspace composes.** It is about one kind of thing made of other things: a chapter of slices, a scene of a place and who is in it, a character of looks, a model, a voice and a cast, a show of rows, a rig of tracks and nodes, a map of places and routes. Its work is assigning and arranging.
- **An asset view shows one asset by itself.** A 3D model with its bones and clips, a video with its trim, a sound with its waveform, a module with its code. The deck editor already has exactly this – the Asset tab (media.rs: video, image, sound, 3D; a module and a character open as an asset too).
- **Not twice:** an asset's inside is shown in one place only – the asset view, in the app that edits that kind. For 3D and video that is the deck. The creator shows that the asset is assigned (its name, a still, a line like "17 bones · 2 clips") and **Open in the deck**.

## So a tree is: the composition, top to bottom, ending at the assets
1. **One tree per workspace, named like the workspace** (the owner's concept). It runs from the collection down through the open item to its parts: "Toon": toons → seasons → chapters → slices. "Scenes": scenes → a scene's place, spots and who. "Characters": characters → looks → what is assigned. "Map": layers, places, routes, regions. "Catalog": kinds → entries. Deck: "Rigging": rig → tracks → nodes; "Show": conductors, roles, rows.
2. **The answer to "inside a document":** it depends on what the inside is.
   - Parts you compose (a chapter's slices, a rig's nodes, a scene's spots, a show's rows) are the **same tree, one level deeper** – no second small tree in a panel.
   - An asset's inside (a model's bones and clips, a video's tracks) is **not in any workspace tree**. The tree ends at the asset: a leaf with its name and still. Opening the leaf opens the asset view – for 3D and video, the deck.
3. **Opinionated or free** is each tree's own business: fixed levels where the data has them (seasons, tracks, rows), free groups where it doesn't (scenes, catalog entries, places). Same look, keys, search and drag everywhere.
4. **Trees travel** (the owner's concept): any workspace can open another's tree by its name from the ☰ Views list, to reference – drag a character from "Characters" into a scene, a place from "Map" into the XR's Place tab, a scene from "Scenes" into a show row. Later across the apps too (DECK-LINK.md).
5. **Tree, Library, Inspector – three different jobs:** the tree is what the workspace is made of; the Library is the assets to put into slots; the Inspector is the selected thing's values.

## What it would change on today's screens
- **Creator · Characters · Rig panel:** stays what the character workspace is for – assigning the model to the look, the idle, the placeholder body, the A-pose. The **Bones** and **Clips** lists go; in their place the model's line and **Open in the deck** (the deck's asset view has them).
- **The 3D on the stage in the creator** (the owner's 2D-first / 3D-first thought) is then a viewer of the assigned model – looking, not manipulating.
- **Panel names:** "Tree" (creator), "Rig" and "Outline" (deck) become the workspace's name.
- **Found while checking "not twice":** a module's code is edited in the creator (Sound design, `/edit/module:…`) and in the deck (the Module workspace). Under the rule one of them goes – the owner's call.

## Open for the owner
1. **Characters:** a "Characters" tree (characters → looks → assigned assets), or does the Library stay the way to open characters (lap 1's point 5)?
2. **The Library's "Items to open"** lists the same things as the top of each tree – keep both, or does the tree take that over once it travels?
3. **Borrowed tree, open an item:** jump to its own workspace, or stay and only reference?
4. **Modules:** edited in the creator, in the deck, or (the rule says) only one – which?
5. **Images and sound:** is their asset view the deck's too (it has both), or does the creator keep a light one for 2D work?
