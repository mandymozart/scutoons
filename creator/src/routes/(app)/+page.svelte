<script lang="ts">
	import { Badge, ToonAvatar } from '@scutoons/ui';
	import type { ContentIndexEntity } from '@scutoons/api';
	import { toonAsset } from '@scutoons/content';
	import { getCreatorNav } from '$lib/nav.svelte.ts';
	import { recent } from '$lib/recent.svelte.ts';
	import { reviews } from '$lib/editor/reviews.svelte.ts';
	import { pendingFiles } from '$lib/editor/publish-tree.ts';
	import { placeHref } from '$lib/editor/place-href.ts';
	import { fileLabel } from '$lib/labels.ts';
	import { asset } from '$lib/utils/cdn.ts';
	import { getLocale } from '$lib/paraglide/runtime.js';
	import { m } from '$lib/paraglide/messages.js';
	import ReviewList from '$lib/components/page/review-list.svelte';
	import ToonList from '$lib/components/page/toon-list.svelte';
	import HomeBar from '$lib/components/page/home-bar.svelte';

	/**
	 * The dashboard (CREATOR-NAV.md §2a – the creator's home): Continue (the last place, from this browser) · Toons ·
	 * Not live yet (every file with a draft newer than what readers see, the newest first – the same list Publish
	 * offers) · Waiting for you (the open reviews). Desktop three columns, a phone one. An empty section shows nothing.
	 */
	const nav = getCreatorNav();

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

	const SHOWN = 12;
	const last = $derived(recent.list[0] ?? null);
	const toonOf = (id: string | null | undefined) => (id ? nav.toons.find((t) => t.id === id || t.uri === id) : undefined);
	const avatarOf = (id: string | null | undefined) => {
		const t = toonOf(id);
		return t?.avatar ? asset(toonAsset(t.id, t.avatar)) : null;
	};
	const when = (value: string | number | undefined) => {
		const d = value === undefined ? null : new Date(value);
		return d && !Number.isNaN(d.getTime()) ? new Intl.DateTimeFormat(getLocale(), { dateStyle: 'medium', timeStyle: 'short' }).format(d) : '';
	};
	const title = (e: ContentIndexEntity) => (e.toon ? `${nav.toonName(e.toon)} · ${fileLabel(e.key)}` : fileLabel(e.key));
	const detail = (e: ContentIndexEntity) =>
		[e.published === null ? m['creator.dash_new']() : m['creator.dash_newer'](), when(e.latest_at), e.latest_author ?? ''].filter(Boolean).join(' · ');
	const waiting = $derived((reviews.overview?.to_review?.length ?? 0) + (reviews.overview?.mine ?? []).filter((r) => r.state === 'pending' || r.state === 'returned').length);
</script>

<div class="creator-page">
	<main class="dash" data-testid="dashboard">
		<div class="col">
			{#if last}
				<section data-testid="dash-continue">
					<h2>{m['creator.dash_continue']()}</h2>
					<div class="continue">
						<ToonAvatar src={avatarOf(last.toon)} name={last.trail[0] ?? ''} size="3rem" />
						<div class="what">
							<span class="ell">{last.trail.slice(0, 2).join(' · ')}</span>
							<span class="ell meta">{[...last.trail.slice(2), when(last.at)].filter(Boolean).join(' · ')}</span>
							<a class="btn" href={last.href} data-testid="dash-open">{m['creator.dash_open']()}</a>
						</div>
					</div>
				</section>
			{/if}
			<section>
				<h2>{m['creator.toons']()}</h2>
				<ToonList toons={nav.toons} />
			</section>
		</div>
		{#if pending.length}
			<section data-testid="dash-not-live">
				<h2>{m['creator.dash_not_live']()} <span class="n">{pending.length}</span></h2>
				<ul class="rows">
					{#each pending.slice(0, SHOWN) as e (e.key)}
						<li>
							<a href={placeHref(e.key)}>
								{#if e.toon}<ToonAvatar src={avatarOf(e.toon)} name={nav.toonName(e.toon)} size="2rem" />{/if}
								<span class="what">
									<span class="ell">{title(e)}</span>
									<span class="ell meta">{detail(e)}</span>
								</span>
								<Badge tone="draft">{m['creator.status_draft']()}</Badge>
							</a>
						</li>
					{/each}
				</ul>
				{#if pending.length > SHOWN}<p class="meta more">{m['creator.dash_more']({ count: pending.length - SHOWN })}</p>{/if}
			</section>
		{/if}
		{#if waiting}
			<section data-testid="dash-waiting">
				<h2>{m['creator.dash_waiting']()} <span class="n">{waiting}</span></h2>
				<ReviewList />
			</section>
		{/if}
	</main>
	<HomeBar toons={nav.toons} onpublished={loadPending} />
</div>

<style>
	/* three columns on the desktop, one on a phone (CREATOR-NAV.md §2a) */
	.dash {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(min(22rem, 100%), 1fr));
		align-items: start;
		gap: var(--space-15) var(--space-3);
		padding: var(--space-15) var(--space-1);
	}
	.dash > * {
		min-width: 0;
	}
	.col {
		display: flex;
		flex-direction: column;
		gap: var(--space-15);
		min-width: 0;
	}
	h2 {
		display: flex;
		align-items: baseline;
		gap: var(--space-05);
		margin: 0 0 var(--space-05);
		font-size: var(--type-caption-size);
		line-height: var(--type-caption-line);
		font-weight: var(--font-weight-regular);
		text-transform: uppercase;
		letter-spacing: var(--tracking-label);
	}
	.n {
		color: var(--muted);
	}
	.continue {
		display: flex;
		gap: var(--space-1);
		padding: var(--space-075);
		border: var(--border-width-regular) solid var(--ink);
	}
	.what {
		display: flex;
		flex: 1;
		flex-direction: column;
		gap: var(--space-025);
		min-width: 0;
	}
	.continue .btn {
		align-self: flex-start;
		margin-top: var(--space-05);
	}
	.ell {
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.meta {
		color: var(--muted);
		font-size: var(--type-caption-size);
		line-height: var(--type-caption-line);
	}
	.rows {
		list-style: none;
		margin: 0;
		padding: 0;
		border-top: var(--border-width-regular) solid var(--ink);
	}
	.rows li {
		border-bottom: var(--border-width-hairline) solid var(--ink);
	}
	.rows a {
		display: flex;
		align-items: center;
		gap: var(--space-075);
		min-height: var(--control-height);
		padding: var(--space-025) 0;
		color: var(--ink);
		text-decoration: none;
	}
	.rows a:hover .what > :first-child {
		text-decoration: underline;
	}
	.more {
		margin: var(--space-05) 0 0;
	}
	/* the review list's own headings sit under "Waiting for you" */
	[data-testid='dash-waiting'] :global(.reviews h2) {
		font-size: var(--type-body-size);
		text-transform: none;
		letter-spacing: normal;
	}
</style>
