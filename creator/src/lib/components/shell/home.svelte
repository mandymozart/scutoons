<script lang="ts">
	import { Badge, Icon, ToonAvatar } from '@scutoons/ui';
	import { Button, Card, ListRow, Tile } from '@scutoons/ui/editor';
	import type { ContentIndexEntity } from '@scutoons/api';
	import { toonAsset, toonName } from '@scutoons/content';
	import { getCreatorNav } from '$lib/nav.svelte.ts';
	import { recent } from '$lib/recent.svelte.ts';
	import { reviews } from '$lib/editor/reviews.svelte.ts';
	import { pendingFiles, publishAndLog } from '$lib/editor/publish-tree.ts';
	import { placeHref } from '$lib/editor/place-href.ts';
	import { fileLabel } from '$lib/labels.ts';
	import { asset } from '$lib/utils/cdn.ts';
	import { getLocale } from '$lib/paraglide/runtime.js';
	import { m } from '$lib/paraglide/messages.js';
	import { addressOf, iconOf } from '$lib/shell/address.ts';
	import { places } from '$lib/shell/places.svelte.ts';
	import ReviewList from '$lib/components/page/review-list.svelte';
	import PagePublish from '$lib/components/shell/page-publish.svelte';

	/**
	 * Home (the deck editor's home.rs; research/redesign/workspaces2.html ?w=home): continue where this browser was, what waits for you (reviews, drafts not live yet), the areas, and the toons – a
	 * toon opens its board (seasons › chapters), a chapter there opens its own workspace.
	 */
	const nav = getCreatorNav();
	$effect(() => places.load(nav.toons));

	let pending = $state<ContentIndexEntity[]>([]);
	let loadedFor = '';
	async function loadPending() {
		try {
			const list = await pendingFiles({ kind: 'all', toons: nav.toons.map((t) => ({ id: t.id, name: t.id })) });
			pending = list.sort((a, b) => (b.latest_at ?? '').localeCompare(a.latest_at ?? ''));
		} catch {
			pending = [];
		}
	}
	$effect(() => {
		const ids = nav.toons.map((t) => t.id).join(',');
		if (!ids || ids === loadedFor) return;
		loadedFor = ids;
		void loadPending();
	});
	let publishing = $state(false);
	async function publishEverything() {
		publishing = true;
		try {
			const list = nav.toons.map((t) => ({ id: t.id, name: toonName(t, getLocale()) }));
			if (await publishAndLog({ kind: 'all', toons: list }, m['creator.publish_tree_title_all']())) void loadPending();
		} finally {
			publishing = false;
		}
	}

	const SHOWN = 8;
	const avatarOf = (id: string | null | undefined) => {
		const t = id ? nav.toons.find((x) => x.id === id || x.uri === id) : undefined;
		return t?.avatar ? asset(toonAsset(t.id, t.avatar)) : null;
	};
	const when = (value: string | number | undefined) => {
		const d = value === undefined ? null : new Date(value);
		return d && !Number.isNaN(d.getTime()) ? new Intl.DateTimeFormat(getLocale(), { dateStyle: 'medium', timeStyle: 'short' }).format(d) : '';
	};
	const recentAddress = (href: string) => {
		const [path, search] = href.split(/(?=\?)/);
		return addressOf(path, search ?? '') ?? '';
	};
	const title = (e: ContentIndexEntity) => (e.toon ? `${nav.toonName(e.toon)} · ${fileLabel(e.key)}` : fileLabel(e.key));
	const waiting = $derived((reviews.overview?.to_review?.length ?? 0) + (reviews.overview?.mine ?? []).filter((r) => r.state === 'pending' || r.state === 'returned').length);
	const areas = $derived(places.areas().filter((p) => p.group === 'area' || p.address === 'catalog:items').map((p) => (p.address === 'catalog:items' ? { ...p, label: m['creator.catalog']() } : p)));
</script>

{#snippet topPublish()}
	<!-- the bar's Publish on the dashboard is "publish everything" (nothing is open here to publish on its own) -->
	<Button variant="primary" icon="upload-cloud" disabled={publishing || !nav.toons.length} onclick={publishEverything} data-testid="bar-publish-everything">{m['creator.publish']()} …</Button>
{/snippet}
<PagePublish publish={topPublish} unpublished={pending.length > 0} />

<main class="home" data-testid="dashboard">
	<div class="col main">
		<Card title={m['shell.areas']()}>
			<div class="areas">
				{#each areas as a (a.address)}
					<Tile icon={a.icon} label={a.label} href={a.href} data-testid="home-area" />
				{/each}
			</div>
		</Card>
		{#if recent.list.length}
			<Card title={m['creator.dash_continue']()} data-testid="dash-continue">
				<div class="recent">
					{#each recent.list.slice(0, SHOWN) as r (r.href)}
						{@const address = recentAddress(r.href)}
						<Tile label={r.trail.join(' · ')} sub={address} meta={when(r.at)} href={r.href} data-testid="dash-open">
							{#snippet picture()}<ToonAvatar src={avatarOf(r.toon)} name={r.trail[0] ?? ''} size="2rem" /><Icon name={iconOf(address)} size="1rem" />{/snippet}
						</Tile>
					{/each}
				</div>
			</Card>
		{/if}
	</div>
	<div class="col side">
		<Card title={m['creator.dash_waiting']()} count={waiting} data-testid="dash-waiting">
			{#if waiting}<ReviewList />{/if}
			{#if pending.length}
				<h2 class="head" data-testid="dash-not-live">{m['creator.dash_not_live']()} <span class="n">{pending.length}</span></h2>
				<div class="rows">
					{#each pending.slice(0, SHOWN) as e (e.key)}
						<ListRow icon={iconOf(e.key)} label={title(e)} meta={when(e.latest_at)} href={placeHref(e.key)}><Badge tone="draft">{m['creator.status_draft']()}</Badge></ListRow>
					{/each}
				</div>
				{#if pending.length > SHOWN}<p class="meta">{m['creator.dash_more']({ count: pending.length - SHOWN })}</p>{/if}
			{:else if !waiting}
				<p class="meta">{m['shell.nothing_waiting']()}</p>
			{/if}
			<span class="actions"><Button icon="upload-cloud" disabled={publishing || !nav.toons.length} onclick={publishEverything} data-testid="publish-everything">{m['creator.publish']()} …</Button></span>
		</Card>
		<Card title={m['creator.toons']()} data-testid="home-toons">
			<div class="rows">
				{#each nav.toons as t (t.id)}
					<ListRow label={nav.toonName(t.id)} meta="toon:{t.id}" href="/{t.id}" data-testid="home-toon">
						{#snippet lead()}<ToonAvatar src={avatarOf(t.id)} name={nav.toonName(t.id)} size="2rem" />{/snippet}
					</ListRow>
				{/each}
			</div>
		</Card>
	</div>
</main>

<style>
	.home {
		display: grid;
		grid-template-columns: minmax(0, 1.6fr) minmax(18rem, 1fr);
		align-items: start;
		gap: var(--editor-gap);
		max-width: 58rem;
		margin: 0 auto 0 max(0rem, calc((100% - 88rem) / 2));
		padding: calc(var(--editor-pad) / 2) var(--editor-gap) var(--editor-gap);
		color: var(--tool-text);
	}
	.col {
		display: flex;
		flex-direction: column;
		gap: var(--editor-gap);
		min-width: 0;
	}
	.areas {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(9rem, 1fr));
		gap: var(--kit-gap);
	}
	.recent {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(12rem, 1fr));
		gap: var(--kit-gap);
	}
	.rows {
		display: flex;
		flex-direction: column;
	}
	/* a second title in a card (its first is the Card's) */
	.head {
		display: flex;
		align-items: center;
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
	.n {
		color: var(--tool-warn);
	}
	.meta {
		margin: 0;
		padding: 0 calc(var(--editor-pad) / 2);
		color: var(--tool-muted);
		font-size: var(--editor-small);
	}
	.actions {
		display: flex;
		padding-top: calc(var(--editor-pad) / 2);
	}
</style>
