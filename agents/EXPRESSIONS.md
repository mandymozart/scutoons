# EXPRESSIONS – variables, logic and reader state in texts (owner 2026-10-06)

Owner decisions of 2026-10-06 (session with the coordinator). Research: scratchpad `expression-research.md` (summary
below). Branch `chapter-neutral`.

## 1. The language – our own, small (research result)

No library fits: safe engines with good errors are 6–25× too big (CEL), the right-looking ones have sandbox-escape
CVEs (expr-eval, angular-expressions) or are unmaintained (Jexl, last release 2020), template engines compile to JS
(Nunjucks, Handlebars, filtrex, SugarCube – break "never eval"), i18n formats (ICU, MF2, Fluent) have no logic. So: a
Pratt parser + evaluator + static analyser in `@scutoons/content` (~450 lines, ~3 KB gz in the reader), syntax after
Jexl / Angular, plural via the browser's `Intl.PluralRules`.

- In a text: `{expr | pipe(args) | …}`. Operators `+ - * / %`, `== != < <= > >=`, `and or not` (+ `&& || !`), `??`,
  `cond ? a : b`; strings `"…"` / `'…'` with backslash escapes; numbers; `true false null`. A pipe binds loosest.
- No generic property access: `map.total` is a NAME looked up by a SCUTOONS resolver against fixed namespaces; no `[]`,
  only registered functions; limits 500 chars, depth 32, 64 nodes; runtime errors never break the reader.
- Pipes: `clock`, `number`, `round`, `plural(=0: "…", one: "…", other: "# …")` (`#` = the number; per the text's
  language), `ordinal(...)`, `select(a: "…", b: "…", other: "…")`, `default("…")`, `upper`, `lower`, `capitalize`, `pad(n)`.
- Functions: `min`, `max`, `round`, `floor`, `random(a, b)` (stable per reader + text: the same reader always gets the
  same result), plus the namespace functions below.
- **Old syntax discarded** (owner): `{x:clock}` is gone – a migration rewrites stored texts to `{x | clock}`; no alias.
- Missing value: readers never see raw code – an empty text or the `default(...)`; the creator warns.

## 2. What a text can read (namespaces)

| Namespace | Names (proposal) |
|---|---|
| `var.` | the toon's story variables (VN `setVariable` / choice set+add) |
| `reader.` | `name` (askText, platform scope), `signedIn`, `member`, `memberDays`, `since`, `gifts`, `language`, `daysSinceStart` |
| `read.` | `chapter(n)`, `chapters`, `finished`, `position(n)` |
| `inventory.` | `has(id)`, `count(id)`, `items`, `edition(id)` |
| `avatar.` | `character`, `look`, `wears(id)`, `unlocked` |
| `stats.` / `effects.` | `stats.testo`, `stats.appetite`, `effects.active(id)`, `effects.count` |
| `map.` | `unlocked`, `seen`, `total`, `place(id)`, `here` |
| `now.` | `hour`, `weekday`, `night`, `date` |
| events | `resolved(eventId)`, `count(eventId)` (see §5) |
Never: email, codes, ids of accounts, anything about other readers.

## 3. Where expressions run

Text boxes (slice dialog text – rendered by @scutoons/reader, live when values change), VN messages, choice labels,
event conditions (`if`: the simple form variable–op–value stays the default in the creator; "expression" = advanced
mode on the same engine).

## 4. Validation + editing

- Validated in the creator while typing AND on the server when saving (a PHP port of the parser/analyser, ~300 lines,
  tested against the same golden fixtures as the TS one) – MCP / raw API can't store a broken expression.
- Static analysis: errors with positions, the names/functions an expression reads (→ validation "unknown name",
  "variable set nowhere", the flow graph's dead variables).
- Creator text fields: typing `{` offers "Enter = builder"; Enter opens the builder popover at the cursor; or keep typing –
  on `}` the expression becomes a semi-locked CHIP in the field. Click a chip = the builder; double-click / long-press =
  edit in place as plain text (back to a chip on leave); Backspace next to a chip removes it whole. Invalid → red wavy
  underline + tooltip with the problem and position ("`map.totl` – unknown, did you mean `map.total`?").
- Builder: pick a value (story variables, reader state, functions) → comparison / arithmetic → then / else texts (in the
  header's language) → pipes → live preview with sample readers ("new guest", "halfway", "everything collected").
- Stored as plain text with `{…}` – the chip is only the creator's display.

## 5. Events: repeat modes + the save game

Per event (slice triggers, item taps, group-script events where it makes sense): `every time` (default, e.g. most
onomatopoeia), `once per page view` (today's `once`), `once for good` (checked off in the reader's save game, never
again), `when a condition holds` (fires again whenever its `if` is true). Events get hidden stable ids (auto, like text
boxes) for the save game; expressions read `resolved(id)` / `count(id)` (picked in the builder, never typed). Members'
save games sync like the variables. The flow graph + reachability mark "once for good" events (✓, run at most once).

**One recurrence model for items AND events (owner: "recycle the item pickup logic, so this timing is not implemented
twice").** Placed items already have it: `SliceItem.respawn` (once / never / every visit / every n minutes·hours·days,
once per period via time buckets – `viewer/src/lib/visual-novel/slice-items.ts`, content.ts ~330, ITEMS.md respawn).
Refactor it into one shared module in `@scutoons/content` (types + pure period logic: `recurrence`, `periodOf`,
`isDue(rule, history, now)`) used by item placements and by events; the event modes above map onto it (every time =
every visit, once per page view, once for good = once, interval = every n …, plus the `if` condition). The save game
keeps one history shape for both (the item's server-side once-per-ref grant stays the authority for items). The creator
uses one "Repeats" field component for both.

## 6. `askText` – a text input in the VN

A question (per language, optionally spoken by a character), one input line (placeholder, max length e.g. 24, optional
default when skipped), stores into a variable; scroll frozen until answered. Plain text, trimmed, never markup; shown only
to that reader. Scope: `this toon` (a story variable) or `SCUTOONS` (= `reader.name`, guests on the device, members on
the account, synced, changeable on the account page).

## 7. Order

1. Language core (parser, evaluator, analyser, pipes, Intl plural) + tests; PHP validator port + shared fixtures.
2. Reader integration (namespaces resolver, text boxes / messages / choices / conditions), migration of the old syntax.
3. Event ids + repeat modes + save game; `askText`.
4. Creator: chips, builder, wavy underline, insert-variable list, sample readers; validation in the Problems panel.
5. The example story (a little one searching for their mama – everyone: "I'm not your mama, you have to go on", tips
   where next, remembers visits) + the flow graph on every level (slice → SCUTOONS), using all of the above.
