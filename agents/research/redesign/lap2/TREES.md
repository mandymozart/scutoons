<!-- status: concept for the owner, 2026-11-24 – the owner's idea of trees thought through; nothing decided, nothing drawn yet. -->
# Trees – what they are for

## The owner's concept (2026-11-24, on the Tree panels of Scenes, Catalog and the Rig panel's bones)
*"Tree is a very ambiguous concept at this point. Rethink what its purpose is and how it relates to hierarchies in the other deck edit views and in the XR editor and map editor etc. Some trees are very opinionated, others a bit more free. In the end they all first of all have a name. It seems like the workspace kind of mirrors the tree, so the name should be the same. Which would allow us to open trees across workspaces in case we need some referencing."*

## Today: eight different things, three names
| Where | Called | What it holds | Fixed or free |
|---|---|---|---|
| Creator · Toon / Chapter / Storyboard | Tree | the toon's seasons → chapters → its parts (chapter-tree) | fixed (the story's order) |
| Creator · Scenes | Tree | every scene, filtered by toon and state, with Published / Draft | free (a list, groups by name) |
| Creator · Catalog | Tree | the catalog by kind (items, recipes, albums …) | fixed by kind, free inside |
| Creator · Map | Tree | Layers · Places · Routes · Regions | fixed sections, free inside |
| Creator · Text | Tree | – | – |
| Creator · Characters | – (point 5: no Tree, the Library opens characters) | – | – |
| Creator · Rig panel | Bones | a model's skeleton | fixed (the model's) |
| Deck · Build | Rig | the rig → its tracks → their nodes | fixed |
| Deck · Show | Outline | conductors · roles · rows | fixed |
| Deck · Module | – (the Library lists modules) | – | – |
| XR (drafts) | – | places come from the Map | – |

## The concept, thought through
1. **What a tree is for:** the workspace's own things and how they hang together – where you pick what the stage shows and the Inspector edits. Not the Library (files you drop in) and not the Inspector (one thing's values).
2. **One tree per workspace, named like it:** the Scenes workspace's tree is "Scenes", Map's is "Map", Catalog's "Catalog", Characters' "Characters", the deck's Rigging tree "Rigging", Show's "Show". The panel tab says the workspace's name; "Tree", "Outline" and "Rig" as names go.
3. **Opinionated or free is the tree's own business:** fixed levels where the data has them (a toon's seasons and chapters, a rig's tracks and nodes, a show's rows), free groups where it doesn't (scenes, catalog entries, map places). Same look, same keys, same search, same drag.
4. **Trees travel:** because a tree is named by its workspace, any workspace can open another's tree as a view (the ☰ Views list: "Characters", "Map" …) – to reference: drag a character from "Characters" into a scene, a place from "Map" into the XR's Place tab, a scene from "Scenes" into a show row. Opening an item from a borrowed tree jumps to its own workspace (or stays, holding it as a reference – open).
5. **Across the apps:** the deck's trees (Rigging, Show) and the creator's could be borrowed the same way once the two apps talk (DECK-LINK.md) – e.g. "Scenes" inside the deck's Show.

## Open for the owner
- **Inside a document:** the bones of a model, a chapter's slices, a rig's nodes – are they the same tree one level deeper (the workspace tree opens down into the open item), or the open item's own small tree in its panel?
- **Characters get a tree** ("Characters") again – against point 5 (the Library opens characters) – or the Library stays their tree?
- **Borrowed tree, open an item:** jump to its workspace, or stay and only reference?
