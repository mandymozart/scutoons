<script lang="ts">
	import { Icon } from '@scutoons/ui';
	import {
		characterIndex,
		findPersona,
		lookLabel,
		MAP_PLACE_REQUEST_PREFIX,
		mapMediaRequests,
		mediaRequests,
		mediaRequestsText,
		moodLabel,
		wantedImages,
		wantedImagesCsv,
		wantedImagesText,
		type AssetRequests,
		type CharacterRegistry,
		type MoodVocabulary,
		type WorldMap
	} from '@scutoons/content';
	import { shownCharacterName } from '$lib/editor/text-language.svelte.ts';
	import { m } from '$lib/paraglide/messages.js';

	/**
	 * "Wanted images": every missing portrait of the standard set (+ open asset requests), grouped by character / persona /
	 * look with the convention file name – copy as plain text or CSV to brief the image supplier. Interim until the CDN
	 * upload exists. Below them the open media requests (`view: 'media'`: covers, key visuals … of a toon, no character),
	 * unless the list is for one character – including every place of the world map without an illustration ("Map places",
	 * derived from the map each time, never stored: `mapMediaRequests`).
	 */
	let {
		registry,
		moods,
		requests,
		locale,
		character,
		map = null,
		onopen
	}: {
		registry: CharacterRegistry | null;
		moods: MoodVocabulary;
		requests: AssetRequests | null;
		locale: string;
		/** only this character */
		character?: string;
		/** the world map: its places without an image are wanted media too */
		map?: WorldMap | null;
		/** open a character on the Characters page */
		onopen?: (id: string, persona: string, look: string) => void;
	} = $props();

	let requestedOnly = $state(false);
	let copied = $state('');
	let fallbackText = $state('');

	const index = $derived(characterIndex(registry));
	const mapPlaces = $derived(character ? [] : mapMediaRequests(map, locale));
	const media = $derived(character ? [] : [...mediaRequests(requests), ...mapPlaces]);
	const storedMedia = $derived(media.filter((r) => !r.id.startsWith(MAP_PLACE_REQUEST_PREFIX)));
	const wanted = $derived(wantedImages(registry, { moods, requests, character }).filter((w) => !requestedOnly || w.request));
	const groups = $derived.by(() => {
		const out: { key: string; character: string; persona: string; look: string; items: typeof wanted }[] = [];
		for (const w of wanted) {
			const key = `${w.character}/${w.persona}/${w.look}`;
			let group = out.find((g) => g.key === key);
			if (!group) out.push((group = { key, character: w.character, persona: w.persona, look: w.look, items: [] }));
			group.items.push(w);
		}
		return out;
	});

	const csvCell = (v: string) => (/[",\n]/.test(v) ? `"${v.replace(/"/g, '""')}"` : v);

	async function copy(kind: 'text' | 'csv') {
		const text =
			kind === 'csv'
				? [wantedImagesCsv(wanted), ...media.map((r) => ['', '', '', '', 'media', r.path.split('/').pop() ?? '', r.path, [r.size, r.note].filter(Boolean).join(' – ')].map(csvCell).join(','))].join('\n')
				: [wantedImagesText(wanted), mediaRequestsText(media)].filter(Boolean).join('\n\n');
		try {
			await navigator.clipboard.writeText(text);
			copied = kind;
			fallbackText = '';
			setTimeout(() => (copied = ''), 2000);
		} catch {
			// no clipboard permission: show the text to copy by hand
			fallbackText = text;
		}
	}
</script>

<section class="wanted" data-testid="wanted-images">
	<header>
		<span>{m['creator.characters_wanted_count']({ count: wanted.length + media.length })}</span>
		<label class="check"><input type="checkbox" bind:checked={requestedOnly} /> {m['creator.characters_requested_only']()}</label>
		<span class="actions">
			<button type="button" class="secondary" onclick={() => copy('text')} disabled={!wanted.length && !media.length} data-testid="copy-wanted-text">{copied === 'text' ? '✓' : ''} {m['creator.characters_copy_list']()}</button>
			<button type="button" class="secondary" onclick={() => copy('csv')} disabled={!wanted.length && !media.length} data-testid="copy-wanted-csv">{copied === 'csv' ? '✓' : ''} {m['creator.characters_copy_csv']()}</button>
		</span>
	</header>
	{#if fallbackText}<textarea rows="8" readonly value={fallbackText}></textarea>{/if}
	{#each groups as g (g.key)}
		{@const c = index.get(g.character)}
		<article class="group">
			<h4>
				{#if onopen}
					<button type="button" class="link" onclick={() => onopen?.(g.character, g.persona, g.look)}>{shownCharacterName(c, g.persona)}</button>
				{:else}
					{shownCharacterName(c, g.persona)}
				{/if}
				<span class="hint">· {lookLabel(findPersona(c, g.persona), g.look, locale)} · {m['creator.characters_wanted_count']({ count: g.items.length })}</span>
			</h4>
			<ul>
				{#each g.items as w (w.path)}
					<li>
						<code>{w.path.split('/').pop()}</code>
						<span class="hint">{moodLabel(w.mood, locale, moods)}</span>
						{#if w.request}<span class="sc-mark" title={w.request.requested_by ?? ''}><Icon name="mail" /> {w.request.note || m['creator.characters_requested_short']()}</span>{/if}
					</li>
				{/each}
			</ul>
		</article>
	{/each}
	{#if media.length}
		<!-- toon media without a character (a lettering-free cover …): owner rule 2026-10-01, media are language-independent -->
		<article class="group" data-testid="wanted-media">
			<h4>{m['creator.wanted_media']()} <span class="hint">· {m['creator.characters_wanted_count']({ count: media.length })}</span></h4>
			{#if storedMedia.length}
				<ul class="media-list">
					{#each storedMedia as r (r.id)}
						<li>
							<code>{r.path}</code>
							{#if r.size}<span class="hint">{r.size}</span>{/if}
							{#if r.note}<span class="sc-mark" title={r.requested_by ?? ''}><Icon name="mail" /> {r.note}</span>{/if}
						</li>
					{/each}
				</ul>
			{/if}
			{#if mapPlaces.length}
				<!-- the world map's places without an illustration (derived from the map, agents/MAP.md §5) -->
				<h5 data-testid="wanted-map-places">{m['creator.wanted_map_places']()} <span class="hint">· {mapPlaces.length}</span></h5>
				<ul class="media-list">
					{#each mapPlaces as r (r.id)}
						{@const place = r.id.slice(MAP_PLACE_REQUEST_PREFIX.length)}
						<li data-testid="wanted-map-{place}">
							<code>{r.path}</code>
							{#if r.size}<span class="hint">{r.size}</span>{/if}
							<span>{r.note} · <a href="/map?location={encodeURIComponent(place)}">{m['creator.map']()}</a></span>
							{#if r.brief}<details><summary>{m['creator.wanted_map_prompt']()}</summary><pre>{r.brief}</pre></details>{/if}
						</li>
					{/each}
				</ul>
			{/if}
		</article>
	{/if}
</section>

<style>
	.wanted {
		display: flex;
		flex-direction: column;
		gap: 0.6rem;
	}
	header {
		display: flex;
		flex-wrap: wrap;
		gap: 0.75rem;
		align-items: center;
	}
	.actions {
		display: flex;
		gap: 0.4rem;
		margin-left: auto;
	}
	.check {
		display: flex;
		gap: 0.25rem;
		align-items: center;
		font-size: var(--type-caption-size);
		line-height: var(--type-caption-line);
	}
	.group {
		border-top: 1px solid var(--border-subtle);
		padding-top: 0.4rem;
	}
	h4 {
		margin: 0 0 0.3rem;
	}
	ul {
		list-style: none;
		margin: 0;
		padding: 0;
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(22rem, 1fr));
		gap: 0.15rem 1rem;
	}
	h5 {
		margin: 0.6rem 0 0.25rem;
	}
	details {
		flex-basis: 100%;
	}
	pre {
		white-space: pre-wrap;
		font-size: var(--type-caption-size);
		margin: 0.25rem 0 0;
		padding: 0.4rem;
		background: var(--surface-panel);
	}
	.media-list {
		grid-template-columns: minmax(0, 1fr);
	}
	li {
		display: flex;
		flex-wrap: wrap;
		gap: 0.5rem;
		align-items: baseline;
	}
	.link {
		all: unset;
		display: inline-flex;
		align-items: center;
		min-height: var(--control-height);
		cursor: pointer;
		text-decoration: underline;
	}
	textarea {
		width: 100%;
	}
</style>
