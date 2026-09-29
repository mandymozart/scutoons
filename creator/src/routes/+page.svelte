<script lang="ts">
	import { onMount } from 'svelte';
	import { ActionBar } from '@scutoons/ui';
	import { STATIC_API, VIEWER_URL } from '$lib/config.ts';
	import { getCreatorSession } from '$lib/session.svelte.ts';
	import type { ToonIndex } from '@scutoons/content';
	import { getLocale } from '$lib/paraglide/runtime.js';
	import { m } from '$lib/paraglide/messages.js';
	import { asset } from '$lib/utils/cdn.ts';
	import StyleGuide from '$lib/components/style-guide.svelte';

	const session = getCreatorSession();
	let toons = $state<ToonIndex[]>([]);
	let error = $state<string | null>(null);

	onMount(async () => {
		try {
			// all toons incl. drafts (the root layout only keeps published ones)
			const response = await fetch(`${STATIC_API}/toons.json`);
			toons = await response.json();
		} catch (e) {
			error = String(e);
		}
	});

	const name = (toon: ToonIndex) => toon.i18n?.[getLocale()]?.name ?? toon.i18n?.de?.name ?? toon.id;
</script>

<div class="creator-page">
	<header class="creator-top">
		<h1>{m['creator.title']()}</h1>
	</header>
	<main class="browser">
		<section>
			<h2>{m['creator.toons']()}</h2>
			{#if error}<p class="error">{error}</p>{/if}
			<ul class="toons">
				{#each toons as toon (toon.id)}
					<li>
						<a href="/{toon.id}" data-testid="toon-{toon.id}">
							{#if toon.avatar}
								<img src={asset(`${toon.id}/${toon.avatar}`)} alt="" />
							{:else}
								<span class="placeholder"></span>
							{/if}
							<strong>{name(toon)}</strong>
							<span class="meta">{toon.id} · {toon.status}</span>
						</a>
					</li>
				{/each}
			</ul>
		</section>
		<aside>
			<StyleGuide />
		</aside>
	</main>
	<!-- Bottom action bar: toon count · sign out · open the reader -->
	<ActionBar label={m['creator.page_actions']()} moreLabel={m['creator.more']()}>
		{#snippet start()}
			<span class="hint">{m['creator.toons']()}: {toons.length}</span>
		{/snippet}
		{#snippet secondary()}
			<button class="secondary" onclick={() => session.logout()}>{m['creator.sign_out']()}</button>
		{/snippet}
		{#snippet end()}
			<a class="btn" href={VIEWER_URL}>{m['creator.open_reader']()}</a>
		{/snippet}
	</ActionBar>
</div>

<style>
	.creator-page {
		--action-bar-height: 64px;
	}
	.creator-top {
		display: flex;
		align-items: center;
		min-height: 64px;
		padding: 0 1rem 0 96px;
		border-bottom: 2px solid var(--ink);
	}
	.creator-top h1 {
		font-size: 16px;
		margin: 0;
	}
	.browser {
		display: grid;
		grid-template-columns: minmax(0, 1fr) 22rem;
		gap: 2rem;
		padding: 1.5rem 1rem;
		max-width: 1200px;
	}
	@media (max-width: 900px) {
		.browser {
			grid-template-columns: 1fr;
		}
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
	.toons img,
	.placeholder {
		width: 4rem;
		height: 4rem;
		object-fit: cover;
		background: var(--surface-panel);
	}
	.meta {
		color: var(--muted);
		font-size: 11px;
	}
	.error {
		color: var(--error);
	}
</style>
