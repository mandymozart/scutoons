# Visual Novel System

## One runner (CUTOVER 10.27, 2026-10-03)

`runner.ts` runs a script (a list of VN events) – the group's script and the slice / placed-item triggers use the same
code. It gets `navigate` passed in, so the logic stays out of the views.

- `interactions/vn-page.svelte` – one reader page (linked groups put several groups on it, owner 2026-10-06): the one
  box / stage / characters overlay, the page's script line (`script-queue.ts`: scripts one after another, the page done
  when all it must run are through), the scroll freeze (`vn.state.frozen`) with its pause mark under the logo.
- `interactions/visual-novel.svelte` – a group's script: asks the line to start it (a lone group on mount, in linked
  groups when its first slice is seen), then `runScript(group.events, 0, { navigate: goto, scope, … })`.
- `group-scope.ts` – each group's context for its slices: its script + label (a slice event's `divert` goes into ITS
  group's script), its `scope` (text boxes are kept per group), `done` (above a `?label=` target: read already).
- `event-processor.ts` – slice triggers (`triggers/*.svelte`) and placed items (`slice-items.ts`): runs their events and
  returns the URL to navigate to. A single `divert` event continues in its group's script at that named point.
- `vn/choices.svelte` – a picked choice runs its actions (`runChoiceActions`): variables, then a jump (navigates) or a
  divert (handed back to the waiting script via `vn.setPendingDivert`).

## Story flow

- `label { name }` – a named point in the script (does nothing when reached).
- `divert { to }` – continue at a named point of the same script (no page change). Also a choice action and a slice event.
- `if { variable, op, value }` on any event – the event is skipped when the test fails (`testCondition`).
- `setVariable { variable, value, op: set | add }`.
- `jumpTo` – navigation only: another group / chapter; `target.anyOf` = a random group label, never the current one.
- Text (messages, choice labels, text boxes): ink's inline syntax – `{coins}`, `{coins == 1: coin|coins}`, `{a|b|c}`,
  `{clock(t)}` (minutes since midnight → 5:30), reader state `{map.total}` … (agents/EXPRESSIONS.md; the resolver:
  `lib/expressions/`).

Pure helpers: `@scutoons/content` `vn-script.ts`. Utilities here: `jump-to.ts` (`buildJumpToUrl`),
`variable-actions.ts` (`executeVariableAction`, `runChoiceActions`).
