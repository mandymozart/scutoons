<script lang="ts">
	import { Badge, Icon, ToonAvatar } from '@scutoons/ui';
	import type { ContentIndexEntity } from '@scutoons/api';
	import { toonAsset, toonName } from '@scutoons/content';
	import { getCreatorNav } from '$lib/nav.svelte.ts';
	import { recent } from '$lib/recent.svelte.ts';
	import { reviews } from '$lib/editor/reviews.svelte.ts';
	import { pendingFiles, publishTree } from '$lib/editor/publish-tree.ts';
	import { placeHref } from '$lib/editor/place-href.ts';
	import { fileLabel } from '$lib/labels.ts';
	import { asset } from '$lib/utils/cdn.ts';
	import { toast } from '$lib/toasts.ts';
	import { getLocale } from '$lib/paraglide/runtime.js';
	import { m } from '$lib/paraglide/messages.js';
	import { addressOf, iconOf } from '$lib/shell/address.ts';
	import { places } from '$lib/shell/places.svelte.ts';
	import ReviewList from '$lib/components/page/review-list.svelte';

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
			const result = await publishTree({ kind: 'all', toons: list }, m['creator.publish_tree_title_all']());
			if (result) {
				for (const n of result.notices) toast(n.text, { type: n.type, kind: 'publish', ...(n.detail ? { detail: n.detail } : {}) });
				void loadPending();
			}
		} catch (e) {
			toast(e instanceof Error ? e.message : String(e), { type: 'error', kind: 'publish' });
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

<main class="home" data-testid="dashboard">
	<div class="col main">
		<section class="card">
			<h2>{m['shell.areas']()}</h2>
			<div class="areas">
				{#each areas as a (a.address)}
					<a class="area" href={a.href} data-testid="home-area"><Icon name={a.icon} size="1.3rem" /><span>{a.label}</span></a>
				{/each}
			</div>
		</section>
		{#if recent.list.length}
			<section class="card" data-testid="dash-continue">
				<h2>{m['creator.dash_continue']()}</h2>
				<div class="recent">
					{#each recent.list.slice(0, SHOWN) as r (r.href)}
						{@const address = recentAddress(r.href)}
						<a class="tile" href={r.href} data-testid="dash-open">
							<span class="tile-head"><ToonAvatar src={avatarOf(r.toon)} name={r.trail[0] ?? ''} size="2rem" /><Icon name={iconOf(address)} size="1rem" /></span>
							<span class="ell">{r.trail.join(' · ')}</span>
							<span class="addr ell">{address}</span>
							<span class="meta">{when(r.at)}</span>
						</a>
					{/each}
				</div>
			</section>
		{/if}
	</div>
	<div class="col side">
		<section class="card" data-testid="dash-waiting">
			<h2>{m['creator.dash_waiting']()} {#if waiting}<span class="n">{waiting}</span>{/if}</h2>
			{#if waiting}<ReviewList />{/if}
			{#if pending.length}
				<h2 data-testid="dash-not-live">{m['creator.dash_not_live']()} <span class="n">{pending.length}</span></h2>
				<ul class="rows">
					{#each pending.slice(0, SHOWN) as e (e.key)}
						<li>
							<a href={placeHref(e.key)}>
								<Icon name={iconOf(e.key)} size="1rem" />
								<span class="ell">{title(e)}</span>
								<span class="meta">{when(e.latest_at)}</span>
								<Badge tone="draft">{m['creator.status_draft']()}</Badge>
							</a>
						</li>
					{/each}
				</ul>
				{#if pending.length > SHOWN}<p class="meta">{m['creator.dash_more']({ count: pending.length - SHOWN })}</p>{/if}
			{:else if !waiting}
				<p class="meta">{m['shell.nothing_waiting']()}</p>
			{/if}
			<button type="button" class="pill" disabled={publishing || !nav.toons.length} onclick={publishEverything} data-testid="publish-everything">
				<Icon name="upload-cloud" size="1rem" />{m['creator.publish']()} …
			</button>
		</section>
		<section class="card" data-testid="home-toons">
			<h2>{m['creator.toons']()}</h2>
			<ul class="rows">
				{#each nav.toons as t (t.id)}
					<li>
						<a href="/{t.id}" data-testid="home-toon">
							<ToonAvatar src={avatarOf(t.id)} name={nav.toonName(t.id)} size="2rem" />
							<span class="ell">{nav.toonName(t.id)}</span>
							<span class="addr">toon:{t.id}</span>
						</a>
					</li>
				{/each}
			</ul>
		</section>
	</div>
</main>

<style>
	.home {
		display: grid;
		grid-template-columns: minmax(0, 1.6fr) minmax(18rem, 1fr);
		align-items: start;
		gap: 1rem;
		max-width: 58rem;
		margin: 0 auto 0 max(0rem, calc((100% - 88rem) / 2));
		padding: 0.4rem 1rem 1rem;
		color: var(--editor-text);
	}
	.col {
		display: flex;
		flex-direction: column;
		gap: 1rem;
		min-width: 0;
	}
	.card {
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
		padding: var(--editor-pad) 0.85rem 0.85rem;
		border: var(--editor-edge) solid var(--editor-line);
		border-radius: var(--editor-radius);
		background: var(--editor-surface);
	}
	h2 {
		display: flex;
		align-items: baseline;
		gap: 0.5rem;
		margin: 0.3rem 0 0;
		color: var(--editor-muted);
		font-size: var(--editor-label);
		letter-spacing: var(--editor-label-spacing);
		text-transform: uppercase;
	}
	.n {
		color: var(--editor-warn);
	}
	.areas {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(9rem, 1fr));
		gap: 0.45rem;
	}
	.area,
	.tile {
		display: flex;
		flex-direction: column;
		gap: 0.3rem;
		min-width: 0;
		padding: 0.6rem 0.7rem;
		border: var(--editor-edge) solid var(--editor-line);
		border-radius: var(--editor-radius);
		background: var(--editor-control);
		color: var(--editor-text);
		text-decoration: none;
	}
	.area:hover,
	.tile:hover {
		background: var(--editor-hover);
	}
	.recent {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(12rem, 1fr));
		gap: 0.6rem;
	}
	.tile-head {
		display: flex;
		align-items: center;
		justify-content: space-between;
		color: var(--editor-muted);
	}
	.rows {
		margin: 0;
		padding: 0;
		list-style: none;
	}
	.rows a {
		display: flex;
		align-items: center;
		gap: 0.6rem;
		min-height: var(--editor-row);
		padding: 0.15rem 0.3rem;
		border-radius: var(--editor-radius);
		color: var(--editor-text);
		text-decoration: none;
	}
	.rows a:hover {
		background: var(--editor-hover);
	}
	.ell {
		flex: 1 1 auto;
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.addr {
		flex: none;
		color: var(--editor-muted);
		font-family: var(--font-mono);
		font-size: var(--editor-small);
	}
	.meta {
		margin: 0;
		color: var(--editor-muted);
		font-size: var(--editor-small);
	}
	.pill {
		display: inline-flex;
		align-self: flex-start;
		align-items: center;
		gap: 0.4rem;
		padding: 0 0.85rem;
		border: var(--editor-edge) solid var(--editor-line);
		border-radius: 1.2rem;
		background: var(--editor-control);
		color: var(--editor-text);
	}
</style>
