# DECISIONS — the owner's decisions: an index (the lines live in agents/decisions/, one file per topic)

**Need to know:** open only the file of the area you touch, and grep it by key – never read them all. RULES / AUDIT / ORCHESTRATION
cite `(date [key])`; every such key is in **decisions/law.md**: `grep -n '\[core-only\]' agents/decisions/law.md`. Across all files:
`grep -rn '\[key\]' agents/decisions/`. Split from one 55k-token file on 2026-11-03 (#380).

**Recording (every owner statement, the same day – RULES B6):** one line `- YYYY-MM-DD · area · [key] **the decision** · "the owner's words" · → doc`,
on top of its month in the file whose key list has the key (a rule-making decision: law.md + the rule in RULES, same commit). A new key: add it
to that file's "Keys here" line. Never edit an old line except to mark it superseded; a fully superseded line moves to
archive/DECISIONS-superseded.md (quotes kept). A built feature's history may move to archive/ once its topic doc describes it – owner's OK first.

| File | What | Lines | ≈ tokens | Keys |
|---|---|---|---|---|
| decisions/law.md | the law book's evidence – every key RULES / AUDIT / ORCHESTRATION cite | 94 | 9.4k | ai-license api-content audiovisual audit board board-move bold branch-names browser-is-ui carrier-bag codes config-small … |
| decisions/deck.md | deck, live, engine, modules, rigs, relay | 81 | 10.2k | bunny clock deck-access deck-os engine events live modules outputs perf platform rigs … |
| decisions/items.md | items, avatar, homes (cave), scores, campaigns and gifts | 98 | 7.6k | campaigns cave items scores |
| decisions/creator.md | the creator app, text boxes, the timeline | 72 | 5.9k | creator text-boxes timeline |
| decisions/reader.md | the reader app, PWA, FX, UI and design system | 95 | 5.9k | design fx pwa reader ui |
| decisions/map.md | the world map and places | 36 | 2.1k | map |
| decisions/story.md | storyboard, studio, WEBTOON, content, characters, release dates | 66 | 6.5k | characters content release story storyboard studio webtoon |
| decisions/server.md | data, assets, media, catalog, MCP, embassy, newsletter, deploy, dev, process | 115 | 6.5k | admin assets catalog data deploy embassy git legal mcp media newsletter process |
