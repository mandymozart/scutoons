<script lang="ts">
	import { onMount } from 'svelte';
	import { ActionBar, Badge, PhoneBar, ToonAvatar, type PhoneBarItem } from '@scutoons/ui';
	import { statusWord } from '$lib/editor/status-word.ts';
	import { VIEWER_URL } from '$lib/config.ts';
	import { library } from '$lib/api.ts';
	import { getCreatorSession } from '$lib/session.svelte.ts';
	import { toonName, type ToonIndex } from '@scutoons/content';
	import { getLocale } from '$lib/paraglide/runtime.js';
	import { m } from '$lib/paraglide/messages.js';
	import { asset } from '$lib/utils/cdn.ts';
	import ReviewList from '$lib/components/page/review-list.svelte';
	import { publishTree } from '$lib/editor/publish-tree.ts';
	import { toast } from '$lib/toasts.ts';

	const session = getCreatorSession();
	let toons = $state<ToonIndex[]>([]);
	let error = $state<string | null>(null);

	onMount(async () => {
		try {
			// all toons incl. drafts (newest index entries)
			toons = await library.fetchToons({ draft: true });
		} catch (e) {
			error = String(e);
		}
	});

	const name = (toon: ToonIndex) => toonName(toon, getLocale());

	// the top of the tree (BACKLOG 10.60, owner 2026-10-11): every draft of every toon and of the world as one list to tick
	let publishing = $state(false);
	async function publishEverything() {
		publishing = true;
		try {
			const result = await publishTree({ kind: 'all', toons: toons.map((t) => ({ id: t.id, name: name(t) })) }, m['creator.publish_tree_title_all']());
			if (result) toast(result.message, { type: result.published.length ? 'success' : 'info' });
		} catch (e) {
			toast(e instanceof Error ? e.message : String(e), { type: 'error' });
		} finally {
			publishing = false;
		}
	}
	// the bar (UI-CONCEPTS E.10, the editors' one bar – every width since 2026-10-12): Publish everything · the reader – icon + a small word
	const phoneItems = $derived<PhoneBarItem[]>([
		{ id: 'publish', icon: 'upload-cloud', label: m['creator.publish'](), disabled: publishing || !toons.length, run: publishEverything, testid: 'publish-everything' },
		{ id: 'reader', icon: 'book-open', label: m['creator.reader'](), run: () => window.open(VIEWER_URL, '_blank', 'noopener'), testid: 'open-reader' }
	]);
</script>

<div class="creator-page">
	<main class="browser">
		<ReviewList />
		<!-- owner 2026-10-12, approved from the draft: "mehr Konzepte aus der Mobile in die Desktop-Variante", "Platz sparen": no "Toons"
		     heading (the sidebar / drawer says it), one compact row per toon – avatar · name · state – at every width -->
		{#if error}<p class="error">{error}</p>{/if}
		<ul class="toons" aria-label={m['creator.toons']()}>
			{#each toons as toon (toon.id)}
				<li>
					<a href="/{toon.id}" data-testid="toon-{toon.id}">
						<ToonAvatar src={toon.avatar ? asset(`${toon.id}/${toon.avatar}`) : null} name={name(toon)} size="2.5rem" />
						<span class="name">{name(toon)}</span>
						<!-- owner 2026-10-02: no id, the state as a filled label -->
						<Badge tone={toon.status === 'published' ? 'published' : toon.status === 'coming-soon' ? 'soon' : 'draft'}>{statusWord(toon.status)}</Badge>
					</a>
				</li>
			{/each}
		</ul>
	</main>
	<!-- Bottom action bar: open the reader (no toon count, owner 2026-10-02; sign out: the search palette) -->
	<ActionBar label={m['creator.page_actions']()} moreLabel={m['creator.more']()}>
		{#snippet phone()}
			<PhoneBar items={phoneItems} label={m['creator.bar_actions']()} backLabel={m['creator.bar_back']()} />
		{/snippet}
	</ActionBar>
</div>

<style>
	.browser {
		padding: 1.5rem 1rem;
		max-width: 75rem;
	}
	/* compact rows (owner 2026-10-12): two or more columns when there is room, one on a phone; rows split by 1px ink like
	   the toon page's chapter list */
	.toons {
		list-style: none;
		padding: 0;
		margin: 0;
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(min(18rem, 100%), 1fr));
		column-gap: 1.5rem;
		border-top: var(--border-width-regular) solid var(--ink);
	}
	.toons li {
		border-bottom: var(--border-width-hairline) solid var(--ink);
	}
	.toons a {
		display: flex;
		align-items: center;
		gap: var(--space-075);
		min-height: var(--control-height);
		padding: var(--space-05) 0;
		color: var(--ink);
		text-decoration: none;
	}
	.toons a:hover .name {
		text-decoration: underline;
	}
	.name {
		flex: 1;
		min-width: 0;
		overflow-wrap: anywhere;
	}
	.error {
		color: var(--error);
	}
</style>
