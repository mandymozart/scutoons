# DOCS-REFACTOR — approved by the owner 2026-10-12 ("The recommendation works"; Ask clean-up + newsletter go-live are done), Phase 2 in progress

Owner 2026-10-12: "in agents/ werden die ganzen Markdown-Dokumente langsam zu einer Art Ticketsystem … die
Entscheidungen und die Regeln für die Agents straffer ziehen, und Issues und Feature Requests separat halten, und das
nicht zu sehr ausrasten lassen."

## 1. What is there today (analysis 2026-10-12)

agents/ = 30 files, 14 400 lines, 1.4 MB. Roughly by content: history / logs ~35 %, how-it-is ~30 %, concepts /
research ~15 %, open work ~10 %, decisions ~7 %, rules ~3 %. **The rules are scattered in about 10 places.**

| File | Lines | What it has become | Proposal |
|---|---|---|---|
| HANDOVER | 139 | state + rules + ~10 stale "Production: run X.sql" todos (header still "2026-10-02/03 session") | state only, ≤ 60 lines |
| BACKLOG | 1165 | 315 lines of run logs at the top · 350 Open (11 of them already [x]) · 490 Done | → ISSUES + FEATURES; logs/Done → archive |
| MEMORY | 1833 | build log 09-28…10-04, nothing newer; SQLite, `cdn/`, qr repo – mostly stale; ~30 useful gotchas | gotchas → GOTCHAS; rest → archive |
| PLAN | 422 | Chapter-2 / `anger` plan (done) with **11 still-valid `RULE:` sections** hidden in it | rules → RULES; rest → archive |
| CUTOVER | 468 | the cutover runbook, finished 2026-10-05; 17 `[ ]` that will never be ticked | archive |
| MERGE-media-quality | 64 | merge note, merged long ago | 3 open questions → FEATURES; archive |
| NEUTRAL | 469 | the chapter-neutral rewrite: ~50 % log, model + decisions | model → DATA; archive |
| COMPONENTS | 373 | "proposal, not decided" – all done | 4.2 rules → DESIGN; archive |
| EXAMPLES | 136 | one lane's dev-DB write log | story map → DATA; archive |
| UI-CONCEPTS | 495 | "nothing built yet" – E.1–E.10 all built | built parts → DESIGN/DOCKING; archive |
| DESIGN, DOCKING, DATA, ITEMS, MAP, PLATFORM, ASSETS, CATALOG, EXPRESSIONS, MCP, EMBASSY, NEWSLETTER | 6 600 | topic docs, but each carries a build log, an open list and stale branch/phpMyAdmin text | **keep names** (code cites them ~500×), trim to "how it is" |
| STORYBOARD, FX, LIVE, ACCOUNT-RESTART, CATALOG-IN-CREATOR, AUTOMATION, TREATMENTS, PERFORMANCE | 2 000 | concepts / research / parked plans | → research/ with a status line |

## 2. Contradictions and stale statements found (the main ones)

1. **Migrations by hand.** About 25 places still have the owner running SQL in phpMyAdmin or "Production: run X.sql":
   HANDOVER l.97–131, BACKLOG 10.35/10.38/10.61, PLAN, MEMORY, PLATFORM, ASSETS, CATALOG, EXPRESSIONS l.56, MAP l.863,
   MCP l.212, EMBASSY l.210, NEWSLETTER l.79, server/docs/DATABASE.md l.166+, server/docs/DEPLOY.md l.44.
   `migration_status` (2026-10-12) shows **0 pending**: every one of those todos is done.
2. **Worktrees / own DBs.** CLAUDE.md ("agents on their own branches", `SCUTOONS_DB_NAME`), HANDOVER l.33 + l.57, the
   user memory `local-dev-mariadb`, PLAN l.289/420, ITEMS l.1123 (packs workflow), ASSETS l.3 and CUTOVER l.13 all
   contradict "one checkout, one DB, no worktree unless the owner asks".
3. **Branch era.** "branch `platform` / `chapter-neutral` / `anger` … merges after the release" in the headers of
   PLATFORM, DATA, MAP, NEUTRAL, EXPRESSIONS, EMBASSY, DOCKING, COMPONENTS and CATALOG.
4. **Ask leftovers.** These still read as if Ask were current: UI-CONCEPTS l.83/94/193/383/409, BACKLOG l.161–302,
   AUTOMATION l.51–57 (in-app assistant). The HANDOVER Ask clean-up todos (l.101–106) have no done status.
5. **Header says the opposite of the content.** UI-CONCEPTS "nothing built", COMPONENTS "not decided", FX "not
   started", STORYBOARD "nothing built", CATALOG "moves from config files", LIVE "building starts 2026-11-02" (the plan
   is parked until Feb 2027).
6. **Facts that disagree.**
   - DESIGN: tools at 70 % (l.39) vs l.987/1051/1134; the creator's "13 px base" (l.273 vs l.988).
   - Field groups: HANDOVER says "without frames" but DESIGN l.1090 says "fieldset = 2px ink frame".
   - DESIGN leaves the close cross open (l.1028) but `CloseButton` is defined at l.1164.
   - Seasons: DATA says they are neutral (l.20) and also "no neutral entity" (l.489).
   - Catalog: ITEMS/PLATFORM still put items in `items.json` / products in files, but the catalog is in the DB.
   - bunny init: ASSETS says "not run" but PLATFORM says "done".
   - MAP/DATA still describe the per-toon map lists, which were purged 2026-10-12.
   - ITEMS still gives the API base as qr.scutoons.com.
7. **The same rule in several places.** Never push / main only: HANDOVER, BACKLOG ×3, PLAN. Migration process:
   HANDOVER, BACKLOG header, memory. The UI rules: HANDOVER as a summary, canonical in DESIGN/DOCKING (they agree,
   except the field-group frame).
8. **Open work outside BACKLOG.** ITEMS, MAP §8.6, ASSETS, DATA, PERFORMANCE P4–P7, MCP l.219,
   CATALOG-IN-CREATOR Q1–Q8, ACCOUNT-RESTART Q1–Q9, STORYBOARD §10, EXPRESSIONS l.21.

## 3. Proposed structure

```
CLAUDE.md            ≤ 15 lines: read RULES + HANDOVER, then the doc map
agents/RULES.md      ≤ 150 lines, every agent reads it whole – how to work (git, migrations, dev, UI, product rules, docs)
agents/HANDOVER.md   ≤ 60 lines – current state + "last run" summary; replace, never append; no rules, no todos
agents/DECISIONS.md  one line per owner decision, dated, with the quote and a link; newest on top; never edited, only superseded
agents/ISSUES.md     bugs (something that works wrong)            } open only, small template, one shared
agents/FEATURES.md   feature requests / changes / owner steps     } number line 10.N (the old numbers stay)
agents/GOTCHAS.md    ≤ 80 lines – tool traps (CRLF, heredoc, paraglide, vite preview cache, PowerShell …)
agents/<TOPIC>.md    DESIGN, DOCKING, DATA, ITEMS, MAP, PLATFORM, ASSETS, CATALOG, EXPRESSIONS, MCP, EMBASSY, NEWSLETTER:
                     only how it IS now – no logs, no todo lists, no "built on branch"
agents/research/     concepts, research, parked plans – line 1 = status ("research" | "concept, waits for owner" |
                     "parked until …" | "built → see X")
agents/archive/      frozen history: CUTOVER, PLAN, MEMORY, the BACKLOG run logs + Done, the one-off notes.
                     Never edited, never read on start – grep it only when you need the history.
```

**Item template** (ISSUES / FEATURES). At most 5 lines; anything longer goes into the topic doc or research/ and the
item links to it:
```
- [ ] 10.N **Title** – owner YYYY-MM-DD: "quote". What / why in 1–3 lines. → DOC.md §x   [owner] if only the owner can do it
```

**Keeping it tight ("nicht ausrasten lassen")** – these become the "Docs" section of RULES:
- A **done item leaves the list** in the same commit. The commit message says `10.N done`; there is no Done section.
  The history is git.
- Run results (night / day runs) go into HANDOVER "Last run", which each run replaces. Open questions from a run
  become items.
- An owner decision gets **one line** in DECISIONS. If it changes how agents work, RULES changes in the same commit.
  A topic doc gets the resulting "how it is" text, without the story behind it.
- Caps: RULES 150 · HANDOVER 60 · GOTCHAS 80 · ISSUES + FEATURES 40 open items together (more = ask the owner to
  prune). A topic doc over ~500 lines gets trimmed when touched.
- No new top-level file in agents/ without the owner. New concepts go in research/.
- Stale text you notice gets fixed in the same commit (or becomes a one-line issue). Never append a correction below it.

## 4. Phase 2 – order of the commits (after the owner's OK, all on `main`, `git mv` for moves, nothing pushed)

1. `archive/`: git mv CUTOVER, MERGE-media-quality, COMPONENTS, EXAMPLES, NEUTRAL, UI-CONCEPTS (each after its
   still-valid rules / model went to RULES, DESIGN or DATA).
2. RULES.md + GOTCHAS.md from HANDOVER rules, PLAN `RULE:` sections, MEMORY gotchas and the user memory feedback.
   Then git mv PLAN and MEMORY → archive/. HANDOVER is trimmed to the state.
3. DECISIONS.md. Every dated owner quote is collected, about 350 of them (most in DESIGN, BACKLOG, STORYBOARD and
   HANDOVER). One line each with a link. The quote stays where it is too, so nothing is lost.
4. BACKLOG → git mv to FEATURES.md, then split out ISSUES.md. The run logs and Done section go to archive/BACKLOG-done.md.
   The open lists from the topic docs (item 8 above) become items. Ticked items still listed under Open go out.
5. research/: git mv STORYBOARD, FX, LIVE, ACCOUNT-RESTART, CATALOG-IN-CREATOR, AUTOMATION, TREATMENTS, PERFORMANCE,
   with a status line each. Fix LIVE's dates.
6. Topic docs, one commit each: fix the contradictions in §2, cut the logs (→ archive/<doc>-log.md), and keep the
   names.
7. Pointers: CLAUDE.md, user memory (the 7 files point to RULES; `local-dev-mariadb` loses the worktree DB text,
   `night-runs` gets "summary → HANDOVER"), server/docs/DATABASE.md + DEPLOY.md (stale phpMyAdmin parts).

Items keep their numbers. About 200 code files cite "BACKLOG/CUTOVER 10.x", so `grep 10.61 agents/` still finds the
item or, once done, its archive entry.

## 5. Questions for the owner

1. **Is this structure OK?** It means RULES, HANDOVER and DECISIONS on top, ISSUES and FEATURES for open work, topic
   docs describing how things are, and research/ plus archive/.
2. **Open work.** The recommendation is two files in agents/, ISSUES + FEATURES. The alternatives are GitHub Issues,
   or one BACKLOG with two sections. (Your decision of 2026-10-06: open work lives in agents/, not GitHub. GitHub would
   give labels, closing and history for free, but agents need `gh` and the network, and the docs and code citations
   would point outside the repo.)
3. **History.** The recommendation is a frozen `archive/` folder, which keeps it greppable. The alternative is to
   delete it and rely on git alone.
4. **Topic docs.** The recommendation is to fix the contradictions now and trim the logs on the next touch. The
   alternative is a full pass now over 6 600 lines, which is a big diff.
5. The HANDOVER todos for **production migrations are all done** (`migration_status`: 0 pending). Can I drop them?
   The remaining Ask clean-up (bunny app, `ask.` DNS, ghcr image, GitHub vars, Anthropic key) and the newsletter
   going-live steps (secret, KAS cron): are they done, or should they become [owner] items?
