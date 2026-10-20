<script lang="ts">
	import { Icon, region } from '@scutoons/ui';
	import { characterImage, personaIds, type RegistryCharacter } from '@scutoons/content';
	import { isKnownMissing, type ToonInfo } from '$lib/editor/characters.ts';
	import { shownCharacterName } from '$lib/editor/text-language.svelte.ts';
	import { asset } from '$lib/utils/cdn.ts';
	import { m } from '$lib/paraglide/messages.js';

	/**
	 * The Characters area's overview as cards (research/CHARACTER-PAGE.md E – owner 2026-10-20: "in b find ich die
	 * Kartenansicht gut"): the neutral avatar big, the name, the toons it is in, the personas' faces, "missing images".
	 * Search, a toon filter and "only with missing images" (a square) above them. A card opens the character.
	 */
	let {
		characters,
		toons,
		toonsOf,
		issues,
		query = $bindable(''),
		toonFilter = $bindable(''),
		missingOnly = $bindable(false),
		onselect
	}: {
		characters: RegistryCharacter[];
		toons: ToonInfo[];
		/** the toons a character is in (its own + the casts), each once */
		toonsOf: (c: RegistryCharacter) => string[];
		issues: (c: RegistryCharacter) => number;
		query?: string;
		toonFilter?: string;
		missingOnly?: boolean;
		onselect: (id: string) => void;
	} = $props();

	const avatar = (c: RegistryCharacter, persona?: string) => characterImage(c, persona ? { persona } : {}, 'avatar', { isMissing: isKnownMissing });
	const toonName = (id: string) => toons.find((t) => t.id === id)?.name ?? id;
</script>

<div class="cards-page">
	<div class="filters">
		<input type="search" bind:value={query} placeholder={m['creator.characters_search']()} aria-label={m['creator.characters_search']()} data-testid="character-search" />
		<select bind:value={toonFilter} aria-label={m['creator.characters_filter_toon']()} data-testid="filter-toon">
			<option value="">{m['creator.characters_all_toons']()}</option>
			{#each toons as t (t.id)}<option value={t.id}>{t.name}</option>{/each}
			<option value="__none">{m['creator.characters_no_toon']()}</option>
		</select>
		<button type="button" class="sq" aria-pressed={missingOnly} onclick={() => (missingOnly = !missingOnly)} title={m['creator.characters_missing_images']()} aria-label={m['creator.characters_missing_images']()} data-testid="filter-missing"><Icon name="alert-triangle" /></button>
	</div>
	<ul class="cards" use:region={'characters'} data-testid="character-list">
		{#each characters as c (c.id)}
			{@const src = avatar(c)}
			{@const personas = personaIds(c)}
			{@const n = issues(c)}
			<li>
				<button type="button" class="card" onclick={() => onselect(c.id)} data-control-exempt="a character card" data-testid="character-{c.id}">
					<span class="pic" style:background={c.color || null}>
						{#if src}<img src={asset(src)} alt="" loading="lazy" />{/if}
						{#if n}<span class="chip warn">{m['creator.characters_missing_images']()}</span>{/if}
					</span>
					<span class="t">
						<span class="name">{shownCharacterName(c)}</span>
						<span class="sub">{toonsOf(c).map(toonName).join(' · ') || m['creator.characters_no_toon']()}</span>
						{#if personas.length > 1}
							<span class="faces">{#each personas as p (p)}{@const f = avatar(c, p)}{#if f}<img src={asset(f)} alt="" loading="lazy" />{/if}{/each}</span>
						{/if}
					</span>
				</button>
			</li>
		{/each}
	</ul>
</div>

<style>
	.cards-page {
		display: flex;
		flex-direction: column;
		gap: var(--space-1);
		padding-top: var(--space-1);
	}
	.filters {
		display: flex;
		gap: var(--space-025);
		align-items: center;
		max-width: 40rem;
	}
	.filters input {
		flex: 1 1 12rem;
		min-width: 0;
	}
	.filters select {
		flex: 0 1 auto;
		min-width: 0;
	}
	.sq {
		all: unset;
		box-sizing: border-box;
		flex: none;
		display: inline-flex;
		align-items: center;
		justify-content: center;
		width: var(--control-height);
		height: var(--control-height);
		cursor: pointer;
	}
	.sq:hover,
	.sq:focus-visible {
		outline: var(--border-width-regular) solid var(--ink);
		outline-offset: calc(-1 * var(--border-width-regular));
	}
	.sq[aria-pressed='true'] {
		background: var(--ink);
		color: var(--paper);
	}
	.cards {
		list-style: none;
		margin: 0;
		padding: 0;
		display: grid;
		gap: var(--space-1);
		grid-template-columns: repeat(auto-fill, minmax(min(11rem, 100%), 1fr));
	}
	@media (max-width: 767px) {
		.cards {
			grid-template-columns: repeat(2, minmax(0, 1fr));
			gap: var(--space-05);
		}
	}
	.card {
		all: unset;
		box-sizing: border-box;
		width: 100%;
		display: flex;
		flex-direction: column;
		border: var(--border-width-hairline) solid var(--border-subtle);
		cursor: pointer;
	}
	.card:hover,
	.card:focus-visible {
		border-color: var(--ink);
	}
	.pic {
		position: relative;
		display: block;
		aspect-ratio: 1;
		background: var(--surface-hover);
		overflow: hidden;
	}
	.pic img {
		width: 100%;
		height: 100%;
		object-fit: cover;
		display: block;
	}
	.chip {
		position: absolute;
		left: 0;
		top: 0;
		padding: 0 var(--space-05);
		font-size: var(--type-caption-size);
		text-transform: uppercase;
		letter-spacing: var(--tracking-label);
	}
	.chip.warn {
		background: var(--state-warning);
		color: var(--ink);
	}
	.t {
		display: flex;
		flex-direction: column;
		gap: var(--space-025);
		padding: var(--space-05);
		min-width: 0;
	}
	.name,
	.sub {
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}
	.sub {
		font-size: var(--type-caption-size);
		color: var(--text-muted);
	}
	.faces {
		display: flex;
	}
	.faces img {
		width: 1.6rem;
		height: 1.6rem;
		object-fit: cover;
		margin-left: -0.4rem;
		border: var(--border-width-regular) solid var(--paper);
		background: var(--surface-hover);
	}
	.faces img:first-child {
		margin-left: 0;
	}
	@media (pointer: coarse) {
		.sq {
			width: max(2.75rem, var(--control-height));
			height: max(2.75rem, var(--control-height));
		}
	}
</style>
