# agents/ — the map: what exists, how big, what to read (measured 2026-11-03)

Owner 2026-11-03 [law-book]: "we are reading too much context on each session, hence stuff gets lost". So: a fresh agent reads
a small fixed set, everything else is looked up by grep or opened only for the area it touches. This file is NOT read at start.

## 1 · Read at session start — budget ≤ ~250 lines, in this order
| # | File | Lines | ≈ tokens | What it is |
|---|---|---|---|---|
| 1 | `C:\Development\CLAUDE.md` | 53 | 0.8k | workspace rules (scope creep, publishing, attribution, usage report) |
| 2 | `scutoons\CLAUDE.md` | ≤ 20 | 0.4k | entry point: this reading order, the issue rule |
| 3 | memory index `MEMORY.md` | ≤ 25 | 0.8k | one line per remembered fact; the files are opened only when the line matches your task |
| 4 | `agents/RULES.md` | ≤ 150 | ≤ 4.5k | **the law book**: numbered rules, each with its decision date + grep key |
| 5 | `agents/HANDOVER.md` | ≤ 40 (cap 60) | 1k | the current state only (no rules, no todos) |

Before 2026-11-03 this set was 312 lines / ≈ 7.6k tokens and was *followed* by whatever the agent grepped: DECISIONS.md alone is
698 lines / ≈ 55k tokens, the topic docs ≈ 110k tokens. After the refactor: see "Sizes" at the end.

## 2 · Read only when you do this
| You are … | Read | Lines |
|---|---|---|
| orchestrating / starting workers / a night shift | `agents/ORCHESTRATION.md` (brief template!), `C:\Development\agents\ORCHESTRATION.md` §1–§2, §5–§6, §11 (+ `guides/`), `agents/HANDOFF.md` | 85 · 199 · 90 |
| merging deck / shared / viewer work, or auditing | `agents/AUDIT.md`, `research/BRIDGE-AUDIT.md` §1 table | ~70 · ~40 |
| in trouble with a tool (Windows, vite, paraglide, browser pane, MCP restart) | `agents/GOTCHAS.md` (grep it) | 79 |
| touching UI | `DESIGN.md` (the part you build), `DOCKING.md` | 1532 · 275 |
| touching stored data, API, server | `DATA.md`, `PLATFORM.md`, `CATALOG.md`; server/docs/DATABASE.md | 652 · 942 · 271 |
| touching items, avatar, carrier bag | `ITEMS.md`, `CATALOG.md` | 1565 · 271 |
| touching the map / places | `MAP.md` §8 | 1107 |
| touching media, slices, CDN | `ASSETS.md` | 461 |
| texts, variables, logic | `EXPRESSIONS.md` | 368 |
| MCP / AI connections | `MCP.md` | 300 |
| embassy / newsletter | `EMBASSY.md`, `NEWSLETTER.md` | 337 · 168 |
| deck / live / engine | `research/ENGINE-REFACTOR-PLAN.md`, `research/ENGINE-ALIGNMENT.md` §0, `research/LIVE.md`, `research/SHOW.md` (the show concept, #462), `research/CATALOG-LINK.md` (catalog ↔ show, assignments per entity, #472), `research/XR.md` (taking over OSCT's XR methods: the experience as the unit, the four ways in, #482), `research/XR-SHARED-CORE.md` (one XR core out of OSCT with SCUTOONS as its second title: the cut line, the steps, the advice for the copy), `research/OPEN-WORLD.md` (what real-time engines do, checked against their documentation, and the refactor that follows, #472), `deck/*/README.md`, + the module/rig docs `research/ABI-V2.md`, `RIG-EDITING.md`, `EDITOR.md`, `EVENTS.md`, `INSTRUMENTS.md` | line 1 = status, read that first |

Topic docs describe **how it is now** (read the section you touch, never the whole file: DESIGN, ITEMS, MAP and PLATFORM are
24–38k tokens each).

## 3 · Look up by grep, never read whole
- `agents/decisions/<topic>.md` (the owner's words, one line each, 656 decisions in 9 files; a `[key]` per line; index + recording rule:
  `agents/DECISIONS.md`). Need to know: open only your area's file. Every key RULES cites is in `decisions/law.md`: `grep -n '\[core-only\]' agents/decisions/law.md`.
- `agents/research/` (≈ 70 concept / research / parked-plan files; line 1 = status, so `head -1 agents/research/*.md` is the index).
- `agents/archive/` (frozen history, incl. superseded decisions) — grep only, never edited.
- GitHub issues: `npm run backlog`, `gh issue view N --comments`. Open work lives only there.

## 4 · Who owns what (to keep it from growing back)
| File | Holds | Limit | Changes when |
|---|---|---|---|
| CLAUDE.md | reading order, the issue rule | 20 | the reading order changes |
| RULES.md | the law book | 150 | an owner decision changes how agents work — same commit as its DECISIONS line |
| HANDOVER.md | state of the world, "Last run" (replaced, never appended) | 60 (owner), aim ≤ 40 | a state fact changes |
| DECISIONS.md + decisions/ | index + evidence by topic: owner words, date, key, → doc | append-only, newest on top | every owner statement (record it the same day) |
| AUDIT.md | the audit protocol | 80 | the audit method changes |
| ORCHESTRATION.md | SCUTOONS specifics of orchestration + the brief template | 100 | the orchestration changes |
| GOTCHAS.md | tool traps | 80 | a trap cost time |
| topic docs | how it is now | trim over ~500 lines | the thing changes |
| research/ | concepts, reports, parked plans | — | line 1 = status, always |
| archive/ | frozen | never | — |
| memory (outside the repo) | personal-to-this-machine facts + pointers; ≤ 25 index lines | — | a fact is not in the repo |

A fact lives in ONE place. If RULES states it, memory holds at most a pointer; if a topic doc describes it, RULES holds only
the rule. No new top-level file in agents/ without the owner.

## 5 · Sizes, measured 2026-11-03 (lines / ≈ tokens at 4 bytes per token)
| What a fresh agent loads | before #380 | after #380 |
|---|---|---|
| workspace `CLAUDE.md` | 53 / 0.8k | 53 / 0.8k (not ours) |
| `scutoons\CLAUDE.md` | 16 / 0.3k | 17 / 0.3k |
| memory index | 20 / 0.8k (22 files behind it) | 7 / 0.3k (7 files) |
| `RULES.md` | 160 / 4.1k (over its own cap) | 108 / 4.1k (more rules, denser) |
| `HANDOVER.md` | 63 / 1.6k (state of 2026-10-12 / 11-01) | 27 / 0.9k |
| **Total at start** | **312 / 7.6k** | **212 / 6.4k** |
Need-to-know: `DECISIONS.md` was one 698-line / ≈ 55k-token file; now a 22-line index + 8 topic files of 2–10k tokens each (law 9.5k, deck 10.2k, map 2.2k …;
37 superseded lines in archive/); topic docs ≈ 110k tokens (unchanged, read by section).
