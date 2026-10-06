<script lang="ts">
	import { onMount } from 'svelte';
	import { ActionBar, Badge, ToonAvatar } from '@scutoons/ui';
	import { statusWord } from '$lib/editor/status-word.ts';
	import { VIEWER_URL } from '$lib/config.ts';
	import { library } from '$lib/api.ts';
	import { getCreatorSession } from '$lib/session.svelte.ts';
	import { toonName, type ToonIndex } from '@scutoons/content';
	import { getLocale } from '$lib/paraglide/runtime.js';
	import { m } from '$lib/paraglide/messages.js';
	import { asset } from '$lib/utils/cdn.ts';
	import ReviewList from '$lib/components/page/review-list.svelte';

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
</script>

<div class="creator-page">
	<main class="browser">
		<ReviewList />
		<section>
			<h2>{m['creator.toons']()}</h2>
			{#if error}<p class="error">{error}</p>{/if}
			<ul class="toons">
				{#each toons as toon (toon.id)}
					<li>
						<a href="/{toon.id}" data-testid="toon-{toon.id}">
							<ToonAvatar src={toon.avatar ? asset(`${toon.id}/${toon.avatar}`) : null} name={name(toon)} size="4rem" />
							<span>{name(toon)}</span>
							<!-- owner 2026-10-02: no id, the state as a filled label -->
							<span class="meta"><Badge tone={toon.status === 'published' ? 'published' : toon.status === 'coming-soon' ? 'soon' : 'draft'}>{statusWord(toon.status)}</Badge></span>
						</a>
					</li>
				{/each}
			</ul>
		</section>
	</main>
	<!-- Bottom action bar: open the reader (no toon count, owner 2026-10-02; sign out: the search palette) -->
	<ActionBar label={m['creator.page_actions']()} moreLabel={m['creator.more']()}>
		{#snippet end()}
			<a class="btn" href={VIEWER_URL}>{m['creator.open_reader']()}</a>
		{/snippet}
	</ActionBar>
</div>

<style>
	.browser {
		padding: 1.5rem 1rem;
		max-width: 1200px;
	}
	.toons {
		list-style: none;
		padding: 0;
		margin: 0;
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(12rem, 1fr));
		gap: 1rem;
	}
	.toons a {
		display: flex;
		flex-direction: column;
		gap: 0.25rem;
		padding: 0.75rem;
		border: 2px solid var(--border-muted);
		color: var(--ink);
		text-decoration: none;
	}
	.toons a:hover {
		border-color: var(--ink);
	}
	.meta {
		color: var(--muted);
		font-size: var(--type-caption-size);
		line-height: var(--type-caption-line);
	}
	.error {
		color: var(--error);
	}
</style>
