<script lang="ts">
	import { onMount } from 'svelte';
	import { PLACE_IMAGE_SPEC, mapImageRequests, mapImageRequestsText, mapName, type WorldMap } from '@scutoons/content';
	import { getLocale } from '$lib/paraglide/runtime.js';
	import { m } from '$lib/paraglide/messages.js';
	import { mapDoc, mapOf } from '$lib/editor/map.ts';

	/**
	 * "Wanted images" → Map places (owner 2026-10-01): every place of the world map without an illustration, derived from
	 * the map (its newest version) – never a hand-kept list, so it can't drift. Each with the target path
	 * (`<toon>/images/map/places/<id>.webp`), the spec (256 × 256 transparent WebP ≤ 40 KB, ground point 50 / 90 %) and the
	 * brief of agents/MAP.md §5 (for an image generator or a designer). Read-only here; the image is set in the map builder.
	 */
	const doc = mapDoc();
	onMount(() => doc.load());
	const locale = $derived(getLocale());
	const wanted = $derived(mapImageRequests(mapOf(doc) as WorldMap | null));
	let copied = $state(false);

	async function copy() {
		try {
			await navigator.clipboard.writeText(mapImageRequestsText(wanted));
			copied = true;
			setTimeout(() => (copied = false), 1500);
		} catch {
			/* no clipboard (http, permissions): the list is on the page */
		}
	}
</script>

<article class="group" data-testid="wanted-map-places">
	<h4>
		{m['creator.wanted_map_places']()} <span class="hint">· {m['creator.characters_wanted_count']({ count: wanted.length })}</span>
		{#if wanted.length}<button type="button" class="secondary" onclick={copy} data-testid="copy-wanted-map">{copied ? '✓' : ''} {m['creator.characters_copy_list']()}</button>{/if}
	</h4>
	<p class="hint">{m['creator.wanted_map_hint']()} {PLACE_IMAGE_SPEC}.</p>
	{#if doc.loading}
		<p class="hint">{m['creator.loading']()}</p>
	{:else}
		<ul>
			{#each wanted as r (r.place)}
				<li data-testid="wanted-map-{r.place}">
					<span class="name"><b>{mapName(r.name, locale) || r.place}</b> <a href="/map?location={encodeURIComponent(r.place)}">{m['creator.map']()}</a></span>
					<code>{r.path}</code>
					<span class="hint">{r.size}</span>
					<details>
						<summary>{m['creator.wanted_map_prompt']()}</summary>
						<pre>{r.brief}</pre>
					</details>
				</li>
			{/each}
		</ul>
	{/if}
</article>

<style>
	.group {
		margin-top: 1rem;
		border-top: 1px solid var(--border-subtle);
		padding-top: 0.5rem;
	}
	h4 {
		display: flex;
		gap: 0.5rem;
		align-items: center;
		margin: 0 0 0.25rem;
	}
	h4 button {
		margin-left: auto;
	}
	.hint {
		font-size: 11px;
		color: var(--muted);
		margin: 0 0 0.5rem;
	}
	ul {
		list-style: none;
		margin: 0;
		padding: 0;
		display: flex;
		flex-direction: column;
		gap: 0.35rem;
	}
	li {
		display: grid;
		grid-template-columns: minmax(10rem, 16rem) minmax(0, 1fr) auto;
		gap: 0.2rem 0.75rem;
		align-items: baseline;
		font-size: 12px;
	}
	details {
		grid-column: 1 / -1;
	}
	summary {
		cursor: pointer;
		font-size: 11px;
		color: var(--muted);
	}
	pre {
		white-space: pre-wrap;
		font-size: 11px;
		margin: 0.25rem 0 0;
		padding: 0.4rem;
		background: var(--surface-panel);
	}
	code {
		overflow-wrap: anywhere;
	}
</style>
