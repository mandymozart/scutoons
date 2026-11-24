<script lang="ts">
	import { Icon } from '@scutoons/ui';
	import { ListRow } from '@scutoons/ui/editor';
	import { REGULAR_PERSONA, entities, findPersona, pickText, type RegistryCharacter, type ToonCast } from '@scutoons/content';
	import type { ToonInfo } from '$lib/editor/characters.ts';
	import { castIn } from '$lib/editor/season-cast.ts';
	import { textLanguage } from '$lib/editor/text-language.svelte.ts';
	import { fileLabel } from '$lib/labels.ts';
	import { m } from '$lib/paraglide/messages.js';

	/**
	 * Cast in (lap2 C4b): the seasons this character is cast in, each with the personas it appears as there and a link to
	 * that season's cast in the toon – where the cast is edited.
	 */
	let { character, casts, toons }: { character: RegistryCharacter; casts: Record<string, ToonCast | null>; toons: ToonInfo[] } = $props();

	const rows = $derived(castIn(character.id, casts));
	const toonName = (t: string) => toons.find((x) => x.id === t)?.name ?? t;
	const personaLabel = (p: string) => (p === REGULAR_PERSONA ? m['creator.characters_regular']() : pickText(findPersona(character, p)?.label, textLanguage.names, []) || p);
</script>

<div class="cast-in" data-testid="character-cast-in">
	{#if rows.length}
		<span class="label">{m['creator.cast_in']()}</span>
		{#each rows as r (`${r.toon}/${r.season}`)}
			<ListRow
				href="/{r.toon}/season/{r.season}/cast"
				label="{toonName(r.toon)} · {fileLabel(entities.season(r.toon, r.season))}"
				meta={r.personas.map(personaLabel).join(' · ')}
				title={m['creator.cast_in_open']()}
				data-testid="cast-in-{r.toon}-{r.season}"
			>
				<Icon name="chevron-right" size="1rem" />
			</ListRow>
		{/each}
	{:else}
		<p class="sc-empty">{m['creator.cast_in_none']()}</p>
	{/if}
</div>

<style>
	.cast-in {
		display: flex;
		flex-direction: column;
	}
	.label {
		margin: 0.2rem 0 0.5rem;
		color: var(--tool-muted);
		font-size: var(--editor-label);
		letter-spacing: var(--editor-label-spacing);
		text-transform: uppercase;
	}
	.cast-in > :global(.kit-list-row .name) {
		text-decoration: underline;
		text-underline-offset: 0.2em;
	}
	.sc-empty {
		margin: 0;
	}
</style>
