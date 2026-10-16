<script lang="ts">
	/**
	 * A running season save on the account page (PWA phase 3, lib/offline/save-season.svelte.ts, owner 2026-10-16
	 * "alle Pakete angehen"): the season's bar, then one bar per chapter – saved, saving now (its bytes), waiting, failed –
	 * and × to stop (the chapters already saved stay). The running chapter's per-kind bars follow below (save-progress.svelte).
	 */
	import { CloseIcon } from '@scutoons/ui';
	import { m } from '$lib/paraglide/messages.js';
	import { savedChapters } from '$lib/offline/saved-chapters.svelte.ts';
	import { seasonSaves, type SeasonJob } from '$lib/offline/save-season.svelte.ts';

	let { job }: { job: SeasonJob } = $props();
	const toon = $derived(job.toon);
	const season = $derived(job.season);

	const total = $derived(Math.round(seasonSaves.share(toon, season) * 100));
	/** a chapter's share: saved 100, the running one by its bytes, the rest 0 */
	function share(n: number): number {
		if (job.done.includes(n)) return 100;
		const running = n === job.current ? savedChapters.job(toon, n) : undefined;
		return running ? Math.min(99, Math.round((running.done / Math.max(1, running.total)) * 100)) : 0;
	}
	const note = (n: number) =>
		job.done.includes(n) ? '✓' : job.failed.includes(n) ? '!' : n === job.current ? `${share(n)} %` : m['saved.season_waiting']();
</script>

<div class="season-progress" data-testid="season-progress">
	<div class="head">
		<strong>{job.label}</strong>
		<button type="button" class="round" title={m['saved.stop']()} aria-label={m['saved.stop']()} onclick={() => seasonSaves.cancel(toon, season)}>
			<CloseIcon size="0.875rem" />
		</button>
	</div>
	<div class="bar total" role="progressbar" aria-label={job.label} aria-valuemin="0" aria-valuemax="100" aria-valuenow={total}>
		<span style:width="{total}%"></span>
	</div>
	<small>{m['saved.season_progress']({ done: job.done.length, count: job.chapters.length })}</small>
	<ul>
		{#each job.chapters as chapter (chapter.number)}
			<li data-testid="season-progress-chapter" data-chapter={chapter.number}>
				<span class="what">{chapter.label}</span>
				<div class="bar" role="progressbar" aria-label={chapter.label} aria-valuemin="0" aria-valuemax="100" aria-valuenow={share(chapter.number)}>
					<span style:width="{share(chapter.number)}%"></span>
				</div>
				<small class="amount">{note(chapter.number)}</small>
			</li>
		{/each}
	</ul>
</div>

<style>
	/* the look of save-progress.svelte: thin framed bars in ink, square */
	.season-progress {
		display: flex;
		flex-direction: column;
		gap: 0.375rem;
		padding: 0.5rem 0 0.75rem;
		border-bottom: var(--border-width-hairline) solid var(--border-subtle);
	}
	.head {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 0.75rem;
	}
	strong {
		font-weight: var(--font-weight-regular);
		min-width: 0;
		overflow-wrap: anywhere;
	}
	.bar {
		height: 0.5rem;
		border: var(--border-width-regular) solid var(--ink);
		background: var(--paper);
		overflow: hidden;
	}
	.bar span {
		display: block;
		height: 100%;
		background: var(--ink);
		transition: width 0.2s linear;
	}
	.bar.total {
		height: 0.75rem;
	}
	ul {
		list-style: none;
		margin: 0.25rem 0 0;
		padding: 0;
		display: grid;
		grid-template-columns: auto minmax(0, 1fr) auto;
		align-items: center;
		gap: 0.375rem 0.75rem;
	}
	li {
		display: contents;
	}
	.what {
		font-size: 0.75rem;
	}
	.amount {
		font-variant-numeric: tabular-nums;
		text-align: right;
	}
	small {
		font-size: 0.75rem;
		line-height: 1.3;
		color: #6b6b6b;
	}
	.round {
		flex: none;
		display: inline-flex;
		align-items: center;
		justify-content: center;
		width: var(--control-height);
		height: var(--control-height);
		padding: 0;
		border-radius: 50%;
		border: var(--border-width-regular) solid var(--ink);
		background: var(--paper);
		color: var(--ink);
		cursor: pointer;
	}
	@media (prefers-reduced-motion: reduce) {
		.bar span {
			transition: none;
		}
	}
</style>
