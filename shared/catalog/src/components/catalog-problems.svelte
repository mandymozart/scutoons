<script lang="ts">
	// The problems of a catalog list (admin → Catalog, owner 2026-10-04) – the creator's Problems panel in the admin:
	// errors and warnings, a row opens the entry it is about, ✓ "Looks fine" accepts a warning (it moves behind
	// "accepted"; stored in the entry, lib/catalog/problems.ts). Errors can't be accepted.
	import { Icon } from '@scutoons/ui';
	import { KINDS } from '../kinds.ts';
	import { canAccept, entryOf, type CatalogProblem } from '../problems.ts';
	import type { CatalogLists } from '../kinds.ts';
	import { m } from '../messages.ts';

	let {
		problems,
		lists,
		accepted,
		busy = null,
		onopen,
		ontoggle
	}: {
		problems: CatalogProblem[];
		lists: CatalogLists;
		/** accepted as the editor stands (the open entry's unsaved form counts) */
		accepted: (p: CatalogProblem) => boolean;
		/** the problem whose acceptance is being saved */
		busy?: CatalogProblem | null;
		onopen: (p: CatalogProblem) => void;
		ontoggle: (p: CatalogProblem) => void;
	} = $props();

	let showAccepted = $state(false);

	const counts = $derived({
		error: problems.filter((p) => p.level === 'error').length,
		warning: problems.filter((p) => p.level === 'warning' && !accepted(p)).length,
		accepted: problems.filter(accepted).length
	});
	const visible = $derived(problems.filter((p) => p.level !== 'info' && (showAccepted || !accepted(p))));

	/** Entry title · field (array items as #n, the entry's own id prefix of the message stays in the message) */
	function placeOf(p: CatalogProblem): string {
		const list = KINDS[p.list as keyof typeof KINDS];
		if (!p.place || p.place.kind === 'meta') return m['catalog.problems_all']({ what: list?.label() ?? p.list });
		const entry = entryOf(lists, p.place);
		const parts = [entry ? KINDS[p.place.kind].title(entry.data, entry.id) : p.place.id];
		for (const key of p.place.path) parts.push(typeof key === 'number' ? `#${key + 1}` : String(key));
		return parts.join(' · ');
	}
</script>

<div class="problems" data-testid="catalog-problems-panel">
	<p class="summary">
		<span class:bad={counts.error > 0}>{m['catalog.problems_errors']({ count: counts.error })}</span> · {m['catalog.problems_warnings']({ count: counts.warning })}
	</p>
	<div class="filters">
		<label><input type="checkbox" bind:checked={showAccepted} disabled={!counts.accepted} /> {m['catalog.problems_accepted']({ count: counts.accepted })}</label>
	</div>
	<ul>
		{#each visible as p, i (i)}
			{@const done = accepted(p)}
			<li class="level-{p.level}" class:is-accepted={done}>
				<button type="button" class="goto" onclick={() => onopen(p)} title={m['catalog.problems_open']()} disabled={!entryOf(lists, p.place) || p.place?.kind === 'meta'}>
					<span class="where">{placeOf(p)}</span>
					<span class="msg">{p.message}</span>
				</button>
				{#if canAccept(lists, p)}
					<span class="actions">
						<button
							type="button"
							class="sc-round"
							class:is-active={done}
							aria-pressed={done}
							disabled={!!busy}
							title={done ? m['catalog.problems_unaccept']() : m['catalog.problems_accept']()}
							aria-label={done ? m['catalog.problems_unaccept']() : m['catalog.problems_accept']()}
							onclick={() => ontoggle(p)}
							data-testid="catalog-accept"><Icon name="check" /></button
						>
					</span>
				{/if}
			</li>
		{/each}
	</ul>
</div>

<style>
	.summary {
		margin: 0 0 var(--space-05);
		color: var(--text-muted);
		font-size: var(--type-caption-size);
		line-height: var(--type-caption-line);
	}
	.summary .bad {
		color: var(--state-error);
	}
	.filters {
		display: flex;
		flex-wrap: wrap;
		gap: var(--space-05) var(--space-1);
		padding-bottom: var(--space-05);
		border-bottom: var(--border-width-hairline) solid var(--border-subtle);
		font-size: var(--type-caption-size);
		line-height: var(--type-caption-line);
	}
	.filters label {
		display: flex;
		align-items: center;
		gap: var(--space-05);
	}
	.hint {
		color: var(--text-muted);
	}
	ul {
		list-style: none;
		margin: 0;
		padding: 0;
	}
	li {
		display: flex;
		align-items: flex-start;
		gap: var(--space-05);
		padding: var(--space-05) 0 var(--space-05) var(--space-075);
		border-left: var(--border-width-focus) solid var(--border-muted);
		border-bottom: var(--border-width-hairline) solid var(--border-subtle);
	}
	li.level-error {
		border-left-color: var(--state-error);
	}
	li.level-warning {
		border-left-color: var(--state-warning);
	}
	li.is-accepted {
		border-left-color: var(--border-muted);
	}
	li.is-accepted .msg {
		color: var(--text-muted);
	}
	.goto {
		flex: 1;
		min-width: 0;
		display: flex;
		flex-direction: column;
		gap: var(--space-025);
		padding: 0;
		border: 0;
		background: none;
		color: inherit;
		font: inherit;
		text-align: left;
		cursor: pointer;
		font-size: var(--type-caption-size);
		line-height: var(--type-caption-line);
	}
	.goto:disabled {
		cursor: default;
	}
	.goto:not(:disabled):hover .msg,
	.goto:not(:disabled):focus-visible .msg {
		text-decoration: underline;
	}
	.where {
		color: var(--text-muted);
		overflow-wrap: anywhere;
	}
	.actions {
		display: flex;
		gap: var(--space-025);
		flex: none;
	}
</style>
