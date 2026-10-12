<script lang="ts">
	// Review before publish on the dashboard (agents/CUTOVER.md 10.28): "To review" – the open submits of the toons this
	// account reviews; "Your submissions" – its own, open or sent back. One row per submit (a chapter = its languages +
	// scenes). Shown only when there is something in it (less UI).
	import { askDialog, Badge } from '@scutoons/ui';
	import { entityOf, parseEntityKey } from '@scutoons/content';
	import { catalogDocKey } from '@scutoons/api';
	import type { ContentReview } from '@scutoons/api';
	import { contentApi } from '$lib/api.ts';
	import { reviews } from '$lib/editor/reviews.svelte.ts';
	import { fileLabels } from '$lib/labels.ts';
	import { getCreatorNav } from '$lib/nav.svelte.ts';
	import { getLocale } from '$lib/paraglide/runtime.js';
	import { m } from '$lib/paraglide/messages.js';

	const nav = getCreatorNav();
	let busy = $state(false);
	let error = $state<string | null>(null);

	interface Group {
		id: string;
		toon: string;
		items: ContentReview[];
	}

	/** reviews of one submit (same author, time and toon) together */
	function group(list: ContentReview[]): Group[] {
		const out = new Map<string, Group>();
		for (const r of list) {
			const id = `${r.author?.id ?? ''}|${r.created_at}|${r.toon}|${r.state}`;
			if (!out.has(id)) out.set(id, { id, toon: r.toon, items: [] });
			out.get(id)!.items.push(r);
		}
		return [...out.values()];
	}

	const toReview = $derived(group(reviews.overview?.to_review ?? []));
	// own submits: open and sent back (the newest 10), published ones only in the mail
	const mine = $derived(group((reviews.overview?.mine ?? []).filter((r) => r.state === 'pending' || r.state === 'returned')).slice(0, 10));

	const files = (g: Group) => fileLabels([...new Set(g.items.map((r) => entityOf(r.key)))]);
	const locales = (g: Group) => [...new Set(g.items.map((r) => r.locale).filter(Boolean))].sort().join(', ');
	const author = (g: Group) => g.items[0].author?.email ?? '–';
	const newer = (g: Group) => g.items.some((r) => r.newest !== null && r.newest > r.version);
	const when = (iso: string | null) => {
		const d = iso ? new Date(iso) : null;
		return d && !Number.isNaN(d.getTime()) ? new Intl.DateTimeFormat(getLocale(), { dateStyle: 'medium', timeStyle: 'short' }).format(d) : '';
	};

	/** the page of a group's first file (chapter / season / toon entry / cast / credits / script) */
	function href(g: Group): string {
		const ref = parseEntityKey(g.items[0].key);
		const toon = g.toon;
		// a catalog entry (agents/CATALOG-IN-CREATOR.md): its page in the catalog
		const doc = catalogDocKey(g.items[0].key);
		if (doc) return `/catalog/${doc.kind}?id=${encodeURIComponent(doc.id)}`;
		switch (ref?.kind) {
			case 'chapter':
			case 'scene':
				return `/${toon}/chapter/${ref.number}`;
			case 'season':
				return `/${toon}/season/${ref.number}`;
			case 'toon':
				return `/${toon}/toon`;
			case 'cast':
			case 'credits':
			case 'script':
				return `/${toon}/${ref.kind}`;
			default:
				return `/${toon}`;
		}
	}

	async function decide(g: Group, action: 'publish' | 'return') {
		const vars = { file: files(g), author: author(g) };
		const note = await askDialog({
			title: action === 'publish' ? m['creator.review_publish_title']() : m['creator.review_return_title'](),
			message: action === 'publish' ? m['creator.review_publish_message'](vars) : m['creator.review_return_message'](vars),
			label: m['creator.review_note_label'](vars),
			confirmLabel: action === 'publish' ? m['creator.review_publish']() : m['creator.review_return']()
		});
		if (note === null) return;
		busy = true;
		error = null;
		try {
			await contentApi.decideReviews(action, g.items.map((r) => r.id), note.trim());
			await reviews.load();
		} catch (e) {
			error = e instanceof Error ? e.message : String(e);
		} finally {
			busy = false;
		}
	}
</script>

{#if toReview.length}
	<section class="reviews" data-testid="to-review">
		<h2>{m['creator.reviews_to_review']()} <Badge tone="draft">{toReview.length}</Badge></h2>
		<ul>
			{#each toReview as g (g.id)}
				<li>
					<div class="what">
						<a href={href(g)}>{nav.toonName(g.toon)} · {files(g)}</a>
						<span class="meta">{author(g)} · {when(g.items[0].created_at)}{locales(g) ? ` · ${locales(g)}` : ''}</span>
						{#if g.items[0].note}<i>„{g.items[0].note}“</i>{/if}
						{#if newer(g)}<span class="meta">{m['creator.review_newer']()}</span>{/if}
					</div>
					<div class="acts">
						<button class="secondary" disabled={busy} onclick={() => decide(g, 'return')}>{m['creator.review_return']()}</button>
						<button disabled={busy} onclick={() => decide(g, 'publish')} data-testid="review-publish">{m['creator.review_publish']()}</button>
					</div>
				</li>
			{/each}
		</ul>
		{#if error}<p class="error">{error}</p>{/if}
	</section>
{/if}

{#if mine.length}
	<section class="reviews" data-testid="my-reviews">
		<h2>{m['creator.reviews_mine']()}</h2>
		<ul>
			{#each mine as g (g.id)}
				<li>
					<div class="what">
						<a href={href(g)}>{nav.toonName(g.toon)} · {files(g)}</a>
						<span class="meta">{when(g.items[0].created_at)}{locales(g) ? ` · ${locales(g)}` : ''}</span>
						{#if g.items[0].state === 'returned' && g.items[0].decision_note}<i>{g.items[0].decided_by?.email}: „{g.items[0].decision_note}“</i>{/if}
					</div>
					<div class="acts">
						<Badge tone={g.items[0].state === 'returned' ? 'soon' : 'draft'}>
							{g.items[0].state === 'returned' ? m['creator.review_state_returned']() : m['creator.review_state_pending']()}
						</Badge>
					</div>
				</li>
			{/each}
		</ul>
	</section>
{/if}

<style>
	.reviews {
		margin-bottom: 2rem;
	}
	h2 {
		display: flex;
		align-items: center;
		gap: 0.5rem;
	}
	ul {
		list-style: none;
		margin: 0;
		padding: 0;
	}
	li {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		justify-content: space-between;
		gap: 0.5rem 1rem;
		padding: 0.75rem 0;
		border-bottom: 2px solid var(--border-muted);
	}
	.what {
		display: flex;
		flex-direction: column;
		gap: 0.15rem;
		min-width: 0;
	}
	.what a {
		color: var(--ink);
	}
	.meta {
		color: var(--muted);
		font-size: var(--type-caption-size);
		line-height: var(--type-caption-line);
	}
	.acts {
		display: flex;
		flex-wrap: wrap;
		gap: 0.5rem;
	}
	.error {
		color: var(--error);
	}
</style>
