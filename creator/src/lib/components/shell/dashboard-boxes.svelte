<script lang="ts">
	import { Card, ListRow } from '@scutoons/ui/editor';
	import { toonName } from '@scutoons/content';
	import { getCreatorNav } from '$lib/nav.svelte.ts';
	import { recent } from '$lib/recent.svelte.ts';
	import { reviews } from '$lib/editor/reviews.svelte.ts';
	import { notPublished } from '$lib/editor/not-published.svelte.ts';
	import { placeHref } from '$lib/editor/place-href.ts';
	import { kindWord, ownName } from '$lib/labels.ts';
	import { getLocale } from '$lib/paraglide/runtime.js';
	import { m } from '$lib/paraglide/messages.js';
	import { addressOf, iconOf, workspaceOf } from '$lib/shell/address.ts';
	import { ago } from '$lib/shell/dashboard.ts';
	import { openings } from '$lib/shell/openings.svelte.ts';
	import { workspaceName } from '$lib/shell/workspace-names.ts';
	import ReviewList from '$lib/components/page/review-list.svelte';

	/**
	 * The switcher's two boxes: Last opened · Needs review / Not published – each line named by the item's own name,
	 * its kind beside it (scenes and shows too); sending out is Publish's alone (RULES H18). The welcome has nothing
	 * opened yet: Not published alone, wide.
	 */
	let { welcome = false }: { welcome?: boolean } = $props();

	const ROWS = 3;
	const nav = getCreatorNav();
	const toons = $derived(nav.toons.map((t) => ({ id: t.id, name: toonName(t, getLocale()) })));
	$effect(() => void notPublished.load(toons));
	$effect(() => void openings.load());

	const names = $derived(new Map(Object.values(openings.rows).flatMap((rows) => rows.map((p) => [p.address, p.label] as const))));
	const now = Date.now();
	function since(at: number | undefined): string {
		const a = ago(at, now);
		if (!a) return '';
		return a.unit === 'now' ? m['shell.dash_now']() : a.unit === 'minute' ? m['shell.dash_minutes']({ n: a.n }) : a.unit === 'hour' ? m['shell.dash_hours']({ n: a.n }) : m['shell.dash_days']({ n: a.n });
	}
	const last = $derived(
		recent.list.slice(0, ROWS).map((r) => {
			const [path, search] = r.href.split(/(?=\?)/);
			const address = addressOf(path, search ?? '');
			const ws = address ? workspaceOf(address) : null;
			return { ...r, address, name: r.trail.at(-1) ?? r.href, meta: [ws ? workspaceName(ws) : '', since(r.at)].filter(Boolean).join(' · ') };
		})
	);
	const waiting = $derived((reviews.overview?.to_review?.length ?? 0) + (reviews.overview?.mine ?? []).filter((r) => r.state === 'pending' || r.state === 'returned').length);
	const shown = $derived(welcome ? 6 : Math.max(1, ROWS - Math.min(waiting, 2)));
	const rowMeta = (key: string, toon: string | null) => [m['shell.dash_not_published_row'](), kindWord(key), toon ? nav.toonName(toon) : ''].filter(Boolean).join(' · ');
</script>

<div class="boxes" class:welcome>
	{#if !welcome}
		<Card title={m['shell.dash_last_opened']()} data-testid="dash-last">
			<div class="rows">
				{#each last as r (r.href)}
					<ListRow icon={r.address ? iconOf(r.address) : 'file'} label={r.name} meta={r.meta} href={r.href} data-testid="dash-open" />
				{:else}
					<p class="quiet">{m['shell.dash_nothing_open']()}</p>
				{/each}
			</div>
		</Card>
	{/if}
	<Card data-testid="dash-waiting">
		<h2 class="heads">
			{#if !welcome}<span>{m['shell.dash_needs_review']()}{#if waiting}<span class="n"> · {waiting}</span>{/if}</span>{/if}
			<span class:end={!welcome} data-testid="dash-not-live">{m['shell.dash_not_published']()}{#if notPublished.list.length}<span class="n"> · {notPublished.list.length}</span>{/if}</span>
			{#if welcome}<span class="end">{m['shell.dash_in_your_toons']()}</span>{/if}
		</h2>
		<div class="rows">
			{#if !welcome && waiting}<ReviewList limit={2} />{/if}
			{#each notPublished.list.slice(0, shown) as e (e.key)}
				<ListRow icon={iconOf(e.key)} label={ownName(e.key, names.get(e.key))} meta={rowMeta(e.key, e.toon)} href={placeHref(e.key, e.toon)} class="unpublished" data-testid="dash-unpublished" />
			{/each}
			{#if !waiting && !notPublished.list.length}<p class="quiet">{m['shell.nothing_waiting']()}</p>{/if}
		</div>
		{#if notPublished.list.length > shown}
			<span class="actions"><span class="quiet" title={m['creator.dash_more_title']()}>{m['creator.dash_more']({ count: notPublished.list.length - shown })}</span></span>
		{/if}
	</Card>
</div>

<style>
	.boxes {
		display: grid;
		grid-template-columns: repeat(2, minmax(0, 1fr));
		gap: var(--editor-gap);
	}
	.boxes.welcome {
		grid-template-columns: minmax(0, 1fr);
	}
	.rows {
		display: flex;
		flex-direction: column;
	}
	/* the Card's title, two ends: Needs review at the start, Not published at the end */
	.heads {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: var(--kit-icon-gap);
		min-height: var(--control-height);
		margin: 0;
		padding: 0 calc(var(--editor-pad) / 2);
		color: var(--tool-muted);
		font-size: var(--editor-label);
		font-weight: inherit;
		letter-spacing: var(--editor-label-spacing);
		text-transform: uppercase;
	}
	.end {
		margin-left: auto;
	}
	.n {
		color: var(--tool-warn);
	}
	.rows :global(.unpublished .meta) {
		color: var(--tool-warn);
	}
	.quiet {
		margin: 0;
		padding: 0 calc(var(--editor-pad) / 2);
		color: var(--tool-muted);
		font-size: var(--editor-small);
	}
	.actions {
		display: flex;
		align-items: center;
		justify-content: flex-end;
		gap: var(--kit-icon-gap);
	}
</style>
