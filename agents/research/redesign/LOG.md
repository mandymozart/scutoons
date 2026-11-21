<!-- status: concept, 2026-11-21 – the owner thought aloud ("create some concept"); nothing decided, nothing built. Issue: #767. -->
# One Log for everything that happens

> The owner, 2026-11-21, thinking aloud: *"about problems. logs, infos, errors, events, warning etc. I was thinking of a unified log which can be target for all that is happening across the app, scope can be applied to the selection or all or toon or chapter or season or show. so we can filter also for all of this. maybe problems go into it as well. a panel or tab. create some concept"*

## 1. What exists today, in pieces

| Where | What | Shape |
|---|---|---|
| Deck editor | **Log panel** + status bar (`deck/editor/src/log.rs`, `deck/kit/src/editor/status.rs`) | One bounded ring in the core: `deck-ui messages` (`deck/ui/src/messages.rs`): `Message { level: Info/Ok/Warn/Error, source, text, at, detail, times }`, filters All / Errors / Warnings / Info / Technical, find, Copy all, Save as a bug report. Tracing lines (warn+) feed in as source `tech`; the conductor's log lines too. |
| Creator | **Problems tab** (#751) | Content validation, `shared/content` `Problem { level: error/warning/info, file, locale, path, message, code }`, plus accept / notes toggles. Recomputed while you edit. |
| Creator | **Status bar** (#751) | The newest toast (`$lib/toasts.ts`), the save chips, the error / warning counts. A press opens Problems. |
| Creator | **Toasts** | Passing messages (saved, failed, copied …). They're gone once they fade. |
| Server | events / activity | Generations, publishes, admin and embassy activity (EventsAdmin, the MCP `list_events` / `log_activity`). None of it reaches an editor. |

So the deck editor already has "one inbox every part posts to". The creator has Problems (state) and toasts (moments), with nothing joining them. Nothing is scoped to *what you're looking at*.

## 2. The idea: one Log, two kinds of line, scoped by address

**One Log, in the core.** `deck-ui messages` grows into the one model for both editors, as the Library did (law.md `[editor-style]` 2026-11-21: one model in the core, reaching the creator through wasm, the deck editor's features kept). The creator stops having its own toast list and Problems list as separate truths.

**Two kinds of line:**
- **Standing:** a *state* that's true now. Problems are standing lines: a missing title, a jump to a gone group, a text past the safe area. They stay while true and leave when fixed, without becoming history. They can be accepted, carry a note, and jump to their place.
- **Passing:** a *moment* that happened: saved, published, a generation finished or failed, a Library upload, a live show's cue, a sync, an API error, a technical line. They age and keep their time. Repeats fold (`times`, as today).

**Every line has an address.** The creator's addresses (`shell/address.ts`: toon, season, chapter, group, slice, character, place, experience, show …) become the line's `scope`. A problem in slice 2.3 is scoped `catpope/season/1/chapter/2/group/2/slice/3`. A generation is scoped to its target, and a publish to its item. Lines without a place (a technical line, a sign-in) are scoped to the app.

**The scope filter is the address prefix.** One row of pills over the Log:

`Selection · Item · Chapter · Season · Toon · Show · All`

- **Selection:** the selected slice / node / tile; follows the selection.
- **Item:** the open item (the front tab).
- **Chapter / Season / Toon:** the open item's chapter, season or toon (its ancestors in the address). Only the ones the open item has are offered.
- **Show:** the live show's lines (conductor, cues, devices), scoped to the show.
- **All:** everything, across items.

Because a scope is just a prefix, every line knows which scopes it belongs to without extra bookkeeping.

**The other filters**, as in the deck's Log:
- level: errors · warnings · info;
- kind: problems (standing) · events (passing) · technical;
- find;
- accepted on/off (problems).
Copy all and Save stay (for bug reports).

## 3. Where it shows

- **One tab: Log.** It replaces the Problems tab. "Problems" becomes a **preset filter of the Log** (standing lines, errors and warnings, scope Item), so the chapter preset's tab still reads as Problems where that's what matters. A workspace preset can keep its own filter (the Flow workspace: scope Chapter; the live show: scope Show, kind events).
- **The status bar** stays the deck's: the newest line (dot, words, age), the counts, and a press opens the Log. The counts follow the **Item** scope by default, so you see *this* item's problems, not the whole toon's.
- **On the stage (#765):** the Log can take the stage as a full-height list (a page tab, clear of the bar), for a review pass over a whole season.
- **In place:** a field with a standing line shows it quietly (frame colour, words, never a side bar, H10). The Log is where you see them all; the field is where you fix one. A press on a line jumps to its address and selects it.
- **Toasts** shrink to the status bar's newest line. An error that needs an answer (a save conflict) still asks, in the bar or a dialog, and is also logged.

## 4. Who posts

| Source | Kind | Scope |
|---|---|---|
| Content validation (shared/content, the core's checks) | standing | the problem's path |
| Autosave / publish (#760) | passing | the item |
| History restore | passing | the item |
| Generations (start, done, failed, approved) | passing (a running job may stand while it runs) | its target |
| Library (upload, replace, delete, a misfit drop) | passing | the asset / the slot's item |
| Live show / conductor / devices | passing | the show |
| API errors, sign-in, sync | passing | the app (or the item asked about) |
| Technical (console errors, the wasm core's tracing) | passing, kind technical | the app |
| Server events (others' publishes, generations finished elsewhere) | passing, fetched | their item |

## 5. What stays out, deliberately

- **No second store.** The ring lives in the session's memory, as today. Server events are fetched for the scope you're looking at, not mirrored. History (#760) is the record of versions; the Log isn't an audit trail.
- **No per-feature panels** (a generations log, a sync log …): they're kinds and scopes of the one Log.
- **No new severity levels** beyond error / warning / info (+ ok for the dot). "Event" is a kind, not a level.

## 6. Questions for the owner

1. **Problems inside the Log, or beside it?** The concept puts them in as standing lines with a "Problems" preset filter. The alternative is that the Log only holds passing lines and Problems stays its own tab, sharing the scope pills.
2. **Default scope of the status bar's counts:** Item (this concept), or the workspace's context (the whole chapter in the chapter workspace)?
3. **Server events:** pull them into the Log for the scope (others' publishes and generations visible to you), or session-only lines for now?
4. **The deck editor:** take the scope pills and standing lines there too (its show, rig and scene addresses), as one model, or creator first?

## 7. If it goes ahead (not before the owner says the concept is clear)

1. The core: `deck-ui messages` gets `scope: Address`, `standing: bool` (with a key to replace / clear it), `code` and a jump target, scope filtering by prefix, and the Problems preset. Unit-tested in the core and reached through wasm.
2. The creator: one Log tab replaces the Problems tab and the toast list. Validation posts standing lines; toasts become passing lines; the status bar reads the ring.
3. Sources one by one (autosave/publish with #760, generations, Library, the show).
4. Server events, if question 3 says so.
