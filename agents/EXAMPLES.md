# EXAMPLES – example content in the dev DB (owner 2026-10-06)

## The mama story (LANE A, branch `chapter-neutral`, 2026-10-06)

Dev DB `scutoons` only, saved as DRAFTS through the API (`PUT /api/entities/<key>`, message `example: mama story`,
test user `agent@scutoons.test`). Never published. Before the first write: `SELECT MAX(id) FROM entity_versions` = **539**
(`content_versions` empty, unused).

### Rollback (after the deploy)

```sql
-- the SELECT first: every version the example wrote
SELECT id, entity_key, status, message FROM entity_versions WHERE id > 539 AND message = 'example: mama story';
DELETE FROM entity_versions WHERE id > 539 AND message = 'example: mama story';
```

The `message` filter keeps anything else saved after id 539 (the owner's own edits). Note: `npm run check:mcp` saves on
`scene:catpope:2:scene-14` and restores with `restore_version` – a NEW draft (`mcp-check restore`) that copies the newest
one (the mama story's strip draft 562); delete a run's `mcp-check (agent)` / `mcp-check restore` rows after id 562 too
(`… WHERE entity_key = 'scene:catpope:2:scene-14' AND id > 562 AND message LIKE 'mcp-check%'`), else the strip stays
after the rollback (the 2026-10-06 run's 666 / 667 are deleted). New keys the story created
(seasons / chapters of toptoy and yellowmouse, catpope chapter drafts) disappear with their only versions; existing keys
fall back to their previous newest version.

Saved versions (ids 540–553 + 561–564 + 605 + 631–633, all `draft`; the chapters were saved again after the stage rework): `chapter:catpope:2` (groups appended), `chapter:catpope:3` (groups
appended), `season:toptoy:1` + `chapter:toptoy:1`, `season:yellowmouse:1` + `chapter:yellowmouse:1` (new),
`cast:catpope` / `cast:toptoy` / `cast:yellowmouse` (cast additions so every character may be on stage there).

### The story

A little kitten (Cat Pope · Cat) wakes up alone and looks for its mama. Everyone it meets says "I'm not your mama,
you have to go on", gives a tip and remembers how often the kitten was there. All registry characters + every Cat Pope
persona meet it: boy, crows, Bill the Frog, passerby, yellowbubu, Cat Pope (regular), mountain tiger, biker (helmet),
Top Toy, Yellow Mouse – and the mama is Cat Pope · Human (the cat was her own split identity all along).
Texts de + en (no toon has ko).

The stage (owner 2026-10-06, DESIGN.md "VN layout"): LEFT (`avatar`) = the little one, always there, and whoever teams
up with it – the boy after the first meeting, the crows at the harbour, Bubu at the market, the tiger for a stretch of the
forest; they stay on the left and comment the next encounters (3 avatars at the pier, the whole team at the reunion).
RIGHT (`portrait`) = the one met (two at once on the crowded market: the speaker comes to the front). CENTRE
(`full`) = the big moments: the boy turns to the reader ("Bist DU vielleicht ihre Mama?", first visit only), the
Pope's entrance, Top Toy's "Hey, DU! Ja, du am Handy", Mama at the reunion. Moods and looks change through the refs
(crows laughing → thinking → happy; the biker and Yellow Mouse take the helmet off: look helmet → default).

### Story map

| where | group (label) | what happens | leads to |
|---|---|---|---|
| catpope 2 | (owner's groups 0–4 unchanged) | – | the last one's follow-up bar → `mama-start` |
| catpope 2 | `mama-start` (continues) | the kitten (left) wakes; "Mama?"; text box "WO BIST DU, MAMA?!" (`showTextBox`, shiver, waits for a tap); `searching` set (never read – a dead variable); a **slice event** (tap on slice 3) diverts into the group's script at `miau` (counts `shouts`) | chain on |
| catpope 2 | `mama-boy` (continues) | the boy: not your mama, ask the crows; a placed **cucumber** (item tap → message + `found_cucumber`) | chain on |
| catpope 2 | `mama-crossroads` | the hub: `visits_crossroads` +1, "{visits_harbour} times at the harbour", "not at the market yet" (`looked_market unset`), **never true**: `rocket_ready == yes` (set nowhere); text box pulse; label `ask` → question: Hafen / Markt / Wald (ch 3) / im Kreis / "Zu Mama!" (only `tips >= 3`); `circle`: **running in circles** (`circles` +1, "Noch eine Runde!" diverts back, "Stopp" diverts to `ask`; at 5 laps a conditional `jumpTo` event back to `mama-start`) | jumps: `mama-harbour`, `mama-market`, ch 3 `mama-forest`, `mama-reunion` |
| catpope 2 | `mama-harbour` (continues) | crows (`visits_harbour`, "{visits_harbour}. time", ≥ 3: "you'll wear your paws out"; "been to the market?" by `looked_market`); `looked_harbour` | chain on |
| catpope 2 | `mama-pier` | Bill the Frog: "you asked {visits_harbour} times"; tip (hat): `tips` +1 once (`tip_hat unset`) | **end** → `mama-crossroads` (follow-up bar with countdown) |
| catpope 2 | `mama-market` (continues) | passerby (`visits_market`); yellowbubu (by `looked_harbour`); a question "play with me?" (`played_bubu`) | chain on |
| catpope 2 | `mama-pope` | Cat Pope: "father to many – not your mama"; text box "WO IST SIE DANN?!" (`stay: false`); tip `tip_crown` | **end auto** → `mama-crossroads` |
| catpope 2 | `mama-attic` | **deliberately unreachable** (after an authored end, nothing jumps to its label) | – |
| catpope 2 | `mama-reunion` | Mama (Cat Pope · Human): "There you are!"; text box "MAMA!!!"; "even if you ran {circles} laps" (`circles >= 3`), the cucumber (`found_cucumber`); purr | **end** → chapter 0 (the prologue "Roar Little Kitty") |
| catpope 3 | `mama-forest` (continues) | after the owner's group `chapter-3`: mountain tiger (`visits_forest`), tip `tip_road` | chain on |
| catpope 3 | `mama-biker` | the biker (helmet): a question | jumps: toptoy ch 1 `mama-hypercity`, yellowmouse ch 1 `mama-boss`, ch 2 `mama-crossroads`, ch 1 (start) |
| toptoy 1 | `mama-hypercity` | Top Toy (`visits_hypercity` – toptoy's own variable) | jumps: yellowmouse ch 1 `mama-boss`, catpope ch 2 `mama-pope` ("Heim zum Katzenpapst") |
| yellowmouse 1 | `mama-boss` | Yellow Mouse (helmet): "she lives behind the cathedral" | **end** → catpope ch 2 `mama-reunion` |

Deliberate warnings (validation, the creator's chips, the Flow panel): `mama-attic` unreachable; `rocket_ready` never
true. The circle loop is NOT an "endless loop" warning – it has a way out (a choice), which is the point.

Limits found (today's model): variables are per toon (toptoy's `visits_hypercity` is not catpope's – after a jump into
another toon that toon's variables apply; catpope's `tips` don't travel to the reunion via Yellow Mouse); no catpope
season 2 exists, so no season jump; the viewer has no `/season/<n>` route either. (Fixed 2026-10-06: a jump into
another toon used to land on that toon's page only – now `JumpToTarget.toon` lands at a chapter + label there, below.)

### Jumps into the other toons at a label (2026-10-06, versions 631–633, same message)

The five `type: 'toon'` targets became chapter jumps with `toon` (DATA.md "Jumps into another toon at a label"):
`chapter:catpope:3` (631) – the biker's choices → toptoy 1 `mama-hypercity` / yellowmouse 1 `mama-boss`;
`chapter:toptoy:1` (632) – Top Toy's choices → yellowmouse 1 `mama-boss` / catpope 2 `mama-pope`;
`chapter:yellowmouse:1` (633) – Yellow Mouse's end → catpope 2 `mama-reunion`. Versions before: 550 / 551 / 552. The
flow index lists the five ways with their `toon`; TS `deadThings` and the PHP flow index agree on the dev DB for every
toon. The rollback above removes them too.

### Step 3 in the story (2026-10-06, version 605 of `chapter:catpope:2`, same message)

The repeat migration (`2026-10-06-repeat`) rewrote the story's drafts in place (no new versions): the slice tap in
`mama-start` and the cucumber's taps are `repeat: { mode: 'view' }` (once per page view – as before). Added as a new
draft:

- `mama-harbour`, last slice: the crows' welcome "Willkommen am Hafen! Das sagen wir nur einmal." – a scroll trigger
  with `repeat: { mode: 'once' }` (once for good; event id `emamacrw`, the save game's `event:emamacrw`);
- `mama-boy`: the boy asks the READER's name (`askText`, scope `reader`, placeholder "Dein Name") only while there is
  none (`if: { expr: '!reader.name' }`);
- `mama-reunion`: Mama "Und danke dir, {reader.name}, dass du mein Kleines begleitet hast." (`if: { expr: 'reader.name' }`).

Checked in the viewer (phone, draft preview, the test user; progress + name put back): the boy asks once, the answer
becomes `users.display_name`, the welcome shows on the first harbour visit and not on the second ("Zum 2. Mal"), the
cucumber's message once per page view, the reunion names the reader. Validation: 0 errors (warnings: the owner's
groups, missing en texts, events without ids until the next creator save). Screenshots `flow2-mama-*.png`. The rollback
above removes it too (`message = 'example: mama story'`).

Read in the viewer (phone, draft preview): screenshots `mama-*.png` in the session scratchpad. Found on the way: the
draft preview loaded the PUBLISHED cast (new characters of a draft cast had no name / image) – fixed in
`viewer/src/lib/api.ts` (`fetchCharacters` through the preview); a text box revealed with effect `pulse` logs
`Cannot read properties of null (reading 'onComplete')` (shared/reader dialog.svelte → effect-pulse.svelte) – fixed by the other lane in 30caaed0.

## Horizontal strips in catpope chapter 2 (owner 2026-10-06, for testing the scrolling)

Drafts (same message `example: mama story`, so the rollback above removes them too): versions 561 (`chapter:catpope:2`,
group `scene-12`), 562 (`scene:catpope:2:scene-14`, label `scene-13`), 563 (`scene:catpope:2:scene-17`, label `scene-14`),
564 (`scene:catpope:2:scene-18`, label `scene-15`). Each is now `renderer: { type: 'horizontal', params: { fullWidth: true } }`
with `continues: true`, so the chain is scene-12 → 13 → 14 → 15 → scene-17-24 (strip, strip, strip, strip, then vertical).
Old values (the versions before: 553, 536, 451, 452):

| group | renderer before | continues before |
|---|---|---|
| `scene-12` (inline) | `{ type: 'horizontal', params: { loop: true, autoPlay: { enabled: false }, fullWidth: true } }` | – |
| `scene-13` (scene-14 doc) | `{ type: 'horizontal', params: { freeMode: true, loop: true } }` | – |
| `scene-14` (scene-17 doc) | `{ type: 'vertical' }` | – |
| `scene-15` (scene-18 doc) | `{ type: 'vertical' }` | – |

Checked: the library's draft answer (`/library/toons/catpope/chapters/2?draft=1`) has the four strips with `continues: true`
and `scene-17-24` vertical; read at phone size (375 × 812, the wheel in 120 px steps): each strip pins at the top (pin top 0),
moves sideways 1:1 to its end (travel 2824 / 5109 / 5109 / 5566 px), releases, the next one arrives and pins – offsets only
grow, never two strips moving at once, no scroll jumps, then the vertical group. (One run reset to the top once: a Vite
reload of the viewer while the other lane edited viewer files – not reproducible in the next two runs.)
