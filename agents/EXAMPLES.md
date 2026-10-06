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

The `message` filter keeps anything else saved after id 539 (the owner's own edits). New keys the story created
(seasons / chapters of toptoy and yellowmouse, catpope chapter drafts) disappear with their only versions; existing keys
fall back to their previous newest version.

Saved versions (ids 540–548, all `draft`): `chapter:catpope:2` (groups appended), `chapter:catpope:3` (groups
appended), `season:toptoy:1` + `chapter:toptoy:1`, `season:yellowmouse:1` + `chapter:yellowmouse:1` (new),
`cast:catpope` / `cast:toptoy` / `cast:yellowmouse` (cast additions so every character may be on stage there).

### The story

A little kitten (Cat Pope · Cat) wakes up alone and looks for its mama. Everyone it meets says "I'm not your mama,
you have to go on", gives a tip and remembers how often the kitten was there. All registry characters + every Cat Pope
persona meet it: boy, crows, Bill the Frog, passerby, yellowbubu, Cat Pope (regular), mountain tiger, biker (helmet),
Top Toy, Yellow Mouse – and the mama is Cat Pope · Human (the cat was her own split identity all along).
Texts de + en (no toon has ko).

### Story map

| where | group (label) | what happens | leads to |
|---|---|---|---|
| catpope 2 | (owner's groups 0–4 unchanged) | – | the last one's follow-up bar → `mama-start` |
| catpope 2 | `mama-start` (continues) | the kitten wakes; "Mama?"; text box "WO BIST DU, MAMA?!" (`showTextBox`, shiver, waits for a tap); `searching` set (never read – a dead variable); a **slice event** (tap on slice 3) diverts into the group's script at `miau` (counts `shouts`) | chain on |
| catpope 2 | `mama-boy` (continues) | the boy: not your mama, ask the crows; a placed **cucumber** (item tap → message + `found_cucumber`) | chain on |
| catpope 2 | `mama-crossroads` | the hub: `visits_crossroads` +1, "{visits_harbour} times at the harbour", "not at the market yet" (`looked_market unset`), **never true**: `rocket_ready == yes` (set nowhere); text box pulse; label `ask` → question: Hafen / Markt / Wald (ch 3) / im Kreis / "Zu Mama!" (only `tips >= 3`); `circle`: **running in circles** (`circles` +1, "Noch eine Runde!" diverts back, "Stopp" diverts to `ask`; at 5 laps a conditional `jumpTo` event back to `mama-start`) | jumps: `mama-harbour`, `mama-market`, ch 3 `mama-forest`, `mama-reunion` |
| catpope 2 | `mama-harbour` (continues) | crows (`visits_harbour`, "{visits_harbour}. time", ≥ 3: "you'll wear your paws out"; "been to the market?" by `looked_market`); `looked_harbour` | chain on |
| catpope 2 | `mama-pier` | Bill the Frog: "you asked {visits_harbour} times"; tip (hat): `tips` +1 once (`tip_hat unset`) | **end** → `mama-crossroads` (follow-up bar with countdown) |
| catpope 2 | `mama-market` (continues) | passerby (`visits_market`); yellowbubu (by `looked_harbour`); a question "play with me?" (`played_bubu`) | chain on |
| catpope 2 | `mama-pope` | Cat Pope: "father to many – not your mama"; text box "WO IST SIE DANN?!" (`stay: false`); tip `tip_crown` | **end auto** → `mama-crossroads` |
| catpope 2 | `mama-attic` | **deliberately unreachable** (after an authored end, nothing jumps to its label) | – |
| catpope 2 | `mama-reunion` | Mama (Cat Pope · Human): "There you are!"; text box "MAMA!!!"; "even if you ran {circles} laps" (`circles >= 3`), the cucumber (`found_cucumber`); purr | **end** → chapter 0 (the prologue "Roar Little Kitty") |
| catpope 3 | `mama-forest` (continues) | after the owner's group `chapter-3`: mountain tiger (`visits_forest`), tip `tip_road` | chain on |
| catpope 3 | `mama-biker` | the biker (helmet): a question | jumps: toon `toptoy`, toon `yellowmouse`, ch 2 `mama-crossroads`, ch 1 (start) |
| toptoy 1 | `mama-hypercity` | Top Toy (`visits_hypercity` – toptoy's own variable) | jumps: toon `yellowmouse`, toon `catpope` |
| yellowmouse 1 | `mama-boss` | Yellow Mouse (helmet): "she lives behind the cathedral" | **end** → toon `catpope` |

Deliberate warnings (validation, the creator's chips, the Flow panel): `mama-attic` unreachable; `rocket_ready` never
true. The circle loop is NOT an "endless loop" warning – it has a way out (a choice), which is the point.

Limits found (today's model): a jump to another toon lands on that toon's page (`/toptoy`) – a label / chapter of
another toon can't be targeted; variables are per toon (toptoy's `visits_hypercity` is not catpope's); no catpope
season 2 exists, so no season jump; the viewer has no `/season/<n>` route either.
