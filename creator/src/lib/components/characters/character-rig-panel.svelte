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
	 * The Rig tab: the look's and the persona's 3D slots and the look's A-pose, then what the core read from the model –
	 * its bones as a tree, its clips (a press plays one on the 3D place) and its spots.
	 */
	let {
		registry,
		character,
		persona,
		look,
		edits,
		bust = 0,
		model,
		onplay
	}: {
		registry: ContentDoc;
		character: RegistryCharacter;
		persona: CharacterPersona;
		look: CharacterLook;
		edits: LookEdits;
		bust?: number;
		model: CharacterModel;
		/** a clip pressed: the 3D place shows it */
		onplay: (clip: number) => void;
	} = $props();

	const facts = $derived(model.facts);
	/** the bones in tree order, each with its depth */
	const tree = $derived.by(() => {
		const bones = facts?.bones ?? [];
		const out: { name: string; depth: number }[] = [];
		const walk = (parent: string | null, depth: number) => {
			for (const b of bones) if (b.parent === parent) (out.push({ name: b.name, depth }), walk(b.name, depth + 1));
		};
		walk(null, 0);
		return out;
	});
</script>

<div class="rig" data-testid="character-rig">
	<Section id="character-rig-slots" label={m['character.rig_model']()} summary={look.model ? look.model.split('/').at(-1) : ''}>
		<FileSlot kind="model" value={look.model} label={m['creator.characters_look_model']()} place="characters/{character.id}/" player={false} onchange={(p) => edits.setModel(p)} testid="look-model" />
		<PersonaModel {registry} {character} {persona} lookId={look.id} />
		{#key `${persona.id}/${look.id}`}<AposeEditor character={character.id} persona={persona.id} {look} {bust} onchange={(p) => edits.setApose(p)} />{/key}
	</Section>
	{#if !model.path}
		<p class="sc-empty">{m['character.rig_none']()}</p>
	{:else if model.why}
		<p class="sc-empty">{model.why}</p>
	{:else if facts}
		<Section id="character-rig-bones" label={m['character.rig_bones']()} summary={String(facts.bones.length)}>
			<ul class="rows" data-testid="character-rig-bones">
				{#each tree as b (b.name)}<li style:padding-left="{0.5 + b.depth * 0.75}rem">{b.name}</li>{/each}
			</ul>
		</Section>
		{#if facts.clips.length}
			<Section id="character-rig-clips" label={m['character.rig_clips']()} summary={String(facts.clips.length)}>
				<ul class="rows">
					{#each facts.clips as c, i (i)}
						<li><button type="button" class="row" data-control-exempt="list row" class:on={model.clip === i} onclick={() => onplay(i)}>{c.name || i + 1}<span class="n">{c.seconds.toFixed(1)} s</span></button></li>
					{/each}
				</ul>
			</Section>
		{/if}
		{#if facts.spots.length}
			<Section id="character-rig-spots" label={m['character.rig_spots']()} summary={String(facts.spots.length)}>
				<ul class="rows">
					{#each facts.spots as s (s.name)}<li>spot:{s.name}<span class="n">{s.kind}</span></li>{/each}
				</ul>
			</Section>
		{/if}
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
	.rows .row {
		all: unset;
		box-sizing: border-box;
		display: flex;
		align-items: center;
		flex: 1;
		min-height: var(--editor-tree-row);
		margin: 0 -0.5rem;
		padding: 0 0.5rem;
		border-radius: var(--editor-radius);
		cursor: pointer;
	}
	.rows .row:hover {
		background: var(--tool-hover);
	}
	.rows .row.on {
		background: var(--tool-selected);
	}
	.n {
		margin-left: auto;
		color: var(--tool-muted);
		font-size: var(--editor-label);
	}
</style>
