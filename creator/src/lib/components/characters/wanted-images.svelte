<script lang="ts">
	import {
		characterIndex,
		characterName,
		findPersona,
		lookLabel,
		moodLabel,
		wantedImages,
		wantedImagesCsv,
		wantedImagesText,
		type AssetRequests,
		type CharacterRegistry,
		type MoodVocabulary
	} from '@scutoons/content';
	import { m } from '$lib/paraglide/messages.js';

	/**
	 * "Wanted images": every missing portrait of the standard set (+ open asset requests), grouped by character / persona /
	 * look with the convention file name – copy as plain text or CSV to brief the image supplier. Interim until the CDN
	 * upload exists.
	 */
	let {
		registry,
		moods,
		requests,
		locale,
		character,
		onopen
	}: {
		registry: CharacterRegistry | null;
		moods: MoodVocabulary;
		requests: AssetRequests | null;
		locale: string;
		/** only this character */
		character?: string;
		/** open a character on the Characters page */
		onopen?: (id: string, persona: string, look: string) => void;
	} = $props();

	let requestedOnly = $state(false);
	let copied = $state('');
	let fallbackText = $state('');

	const index = $derived(characterIndex(registry));
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

	async function copy(kind: 'text' | 'csv') {
		const text = kind === 'csv' ? wantedImagesCsv(wanted) : wantedImagesText(wanted);
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
		<strong>{m['creator.characters_wanted_count']({ count: wanted.length })}</strong>
		<label class="check"><input type="checkbox" bind:checked={requestedOnly} /> {m['creator.characters_requested_only']()}</label>
		<span class="actions">
			<button type="button" class="secondary" onclick={() => copy('text')} disabled={!wanted.length} data-testid="copy-wanted-text">{copied === 'text' ? '✓' : ''} {m['creator.characters_copy_list']()}</button>
			<button type="button" class="secondary" onclick={() => copy('csv')} disabled={!wanted.length} data-testid="copy-wanted-csv">{copied === 'csv' ? '✓' : ''} {m['creator.characters_copy_csv']()}</button>
		</span>
	</header>
	{#if fallbackText}<textarea rows="8" readonly value={fallbackText}></textarea>{/if}
	{#each groups as g (g.key)}
		{@const c = index.get(g.character)}
		<article class="group">
			<h4>
				{#if onopen}
					<button type="button" class="link" onclick={() => onopen?.(g.character, g.persona, g.look)}>{characterName(c, locale, g.persona)}</button>
				{:else}
					{characterName(c, locale, g.persona)}
				{/if}
				<span class="hint">· {lookLabel(findPersona(c, g.persona), g.look, locale)} · {m['creator.characters_wanted_count']({ count: g.items.length })}</span>
			</h4>
			<ul>
				{#each g.items as w (w.path)}
					<li>
						<code>{w.path.split('/').pop()}</code>
						<span class="hint">{moodLabel(w.mood, locale, moods)}</span>
						{#if w.request}<span class="req" title={w.request.requested_by ?? ''}>✉ {w.request.note || m['creator.characters_requested_short']()}</span>{/if}
					</li>
				{/each}
			</ul>
		</article>
	{:else}
		<p class="hint">{m['creator.characters_nothing_wanted']()}</p>
	{/each}
</section>

<style>
	.wanted {
		display: flex;
		flex-direction: column;
		gap: 0.6rem;
		font-size: 12px;
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
		font-size: 11px;
	}
	.group {
		border-top: 1px solid var(--border-subtle);
		padding-top: 0.4rem;
	}
	h4 {
		margin: 0 0 0.3rem;
		font-size: 12px;
	}
	ul {
		list-style: none;
		margin: 0;
		padding: 0;
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(22rem, 1fr));
		gap: 0.15rem 1rem;
	}
	li {
		display: flex;
		gap: 0.5rem;
		align-items: baseline;
	}
	.link {
		all: unset;
		cursor: pointer;
		text-decoration: underline;
	}
	.req {
		background: var(--primary-cyan);
		padding: 0 0.25rem;
		font-size: 10px;
	}
	textarea {
		width: 100%;
		font-size: 11px;
	}
</style>
