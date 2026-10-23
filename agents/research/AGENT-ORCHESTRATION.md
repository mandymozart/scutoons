Status 2026-10-23: research brief, nothing tried yet (#169; research started 2026-10-23 as a cloud session). Written project-neutral: the result is meant to move into a small repository of its own that every project reads into its rules.

# Agent orchestration and usage costs

## 1. Problem (owner 2026-10-23)
- Subscription: Claude Max 20x. Tools: Claude Code and claude.ai.
- The weekly usage limits are exhausted although the Fable-specific usage is low; extra usage is expensive.
- Wanted: intensive, parallel development without usage top-ups.
- Out of scope: image and video generation (external services cover them), replacing the creative services.

## 2. Two levers, in this order
1. **Spend fewer tokens per accepted task** in the current setup. Limits reached with low Fable usage means the volume
   is in routine work: large contexts at session start (CLAUDE.md + agents/ docs), whole-file reads, repeated
   exploration, memory kept in several places, strong models on routine tasks. This lever costs nothing and
   applies to every provider later.
2. **Add an independent coding provider** for implementation, tests and review, and keep Claude as the lead
   architect for the hard parts. Test one provider first, measured, before buying several.

## 3. Shape of the result: one small repository, read into every project
Working name `agent-ops` (final name: owner). Small enough to be read whole at session start. Contents:
- **ORCHESTRATION.md** – roles (lead architect · implementer · tester · reviewer · coordinator), which tool plays
  which role, how work is handed over (an issue, a branch or worktree, a review before merge), what never runs
  unattended.
- **ALLOCATION.md** – the model / effort per task class (architecture and difficult debugging → the strongest
  Claude; routine implementation, tests, lint fixes → the cheapest tool that passes review), subagents vs. sessions,
  when to run in parallel and how many at once.
- **skills/** – the shared skills every project installs (start a session small, end a session on the issue,
  review before merge, efficiency check); project-specific skills stay in the project.
- **workflows/** – the repeatable flows: issue → branch/worktree → implementation by agent X → tests → independent
  review → merge; the night run; the release check.
- **CHECKS.md** – routine efficiency checks (§6) and how often they run.
- **LEARNINGS.md** – dated, one line each: what worked, what did not, with the measurement. The only place for it.
- **BUDGET.md** – the subscriptions, their limits, the spending caps, and the rule against silent paid API fallback.
The projects' own CLAUDE.md / agents/RULES.md keep only what is specific to the project and point to the repo.
Redundant memory (the same rule in several projects' memory and docs) moves there once and is deleted elsewhere.

## 4. Candidate tools (validate plan pricing, limits and subscription sign-in before choosing)
- **OpenAI Codex** – independent agent for implementation, debugging, tests, review; cloud and CLI.
- **GitHub Copilot Pro+** – cheaper; coding agent on issues and PRs inside GitHub; fits the board workflow.
- **Gemini CLI** – possible low-cost supplement.
- **Claude** – Fable / Opus for architecture and hard problems only; Sonnet / Haiku for routine work; subagents on
  cheaper models for reading and searching.
Record for each: price, included capacity, parallel-agent limits, how it authenticates (subscription, not API key),
how spending is capped.

## 5. Trial (in SCUTOONS first; the procedure is the same for every project)
1. Pick three representative tasks from the open issues: a bug fix, a multi-file feature, a difficult debugging task.
2. Baseline: the current Claude workflow on the three tasks – tokens, wall time, supervision, accepted after review.
3. The candidate provider on equivalent tasks (equal briefs), each in its own branch or worktree.
4. Coordinator: GitHub issues + the project board assign the work, carry dependencies, show completion.
5. Before merge: the project's automated checks green + an independent review (another agent or the owner).
6. Spending caps set before the first run; no fallback to paid API usage – a failed run stops, it does not pay.
7. Record every run in LEARNINGS.md with the numbers; choose the setup with the lowest cost per accepted task.

## 6. Evaluation criteria and routine checks
Per candidate and task: correctness and regressions · quality of multi-file changes and debugging · test coverage
and adherence to the project's conventions · share of changes accepted after review · completion time and
supervision needed · cost per accepted, mergeable task · included capacity and parallel limits.
Routine checks (weekly, or at the start of a night run): tokens per accepted task · context size at session start
(what every session reads before the first edit) · whole-file reads that a grep would have covered · the same fact
kept in more than one place (memory, docs, rules) · strong-model minutes spent on routine work · usage against the
weekly limit, with a stop before the top-up.

## 7. Acceptance
- A documented comparison of at least two setups (current Claude vs. Claude + one provider), with the actual costs
  and limits recorded.
- A repeatable workflow for parallel agents and safe integration (branches/worktrees, checks, review, merge).
- Explicit spending caps; no unintended paid usage.
- A recommendation based on code quality, throughput and total cost.
- The `agent-ops` repository exists with §3's files, read into SCUTOONS's rules; the other projects follow.

## 8. Open (owner)
- Decide: which provider to test first. Recommendation: Codex (an independent agent with its own worktrees and
  review, the closest match to the lead-architect + implementer split); Copilot Pro+ second if cost decides.
- Owner steps: the subscription and its spending cap, the sign-in on this machine, the repository's name and home.
