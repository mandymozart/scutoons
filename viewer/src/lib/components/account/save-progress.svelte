<script lang="ts">
	/**
	 * A running save's progress on the account page (owner 2026-10-16: "Progress Bars …, die zeigen, wie viel von was
	 * runtergeladen wurde" – like OSCT's download section: a thin framed bar, "x of y MB" under it): one bar for the
	 * whole chapter, then one per kind it holds (pictures, music + sounds, videos, the rest), a tap on × stops it. A
	 * refresh (a new version, only the changed files) says "updating" – stopped, the saved version stays.
	 */
	import { Icon } from '@scutoons/ui';
	import { m } from '$lib/paraglide/messages.js';
	import { roundMb, SAVE_KINDS, type SaveKind } from '$lib/sw/saved-rules.ts';
	import type { SaveJob } from '$lib/offline/saved-chapters.svelte.ts';

	let { job, onstop }: { job: SaveJob; onstop: () => void } = $props();

	const KIND_LABEL: Record<SaveKind, () => string> = {
		image: () => m['saved.kind_image'](),
		audio: () => m['saved.kind_audio'](),
		video: () => m['saved.kind_video'](),
		data: () => m['saved.kind_data']()
	};
	const share = (done: number, total: number) => (total > 0 ? Math.min(100, Math.round((done / total) * 100)) : 100);
	const kinds = $derived(SAVE_KINDS.filter((k) => job.kinds[k].total > 0));
</script>

<div class="save-progress" data-testid="save-progress">
	<div class="head">
		<strong>{job.label}{#if job.mode === 'refresh'} · {m['saved.updating']()}{/if}</strong>
		<button type="button" class="round" title={m['saved.stop']()} aria-label={m['saved.stop']()} onclick={onstop}>
			<Icon name="x" size="0.875rem" />
		</button>
	</div>
	<div class="bar total" role="progressbar" aria-label={job.label} aria-valuemin="0" aria-valuemax="100" aria-valuenow={share(job.done, job.total)}>
		<span style:width="{share(job.done, job.total)}%"></span>
	</div>
	<small>{m['saved.progress']({ done: roundMb(job.done), total: roundMb(job.total), percent: share(job.done, job.total) })}</small>
	<ul>
		{#each kinds as kind (kind)}
			{@const k = job.kinds[kind]}
			<li data-testid="save-progress-{kind}">
				<span class="what">{KIND_LABEL[kind]()}</span>
				<div class="bar" role="progressbar" aria-label={KIND_LABEL[kind]()} aria-valuemin="0" aria-valuemax="100" aria-valuenow={share(k.done, k.total)}>
					<span style:width="{share(k.done, k.total)}%"></span>
				</div>
				<small class="amount">{roundMb(k.done)} / {roundMb(k.total)} MB</small>
			</li>
		{/each}
	</ul>
</div>

<style>
	.save-progress {
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
	/* the bar: thin, framed, filled in ink (OSCT's settings-download, square as SCUTOONS is) */
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
