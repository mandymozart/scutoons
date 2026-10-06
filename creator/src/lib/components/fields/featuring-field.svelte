<script lang="ts">
	import { movedAt } from '$lib/editor/move.ts';
	import ReorderButtons from '$lib/components/controls/reorder-buttons.svelte';
	import { CloseIcon, Icon } from '@scutoons/ui';
	import { characterImage, characterIndex, characterName, suggestFeaturing, type CastEntry, type CharacterRegistry } from '@scutoons/content';
	import type { ContentDoc } from '$lib/editor/doc.svelte.ts';
	import { imageExists, isKnownMissing } from '$lib/editor/characters.ts';
	import { asset } from '$lib/utils/cdn.ts';
	import { m } from '$lib/paraglide/messages.js';

	/**
	 * The chapter's `featuring` (who appears in it, language-neutral – the same list in every locale), agents/ITEMS.md
	 * "Cast & featuring": chips in order, add from the season cast first (someone outside the cast is added to the cast,
	 * with the chapter editor's undo notice), "Suggest from the scenes" = the star + everyone the VN scenes use.
	 */
	let {
		doc,
		cast,
		registry,
		star,
		locale,
		sceneGroups,
		addToCast
	}: {
		doc: ContentDoc;
		/** the season's cast */
		cast: CastEntry[];
		registry: CharacterRegistry | null;
		/** the toon's star (own_toon) */
		star: string | null;
		/** names in this language (the toon's source) */
		locale: string;
		/** the chapter's groups with its scene documents in place of their refs (source language) */
		sceneGroups: () => unknown[];
		/** puts a character into the season cast (the editor shows an undo notice) */
		addToCast: (character: string) => void;
	} = $props();

	const index = $derived(characterIndex(registry));
	const featuring = $derived<string[]>(Array.isArray(doc.data[doc.primary]?.featuring) ? doc.data[doc.primary].featuring : []);
	const inCast = (id: string) => cast.some((e) => e.character === id);
	const castOptions = $derived(cast.filter((e) => !featuring.includes(e.character)).map((e) => e.character));
	const otherOptions = $derived((registry?.characters ?? []).map((c) => c.id).filter((id) => !featuring.includes(id) && !inCast(id)));
	const name = (id: string) => characterName(index.get(id), locale) || id;
	const avatar = (id: string) => {
		const c = index.get(id);
		return c ? characterImage(c, {}, 'avatar', { isMissing: isKnownMissing }) : '';
	};

	function set(list: string[]) {
		doc.edit((file) => {
			if (list.length) file.featuring = [...list];
			else delete file.featuring;
		}, 'featuring');
	}
	function add(id: string) {
		if (!id || featuring.includes(id)) return;
		if (!inCast(id)) addToCast(id);
		set([...featuring, id]);
	}
	function remove(id: string) {
		set(featuring.filter((x) => x !== id));
	}
	function move(from: number, to: number) {
		const list = movedAt(featuring, from, to);
		if (list) set(list);
	}
	function suggest() {
		const suggested = suggestFeaturing(sceneGroups(), star);
		// adds, never removes: the author's list (and order) stays, the suggested ones that are missing come after it
		const next = [...featuring, ...suggested.filter((id) => !featuring.includes(id))];
		for (const id of next) if (!inCast(id)) addToCast(id);
		set(next);
	}
</script>

<fieldset class="featuring" data-testid="featuring">
	<legend>{m['creator.featuring']()}</legend>
	<p class="hint">{m['creator.featuring_hint']()}</p>
	<ul class="chips">
		{#each featuring as id, i (id)}
			{@const src = avatar(id)}
			<li class="sc-chip-control has-image" class:is-warning={!inCast(id)} data-testid="featuring-{id}">
				<span class="avatar" style:background={index.get(id)?.color || 'transparent'}>
					{#if src}<img src={asset(src)} alt="" onerror={() => imageExists(src)} />{/if}
				</span>
				<span class="name">{name(id)}</span>
				{#if !inCast(id)}<span class="hint">{m['creator.featuring_not_in_cast']()}</span>{/if}
				<ReorderButtons index={i} count={featuring.length} onmove={move} axis="horizontal" />
				<button type="button" class="ghost" onclick={() => remove(id)} aria-label={m['creator.featuring_remove']({ name: name(id) })} title={m['creator.featuring_remove']({ name: name(id) })}><CloseIcon /></button>
			</li>
		{:else}
			<li class="hint">{m['creator.featuring_empty']()}</li>
		{/each}
	</ul>
	<div class="actions">
		<select
			aria-label={m['creator.featuring_add']()}
			value=""
			onchange={(e) => {
				add(e.currentTarget.value);
				e.currentTarget.value = '';
			}}
			data-testid="featuring-add"
		>
			<option value="">{m['creator.featuring_add']()}</option>
			{#if castOptions.length}
				<optgroup label={m['creator.cast']()}>
					{#each castOptions as id (id)}<option value={id}>{name(id)}</option>{/each}
				</optgroup>
			{/if}
			{#if otherOptions.length}
				<optgroup label={m['creator.characters']()}>
					{#each otherOptions as id (id)}<option value={id}>{name(id)}</option>{/each}
				</optgroup>
			{/if}
		</select>
		<button type="button" class="secondary" onclick={suggest} title={m['creator.featuring_suggest_hint']()} data-testid="featuring-suggest">{m['creator.featuring_suggest']()}</button>
	</div>
</fieldset>

<style>
	.featuring {
		border: 0;
		padding: 0;
		margin: 0;
		display: flex;
		flex-direction: column;
		gap: 0.4rem;
	}
	.featuring legend {
		font-size: var(--type-caption-size);
		text-transform: uppercase;
		letter-spacing: var(--tracking-label);
		color: var(--ink);
		padding: 0;
		margin-bottom: 0.3rem;
	}
	.featuring .hint {
		margin: 0;
	}
	.chips {
		display: flex;
		flex-wrap: wrap;
		gap: 0.4rem;
		margin: 0;
		padding: 0;
		list-style: none;
	}
	.avatar {
		width: 1.6rem;
		height: 1.6rem;
		display: inline-flex;
		overflow: hidden;
		border-radius: var(--radius-sm, 0.5rem);
	}
	.avatar img {
		width: 100%;
		height: 100%;
		object-fit: cover;
	}
	.actions {
		display: flex;
		flex-wrap: wrap;
		gap: 0.5rem;
		align-items: center;
	}
	.actions select {
		width: auto;
	}
</style>
