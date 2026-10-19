<script>
	// The studio's ActionBar (DESIGN "The bottom bar is grouped", research/STUDIO-LAYOUT.md concept B): the shot stepper
	// (‹ Shot n / N ›), the chapter's measures and the keys on the left; the notes (the storyboard check's problems – a
	// click picks the shot a note names), Revert, Keep as variant, in the Generate step Review (Approve / Ask for a
	// change – owner 2026-10-19: approving happens inside Generate), and Save + frame as the one primary, last. It stays
	// while a panel is maximized (DOCKING "Maximize").
	import { ActionBar, Icon, Menu } from '@scutoons/ui';
	import { allShots, pickShot, revert, studio } from '../studio.svelte.js';
	import { busy, keepVariant, review, saveShot } from '../actions.svelte.js';
	import { steps } from '../steps.svelte.js';

	const shots = $derived(allShots());
	const at = $derived(shots.findIndex((s) => s.id === studio.sel?.id));
	const summary = $derived.by(() => {
		const t = studio.data?.summary?.total;
		return t ? `${t.shots} shots · ${t.screens.toFixed(1)} screens · ${(t.seconds / 60).toFixed(1)} min` : '';
	});
	const problems = $derived(studio.data?.problems ?? []);
	const errors = $derived(problems.filter((p) => p.level === 'error').length);
	const step = (d) => pickShot(shots[Math.max(0, Math.min(shots.length - 1, (at < 0 ? 0 : at) + d))]?.id);
	const named = (m) => m.match(/\bsb-[\w-]+/)?.[0];
</script>

<ActionBar label="Shot actions">
	{#snippet start()}
		<div class="sc-seg" role="group" aria-label="Shot">
			<button type="button" class="is-square" aria-label="Previous shot" title="Previous shot (←  on the timeline)" disabled={at <= 0} onclick={() => step(-1)}><Icon name="chevron-left" /></button>
			<span class="where" aria-live="polite">{at >= 0 ? `${studio.sel.id} · ${at + 1} / ${shots.length}` : `${shots.length} shots`}</span>
			<button type="button" class="is-square" aria-label="Next shot" title="Next shot (→ on the timeline)" disabled={at >= shots.length - 1} onclick={() => step(1)}><Icon name="chevron-right" /></button>
		</div>
		<span class="cap">{summary}</span>
		<span class="cap keys" title="Keys outside text fields: 1–6 step · T shot list · N the step's panels · Ctrl + Space maximize · Ctrl + \ shot list · Ctrl + Shift + \ fold the 3D view"><kbd>1</kbd>–<kbd>6</kbd> step · <kbd>T</kbd> shots · <kbd>N</kbd> panels · <kbd>Ctrl</kbd>+<kbd>Space</kbd> maximize</span>
	{/snippet}
	{#snippet secondary()}
		{#if problems.length}
			<Menu label="{problems.length} notes" drop="up" align="end">
				{#snippet trigger()}<Icon name="alert-triangle" /> {problems.length} note{problems.length === 1 ? '' : 's'}{errors ? ` · ${errors} error${errors === 1 ? '' : 's'}` : ''}{/snippet}
				{#each problems as p, i (i)}
					<button type="button" class="note" class:sc-error={p.level === 'error'} title={p.m} disabled={!named(p.m)} onclick={() => pickShot(named(p.m))}>{p.m}</button>
				{/each}
			</Menu>
		{/if}
	{/snippet}
	{#snippet end()}
		{#if studio.sel}
			<button type="button" class="secondary" onclick={revert} disabled={!studio.dirty || !!busy.text}>Revert</button>
			<button type="button" class="secondary" onclick={keepVariant} disabled={!!busy.text} title="The shot as it is now as a variant (the frame too)">Keep as variant</button>
			{#if steps.current === 'generate'}
				<Menu label="Review" drop="up" align="end">
					{#snippet trigger()}Review <Icon name="chevron-up" />{/snippet}
					<button type="button" onclick={() => review('ok')} title="Approve: the review + the dataset (approve.mjs)">Approve the shot …</button>
					<button type="button" onclick={() => review('change')} title="An open change – shows as a warning until it is done">Ask for a change …</button>
				</Menu>
			{/if}
			<button type="button" onclick={saveShot} disabled={!!busy.text} data-testid="save">{busy.text || `Save + frame${studio.dirty ? ' •' : ''}`}</button>
		{/if}
	{/snippet}
</ActionBar>

<style>
	.where {
		display: inline-flex;
		align-items: center;
		padding: 0 var(--space-075);
		font-variant-numeric: tabular-nums;
		white-space: nowrap;
	}
	.cap {
		font-size: var(--type-caption-size);
		white-space: nowrap;
	}
	.keys {
		color: var(--text-muted);
	}
	kbd {
		font: inherit;
		padding: 0 0.2em;
		border: var(--border-width-hairline) solid currentColor;
	}
	.note {
		max-width: 40rem;
		white-space: normal;
		font-size: var(--type-caption-size);
	}
</style>
