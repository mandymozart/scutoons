# Visual Novel System

## One runner (CUTOVER 10.27, 2026-10-03)

`runner.ts` runs a script (a list of VN events) – the group's script and the slice / placed-item triggers use the same
code. It gets `navigate` passed in, so the logic stays out of the views.

- `interactions/visual-novel.svelte` – the page's group script: `runScript(group.events, 0, { navigate: goto, … })`;
  registers the script + the page's label in the context (`vn.setScript`).
- `event-processor.ts` – slice triggers (`triggers/*.svelte`) and placed items (`slice-items.ts`): runs their events and
  returns the URL to navigate to. A single `divert` event continues in the page's group script at that named point.
- `vn/choices.svelte` – a picked choice runs its actions (`runChoiceActions`): variables, then a jump (navigates) or a
  divert (handed back to the waiting script via `vn.setPendingDivert`).

## Story flow

- `label { name }` – a named point in the script (does nothing when reached).
- `divert { to }` – continue at a named point of the same script (no page change). Also a choice action and a slice event.
- `if { variable, op, value }` on any event – the event is skipped when the test fails (`testCondition`).
- `setVariable { variable, value, op: set | add }`.
- `jumpTo` – navigation only: another group / chapter; `target.anyOf` = a random group label, never the current one.
- Text: `{variable}`, `{variable:clock}` (minutes since midnight → 5:30).

Pure helpers: `@scutoons/content` `vn-script.ts`. Utilities here: `jump-to.ts` (`buildJumpToUrl`), `utils.ts`
(`interpolateVariables`), `variable-actions.ts` (`executeVariableAction`, `runChoiceActions`).
