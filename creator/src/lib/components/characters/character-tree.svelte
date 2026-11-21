<script lang="ts">
	import { characterImage, searchCharacters, type RegistryCharacter } from '@scutoons/content';
	import { isKnownMissing } from '$lib/editor/characters.ts';
	import { shownCharacterName } from '$lib/editor/text-language.svelte.ts';
	import { asset } from '$lib/utils/cdn.ts';
	import { m } from '$lib/paraglide/messages.js';

	/** The character workspace's Tree: every character, the open one marked; a search narrows it, a click opens one */
	let {
		characters,
		selected,
		imageIssues,
		onselect
	}: {
		characters: RegistryCharacter[];
		selected: string;
		imageIssues: (c: RegistryCharacter) => unknown[];
		onselect: (id: string) => void;
	} = $props();

	let query = $state('');
	const shown = $derived(query.trim() ? searchCharacters(characters, query, 10000) : characters);
	const face = (c: RegistryCharacter) => characterImage(c, {}, 'avatar', { isMissing: isKnownMissing });
</script>

<div class="tree" data-testid="character-tree">
	<input class="search" type="search" bind:value={query} placeholder={m['creator.characters_search']()} aria-label={m['creator.characters_search']()} />
	<ul>
		{#each shown as c (c.id)}
			{@const issues = imageIssues(c).length}
			<li>
				<button type="button" class="row" class:on={c.id === selected} aria-current={c.id === selected ? 'true' : undefined} onclick={() => onselect(c.id)} data-control-exempt="tree row" data-testid="tree-character-{c.id}">
					<span class="face" style:background={c.color || null}>{#if face(c)}<img src={asset(face(c))} alt="" loading="lazy" />{/if}</span>
					<span class="name">{shownCharacterName(c) || c.id}</span>
					{#if issues}<span class="n" title={m['creator.characters_missing_images']()}>{issues}</span>{/if}
				</button>
			</li>
		{/each}
	</ul>
</div>

<style>
	.tree {
		display: flex;
		flex-direction: column;
		gap: 0.375rem;
		min-height: 0;
		padding: 0 0.5rem var(--editor-pad);
	}
	.search {
		margin: 0 0.25rem;
	}
	ul {
		margin: 0;
		padding: 0;
		list-style: none;
		overflow: auto;
	}
	.row {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		width: 100%;
		min-height: var(--editor-tree-row);
		padding: 0.15rem 0.3rem;
		border: 0;
		border-radius: var(--editor-radius);
		background: none;
		color: var(--editor-text);
		font: inherit;
		text-align: start;
		cursor: pointer;
	}
	.row:hover {
		background: var(--editor-hover);
	}
	.row.on {
		background: var(--editor-selected, var(--editor-pressed));
	}
	.face {
		flex: none;
		width: 1.6rem;
		height: 1.6rem;
		border-radius: var(--editor-radius);
		overflow: hidden;
		background: var(--editor-control);
	}
	.face img {
		width: 100%;
		height: 100%;
		object-fit: cover;
		display: block;
	}
	.name {
		flex: 1;
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.n {
		color: var(--editor-warn);
		font-size: var(--editor-label);
		font-variant-numeric: tabular-nums;
	}
</style>
