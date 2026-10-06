# EXPRESSIONS – variables, logic and reader state in texts (owner 2026-10-06)

Owner decisions of 2026-10-06 (session with the coordinator). Research: scratchpad `expression-research.md` (summary
below). Branch `chapter-neutral`.

## State (2026-10-06)

All five steps of §7 are done – what is left is polish only:
- **Done**: the language (ink inline syntax: parser, evaluator, analyser, one function registry; PHP validator port with
  shared golden fixtures – step 1); the reader (every namespace of §2, text boxes / VN messages / choice labels /
  conditions, the clock-syntax migration – step 2); event ids, repeat modes on one recurrence model with the items,
  the save game's history, `askText`, expression conditions (step 3); the creator's expression field (chips, builder,
  wavy underlines, sample readers, the Problems panel – step 4); the example story (the mama story, agents/EXAMPLES.md)
  and the flow graph on every level (slice → group → chapter → season / toon → SCUTOONS, `@scutoons/content` flow.ts –
  it reads "once for good" events (✓), askText answers as set variables and `if.expr` conditions; reach.ts the same)
  – step 5.
- **Production**: the migrations `2026-10-06-clock-syntax`, `2026-10-06-repeat`, `2026-10-06-display-name` (CUTOVER
  10.33 / 10.34, HANDOVER "Open owner decisions").
- **Left (polish)**: a chip shows its code, not a summary (the summary is its tooltip).
- **One save game** (owner 2026-10-06, DATA.md "The save game"): story variables are SCUTOONS-wide – set in one toon,
  read in another; the builder offers every toon's names, unknown names / "never true" / dead things judge all toons, a
  look-alike name warns (`similarVariable`).
- Checks: `check:expr` (reader), `check:events` (repeat modes, askText, the account name), `check:creator-expr`
  (the field + builder), `check:creator-reach` (reach chips, Flow levels).

## 1. The language – ink's inline syntax, our small engine (owner: "use inkle, to keep at least some standard")

Research (no library fits: CEL too big, expr-eval / angular-expressions with sandbox CVEs, Jexl unmaintained, template
engines compile to JS, ICU / MF2 / Fluent have no logic) → our own parser + evaluator + static analyser in
`@scutoons/content` (~3 KB gz in the reader). The SYNTAX is ink's inline syntax (inkle, "Writing with ink": variable
printing, conditional text, sequences / cycles / shuffles / once-only, logic) – not ink's runtime/compiler (too big, ink
compiles whole stories; we evaluate single texts).

| ink | meaning |
|---|---|
| `{x}` | print a value |
| `{cond: a\|b}` | conditional text (else part optional) |
| `{a\|b\|c}` | sequence: the next one per visit, then stays on the last |
| `{&a\|b}` | cycle |
| `{~a\|b}` | shuffle (stable within a page view) |
| `{!a\|b}` | once-only: each once, then nothing |
| `and or not && \|\| ! == != < <= > >= + - * / % mod` | logic + arithmetic as in ink; strings `"…"` |

- Visit counts of sequences / cycles / once-only live in the reader's save game (the shared recurrence model, §5).
- Plurals the ink way: `{coins == 1: coin|coins}` – every language writes its own.
- Our extras are ink-style (external) functions: `clock(t)`, `upper(s)`, `lower(s)`, `has("cucumber")`,
  `count_of("cucumber")`, `read_chapter(2)`, `resolved(id)`, `count(id)`, `min`, `max`, `round`, `floor`,
  `random(a, b)` (stable per reader + text). Reader state as names (§2): `map.total`, `reader.name`; story variables bare (`coins`).
- **Functions are defined in code only (owner 2026-10-06):** ONE registry in `@scutoons/content` (name, params, result
  type, a short description de/en, an example) – the builder OFFERS them as a list to pick from (e.g. `clock`), and
  validation, the analyser and the PHP check read the same registry. Adding one = a few lines there. No author-defined
  functions.
- Safety: no generic property access (names resolved by a SCUTOONS resolver against fixed namespaces), only registered
  functions, limits (500 chars, depth 32, 64 nodes), runtime errors never break the reader, never eval.
- **Old syntax discarded** (owner): `{x:clock}` is not supported in code (no alias, no load-time conversion). Stored data
  is moved by a DB MIGRATION (SQL, run in phpMyAdmin like `2026-10-06-show-text-box`): `2026-10-06-clock-syntax.mysql.sql`
  rewrites `{name:clock}` → `{clock(name)}` in `entity_versions` / `content_versions` (MariaDB `REGEXP_REPLACE`,
  idempotent, SELECT first). Dev DB check 2026-10-06: 0 occurrences – it's for production's newer test content. Rule
  (owner): data changes = DB migrations, never code that converts old data on load.
- Story variables are bare names as in ink (`{coins}` keeps working); reader state is namespaced (`map.total`, …).
- Missing value: readers never see raw code – empty; the creator warns.
- Difference to real ink: a text using our names / functions doesn't run in inkle's Inky – the syntax rules are ink's.

## 2. What a text can read (namespaces)

| Namespace | Names |
|---|---|
| (bare) | the story variables – one save game for every toon (a name is the same variable in every toon), ink-style bare names: `coins` (VN `setVariable` / choice set+add) |
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
- Builder (writes ink syntax): pick a value (story variables, reader state, functions) → comparison / arithmetic →
  then / else texts (in the header's language), or a sequence / cycle / shuffle / once-only list → functions → live preview with sample readers ("new guest", "halfway", "everything collected").
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
to that reader. Scope: a story variable (stored `toon`, the default – since the one save game the same in every toon, so
"toon" only means "the story's"; the creator says "a story variable (the same in every toon)") or the reader's name
(`reader`: `reader.name`, the account's field – guests on the device, members on the account, changeable on the account
page; not part of the save game). The scope stays meaningful: a variable vs. the account's name.

## 7. Order

1. Language core (ink inline syntax: parser, evaluator, analyser, function registry) + tests; PHP validator port + shared fixtures.
2. Reader integration (namespaces resolver, text boxes / messages / choices / conditions) + the clock-syntax DB migration.
3. Event ids + repeat modes + save game; `askText`.
4. Creator: chips, builder, wavy underline, insert-variable list, sample readers; validation in the Problems panel.
5. The example story (a little one searching for their mama – everyone: "I'm not your mama, you have to go on", tips
   where next, remembers visits) + the flow graph on every level (slice → SCUTOONS), using all of the above.

## Step 1 – done (2026-10-06, branch chapter-neutral)

`shared/content/src/expr/` (exported from @scutoons/content: `export * from
'./expr/index.ts'`), PHP port `server/src/Domain/Content/Expr/`, golden fixtures `shared/content/test/fixtures/expr/`.

- **Parse** – `parseText(text)` → `{ nodes, problems, constructs, lists }`; `parseExpression(src)` → `{ expr, problems }`.
  Nodes `text | print | cond | list(kind, index, items) | bad`, expressions `num | str | bool | name(path) | call(name,
  args) | unary | binary`; every node has `start`/`end` (UTF-16 offsets, end exclusive). Ink's InnerLogic order: list
  mark `& ~ ! $` (also `~!` shuffle once, `~$` shuffle stopping) → list; expression + `:` → conditional; ≥ 2 `|` elements
  → sequence; else one expression → print. Ink's operator table (`&& || and or` 1 – one level, left to right;
  comparisons 2; `+` 4; `-` 5; `*` 6; `/` 7; `% mod` 8; prefix `- ! not`). Escapes: `\` keeps the next character.
- **Evaluate** – `evaluateText(text, resolver)` → `{ text, problems, visited }`, `evaluateExpression(src, resolver)` →
  `{ value, problems }`. Resolver: `name(path)`, `call(name, args)` (state functions + `ns.fn(…)`), `visit(listIndex)` →
  shows before this one, `seed`, `language`. The reader adds one visit to each `visited` list. Never throws: a construct
  that fails renders nothing + a problem (`missing-value` warning, `division-by-zero`, `not-a-number`, `compare-text`,
  `bad-arguments`). Missing values: print empty, false, equal only to missing, arithmetic stays missing. Ink output
  whitespace clean-up when the text has a construct. A text without `{ } \` comes back untouched.
- **Analyse** – `analyseText(text, { registry?, knownNames? })` / `analyseExpression` → `{ errors: [{ code, severity,
  start, end, args, suggestion?, message }], reads: { names, functions: [{ name, literalArgs }], sequences }, constructs }`;
  `exprMessage(problem)` (English; translate by `code` + `args`), `didYouMean`. Warnings: `unknown-variable` (with
  knownNames), `stray-close`, `old-clock-syntax` (`{x:clock}` parses as ink conditional – no alias), `or-in-list`
  (`{a || b}` is a list in ink), `not-in-list` (`{!x}` is once-only in ink).
- **Registry** (`registry.ts`, one file) – `EXPR_FUNCTIONS` (name, params + types, variadic, result, state, de/en, example),
  `EXPR_IMPLS` (pure ones), `EXPR_NAMESPACES` (§2 members, value / function, types, de/en). PHP `Registry` = same names
  and types, held equal through `fixtures/expr/registry.json`.
- **Limits** – 500 chars / depth 32 / 64 nodes per expression; per text 50 constructs, nesting 32, 10 000 chars when it
  has `{`, 100 problems; strings cut at 2 000. Hostile inputs (100 000 braces, quotes, parens …) finish in milliseconds.
- **Size** – reader part (evaluateText + evaluateExpression, rolldown minify + gzip -9): ~4.4 KB gz; everything incl.
  analyser + registry texts ~8.7 KB gz (creator only).
- **Differences to ink** (deliberate): `/` divides exactly (`7 / 2` = 3.5; ink: integer division – use `floor()`);
  a `|` / `}` in plain text is literal (ink: error; `}` warns); lowercase registry functions instead of ink's
  `RANDOM/MIN/MAX/FLOOR…`; no ink lists (`? has hasnt ^`), `++`/`--`, diverts, read counts, multi-line blocks,
  interpolation inside string literals; missing names are empty + a warning (ink: compile error); `and`/`or`
  short-circuit; shuffles / `random()` are fixed per seed; `clock()` keeps today's `5:30` form (formatClock).

## Step 2 – done (2026-10-06, branch chapter-neutral)

- **Resolver** (viewer `lib/expressions/`): `reader.ts` = pure answers over getters (`readerName(sources, path)`,
  `readerCall(sources, name, args)`; tests `viewer/test/expressions.test.ts`), `resolver.svelte.ts` = the wiring
  (`createReaderExpressions` in the toon layout, context + `getCurrentReaderExpressions()` for the VN runner). Every
  registry namespace: bare names = the toon's story variables; `reader.*` (signedIn, member = device code or account
  access, memberDays / since from the account's created_at – guests 0 / none, gifts = inventory entries from gifts,
  language, daysSinceStart from the game state's created_at; `reader.name` = none until askText); `read.*` + `read_chapter`
  (complete checkpoints; `position(n)` = 0..1 of the current read-through; `finished` = every readable chapter except the
  prologue); `inventory.*` / `has` / `count_of` (entries; `edition` = the entry's edition number); `avatar.*` (the shown
  body, worn items, unlocked characters); `stats.<id>` (decayed levels), `effects.*`; `map.*` (unlocked / seen counted
  over the world map's places, `total` = its places, `here` = the hero position's place name – the toon's map data, else
  the whole world map loaded lazily); `now.*` (local time; night = 22–6). `resolved()` / `count()` answer from the save
  game's history (step 3).
  Never email, codes, account / entry ids. Reactive: getters read the contexts' state, text boxes re-render.
- **History** (one store for step 3 too): the game state's `history` (`@scutoons/content` history.ts, DATA.md "Reading
  progress"): `key → { n, first, last }`; text keys `text:<toon>/<chapter>/box/<box id>/<line>#<list>` and
  `text:<toon>/<chapter>/vn/<group label>/<script|on/<trigger hash>|item/<id>>/<event index>#<list>`. A text box counts
  once per page view (path + `?label=`; it shows the count from before the view while it re-renders), a VN message /
  choice label once per showing. Seed = the game state's created_at + the text identity. Synced for members with the
  state (last write wins); ≤ 1000 entries.
- **Rendering**: VN messages + choice labels through `evaluateText` (runner.ts; without the toon's expressions only story
  variables); text boxes in `@scutoons/reader` (`expressions` prop on Slice / Dialogs / Dialog: `resolver(text, lang)` +
  `shown(text, visited)`; the creator's previews pass none → texts as written, step 4 adds sample readers). `number()`
  etc. use the text's language. `interpolateText` and `{x:clock}` are gone (`formatClock` stays).
- **Validation**: `validateNeutral` analyses every text box line, VN message and choice label per language
  (`Problem.code = expr:<code>`): syntax errors = errors, unknown names / functions / arguments = warnings (story
  variables from `ctx.facts` when the creator has the toon's documents), `{x:clock}` = an error naming `{clock(x)}` and the
  migration. Server: `Expr\TextCheck` in `ContentSchemas::problemsOf` / `guard` – a neutral chapter / scene save with a
  syntax error or `{x:clock}` → 422 `invalid_content` ("… has a text that can't be shown: <pointer>: <message> – at
  <from>–<to> (expr:<code>)"); doubts don't block.
- **Migration** `server/migrations/mysql/2026-10-06-clock-syntax.mysql.sql` (CUTOVER 10.33): dev DB = production dump
  had 0 `{x:clock}` (449 versions); REGEXP_REPLACE escaping checked on 10.11.14 (default + the API's strict sql_mode),
  PHPUnit `testTheClockSyntaxMigration`.
- **Check**: `npm run check:expr` (headless Chrome, sandbox chapter 9 draft: messages, choice label, live text box,
  a sequence over four reloads, the history; draft rows + the test user's sandbox progress put back).
- Event conditions as expressions (`if.expr`): step 3.

## Step 3 – done (2026-10-06, branch chapter-neutral)

- **One recurrence model** – `@scutoons/content` recurrence.ts (tests `recurrence.test.ts`, the respawn interval / period
  tests moved there): `RecurrenceInterval` (`"visit"` | `{ minutes | hours | days: n }`), `intervalMinutes`, `periodOf`
  (fixed UTC buckets since 1970 – the server's respawn check uses the same), and the events' rule `Repeat = { mode:
  every | view | once | interval, every? }` with `repeatOf` / `repeatProblem` / `isDue(rule, entry, now, pageView)`
  (`every` = no `repeat` = the default; `view` = the entry's `last` is before the page view's start; `once` = never
  after the first; `interval` = another bucket than the last run's). "When a condition holds" = every time + `if`.
  respawn.ts keeps only the item rules (catalog range, placement override); `respawnAsRepeat` / `repeatAsRespawn` put a
  pickup into the same words. Items keep the server's once-per-ref grant as the authority and their `picked:` game
  variables (readers' saved states hold them; a guest's localStorage can't be migrated) – the period logic is shared.
- **Stored format** – `VNEventTrigger.repeat` and `SliceItem.repeat` (tap events) replace the boolean `once` (no alias;
  validation calls `once` an error naming the migration). Analysis on the dev DB (= production dump): 221 slice events
  (166 `once:true`, 0 `once:false`, 55 without – which meant true), 4 tap-event items without `once`, no other `"once":`
  anywhere. **Migration** `server/migrations/mysql/2026-10-06-repeat.mysql.sql` (CUTOVER 10.34): MariaDB PCRE2 with a
  DEFINE block that walks JSON exactly (strings with escapes, objects, arrays – only top-level keys count), so VN slice
  events (top-level `type` + `trigger`) are told apart from audio samples' triggers (no top-level `type`); missing once →
  `{"mode":"view"}`, `once:true` → `{"mode":"view"}`, `once:false` → `{"mode":"every"}` (written out, so a second run
  sees nothing to do). Checked: the result equals a JSON-walking reference transform for all 457 versions (81 changed);
  idempotent; PHPUnit `EntityStoreTest::testTheRepeatMigration` (edge cases, strict sql_mode). Dev DB migrated.
- **Event ids** (decision): generated ids can't reasonably come from SQL (one per event, unique per chapter incl. scenes,
  in nested JSON), so the CREATOR assigns them (creator `editor/event-ids.ts`: `e` + 7 random letters / digits; random,
  so a chapter and its scenes – separate documents – don't collide; a copy in the same document gets a fresh one):
  on load in memory – the document stays saved (no autosave, no "unsaved"), a warning bar names the documents and the
  count ("… events have no hidden id yet – the next save stores them") – and with every structural edit (new, pasted,
  imported events). Validation: one warning per document while events lack ids, an error for duplicates in the chapter,
  an error for `once` / `interval` rules on events without ids (the save game needs them). The reader keeps id-less
  events' "once per page view" in memory (as before). Group-script events get ids too (the flow graph, the builder);
  they are not recorded (no repeat in scripts – an `if` covers it; keeps the 1000-entry history for what repeats).
- **Reader** – viewer `visual-novel/repeat.ts` (`isRunDue` / `recordRun` over the game state's history `event:<id>`;
  events that run as one – a trigger's group, an item's taps – share the newest entry), `visual-novel/page-view.ts`
  (path + `?label=`, renewed after every navigation – the items' "every visit" uses it too). `resolved(id)` /
  `count(id)` answer from the history; `reader.name` from `reader-name.svelte.ts`. Trigger texts' identity is now the
  first event's id (`on/<id>`, stable across rule edits) instead of the trigger JSON's hash.
- **askText** (§6) – `VNAskTextEvent { text, character?, persona?, variable?, placeholder?, maxLength? (24, ≤ 200),
  default?, scope?: toon | reader }`; texts per language (neutral `EVENT_TEXT_FIELDS` – TS + PHP NeutralDoc, golden
  fixture); the question goes through the expression engine, placeholder / default are plain. The message box shows
  one input line + a round ✓ (Enter) + Skip (→ default); the page doesn't scroll meanwhile (`vn.state.question` in
  `frozen`). Answers: control characters → spaces, trimmed, cut; toon scope → the story variable, reader scope → the
  reader's name (≤ 32): members `users.display_name` (`PUT /me/ { display_name }`, migration
  2026-10-06-display-name, PHPUnit `DisplayNameTest`), guests localStorage `scutoons_reader_name`; the account page has
  the one field ("Your name"). Scripts wait (≤ 3 s) until the session is known, so a member's name / access are there
  from the first text.
- **Conditions as expressions** (§3) – `if: { expr }` besides `{ variable, op, value }` (`VNEventCondition`);
  `testCondition(cond, get, resolver)` evaluates it with the reader's resolver (missing / error = false); validation:
  syntax errors = errors, doubts = warnings (story variables from the facts); the PHP save guard blocks syntax errors in
  `if.expr` (and in askText questions). Creator: "Only if" → "Condition as Simple / Expression" (a text field with the
  analyser's problems; switching converts the simple test, `coins >= 3`).
- **Creator** – one `fields/repeats-field.svelte` for slice triggers (all events sharing the trigger change together),
  item tap events and pickups (respawn in the same words); the askText form (`events/event-ask-text.svelte`).
- **Checks** – `npm run check:events` (headless Chrome, sandbox chapter 9 draft: once for good / per page view / every
  10 min / every time over three page views with the page's clock 61 min ahead, item taps, a pickup's respawn and the
  server's refusal in its real period, askText both scopes + Skip, an expression condition, the account field, a
  guest's name; draft, progress, name, inventory and grant refs put back). Screenshots `step3-*`.
- **Flow graph / reachability** (`@scutoons/content` reach.ts / flow.ts, tests `flow-levels.test.ts` "step 3"): "once
  for good" runs are marked (✓, `ChapterReach.once`, the flow's `repeat`); an askText (scope toon) sets its variable
  (`askVariable` – facts, dead things, the flow's "asks"); `if.expr` conditions count as conditional branches and their
  names as reads (dead things, also in the PHP flow index).

## Step 4 – done (2026-10-06, branch chapter-neutral)

Creator: editing expressions (owner's design, §4). DESIGN.md "Expressions in text fields" has the UI rules.
- **One field** – `creator/src/lib/components/fields/expr-text.svelte` (`LocaleText expressions`): text box text, VN
  message, choice label, askText question; the condition's "Expression" mode (`expression`: the whole value is code).
  askText's placeholder / default stay plain (the reader shows them as written, step 3). Stored value = plain text.
- **Component choice**: an overlay, not contenteditable – a real `<textarea>` with transparent text over a mirror that
  draws the same text (chips, wavy underlines) and a transparent hit layer on top (chip / range clicks + `title`
  tooltips). Caret, selection, IME, undo / redo, paste (plain; single-line fields turn line breaks into spaces), labels
  and screen readers stay native; problems are also in an `aria-describedby` text. The mirror sizes the field (grows with
  its text; single-line = a one-row textarea at `--control-height`). Cost: a chip shows its code in the field's font
  (the tools' mono), not a summary – the summary is the chip's tooltip. The field's own edits (a chip removed, the
  builder's code) go through `execCommand('insertText')` so Ctrl+Z / Ctrl+Y include them.
- **Chips** (`editor/expr-edit.ts`, pure, tests `creator/test/expr-edit.test.ts`): top-level closed `{…}` (print /
  conditional / list, and closed ones that don't parse – a chip with its error). Click (250 ms, so a double-click wins)
  = builder; double-click / long-press (500 ms, touch / pen) = edit in place until the caret leaves it, Escape, blur or
  a typed `}`; Backspace / Delete next to one removes it whole; ←/→ step over it; a caret put inside snaps to the edge
  it came from; selections grow to whole chips.
- **Problems**: the analyser with the toon's story variables (`knownNames`) and validation's levels (`exprLevel`,
  @scutoons/content validate.ts – doubts warn); a text box's lines (string[]) analysed one by one like validation.
  Settled 300 ms after the last keystroke (chips follow every keystroke). Messages: ONE map `creator.expr_<code>` in
  `creator/messages/{de,en}.json` + `editor/expr-messages.ts` (`exprText` / `exprTextAt`: args, "did you mean", place;
  argument counts and types worded per language) – tooltips, builder and the Problems panel use it.
- **Builder** – `fields/expr-builder.svelte` (in-page, portalled into `.sc-tool`, placed under the caret / chip or
  above, inside the window; Escape closes, Enter applies, Enter on a highlighted search hit picks it): Value · If … else
  · Per visit · Code; value search over story variables (`exprContext.variables` – the chapter editor's toon facts),
  `EXPR_NAMESPACES` (grouped, de/en descriptions; functions with placeholder arguments) and `EXPR_FUNCTIONS` (description
  + example; picked after a value it wraps it); operator (== != < <= > >= + - * / %) + second value (a datalist of
  names); then / else; list kind (sequence / cycle / shuffle / once only) + items. `modelOf` reads a chip back (a value
  with its top operator split off, a conditional, a list), anything else opens as Code; `codeOf` writes ink syntax
  (`{a: then|else}`, `{&a|b}`, a one-item sequence `{$a}`). Preview: evaluateText / evaluateExpression with a sample
  reader + the code (muted) + its problems.
- **Sample readers** – `editor/sample-readers.ts`: new guest / halfway / everything collected answer every registry
  name and state function (reader.*, read.*, inventory.*, avatar.*, stats.*, effects.*, map.*, has / count_of /
  read_chapter / resolved / count; `now.*` = the creator's clock); story variables unset / 3 / 10 (the creator doesn't
  know their kind); lists show their first / second / last item. One choice (`exprContext.sample`) for the builder's
  preview and the canvas.
- **Insert-variable shortcut** (decided): no `{}` button – `{` + Enter is shown at the caret when it matters, Ctrl+Enter
  opens the builder anywhere, and the builder's value search IS the list of all names.
- **Canvas** – text boxes evaluated for the toolbar's sample reader (`@scutoons/reader` `expressions` prop); the select
  (as written / new guest – the default / halfway / everything collected) only on slices whose text boxes hold `{…}`.
- **Problems panel** – `Problem.expr` (validate.ts: start / end, a text box's `line`, args, suggestion) → the message in
  the creator's language; a row sets `exprContext.focus` – the field at that place (`fieldKey(file, path, locale)`)
  scrolls in, focuses and selects the range (a VN event's form opens for it).
- **Save** – the server refuses a syntax error (422, step 2/3 guards); the creator no longer replaces the editor with a
  refused save's message (`doc.error` is for load errors – the session's bar says why), so the text can be fixed.
- **Check** – `npm run check:creator-expr` (headless Chrome 1280 wide, own profile, a sandbox chapter 9 draft with a
  test group; every version the run adds is deleted again, checked): chip, `{` + Enter → builder (if / else, preview
  new guest "arm" / halfway "reich", Apply → `{coins >= 3: reich|arm}`, undo / redo), `{map.totl}` wavy + tooltip with
  `map.total`, Backspace / arrows, double-click edit, click → builder parsed back, the canvas' sample reader, Problems
  panel jumps (text box + condition), Enter in a condition, save 422 with a syntax error (the field stays) and 200 with a
  warning. `EXPR_SHOTS=<dir>` → screenshots `builder-*.png`.
