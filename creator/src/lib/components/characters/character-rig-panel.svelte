<script lang="ts">
	import { Section } from '@scutoons/ui/editor';
	import { FileSlot } from '@scutoons/media';
	import type { CharacterLook, CharacterPersona, RegistryCharacter } from '@scutoons/content';
	import type { ContentDoc } from '$lib/editor/doc.svelte.ts';
	import type { CharacterModel } from '$lib/editor/character-model.svelte.ts';
	import type { LookEdits } from '$lib/editor/look-edit.ts';
	import { m } from '$lib/paraglide/messages.js';
	import PersonaModel from '$lib/components/characters/persona-model.svelte';
	import AposeEditor from '$lib/components/characters/apose-editor.svelte';

	/**
	 * The Rig tab: the look's and the persona's 3D slots and the look's A-pose, then the spots the core read from the
	 * model. Its bones and clips are the deck's asset view (not twice).
	 */
	let {
		registry,
		character,
		persona,
		look,
		edits,
		model
	}: {
		registry: ContentDoc;
		character: RegistryCharacter;
		persona: CharacterPersona;
		look: CharacterLook;
		edits: LookEdits;
		model: CharacterModel;
	} = $props();

	const facts = $derived(model.facts);
</script>

<div class="rig" data-testid="character-rig">
	<Section id="character-rig-slots" label={m['character.rig_model']()} summary={look.model ? look.model.split('/').at(-1) : ''}>
		<FileSlot kind="model" value={look.model} label={m['creator.characters_look_model']()} place="characters/{character.id}/" player={false} onchange={(p) => edits.setModel(p)} testid="look-model" />
		<PersonaModel {registry} {character} {persona} lookId={look.id} {model} />
		{#key `${persona.id}/${look.id}`}<AposeEditor character={character.id} persona={persona.id} {look} onchange={(p) => edits.setApose(p)} />{/key}
	</Section>
	{#if !model.path}
		<p class="sc-empty">{m['character.rig_none']()}</p>
	{:else if model.why}
		<p class="sc-empty">{model.why}</p>
	{:else if facts?.spots.length}
		<Section id="character-rig-spots" label={m['character.rig_spots']()} summary={String(facts.spots.length)}>
			<ul class="rows">
				{#each facts.spots as s (s.name)}<li>spot:{s.name}<span class="n">{s.kind}</span></li>{/each}
			</ul>
		</Section>
	{/if}
</div>

<style>
	.rig {
		display: flex;
		flex-direction: column;
		min-width: 0;
		padding-bottom: var(--editor-pad);
	}
	.rig > .sc-empty {
		margin: 0;
		padding: 0 var(--editor-pad);
	}
	.rows {
		margin: 0;
		padding: 0;
		list-style: none;
		display: flex;
		flex-direction: column;
		gap: var(--editor-edge);
		font-size: var(--editor-small);
	}
	.rows li {
		display: flex;
		align-items: center;
		min-height: var(--editor-tree-row);
		padding: 0 0.5rem;
	}
	.n {
		margin-left: auto;
		color: var(--tool-muted);
		font-size: var(--editor-label);
	}
</style>
