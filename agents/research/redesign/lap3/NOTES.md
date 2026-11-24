<!-- status: collecting notes for lap 3 of the redesign (epic #903), started 2026-11-24 – nothing decided, no tickets, no code and no designs from these notes until the owner starts the planning session -->
# Lap 3 – collecting notes

The owner's words (dictation typos fixed), then what they mean and where it shows. Input for the planning session, not decisions.
Lap 2 is epic #626, its review round #885; everything said after lap 2's run lands here (owner 2026-11-24: *"anything that I add now will be lap 3 of the redesign round … let's start collecting in a new session, lap 3 epic redesign."*).

## The owner, 2026-11-24
1. **Translations and the language switch.** *"Another one to add to the review list is to check for translations, and if the language switcher is properly respected everywhere, because I've seen like in Catalog or in some other places where there is some like 2 lines for each language visible at one time. I think it needs to be switched by language in order to get to the second language."*
   → the edit language (the ☰ menu's Text language; in the bar for the translator) is respected everywhere: a text shows one line, in the chosen language; the other language is reached by switching, never as a second line under the first. Two checks: every text field and shown name follows the switch, and the translations themselves (the wording) are reviewed.
   Already decided once, for the content pages (2026-10-06, decisions/creator.md:109: "One text language at a time – no side-by-side"; `creator/src/lib/editor/text-language.svelte.ts`). The content pages' two fields follow it (`fields/locale-text.svelte:71`, `fields/locale-map-field.svelte:49` – one language, the source as a muted line under a translation). The Catalog never got it.

   **Found in code (read only, 2026-11-24, main at 546171763) – nothing fixed:**
   - *Every language as its own line, a text field:*
     - `shared/catalog/src/components/locale-text.svelte:38` – the Catalog's text field: one input per language, stacked ("DE" / "EN"), no link to the switch. Used at `item-form.svelte:55` (name), `:56` (description), `:140` (a variant's name) · `product-form.svelte:74` (a variant's name) · `album-form.svelte:31` (title) · `home-scene-form.svelte:30` (name) · `airlift-carrier-form.svelte:17` (name). This is the two lines the owner saw. The same forms serve the admin.
     - Its language list is the interface's, not the content's: `creator/src/routes/(app)/+layout.svelte:59` hands `setupCatalog` paraglide's `locales` (`shared/catalog/src/setup.ts:12`, `:30`) – the Catalog's languages are the UI's languages, whatever a toon has.
     - `creator/src/lib/components/toon/season-cast.svelte:79` – a cast member's role: one field per language side by side.
   - *Every language as its own line, read only:*
     - `creator/src/lib/components/experiences/experience-meta.svelte:29` – an XR experience's title: one line per language.
   - *One control per language – may be meant, to ask in the planning session:*
     - `creator/src/lib/components/chapter/meta-panel.svelte:61` and `toon/toon-editor.svelte:61` – the publishing status, a select per language (a status is per language by its nature).
     - `creator/src/lib/components/cover/cover-previews.svelte:25` – the cover as each language's reader sees it, one preview per language.
     - `creator/src/lib/components/writing/writing-status.svelte:51` and `writing/kanon-editor.svelte:170` – the list of a text's languages as marks, not its texts.
   - *A language fixed in code – the switch cannot reach it:*
     - `creator/src/lib/components/storyboard/shot-inspector.svelte:29` (Action, always `de`), `:32` (Image prompt, always `en`), `:89` (Light note, always `de`).
     - `creator/src/lib/components/fields/give-item-field.svelte:33` and `items/item-inspector.svelte:138` – an item's and a variant's shown name: `de`, else `en`, not the chosen language (the creator's one way is `contentName`, text-language.svelte.ts:92).
   - *The interface language where the text language is meant:*
     - `shared/catalog/src/kinds.ts:69` – a catalog name in the UI language (`getLocale()`), then en, de.
     - `creator/src/lib/editor/publish-tree.ts:137` – the Publish dialog's names in the UI language, then `de`.
     - `creator/src/lib/editor/signal-strip.ts:23` – a signal's label by a passed locale, then `en` (to check which locale is passed).
   - *The switch itself:* `text/text-language-switch.svelte`, shown in the ☰ menu (`shell/main-menu.svelte:114`) and in the bar (`shell/shell-bar.svelte:50`); 36 files read `textLanguage`.
   - Not searched: the deck (its editor's texts and names), the admin's own pages, the running app (no screen was opened – the list is from the code, the Catalog's two lines are the only place the owner named).
   - From lap 2's review, same subject: de = en for 2575 interface keys, ~47 hard-coded texts (lap2/NOTES.md "Wording") – the "check for translations" half of this note.

## Open questions for the planning session
- Note 1: whether the per-language status, the cover previews and the language marks stay as they are (one per language on purpose) or follow the switch too.
- Note 1: the Catalog's languages – the platform's content languages or the interface's (today the interface's).
